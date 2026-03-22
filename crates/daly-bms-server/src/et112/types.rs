//! Types de données pour le compteur Carlo Gavazzi ET112.
//!
//! Le ET112 est un compteur monophasé RS485/Modbus RTU.
//! Registres lus via FC=04 (Read Input Registers), FLOAT32 big-endian.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Snapshot complet d'un ET112 à un instant donné.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Et112Snapshot {
    /// Adresse Modbus (ex: 3 pour 0x03)
    pub address: u8,

    /// Nom configuré (ex: "Micro-inverseurs")
    pub name: String,

    /// Instant de la mesure
    pub timestamp: DateTime<Local>,

    // ── Mesures instantanées ────────────────────────────────────────────────
    /// Tension L1 (V)
    pub voltage_v: f32,

    /// Courant L1 (A) — positif = import, négatif = export
    pub current_a: f32,

    /// Puissance active (W) — positif = import, négatif = export
    pub power_w: f32,

    /// Puissance apparente (VA)
    pub apparent_power_va: f32,

    /// Puissance réactive (VAr)
    pub reactive_power_var: f32,

    /// Facteur de puissance (sans unité, -1.0 à +1.0)
    pub power_factor: f32,

    /// Fréquence réseau (Hz)
    pub frequency_hz: f32,

    // ── Énergies cumulées ───────────────────────────────────────────────────
    /// Énergie importée depuis la mise en service (Wh)
    pub energy_import_wh: f32,

    /// Énergie exportée depuis la mise en service (Wh)
    pub energy_export_wh: f32,
}

impl Et112Snapshot {
    /// Énergie importée en kWh.
    pub fn energy_import_kwh(&self) -> f32 {
        self.energy_import_wh / 1000.0
    }

    /// Énergie exportée en kWh.
    pub fn energy_export_kwh(&self) -> f32 {
        self.energy_export_wh / 1000.0
    }
}
