//! Chargement et désérialisation de la configuration TOML.
//!
//! La configuration est lue depuis `/etc/daly-bms/config.toml` par défaut,
//! ou depuis le chemin spécifié par `DALY_CONFIG` ou en argument CLI.

use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Context, Result};

// =============================================================================
// Structure principale
// =============================================================================

/// Configuration complète de l'application.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub serial: SerialConfig,

    #[serde(default)]
    pub api: ApiConfig,

    #[serde(default)]
    pub logging: LoggingConfig,

    #[serde(default)]
    pub mqtt: MqttConfig,

    #[serde(default)]
    pub influxdb: InfluxConfig,

    #[serde(default)]
    pub alerts: AlertsConfig,

    #[serde(default)]
    pub read_only: ReadOnlyConfig,

    /// Configurations individuelles par BMS (optionnel)
    #[serde(default)]
    pub bms: Vec<BmsDeviceConfig>,
}

// =============================================================================
// Configuration par BMS
// =============================================================================

/// Surcharges de configuration pour un BMS individuel.
///
/// ```toml
/// [[bms]]
/// address        = "0x28"     # adresse RS485 (décimal ou hex)
/// name           = "BMS-360Ah"
/// capacity_ah    = 360.0
/// max_charge_a   = 200.0
/// max_discharge_a= 120.0
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BmsDeviceConfig {
    /// Adresse RS485 (ex : "0x01", "1", "40")
    pub address: String,
    /// Nom affiché dans le dashboard
    pub name: Option<String>,
    /// Capacité nominale installée (Ah)
    pub capacity_ah: Option<f32>,
    /// Courant de charge maximal autorisé (A)
    pub max_charge_a: Option<f32>,
    /// Courant de décharge maximal autorisé (A)
    pub max_discharge_a: Option<f32>,
    /// Index MQTT pour le topic Venus OS (ex: 1 → santuario/bms/1/venus).
    /// Si absent, utilise la position dans le tableau [[bms]] (1-based).
    pub mqtt_index: Option<u8>,
}

impl BmsDeviceConfig {
    /// Parse l'adresse en u8 (supporte "0x28", "40", "1")
    pub fn parsed_address(&self) -> Option<u8> {
        let s = self.address.trim();
        if s.starts_with("0x") || s.starts_with("0X") {
            u8::from_str_radix(&s[2..], 16).ok()
        } else {
            s.parse::<u8>().ok()
        }
    }
}

impl AppConfig {
    /// Charge la configuration depuis un fichier TOML.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Impossible de lire {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Erreur de parsing TOML dans {}", path.display()))
    }

    /// Charge depuis le chemin par défaut ou `DALY_CONFIG`.
    ///
    /// Ordre de recherche :
    /// 1. Variable d'environnement `DALY_CONFIG`
    /// 2. `./Config.toml` (répertoire courant — développement Windows)
    /// 3. `/etc/daly-bms/config.toml` (déploiement Linux)
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

    /// Retourne la liste des adresses BMS configurées.
    pub fn bms_addresses(&self) -> Vec<u8> {
        self.serial.addresses.iter()
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
// Sous-sections
// =============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SerialConfig {
    /// Chemin du port série (ex: /dev/ttyUSB0)
    pub port: String,
    /// Vitesse en bauds (Daly = 9600)
    pub baud: u32,
    /// Intervalle de polling global (ms)
    pub poll_interval_ms: u64,
    /// Nombre de cellules par défaut
    pub default_cell_count: u8,
    /// Nombre de sondes NTC par défaut
    pub default_temp_sensors: u8,
    /// Taille du ring buffer (snapshots conservés en mémoire)
    pub ring_buffer_size: usize,
    /// Activer la découverte automatique
    pub auto_discover: bool,
    pub auto_discover_start: u8,
    pub auto_discover_end: u8,
    /// Liste explicite d'adresses BMS (ex: ["0x01", "0x02"])
    #[serde(default)]
    pub addresses: Vec<String>,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port:               if cfg!(windows) { "COM1".into() } else { "/dev/ttyUSB0".into() },
            baud:               9600,
            poll_interval_ms:   1000,
            default_cell_count: 16,
            default_temp_sensors: 4,
            ring_buffer_size:   3600,
            auto_discover:      false,
            auto_discover_start: 1,
            auto_discover_end:  16,
            addresses:          vec!["0x01".into()],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    /// Adresse de bind (ex: 0.0.0.0:8000)
    pub bind: String,
    /// Clé API pour les endpoints d'écriture (vide = pas d'auth)
    pub api_key: String,
    /// Activer CORS pour tous les origines
    pub cors_allow_all: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind:          "0.0.0.0:8000".into(),
            api_key:       String::new(),
            cors_allow_all: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    /// Niveau de log (trace, debug, info, warn, error)
    pub level: String,
    /// Format (pretty | json)
    pub format: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level:  "info".into(),
            format: "pretty".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MqttConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub topic_prefix: String,
    pub publish_interval_sec: f64,
    pub username: Option<String>,
    pub password: Option<String>,
    /// "json" | "simple"
    pub format: String,
}

impl MqttConfig {
    #[allow(dead_code)]
    pub fn default_enabled() -> Self {
        Self {
            enabled:              false,
            host:                 "localhost".into(),
            port:                 1883,
            topic_prefix:         "santuario/bms".into(),
            publish_interval_sec: 5.0,
            username:             None,
            password:             None,
            format:               "json".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct InfluxConfig {
    pub enabled: bool,
    pub url: String,
    pub token: String,
    pub org: String,
    pub bucket: String,
    pub bucket_downsampled: String,
    pub batch_size: usize,
    pub batch_flush_interval_sec: f64,
}

impl InfluxConfig {
    #[allow(dead_code)]
    pub fn default_enabled() -> Self {
        Self {
            enabled:                  false,
            url:                      "http://localhost:8086".into(),
            token:                    String::new(),
            org:                      "santuario".into(),
            bucket:                   "daly_bms".into(),
            bucket_downsampled:       "daly_bms_1m".into(),
            batch_size:               50,
            batch_flush_interval_sec: 5.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AlertsConfig {
    pub db_path: String,
    pub check_interval_sec: f64,
    pub telegram_token: String,
    pub telegram_chat_id: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub smtp_to: String,
    #[serde(default)]
    pub thresholds: AlertThresholds,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlertThresholds {
    pub cell_ovp_v: f32,
    pub cell_uvp_v: f32,
    pub cell_delta_mv: f32,
    pub soc_low_percent: f32,
    pub soc_critical_percent: f32,
    pub temp_high_c: f32,
    pub current_high_a: f32,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cell_ovp_v:            3.60,
            cell_uvp_v:            2.90,
            cell_delta_mv:         100.0,
            soc_low_percent:       20.0,
            soc_critical_percent:  10.0,
            temp_high_c:           45.0,
            current_high_a:        80.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ReadOnlyConfig {
    pub enabled: bool,
}
