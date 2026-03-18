//! `daly-bms-venus` — Bridge MQTT → D-Bus Venus OS (batteries + capteurs)
//!
//! Ce binaire enregistre sur le D-Bus du Victron GX (Venus OS) :
//! - `com.victronenergy.battery.{n}` pour chaque BMS Daly (topic `bms/{n}/venus`)
//! - `com.victronenergy.temperature.{n}` pour chaque capteur température
//!   (topic `heat/{n}/venus` — outdoor temp, water heater…)
//!
//! ## Flux
//!
//! ```text
//! [MQTT: bms/{n}/venus]  → [BatteryManager] → [D-Bus: com.victronenergy.battery.{n}]
//! [MQTT: heat/{n}/venus] → [SensorManager]  → [D-Bus: com.victronenergy.temperature.{n}]
//!                                                    ↓
//!                                             [Venus systemcalc → VRM Portal]
//! ```
//!
//! ## Utilisation
//!
//! ```sh
//! # Production (Venus OS)
//! daly-bms-venus --config /etc/daly-bms/config.toml
//!
//! # Développement (D-Bus session bus)
//! DALY_CONFIG=Config.toml daly-bms-venus
//! ```

mod battery_service;
mod config;
mod heatpump_manager;
mod heatpump_service;
mod manager;
mod meteo_manager;
mod meteo_service;
mod mqtt_source;
mod sensor_manager;
mod temperature_service;
mod types;

use anyhow::Result;
use clap::Parser;
use config::VenusServiceConfig;
use heatpump_manager::HeatpumpManager;
use manager::BatteryManager;
use meteo_manager::MeteoManager;
use mqtt_source::{
    start_heatpump_mqtt_source, start_meteo_mqtt_source, start_mqtt_source,
    start_sensor_mqtt_source,
};
use sensor_manager::SensorManager;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt};

// =============================================================================
// CLI
// =============================================================================

#[derive(Parser, Debug)]
#[command(
    name    = "daly-bms-venus",
    about   = "Venus OS D-Bus battery service bridge for Daly BMS",
    version = env!("CARGO_PKG_VERSION"),
)]
struct Cli {
    /// Chemin vers le fichier de configuration TOML.
    /// Si absent, utilise DALY_CONFIG ou Config.toml / /etc/daly-bms/config.toml.
    #[arg(short, long, env = "DALY_CONFIG")]
    config: Option<PathBuf>,

    /// Override: bus D-Bus à utiliser ("system" ou "session")
    #[arg(long, env = "VENUS_DBUS_BUS")]
    dbus_bus: Option<String>,
}

// =============================================================================
// Point d'entrée
// =============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Initialiser le logging
    fmt()
        .with_env_filter(
            EnvFilter::try_from_env("RUST_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Charger la configuration
    let mut cfg = match &cli.config {
        Some(path) => VenusServiceConfig::load(path)?,
        None       => VenusServiceConfig::load_default()?,
    };

    // Override CLI
    if let Some(bus) = cli.dbus_bus {
        cfg.venus.dbus_bus = bus;
    }

    info!(
        version          = env!("CARGO_PKG_VERSION"),
        dbus_bus         = %cfg.venus.dbus_bus,
        mqtt_host        = %cfg.mqtt.host,
        bms_prefix       = %cfg.mqtt.topic_prefix,
        heat_prefix      = %cfg.heat.topic_prefix,
        heatpump_prefix  = %cfg.heatpump.topic_prefix,
        meteo_topic      = %cfg.meteo.topic,
        bms_count        = cfg.bms.len(),
        sensor_count     = cfg.sensors.len(),
        heatpump_count   = cfg.heatpumps.len(),
        "daly-bms-venus démarrage"
    );

    if !cfg.venus.enabled {
        info!("Service Venus désactivé dans la config (venus.enabled = false). Sortie.");
        return Ok(());
    }

    // -------------------------------------------------------------------------
    // Bridge BMS batteries : MQTT bms/{n}/venus → D-Bus battery.{n}
    // -------------------------------------------------------------------------
    let (bms_tx, bms_rx) = mpsc::channel(64);
    let mqtt_cfg = cfg.mqtt.clone();
    tokio::spawn(async move {
        start_mqtt_source(mqtt_cfg, bms_tx).await;
    });

    let battery_manager = BatteryManager::new(cfg.venus.clone(), cfg.bms, bms_rx);
    tokio::spawn(async move {
        if let Err(e) = battery_manager.run().await {
            error!("BatteryManager terminé avec erreur : {:#}", e);
        }
    });

    // -------------------------------------------------------------------------
    // Bridge capteurs température : MQTT heat/{n}/venus → D-Bus temperature.{n}
    // -------------------------------------------------------------------------
    let (sensor_tx, sensor_rx) = mpsc::channel(64);
    let mqtt_cfg2    = cfg.mqtt.clone();
    let heat_prefix  = cfg.heat.topic_prefix.clone();
    tokio::spawn(async move {
        start_sensor_mqtt_source(mqtt_cfg2, heat_prefix, sensor_tx).await;
    });

    let sensor_manager = SensorManager::new(cfg.venus.clone(), cfg.sensors, sensor_rx);
    tokio::spawn(async move {
        if let Err(e) = sensor_manager.run().await {
            error!("SensorManager terminé avec erreur : {:#}", e);
        }
    });

    // -------------------------------------------------------------------------
    // Bridge heatpump : MQTT heatpump/{n}/venus → D-Bus heatpump.{n}
    // -------------------------------------------------------------------------
    let (heatpump_tx, heatpump_rx) = mpsc::channel(64);
    let mqtt_cfg3       = cfg.mqtt.clone();
    let heatpump_prefix = cfg.heatpump.topic_prefix.clone();
    tokio::spawn(async move {
        start_heatpump_mqtt_source(mqtt_cfg3, heatpump_prefix, heatpump_tx).await;
    });

    let heatpump_manager = HeatpumpManager::new(cfg.venus.clone(), cfg.heatpumps, heatpump_rx);
    tokio::spawn(async move {
        if let Err(e) = heatpump_manager.run().await {
            error!("HeatpumpManager terminé avec erreur : {:#}", e);
        }
    });

    // -------------------------------------------------------------------------
    // Bridge météo : MQTT santuario/meteo/venus → D-Bus com.victronenergy.meteo
    // -------------------------------------------------------------------------
    let (meteo_tx, meteo_rx) = mpsc::channel(16);
    let mqtt_cfg4    = cfg.mqtt.clone();
    let meteo_topic  = cfg.meteo.topic.clone();
    tokio::spawn(async move {
        start_meteo_mqtt_source(mqtt_cfg4, meteo_topic, meteo_tx).await;
    });

    // Le MeteoManager est le dernier et bloque le thread principal
    let meteo_manager = MeteoManager::new(cfg.venus, cfg.meteo, meteo_rx);
    if let Err(e) = meteo_manager.run().await {
        error!("MeteoManager terminé avec erreur : {:#}", e);
        std::process::exit(1);
    }

    Ok(())
}
