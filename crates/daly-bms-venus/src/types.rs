//! Types du payload Venus OS reçu depuis MQTT.
//!
//! Ce module est un miroir de `build_venus_payload()` dans `bridges/mqtt.rs`.
//! Les champs correspondent exactement au JSON publié sur `{prefix}/{n}/venus`.

use serde::{Deserialize, Serialize};

/// Payload complet au format Venus OS / dbus-mqtt-battery.
///
/// Publié par `daly-bms-server` sur le topic `{prefix}/{n}/venus`.
/// Désérialisé ici pour alimenter les services D-Bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenusPayload {
    #[serde(rename = "Dc")]
    pub dc: DcPayload,

    #[serde(rename = "InstalledCapacity")]
    pub installed_capacity: f64,

    #[serde(rename = "ConsumedAmphours")]
    pub consumed_amphours: f64,

    #[serde(rename = "Capacity")]
    pub capacity: f64,

    #[serde(rename = "Soc")]
    pub soc: f64,

    #[serde(rename = "TimeToGo")]
    pub time_to_go: i64,

    #[serde(rename = "Balancing")]
    pub balancing: i32,

    #[serde(rename = "SystemSwitch")]
    pub system_switch: i32,

    #[serde(rename = "Alarms")]
    pub alarms: AlarmsPayload,

    #[serde(rename = "System")]
    pub system: SystemPayload,

    #[serde(rename = "Io")]
    pub io: IoPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcPayload {
    #[serde(rename = "Power")]
    pub power: f64,

    #[serde(rename = "Voltage")]
    pub voltage: f64,

    #[serde(rename = "Current")]
    pub current: f64,

    #[serde(rename = "Temperature")]
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmsPayload {
    #[serde(rename = "LowVoltage")]
    pub low_voltage: i32,

    #[serde(rename = "HighVoltage")]
    pub high_voltage: i32,

    #[serde(rename = "LowSoc")]
    pub low_soc: i32,

    #[serde(rename = "HighChargeCurrent")]
    pub high_charge_current: i32,

    #[serde(rename = "HighDischargeCurrent")]
    pub high_discharge_current: i32,

    #[serde(rename = "HighCurrent")]
    pub high_current: i32,

    #[serde(rename = "CellImbalance")]
    pub cell_imbalance: i32,

    #[serde(rename = "HighChargeTemperature")]
    pub high_charge_temperature: i32,

    #[serde(rename = "LowChargeTemperature")]
    pub low_charge_temperature: i32,

    #[serde(rename = "LowCellVoltage")]
    pub low_cell_voltage: i32,

    #[serde(rename = "LowTemperature")]
    pub low_temperature: i32,

    #[serde(rename = "HighTemperature")]
    pub high_temperature: i32,

    #[serde(rename = "FuseBlown")]
    pub fuse_blown: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPayload {
    #[serde(rename = "MinVoltageCellId")]
    pub min_voltage_cell_id: i32,

    #[serde(rename = "MinCellVoltage")]
    pub min_cell_voltage: f64,

    #[serde(rename = "MaxVoltageCellId")]
    pub max_voltage_cell_id: i32,

    #[serde(rename = "MaxCellVoltage")]
    pub max_cell_voltage: f64,

    #[serde(rename = "MinTemperatureCellId")]
    pub min_temperature_cell_id: i32,

    #[serde(rename = "MinCellTemperature")]
    pub min_cell_temperature: f64,

    #[serde(rename = "MaxTemperatureCellId")]
    pub max_temperature_cell_id: i32,

    #[serde(rename = "MaxCellTemperature")]
    pub max_cell_temperature: f64,

    #[serde(rename = "NrOfCellsPerBattery")]
    pub nr_of_cells_per_battery: i32,

    #[serde(rename = "NrOfModulesOnline")]
    pub nr_of_modules_online: i32,

    #[serde(rename = "NrOfModulesOffline")]
    pub nr_of_modules_offline: i32,

    #[serde(rename = "NrOfModulesBlockingCharge")]
    pub nr_of_modules_blocking_charge: i32,

    #[serde(rename = "NrOfModulesBlockingDischarge")]
    pub nr_of_modules_blocking_discharge: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoPayload {
    #[serde(rename = "AllowToCharge")]
    pub allow_to_charge: i32,

    #[serde(rename = "AllowToDischarge")]
    pub allow_to_discharge: i32,

    #[serde(rename = "AllowToBalance")]
    pub allow_to_balance: i32,

    #[serde(rename = "ExternalRelay")]
    pub external_relay: i32,
}
