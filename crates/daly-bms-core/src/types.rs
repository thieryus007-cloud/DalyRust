//! Structures de données représentant l'état complet d'un BMS Daly.
//!
//! La structure principale est [`BmsSnapshot`] qui agrège toutes les lectures
//! d'un cycle de polling. Elle correspond exactement au format `JSONData.json`
//! du projet Python de référence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Adresse RS485 d'un BMS (0x01–0xFF).
pub type BmsAddress = u8;

// =============================================================================
// Snapshot principal
// =============================================================================

/// Snapshot complet de l'état d'un BMS à un instant donné.
///
/// Produit par [`crate::commands`] après un cycle de polling complet.
/// Compatible avec le format JSON du projet Python (JSONData.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BmsSnapshot {
    /// Adresse RS485 du BMS (0x01–0xFF)
    pub address: BmsAddress,

    /// Timestamp UTC du snapshot
    pub timestamp: DateTime<Utc>,

    /// Données DC (puissance, tension, courant, température)
    #[serde(rename = "Dc")]
    pub dc: DcData,

    /// Capacité installée totale (Ah) — configurée dans config.toml
    #[serde(rename = "InstalledCapacity")]
    pub installed_capacity: f32,

    /// Ampheure-heures consommés depuis la dernière charge complète
    #[serde(rename = "ConsumedAmphours")]
    pub consumed_amphours: f32,

    /// Capacité restante calculée (Ah)
    #[serde(rename = "Capacity")]
    pub capacity: f32,

    /// État de charge (%)
    #[serde(rename = "Soc")]
    pub soc: f32,

    /// État de santé (%)
    #[serde(rename = "Soh")]
    pub soh: f32,

    /// Temps estimé jusqu'à décharge complète (secondes, 0 si inconnu)
    #[serde(rename = "TimeToGo")]
    pub time_to_go: u32,

    /// Équilibrage actif (0 = inactif, 1 = actif)
    #[serde(rename = "Balancing")]
    pub balancing: u8,

    /// Interrupteur principal du système (0 = off, 1 = on)
    #[serde(rename = "SystemSwitch")]
    pub system_switch: u8,

    /// Alarmes actives
    #[serde(rename = "Alarms")]
    pub alarms: Alarms,

    /// Informations de charge (limites, chauffage)
    #[serde(rename = "Info")]
    pub info: InfoData,

    /// Historique de vie du pack
    #[serde(rename = "History")]
    pub history: HistoryData,

    /// État du système (min/max cellules, températures, MOS)
    #[serde(rename = "System")]
    pub system: SystemData,

    /// Tensions individuelles des cellules ("Cell1" → 3.405 V)
    #[serde(rename = "Voltages")]
    pub voltages: BTreeMap<String, f32>,

    /// État d'équilibrage par cellule ("Cell1" → 0/1)
    #[serde(rename = "Balances")]
    pub balances: BTreeMap<String, u8>,

    /// Permissions d'opération
    #[serde(rename = "Io")]
    pub io: IoData,

    /// Chauffage actif (0/1)
    #[serde(rename = "Heating")]
    pub heating: u8,

    /// Temps estimé pour atteindre chaque palier de SOC (% → secondes)
    #[serde(rename = "TimeToSoC")]
    pub time_to_soc: BTreeMap<u8, u32>,
}

// =============================================================================
// Sous-structures
// =============================================================================

/// Données DC : puissance, tension pack, courant, température interne MOS.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DcData {
    /// Puissance instantanée (W) — tension × courant
    #[serde(rename = "Power")]
    pub power: f32,

    /// Tension totale du pack (V) — Data ID 0x90, octets 0-1, /10
    #[serde(rename = "Voltage")]
    pub voltage: f32,

    /// Courant (A) — Data ID 0x90, octets 2-3, (valeur - 30000) / 10
    /// Positif = charge, négatif = décharge
    #[serde(rename = "Current")]
    pub current: f32,

    /// Température interne MOS (°C) — voir SystemData.mos_temperature
    #[serde(rename = "Temperature")]
    pub temperature: f32,
}

/// Alarmes de protection (0 = OK, 1 = alarme active).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Alarms {
    /// Sous-tension pack ou cellule
    #[serde(rename = "LowVoltage")]
    pub low_voltage: u8,

    /// Sur-tension pack ou cellule
    #[serde(rename = "HighVoltage")]
    pub high_voltage: u8,

    /// SOC trop bas
    #[serde(rename = "LowSoc")]
    pub low_soc: u8,

    /// Sur-courant de charge
    #[serde(rename = "HighChargeCurrent")]
    pub high_charge_current: u8,

    /// Sur-courant de décharge
    #[serde(rename = "HighDischargeCurrent")]
    pub high_discharge_current: u8,

    /// Sur-courant générique
    #[serde(rename = "HighCurrent")]
    pub high_current: u8,

    /// Déséquilibre de cellules
    #[serde(rename = "CellImbalance")]
    pub cell_imbalance: u8,

    /// Sur-température de charge
    #[serde(rename = "HighChargeTemperature")]
    pub high_charge_temperature: u8,

    /// Sous-température de charge
    #[serde(rename = "LowChargeTemperature")]
    pub low_charge_temperature: u8,

    /// Sous-tension cellule
    #[serde(rename = "LowCellVoltage")]
    pub low_cell_voltage: u8,

    /// Sous-température globale
    #[serde(rename = "LowTemperature")]
    pub low_temperature: u8,

    /// Sur-température globale
    #[serde(rename = "HighTemperature")]
    pub high_temperature: u8,

    /// Fusible grillé
    #[serde(rename = "FuseBlown")]
    pub fuse_blown: u8,
}

impl Alarms {
    /// Retourne `true` si au moins une alarme est active.
    pub fn any_active(&self) -> bool {
        self.low_voltage | self.high_voltage | self.low_soc
            | self.high_charge_current | self.high_discharge_current
            | self.high_current | self.cell_imbalance
            | self.high_charge_temperature | self.low_charge_temperature
            | self.low_cell_voltage | self.low_temperature
            | self.high_temperature | self.fuse_blown != 0
    }
}

/// Paramètres de charge et chauffage.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InfoData {
    /// Demande de charge active (0/1)
    #[serde(rename = "ChargeRequest")]
    pub charge_request: u8,

    /// Tension maximale de charge autorisée (V)
    #[serde(rename = "MaxChargeVoltage")]
    pub max_charge_voltage: f32,

    /// Courant maximal de charge autorisé (A)
    #[serde(rename = "MaxChargeCurrent")]
    pub max_charge_current: f32,

    /// Courant maximal de décharge autorisé (A)
    #[serde(rename = "MaxDischargeCurrent")]
    pub max_discharge_current: f32,

    /// Tension maximale par cellule autorisée pour la charge (V)
    #[serde(rename = "MaxChargeCellVoltage")]
    pub max_charge_cell_voltage: f32,

    /// Courant de chauffage (A)
    #[serde(rename = "HeatingCurrent")]
    pub heating_current: f32,

    /// Puissance de chauffage (W)
    #[serde(rename = "HeatingPower")]
    pub heating_power: f32,

    /// Température de démarrage du chauffage (°C)
    #[serde(rename = "HeatingTemperatureStart")]
    pub heating_temperature_start: f32,

    /// Température d'arrêt du chauffage (°C)
    #[serde(rename = "HeatingTemperatureStop")]
    pub heating_temperature_stop: f32,
}

/// Historique de vie du pack batterie.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistoryData {
    /// Nombre de cycles de charge effectués
    #[serde(rename = "ChargeCycles")]
    pub charge_cycles: u32,

    /// Tension minimale historique enregistrée (V)
    #[serde(rename = "MinimumVoltage")]
    pub minimum_voltage: f32,

    /// Tension maximale historique enregistrée (V)
    #[serde(rename = "MaximumVoltage")]
    pub maximum_voltage: f32,

    /// Total des ampheure-heures déchargés depuis la mise en service
    #[serde(rename = "TotalAhDrawn")]
    pub total_ah_drawn: f32,
}

/// État instantané du système : cellules, températures, MOS, modules.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemData {
    /// Identifiant de la cellule avec la tension minimale (ex: "C4")
    #[serde(rename = "MinVoltageCellId")]
    pub min_voltage_cell_id: String,

    /// Tension minimale des cellules (V)
    #[serde(rename = "MinCellVoltage")]
    pub min_cell_voltage: f32,

    /// Identifiant de la cellule avec la tension maximale (ex: "C12")
    #[serde(rename = "MaxVoltageCellId")]
    pub max_voltage_cell_id: String,

    /// Tension maximale des cellules (V)
    #[serde(rename = "MaxCellVoltage")]
    pub max_cell_voltage: f32,

    /// Identifiant du capteur de température minimale (ex: "C7")
    #[serde(rename = "MinTemperatureCellId")]
    pub min_temperature_cell_id: String,

    /// Température minimale mesurée (°C)
    #[serde(rename = "MinCellTemperature")]
    pub min_cell_temperature: f32,

    /// Identifiant du capteur de température maximale (ex: "C3")
    #[serde(rename = "MaxTemperatureCellId")]
    pub max_temperature_cell_id: String,

    /// Température maximale mesurée (°C)
    #[serde(rename = "MaxCellTemperature")]
    pub max_cell_temperature: f32,

    /// Température de la carte MOS (°C)
    #[serde(rename = "MOSTemperature")]
    pub mos_temperature: f32,

    /// Nombre de modules en ligne
    #[serde(rename = "NrOfModulesOnline")]
    pub nr_of_modules_online: u8,

    /// Nombre de modules hors ligne
    #[serde(rename = "NrOfModulesOffline")]
    pub nr_of_modules_offline: u8,

    /// Nombre de cellules par batterie/module
    #[serde(rename = "NrOfCellsPerBattery")]
    pub nr_of_cells_per_battery: u8,

    /// Modules bloquant la charge
    #[serde(rename = "NrOfModulesBlockingCharge")]
    pub nr_of_modules_blocking_charge: u8,

    /// Modules bloquant la décharge
    #[serde(rename = "NrOfModulesBlockingDischarge")]
    pub nr_of_modules_blocking_discharge: u8,
}

impl SystemData {
    /// Delta de tension entre cellule max et min (mV).
    pub fn cell_delta_mv(&self) -> f32 {
        (self.max_cell_voltage - self.min_cell_voltage) * 1000.0
    }
}

/// Permissions d'opération retournées par le BMS.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IoData {
    /// Autorisation de charge (0/1)
    #[serde(rename = "AllowToCharge")]
    pub allow_to_charge: u8,

    /// Autorisation de décharge (0/1)
    #[serde(rename = "AllowToDischarge")]
    pub allow_to_discharge: u8,

    /// Autorisation d'équilibrage (0/1)
    #[serde(rename = "AllowToBalance")]
    pub allow_to_balance: u8,

    /// Autorisation de chauffage (0/1)
    #[serde(rename = "AllowToHeat")]
    pub allow_to_heat: u8,

    /// Relais externe (0/1)
    #[serde(rename = "ExternalRelay")]
    pub external_relay: u8,
}

// =============================================================================
// Types intermédiaires produits par les commandes individuelles
// =============================================================================

/// Résultat brut de la commande 0x95–0x98 (tensions individuelles).
#[derive(Debug, Clone, Default)]
pub struct CellVoltages {
    /// Tensions en volts, indexées par numéro de cellule (0-based).
    pub voltages: Vec<f32>,
}

impl CellVoltages {
    /// Construit la map "Cell1" → f32 pour BmsSnapshot.
    pub fn to_named_map(&self) -> BTreeMap<String, f32> {
        self.voltages
            .iter()
            .enumerate()
            .map(|(i, &v)| (format!("Cell{}", i + 1), v))
            .collect()
    }
}

/// Résultat brut de la commande 0x96 (températures individuelles).
#[derive(Debug, Clone, Default)]
pub struct CellTemperatures {
    /// Températures en °C, indexées par numéro de capteur (0-based).
    pub temperatures: Vec<f32>,
}

/// Résultat de la commande 0x97 (flags d'équilibrage).
#[derive(Debug, Clone, Default)]
pub struct BalanceFlags {
    /// État d'équilibrage par cellule (0-based). true = en cours.
    pub flags: Vec<bool>,
}

impl BalanceFlags {
    /// Construit la map "Cell1" → u8 pour BmsSnapshot.
    pub fn to_named_map(&self) -> BTreeMap<String, u8> {
        self.flags
            .iter()
            .enumerate()
            .map(|(i, &f)| (format!("Cell{}", i + 1), u8::from(f)))
            .collect()
    }
}

/// Résultat de la commande 0x90 : SOC, tension, courant.
#[derive(Debug, Clone, Default)]
pub struct SocData {
    pub voltage: f32,
    pub current: f32,
    pub soc: f32,
}

/// Résultat de la commande 0x93 : état MOS, cycles, capacité résiduelle.
#[derive(Debug, Clone, Default)]
pub struct MosStatus {
    pub charge_mos: bool,
    pub discharge_mos: bool,
    pub bms_life: u8,
    pub residual_capacity_mah: u32,
    pub charge_cycles: u32,
}

/// Résultat de la commande 0x94 : informations de statut.
#[derive(Debug, Clone, Default)]
pub struct StatusInfo {
    pub cell_count: u8,
    pub temp_sensor_count: u8,
    pub charger_status: u8,
    pub load_status: u8,
    pub dio_states: u8,
    pub cycle_count: u16,
}
