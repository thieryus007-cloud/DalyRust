//! Types de données pour les prises connectées Tasmota.
//!
//! Tasmota publie nativement via MQTT :
//!   tele/{device}/SENSOR  → mesures énergie (ENERGY block)
//!   stat/{device}/POWER   → état relais (ON/OFF)

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Snapshot complet d'une prise Tasmota à un instant donné.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasmotaSnapshot {
    /// Identifiant interne (clé du ring buffer)
    pub id: u8,

    /// Nom configuré (ex: "Prise Salon")
    pub name: String,

    /// Identifiant Tasmota (nom du device dans les topics MQTT, ex: "tasmota_01")
    pub tasmota_id: String,

    /// Instant de la mesure
    pub timestamp: DateTime<Local>,

    // ── État relais ─────────────────────────────────────────────────────────
    /// true = ON, false = OFF
    pub power_on: bool,

    // ── Mesures instantanées (ENERGY block) ─────────────────────────────────
    /// Puissance active (W)
    pub power_w: f32,

    /// Tension (V)
    pub voltage_v: f32,

    /// Courant (A)
    pub current_a: f32,

    /// Puissance apparente (VA)
    pub apparent_power_va: f32,

    /// Facteur de puissance (0.0 – 1.0)
    pub power_factor: f32,

    // ── Énergie cumulée ─────────────────────────────────────────────────────
    /// Énergie consommée aujourd'hui (kWh)
    pub energy_today_kwh: f32,

    /// Énergie consommée hier (kWh)
    pub energy_yesterday_kwh: f32,

    /// Énergie totale depuis mise en service (kWh)
    pub energy_total_kwh: f32,

    // ── Infos réseau ────────────────────────────────────────────────────────
    /// Signal WiFi (dBm) — None si absent du payload
    pub rssi: Option<i32>,
}
