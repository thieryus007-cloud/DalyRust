//! # daly-bms-server
//!
//! Serveur principal : charge la configuration, ouvre le port série,
//! démarre le polling et expose l'API Axum (REST + WebSocket).
//!
//! ## Démarrage
//! ```bash
//! DALY_CONFIG=/etc/daly-bms/config.toml daly-bms-server
//! # ou en dev :
//! RUST_LOG=debug cargo run --bin daly-bms-server
//! ```

mod config;
mod state;
mod api;
mod bridges;

use crate::bridges::{alerts, influx, mqtt};
use crate::config::AppConfig;
use crate::state::AppState;
use daly_bms_core::bus::{BmsConfig, DalyBusManager, DalyPort};
use daly_bms_core::poll::{poll_loop, PollConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Configuration ──────────────────────────────────────────────────────────
    let config = AppConfig::load_default()
        .or_else(|_| {
            // Fallback : configuration par défaut (utile en dev)
            tracing::warn!("Fichier de config non trouvé — utilisation des valeurs par défaut");
            Ok::<AppConfig, anyhow::Error>(AppConfig {
                serial:    config::SerialConfig::default(),
                api:       config::ApiConfig::default(),
                logging:   config::LoggingConfig::default(),
                mqtt:      config::MqttConfig::default(),
                influxdb:  config::InfluxConfig::default(),
                alerts:    config::AlertsConfig::default(),
                read_only: config::ReadOnlyConfig::default(),
            })
        })?;

    // ── Logging ────────────────────────────────────────────────────────────────
    let log_level = config.logging.level.clone();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level)),
        )
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        port    = %config.serial.port,
        baud    = config.serial.baud,
        api     = %config.api.bind,
        "DalyBMS Server démarrage"
    );

    // ── État partagé ───────────────────────────────────────────────────────────
    let state = AppState::new(config.clone());

    // ── Port série + bus ───────────────────────────────────────────────────────
    let dal_port = DalyPort::open(
        &config.serial.port,
        config.serial.baud,
        500, // timeout ms
    );

    let port = match dal_port {
        Ok(p) => {
            info!("Port série {} ouvert à {} baud", config.serial.port, config.serial.baud);
            state.polling_active.store(true, Ordering::Relaxed);
            Some(p)
        }
        Err(e) => {
            error!("Impossible d'ouvrir {} : {:?} — mode sans port (API seule)", config.serial.port, e);
            None
        }
    };

    // ── Bridges en arrière-plan ─────────────────────────────────────────────────
    let mqtt_cfg     = config.mqtt.clone();
    let influx_cfg   = config.influxdb.clone();
    let alerts_cfg   = config.alerts.clone();
    let state_mqtt   = state.clone();
    let state_influx = state.clone();
    let state_alerts = state.clone();

    tokio::spawn(async move { mqtt::run_mqtt_bridge(state_mqtt, mqtt_cfg).await });
    tokio::spawn(async move { influx::run_influx_bridge(state_influx, influx_cfg).await });
    tokio::spawn(async move { alerts::run_alert_engine(state_alerts, alerts_cfg).await });

    // ── Boucle de polling ──────────────────────────────────────────────────────
    if let Some(port) = port {
        let addresses = config.bms_addresses();
        let devices: Vec<BmsConfig> = addresses
            .iter()
            .map(|&addr| {
                let mut cfg = BmsConfig::new(addr);
                cfg.cell_count      = config.serial.default_cell_count;
                cfg.temp_sensor_count = config.serial.default_temp_sensors;
                cfg
            })
            .collect();

        info!("Polling de {} BMS : {:?}", devices.len(),
              devices.iter().map(|d| format!("{:#04x}", d.address)).collect::<Vec<_>>());

        let manager = Arc::new(DalyBusManager::new(port, devices));
        let poll_cfg = PollConfig {
            interval_ms: config.serial.poll_interval_ms,
            ..Default::default()
        };
        let state_poll = state.clone();

        tokio::spawn(async move {
            poll_loop(manager, poll_cfg, move |snap| {
                let state = state_poll.clone();
                tokio::spawn(async move { state.on_snapshot(snap).await });
            })
            .await;
        });
    }

    // ── Serveur HTTP Axum ──────────────────────────────────────────────────────
    let router = api::build_router(state);
    let addr: SocketAddr = config.api.bind.parse()?;

    info!("API disponible sur http://{}", addr);
    info!("WebSocket      : ws://{}/ws/bms/stream", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
