//! Configuration du service `daly-bms-venus`.
//!
//! Chargée depuis le même `config.toml` que le serveur principal.
//! Section `[venus]` optionnelle + sections `[mqtt]` et `[[bms]]` réutilisées.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

// =============================================================================
// Configuration complète du service Venus
// =============================================================================

/// Configuration du bridge MQTT → D-Bus Venus OS.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VenusServiceConfig {
    /// Section MQTT (réutilisée du serveur principal)
    pub mqtt: MqttRef,

    /// Section Venus spécifique
    #[serde(default)]
    pub venus: VenusConfig,

    /// Configurations par BMS (pour mqtt_index et DeviceInstance)
    #[serde(default)]
    pub bms: Vec<BmsRef>,
}

/// Référence à la config MQTT du serveur principal.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MqttRef {
    pub host:                 String,
    pub port:                 u16,
    pub topic_prefix:         String,
    #[serde(default)]
    pub username:             Option<String>,
    #[serde(default)]
    pub password:             Option<String>,
}

/// Configuration spécifique Venus OS.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VenusConfig {
    /// Activer le service (false = désactivé, pratique pour dev)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Bus D-Bus à utiliser :
    /// - `"system"`  → `/var/run/dbus/system_bus_socket`  (Venus OS production)
    /// - `"session"` → `$DBUS_SESSION_BUS_ADDRESS`        (test développement)
    #[serde(default = "default_dbus_bus")]
    pub dbus_bus: String,

    /// Préfixe du nom de service D-Bus.
    /// Service résultant : `com.victronenergy.battery.{prefix}_{mqtt_index}`
    /// Ex: "mqtt" → `com.victronenergy.battery.mqtt_1`
    #[serde(default = "default_service_prefix")]
    pub service_prefix: String,

    /// Délai watchdog (secondes).
    /// Si aucune donnée MQTT reçue pendant ce délai, `/Connected` passe à 0.
    #[serde(default = "default_watchdog_sec")]
    pub watchdog_sec: u64,

    /// Intervalle de republication forcée des valeurs (secondes).
    /// Garantit que Venus OS reçoit un `ItemsChanged` même sans changement.
    #[serde(default = "default_republish_sec")]
    pub republish_sec: u64,
}

impl Default for VenusConfig {
    fn default() -> Self {
        Self {
            enabled:          true,
            dbus_bus:         default_dbus_bus(),
            service_prefix:   default_service_prefix(),
            watchdog_sec:     default_watchdog_sec(),
            republish_sec:    default_republish_sec(),
        }
    }
}

fn default_enabled()        -> bool   { true }
fn default_dbus_bus()       -> String { "system".to_string() }
fn default_service_prefix() -> String { "mqtt".to_string() }
fn default_watchdog_sec()   -> u64    { 30 }
fn default_republish_sec()  -> u64    { 25 }

/// Référence légère à une configuration BMS individuelle.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BmsRef {
    /// Adresse RS485 (pour identification logs uniquement)
    pub address:    String,
    /// Nom affiché dans Venus OS
    pub name:       Option<String>,
    /// Index MQTT → DeviceInstance D-Bus
    pub mqtt_index: Option<u8>,
    /// Capacité nominale (Ah) — utilisée comme InstalledCapacity si absente du payload
    pub capacity_ah: Option<f32>,
}

impl BmsRef {
    /// Retourne le `mqtt_index` ou un index par défaut (position 1-based dans le tableau).
    pub fn device_instance(&self, fallback_position: u8) -> u8 {
        self.mqtt_index.unwrap_or(fallback_position)
    }
}

// =============================================================================
// Chargement
// =============================================================================

impl VenusServiceConfig {
    /// Charge la config depuis un fichier TOML.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Impossible de lire {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Erreur de parsing TOML dans {}", path.display()))
    }

    /// Charge depuis le chemin par défaut ou `DALY_CONFIG`.
    pub fn load_default() -> Result<Self> {
        if let Ok(path) = std::env::var("DALY_CONFIG") {
            return Self::load(Path::new(&path));
        }
        for candidate in &["Config.toml", "/etc/daly-bms/config.toml"] {
            let p = Path::new(candidate);
            if p.exists() {
                return Self::load(p);
            }
        }
        anyhow::bail!("Config non trouvée — ni Config.toml ni /etc/daly-bms/config.toml");
    }
}
