//! # daly-bms-server
//!
//! Serveur principal : charge la configuration, ouvre le port série (ou simule),
//! démarre le polling et expose l'API Axum (REST + WebSocket).
//!
//! ## Démarrage
//! ```bash
//! # Avec hardware réel :
//! DALY_CONFIG=/etc/daly-bms/config.toml daly-bms-server
//!
//! # Mode simulation (sans Pi ni BMS) :
//! cargo run --bin daly-bms-server -- --simulate
//! cargo run --bin daly-bms-server -- --simulate --sim-bms 0x01,0x02
//! ```

mod autodetect;
mod config;
mod state;
mod api;
mod bridges;
mod simulator;
mod dashboard;

use crate::bridges::{alerts, influx, mqtt};
use crate::config::AppConfig;
use crate::state::AppState;
use daly_bms_core::bus::{BmsConfig, DalyBusManager, DalyPort};
use daly_bms_core::poll::{poll_loop, PollConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tracing::{error, info, warn};

// =============================================================================
// Arguments CLI du serveur
// =============================================================================

#[derive(Debug)]
struct ServerArgs {
    simulate:  bool,
    sim_addrs: Vec<u8>,
    /// Port série explicite (ex: COM3, /dev/ttyUSB0)
    port:      Option<String>,
    /// Adresses BMS pour le mode hardware (ex: 0x01,0x02)
    bms_addrs: Vec<u8>,
}

impl ServerArgs {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let simulate = args.iter().any(|a| a == "--simulate" || a == "-s");

        let sim_addrs = args.windows(2)
            .find(|w| w[0] == "--sim-bms")
            .map(|w| Self::parse_addresses(&w[1]))
            .unwrap_or_default();

        let port = args.windows(2)
            .find(|w| w[0] == "--port" || w[0] == "-p")
            .map(|w| w[1].clone());

        let bms_addrs = args.windows(2)
            .find(|w| w[0] == "--bms")
            .map(|w| Self::parse_addresses(&w[1]))
            .unwrap_or_default();

        Self { simulate, sim_addrs, port, bms_addrs }
    }

    fn parse_addresses(s: &str) -> Vec<u8> {
        s.split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.starts_with("0x") || s.starts_with("0X") {
                    u8::from_str_radix(&s[2..], 16).ok()
                } else {
                    s.parse::<u8>().ok()
                }
            })
            .collect()
    }
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = ServerArgs::parse();

    // ── Configuration ──────────────────────────────────────────────────────────
    let config_from_file;
    let mut config = match AppConfig::load_default() {
        Ok(c) => {
            config_from_file = true;
            c
        }
        Err(e) => {
            eprintln!("Config non trouvée ({}) — utilisation des valeurs par défaut", e);
            config_from_file = false;
            AppConfig {
                serial:    config::SerialConfig::default(),
                api:       config::ApiConfig::default(),
                logging:   config::LoggingConfig::default(),
                mqtt:      config::MqttConfig::default(),
                influxdb:  config::InfluxConfig::default(),
                alerts:    config::AlertsConfig::default(),
                read_only: config::ReadOnlyConfig::default(),
            }
        }
    };

    // ── Override port série depuis CLI ─────────────────────────────────────────
    if let Some(ref port) = args.port {
        config.serial.port = port.clone();
    }
    // Override adresses BMS hardware depuis CLI
    if !args.bms_addrs.is_empty() {
        config.serial.addresses = args.bms_addrs.iter()
            .map(|a| format!("{:#04x}", a))
            .collect();
    }

    // ── Flags d'auto-détection ────────────────────────────────────────────────
    // Actifs uniquement si aucun fichier config ET aucun argument CLI fourni.
    let auto_detect_port    = !args.simulate && args.port.is_none()      && !config_from_file;
    let auto_discover_addrs = !args.simulate && args.bms_addrs.is_empty() && !config_from_file;

    // ── Logging ────────────────────────────────────────────────────────────────
    let log_level = config.logging.level.clone();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level)),
        )
        .init();

    let mode = if args.simulate { "SIMULATION" } else { "HARDWARE" };
    info!(
        version = env!("CARGO_PKG_VERSION"),
        mode,
        api = %config.api.bind,
        "DalyBMS Server démarrage"
    );

    // ── État partagé ───────────────────────────────────────────────────────────
    let state = AppState::new(config.clone());

    // ── Bridges en arrière-plan ─────────────────────────────────────────────────
    tokio::spawn({
        let (s, c) = (state.clone(), config.mqtt.clone());
        async move { mqtt::run_mqtt_bridge(s, c).await }
    });
    tokio::spawn({
        let (s, c) = (state.clone(), config.influxdb.clone());
        async move { influx::run_influx_bridge(s, c).await }
    });
    tokio::spawn({
        let (s, c) = (state.clone(), config.alerts.clone());
        async move { alerts::run_alert_engine(s, c).await }
    });

    // ── Mode SIMULATION ou HARDWARE ────────────────────────────────────────────
    if args.simulate {
        // Adresses depuis --sim-bms, ou depuis config.toml, ou défaut 0x01,0x02
        let addresses = if !args.sim_addrs.is_empty() {
            args.sim_addrs.clone()
        } else {
            let cfg_addrs = config.bms_addresses();
            if !cfg_addrs.is_empty() { cfg_addrs } else { vec![0x01, 0x02] }
        };

        info!(
            "Mode simulation : {} BMS {:?}",
            addresses.len(),
            addresses.iter().map(|a| format!("{:#04x}", a)).collect::<Vec<_>>()
        );

        state.polling_active.store(true, Ordering::Relaxed);
        let state_sim = state.clone();
        let config_sim = config.clone();
        tokio::spawn(async move {
            simulator::run_simulator(state_sim, config_sim, addresses).await;
        });
    } else {
        // Mode hardware réel

        // ── 1. Résoudre le port (auto-détection ou config) ───────────────────
        // find_daly_port() retourne le port déjà ouvert pour éviter la double
        // ouverture Windows ("Accès refusé" si on ouvre, ferme, puis rouvre).
        let (resolved_port, pre_opened_port) = if auto_detect_port {
            info!("Port non spécifié — détection automatique en cours...");
            match autodetect::find_daly_port(config.serial.baud).await {
                Some((name, port)) => (name, Some(port)),
                None => {
                    error!("Aucun Daly BMS détecté sur les ports série disponibles.");
                    warn!("Relancez avec --port COMx pour forcer un port.");
                    warn!("Démarrage en mode API-seule (pas de données BMS).");
                    (String::new(), None)
                }
            }
        } else {
            (config.serial.port.clone(), None)
        };

        if !resolved_port.is_empty() {
            // Réutiliser le port pré-ouvert (auto-détect) ou en ouvrir un nouveau
            let dal_port = match pre_opened_port {
                Some(p) => Ok(p),
                None    => DalyPort::open(&resolved_port, config.serial.baud, 500),
            };
            match dal_port {
                Ok(port) => {
                    info!("Port série {} ouvert à {} baud", resolved_port, config.serial.baud);
                    state.polling_active.store(true, Ordering::Relaxed);

                    // ── 2. Résoudre les adresses BMS (auto-découverte ou config) ──
                    let addresses = if auto_discover_addrs {
                        info!("Découverte automatique des BMS sur le bus RS485 (0x01..0x04)...");
                        let mgr_tmp = DalyBusManager::new(port.clone(), vec![]);
                        let found = mgr_tmp.discover(1, 4).await;
                        if found.is_empty() {
                            warn!("Aucun BMS découvert — polling désactivé.");
                            state.polling_active.store(false, Ordering::Relaxed);
                        }
                        found
                    } else {
                        config.bms_addresses()
                    };

                    if !addresses.is_empty() {
                        let devices: Vec<BmsConfig> = addresses
                            .iter()
                            .map(|&addr| {
                                let mut bms = BmsConfig::new(addr);
                                bms.cell_count        = config.serial.default_cell_count;
                                bms.temp_sensor_count = config.serial.default_temp_sensors;
                                bms
                            })
                            .collect();

                        info!("Polling de {} BMS : {:?}", devices.len(),
                              devices.iter().map(|d| format!("{:#04x}", d.address)).collect::<Vec<_>>());

                        let manager  = Arc::new(DalyBusManager::new(port, devices));
                        let poll_cfg = PollConfig {
                            interval_ms: config.serial.poll_interval_ms,
                            ..Default::default()
                        };
                        let state_poll = state.clone();
                        tokio::spawn(async move {
                            poll_loop(manager, poll_cfg, move |snap| {
                                let s = state_poll.clone();
                                tokio::spawn(async move { s.on_snapshot(snap).await });
                            })
                            .await;
                        });
                    }
                }
                Err(e) => {
                    error!("Impossible d'ouvrir {} : {:?}", resolved_port, e);
                    warn!("Démarrage en mode API-seule (pas de données BMS).");
                    warn!("Astuce : relancez avec --simulate pour tester sans matériel.");
                }
            }
        }
    }

    // ── Serveur HTTP Axum ──────────────────────────────────────────────────────
    let router = api::build_router(state);
    let addr: SocketAddr = config.api.bind.parse()?;

    info!("API  → http://{}", addr);
    info!("WS   → ws://{}/ws/bms/stream", addr);
    if args.simulate {
        info!("Docs → http://{}/api/v1/system/status", addr);
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
