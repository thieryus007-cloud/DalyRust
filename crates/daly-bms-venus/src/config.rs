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

    /// Configuration du préfixe MQTT heat (capteurs température outdoor)
    #[serde(default)]
    pub heat: HeatConfig,

    /// Configurations par capteur de température
    #[serde(default)]
    pub sensors: Vec<SensorRef>,

    /// Configuration MQTT heatpump (pompes à chaleur / chauffe-eau)
    #[serde(default)]
    pub heatpump: HeatpumpConfig,

    /// Configurations par pompe à chaleur
    #[serde(default)]
    pub heatpumps: Vec<HeatpumpRef>,

    /// Configuration du service météo (irradiance RS485)
    #[serde(default)]
    pub meteo: MeteoConfig,
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

// =============================================================================
// Configuration capteurs de température (heat)
// =============================================================================

/// Préfixe MQTT pour les capteurs de température.
///
/// Topic abonné : `{topic_prefix}/+/venus`
/// Exemple : `santuario/heat/1/venus`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeatConfig {
    /// Préfixe des topics heat (ex: "santuario/heat")
    pub topic_prefix: String,
}

impl Default for HeatConfig {
    fn default() -> Self {
        Self { topic_prefix: "santuario/heat".to_string() }
    }
}

/// Configuration d'un capteur de température individuel.
///
/// Une section `[[sensors]]` par capteur dans le TOML.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SensorRef {
    /// Index dans le topic MQTT (ex: 1 → `santuario/heat/1/venus`).
    pub mqtt_index: Option<u8>,

    /// Nom affiché dans Venus OS (`/ProductName` et `/CustomName` par défaut).
    pub name: Option<String>,

    /// Type de température par défaut si absent du payload :
    /// 0=battery, 1=fridge, 2=generic, 3=Room, 4=Outdoor, 5=WaterHeater, 6=Freezer.
    /// Prioritaire sur la valeur du payload.
    pub temperature_type: Option<i32>,

    /// DeviceInstance Venus OS D-Bus (affiché dans VRM).
    /// Si absent, utilise `mqtt_index` comme fallback.
    pub device_instance: Option<u32>,
}

// =============================================================================
// Configuration pompes à chaleur / chauffe-eau (heatpump)
// =============================================================================

/// Préfixe MQTT pour les pompes à chaleur.
///
/// Topic abonné : `{topic_prefix}/{n}/venus`
/// Exemple : `santuario/heatpump/1/venus`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeatpumpConfig {
    /// Préfixe des topics heatpump (ex: "santuario/heatpump")
    pub topic_prefix: String,
}

impl Default for HeatpumpConfig {
    fn default() -> Self {
        Self { topic_prefix: "santuario/heatpump".to_string() }
    }
}

/// Configuration d'une pompe à chaleur individuelle.
///
/// Une section `[[heatpumps]]` par appareil dans le TOML.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HeatpumpRef {
    /// Index dans le topic MQTT (ex: 1 → `santuario/heatpump/1/venus`).
    pub mqtt_index: Option<u8>,

    /// Nom affiché dans Venus OS (`/ProductName`).
    pub name: Option<String>,

    /// DeviceInstance Venus OS D-Bus.
    pub device_instance: Option<u32>,
}

// =============================================================================
// Configuration capteur météo / irradiance (meteo)
// =============================================================================

/// Configuration du service météo D-Bus.
///
/// Topic MQTT fixe : `{topic}` (sans index, un seul capteur).
/// Exemple : `santuario/meteo/venus`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeteoConfig {
    /// Topic MQTT fixe du capteur météo (ex: "santuario/meteo/venus")
    pub topic: String,

    /// Nom affiché dans Venus OS
    pub product_name: String,

    /// DeviceInstance Venus OS D-Bus
    pub device_instance: u32,
}

impl Default for MeteoConfig {
    fn default() -> Self {
        Self {
            topic:           "santuario/meteo/venus".to_string(),
            product_name:    "Irradiance Sensor".to_string(),
            device_instance: 30,
        }
    }
}

// =============================================================================
// Configuration BMS (existante)
// =============================================================================

/// Référence légère à une configuration BMS individuelle.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BmsRef {
    /// Adresse RS485 (pour identification logs uniquement)
    pub address:    String,
    /// Nom affiché dans Venus OS
    pub name:       Option<String>,
    /// Index dans le topic MQTT (ex: 1 → `santuario/bms/1/venus`).
    /// Doit correspondre à l'adresse RS485 décimale publiée par le serveur.
    pub mqtt_index: Option<u8>,
    /// DeviceInstance Venus OS D-Bus (affiché dans VRM, ex: 141, 142).
    /// Si absent, utilise `mqtt_index` comme fallback.
    pub device_instance: Option<u32>,
    /// Capacité nominale (Ah) — utilisée comme InstalledCapacity si absente du payload
    pub capacity_ah: Option<f32>,
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
