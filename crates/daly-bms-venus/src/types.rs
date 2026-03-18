//! Types du payload Venus OS reçu depuis MQTT.
//!
//! Ce module est un miroir de `build_venus_payload()` dans `bridges/mqtt.rs`.
//! Les champs correspondent exactement au JSON publié sur `{prefix}/{n}/venus`.

use serde::{Deserialize, Serialize};

// =============================================================================
// Payload capteurs de température (santuario/heat/{n}/venus)
// =============================================================================

/// Payload pour capteurs de température/chaleur.
///
/// Publié par Node-RED (Open-Meteo, capteurs physiques…) sur
/// `santuario/heat/{n}/venus` et consommé par `SensorManager`.
///
/// Chemins D-Bus Venus OS cibles : `com.victronenergy.temperature.{n}`
///   /Temperature      °C
///   /TemperatureType  0=battery 1=fridge 2=generic 3=Room 4=Outdoor 5=WaterHeater 6=Freezer
///   /Humidity         %
///   /Pressure         kPa
///   /CustomName       string
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatPayload {
    /// Température en degrés Celsius.
    #[serde(rename = "Temperature")]
    pub temperature: f64,

    /// Type de capteur : 0=battery, 1=fridge, 2=generic, 3=Room,
    /// 4=Outdoor, 5=WaterHeater, 6=Freezer.
    /// Peut être surchargé par la config `[[sensors]]`.
    #[serde(rename = "TemperatureType", default)]
    pub temperature_type: i32,

    /// Humidité relative en % (optionnelle — ex: sonde extérieure).
    #[serde(rename = "Humidity", default)]
    pub humidity: Option<f64>,

    /// Pression atmosphérique en kPa (optionnelle).
    #[serde(rename = "Pressure", default)]
    pub pressure: Option<f64>,

    /// Nom personnalisé affiché dans Venus OS (optionnel).
    #[serde(rename = "CustomName", default)]
    pub custom_name: Option<String>,
}

// =============================================================================
// Payload pompe à chaleur / chauffe-eau (santuario/heatpump/{n}/venus)
// =============================================================================

/// Payload pour pompes à chaleur et chauffe-eau.
///
/// Publié par Node-RED sur `santuario/heatpump/{n}/venus`.
/// Cible D-Bus : `com.victronenergy.heatpump.{n}`
///
/// Chemins D-Bus exposés (wiki Victron — Heatpump) :
///   /State              enum état de la pompe (TBD Victron)
///   /Temperature        température eau courante °C (optionnelle)
///   /TargetTemperature  température eau cible °C (optionnelle)
///   /Ac/Power           puissance consommée W
///   /Ac/Energy/Forward  énergie totale consommée kWh
///   /Position           0=AC Output, 1=AC Input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatpumpPayload {
    /// État de la pompe à chaleur (enum Victron, TBD).
    #[serde(rename = "State", default)]
    pub state: i32,

    /// Température eau courante °C (optionnelle).
    #[serde(rename = "Temperature", default)]
    pub temperature: Option<f64>,

    /// Température eau cible °C (optionnelle).
    #[serde(rename = "TargetTemperature", default)]
    pub target_temperature: Option<f64>,

    /// Données électriques AC.
    #[serde(rename = "Ac", default)]
    pub ac: Option<HeatpumpAcPayload>,

    /// Position : 0=AC Output, 1=AC Input.
    #[serde(rename = "Position", default)]
    pub position: i32,
}

/// Sous-payload AC pour la pompe à chaleur.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeatpumpAcPayload {
    /// Puissance consommée en W.
    #[serde(rename = "Power", default)]
    pub power: f64,

    /// Énergie totale consommée.
    #[serde(rename = "Energy", default)]
    pub energy: Option<HeatpumpEnergyPayload>,
}

/// Sous-payload énergie pour la pompe à chaleur.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeatpumpEnergyPayload {
    /// Énergie totale consommée en kWh.
    #[serde(rename = "Forward", default)]
    pub forward: f64,
}

// =============================================================================
// Payload capteur météo / irradiance (santuario/meteo/venus)
// =============================================================================

/// Payload pour capteur d'irradiance et données météo.
///
/// Publié par Node-RED (capteur RS485 sur Pi5) sur `santuario/meteo/venus`.
/// Cible D-Bus : `com.victronenergy.meteo`
///
/// Chemins D-Bus exposés (wiki Victron — Meteo) :
///   /Irradiance    irradiance courante en W/m²
///   /TodaysYield   production du jour en kWh (depuis le lever du soleil)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoPayload {
    /// Irradiance courante en W/m².
    #[serde(rename = "Irradiance", default)]
    pub irradiance: f64,

    /// Production du jour en kWh (depuis le lever du soleil).
    #[serde(rename = "TodaysYield", default)]
    pub todays_yield: f64,
}

// =============================================================================
// Payload batteries (santuario/bms/{n}/venus)
// =============================================================================

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
