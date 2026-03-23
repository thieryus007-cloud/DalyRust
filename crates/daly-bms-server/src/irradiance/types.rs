//! Types pour le capteur d'irradiance RS485 (PRALRAN Solar Radiation Sensor).

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Snapshot d'une mesure d'irradiance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrradianceSnapshot {
    /// Adresse Modbus du capteur (ex: 5 pour 0x05)
    pub address: u8,

    /// Nom configuré
    pub name: String,

    /// Instant de la mesure
    pub timestamp: DateTime<Local>,

    /// Irradiance solaire (W/m²)
    pub irradiance_wm2: f32,
}
