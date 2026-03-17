//! `daly-bms-venus` — Service D-Bus Venus OS pour Daly BMS
//!
//! Ce binaire crée des services `com.victronenergy.battery.{n}` sur le D-Bus
//! du Victron GX (Venus OS) en lisant les données depuis le broker MQTT local.
//!
//! ## Flux
//!
//! ```text
//! [MQTT broker] → [mqtt_source] → [BatteryManager] → [D-Bus: com.victronenergy.battery.*]
//!                                                           ↓
//!                                                    [Venus systemcalc]
//!                                                           ↓
//!                                                    [VRM Portal]
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
mod manager;
mod mqtt_source;
mod types;

use anyhow::Result;
use clap::Parser;
use config::VenusServiceConfig;
use manager::BatteryManager;
use mqtt_source::start_mqtt_source;
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
        version = env!("CARGO_PKG_VERSION"),
        dbus_bus = %cfg.venus.dbus_bus,
        mqtt_host = %cfg.mqtt.host,
        mqtt_prefix = %cfg.mqtt.topic_prefix,
        bms_count = cfg.bms.len(),
        "daly-bms-venus démarrage"
    );

    if !cfg.venus.enabled {
        info!("Service Venus désactivé dans la config (venus.enabled = false). Sortie.");
        return Ok(());
    }

    // Canal MQTT → Manager (buffer 64 messages)
    let (tx, rx) = mpsc::channel(64);

    // Démarrer la source MQTT en arrière-plan
    let mqtt_cfg = cfg.mqtt.clone();
    tokio::spawn(async move {
        start_mqtt_source(mqtt_cfg, tx).await;
    });

    // Démarrer le manager D-Bus (bloquant)
    let manager = BatteryManager::new(cfg.venus, cfg.bms, rx);

    if let Err(e) = manager.run().await {
        error!("BatteryManager terminé avec erreur : {:#}", e);
        std::process::exit(1);
    }

    Ok(())
}
