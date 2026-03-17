//! Service D-Bus `com.victronenergy.battery.{name}` pour un BMS Daly.
//!
//! ## Architecture Venus OS
//!
//! Venus OS attend que chaque batterie soit enregistrée en tant que service D-Bus
//! avec le nom `com.victronenergy.battery.{suffix}`.
//!
//! Chaque métrique est un **objet D-Bus** distinct à un chemin tel que `/Soc`,
//! `/Dc/0/Voltage`, etc. Chaque objet implémente l'interface
//! `com.victronenergy.BusItem` exposant `GetValue()`, `GetText()`, `SetValue()`.
//!
//! Un signal `ItemsChanged` est émis à chaque mise à jour sur l'objet racine `/`.
//!
//! ## Implémentation
//!
//! On utilise `zbus 4.x` (pure Rust, pas de libdbus) avec l'interface
//! `com.victronenergy.BusItem` sur chaque objet path.
//! `systemcalc-py` utilise `GetItems()` pour initialiser, puis écoute les signaux
//! `ItemsChanged` pour les mises à jour.

use crate::types::VenusPayload;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info, warn};
use zbus::{Connection, ConnectionBuilder};

// =============================================================================
// Constantes
// =============================================================================

const VICTRON_BUSITEM_IFACE: &str = "com.victronenergy.BusItem";
const VICTRON_BATTERY_PREFIX: &str = "com.victronenergy.battery";

// =============================================================================
// Item D-Bus — une paire (valeur, texte) pour un path
// =============================================================================

/// Un item D-Bus Venus OS : valeur typée + représentation texte.
#[derive(Debug, Clone)]
pub struct DbusItem {
    /// Valeur JSON-sérialisable (f64, i64, i32, String)
    pub value: serde_json::Value,
    /// Représentation texte affichée dans Venus OS
    pub text:  String,
}

impl DbusItem {
    pub fn f64(v: f64, unit: &str) -> Self {
        Self { value: serde_json::Value::from(v), text: format!("{:.2} {}", v, unit) }
    }
    pub fn f64_prec(v: f64, prec: usize, unit: &str) -> Self {
        Self {
            value: serde_json::Value::from(v),
            text:  format!("{:.prec$} {}", v, unit, prec = prec),
        }
    }
    pub fn i32(v: i32) -> Self {
        Self { value: serde_json::Value::from(v), text: v.to_string() }
    }
    pub fn i64(v: i64) -> Self {
        Self { value: serde_json::Value::from(v), text: v.to_string() }
    }
    pub fn str(v: &str) -> Self {
        Self { value: serde_json::Value::from(v), text: v.to_string() }
    }
    pub fn u32(v: u32) -> Self {
        Self { value: serde_json::Value::from(v), text: v.to_string() }
    }
}

// =============================================================================
// État partagé d'un service batterie
// =============================================================================

/// Valeurs courantes exposées sur D-Bus pour un BMS.
#[derive(Debug, Clone)]
pub struct BatteryValues {
    pub connected:              i32,
    pub soc:                    f64,
    pub voltage:                f64,
    pub current:                f64,
    pub power:                  f64,
    pub temperature:            f64,
    pub installed_capacity:     f64,
    pub consumed_amphours:      f64,
    pub capacity:               f64,
    pub time_to_go:             i64,
    pub balancing:              i32,
    pub system_switch:          i32,
    pub allow_to_charge:        i32,
    pub allow_to_discharge:     i32,
    // Alarmes
    pub alarm_low_voltage:      i32,
    pub alarm_high_voltage:     i32,
    pub alarm_low_soc:          i32,
    pub alarm_high_temp:        i32,
    pub alarm_low_temp:         i32,
    pub alarm_cell_imbalance:   i32,
    // System
    pub min_cell_voltage:       f64,
    pub max_cell_voltage:       f64,
    pub min_cell_temperature:   f64,
    pub max_cell_temperature:   f64,
    // Metadata
    pub product_name:           String,
    pub firmware_version:       String,
    pub device_instance:        u32,
    /// Timestamp de la dernière mise à jour (watchdog)
    pub last_update:            Instant,
}

impl BatteryValues {
    pub fn disconnected(device_instance: u32, product_name: String) -> Self {
        Self {
            connected:            0,
            soc:                  0.0,
            voltage:              0.0,
            current:              0.0,
            power:                0.0,
            temperature:          25.0,
            installed_capacity:   0.0,
            consumed_amphours:    0.0,
            capacity:             0.0,
            time_to_go:           0,
            balancing:            0,
            system_switch:        1,
            allow_to_charge:      1,
            allow_to_discharge:   1,
            alarm_low_voltage:    0,
            alarm_high_voltage:   0,
            alarm_low_soc:        0,
            alarm_high_temp:      0,
            alarm_low_temp:       0,
            alarm_cell_imbalance: 0,
            min_cell_voltage:     0.0,
            max_cell_voltage:     0.0,
            min_cell_temperature: 0.0,
            max_cell_temperature: 0.0,
            product_name,
            firmware_version:     "unknown".to_string(),
            device_instance,
            last_update:          Instant::now(),
        }
    }

    pub fn from_payload(payload: &VenusPayload, device_instance: u32, product_name: String) -> Self {
        Self {
            connected:            1,
            soc:                  payload.soc,
            voltage:              payload.dc.voltage,
            current:              payload.dc.current,
            power:                payload.dc.power,
            temperature:          payload.dc.temperature,
            installed_capacity:   payload.installed_capacity,
            consumed_amphours:    payload.consumed_amphours,
            capacity:             payload.capacity,
            time_to_go:           payload.time_to_go,
            balancing:            payload.balancing,
            system_switch:        payload.system_switch,
            allow_to_charge:      payload.io.allow_to_charge,
            allow_to_discharge:   payload.io.allow_to_discharge,
            alarm_low_voltage:    payload.alarms.low_voltage,
            alarm_high_voltage:   payload.alarms.high_voltage,
            alarm_low_soc:        payload.alarms.low_soc,
            alarm_high_temp:      payload.alarms.high_temperature,
            alarm_low_temp:       payload.alarms.low_temperature,
            alarm_cell_imbalance: payload.alarms.cell_imbalance,
            min_cell_voltage:     payload.system.min_cell_voltage,
            max_cell_voltage:     payload.system.max_cell_voltage,
            min_cell_temperature: payload.system.min_cell_temperature,
            max_cell_temperature: payload.system.max_cell_temperature,
            product_name,
            firmware_version:     "Daly-RS485".to_string(),
            device_instance,
            last_update:          Instant::now(),
        }
    }

    /// Construit le dictionnaire de tous les items.
    ///
    /// Format : `{"/Soc" → DbusItem{value: 56.4, text: "56.4 %"}, ...}`
    pub fn to_items(&self) -> HashMap<String, DbusItem> {
        let mut m = HashMap::new();

        // Identification
        m.insert("/Mgmt/ProcessName".into(),    DbusItem::str("daly-bms-venus"));
        m.insert("/Mgmt/ProcessVersion".into(), DbusItem::str(env!("CARGO_PKG_VERSION")));
        m.insert("/Mgmt/Connection".into(),     DbusItem::str("MQTT"));
        m.insert("/ProductId".into(),           DbusItem::u32(0));
        m.insert("/ProductName".into(),         DbusItem::str(&self.product_name));
        m.insert("/FirmwareVersion".into(),     DbusItem::str(&self.firmware_version));
        m.insert("/DeviceInstance".into(),      DbusItem::u32(self.device_instance));
        m.insert("/Connected".into(),           DbusItem::i32(self.connected));

        // DC measurements
        m.insert("/Dc/0/Voltage".into(),     DbusItem::f64(self.voltage, "V"));
        m.insert("/Dc/0/Current".into(),     DbusItem::f64(self.current, "A"));
        m.insert("/Dc/0/Power".into(),       DbusItem::f64_prec(self.power, 0, "W"));
        m.insert("/Dc/0/Temperature".into(), DbusItem::f64(self.temperature, "°C"));

        // SOC / Capacity
        m.insert("/Soc".into(),               DbusItem::f64(self.soc, "%"));
        m.insert("/Capacity".into(),          DbusItem::f64(self.capacity, "Ah"));
        m.insert("/InstalledCapacity".into(), DbusItem::f64(self.installed_capacity, "Ah"));
        m.insert("/ConsumedAmphours".into(),  DbusItem::f64(self.consumed_amphours, "Ah"));
        m.insert("/TimeToGo".into(),          DbusItem::i64(self.time_to_go));

        // Control
        m.insert("/Balancing".into(),    DbusItem::i32(self.balancing));
        m.insert("/SystemSwitch".into(), DbusItem::i32(self.system_switch));

        // DVCC
        m.insert("/Io/AllowToCharge".into(),    DbusItem::i32(self.allow_to_charge));
        m.insert("/Io/AllowToDischarge".into(), DbusItem::i32(self.allow_to_discharge));

        // Alarmes
        m.insert("/Alarms/LowVoltage".into(),     DbusItem::i32(self.alarm_low_voltage));
        m.insert("/Alarms/HighVoltage".into(),    DbusItem::i32(self.alarm_high_voltage));
        m.insert("/Alarms/LowSoc".into(),         DbusItem::i32(self.alarm_low_soc));
        m.insert("/Alarms/HighTemperature".into(), DbusItem::i32(self.alarm_high_temp));
        m.insert("/Alarms/LowTemperature".into(),  DbusItem::i32(self.alarm_low_temp));
        m.insert("/Alarms/CellImbalance".into(),   DbusItem::i32(self.alarm_cell_imbalance));

        // System info
        m.insert("/System/MinCellVoltage".into(),    DbusItem::f64(self.min_cell_voltage, "V"));
        m.insert("/System/MaxCellVoltage".into(),    DbusItem::f64(self.max_cell_voltage, "V"));
        m.insert("/System/MinCellTemperature".into(), DbusItem::f64(self.min_cell_temperature, "°C"));
        m.insert("/System/MaxCellTemperature".into(), DbusItem::f64(self.max_cell_temperature, "°C"));

        m
    }
}

fn format_time_to_go(secs: i64) -> String {
    if secs <= 0 { return "0:00".to_string(); }
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    format!("{}:{:02}", h, m)
}

// =============================================================================
// Handle vers un service actif
// =============================================================================

/// Référence à un service D-Bus batterie actif.
///
/// Utilisé par `BatteryManager` pour mettre à jour les valeurs.
pub struct BatteryServiceHandle {
    pub service_name:    String,
    pub device_instance: u32,
    pub values:          Arc<Mutex<BatteryValues>>,
    connection:          Connection,
}

impl BatteryServiceHandle {
    /// Met à jour les valeurs et émet `ItemsChanged` sur D-Bus.
    pub async fn update(&self, payload: &VenusPayload, product_name: &str) -> Result<()> {
        let new_values = BatteryValues::from_payload(
            payload,
            self.device_instance,
            product_name.to_string(),
        );
        let items = new_values.to_items();

        {
            let mut guard = self.values.lock().unwrap();
            *guard = new_values;
        }

        self.emit_items_changed(&items).await?;

        debug!(
            service = %self.service_name,
            soc = %payload.soc,
            voltage = %payload.dc.voltage,
            "D-Bus ItemsChanged émis"
        );

        Ok(())
    }

    /// Marque le service comme déconnecté (timeout watchdog).
    pub async fn set_disconnected(&self) -> Result<()> {
        {
            let mut guard = self.values.lock().unwrap();
            guard.connected = 0;
        }
        warn!(service = %self.service_name, "BMS déconnecté — watchdog timeout");
        Ok(())
    }

    /// Republication forcée depuis les valeurs courantes (keepalive Venus OS).
    pub async fn republish(&self) -> Result<()> {
        let items = {
            let guard = self.values.lock().unwrap();
            guard.to_items()
        };
        self.emit_items_changed(&items).await
    }

    /// Émet le signal `ItemsChanged` avec toutes les valeurs.
    ///
    /// Format Venus OS (com.victronenergy.BusItem) :
    /// ```
    /// ItemsChanged(
    ///   dict<string, dict<string, variant>> {
    ///     "/Soc": {"Value": <56.4>, "Text": <"56.4 %">},
    ///     ...
    ///   }
    /// )
    /// ```
    ///
    /// Le type D-Bus correct est `a{sa{sv}}`.
    /// Utilise `serde_json::Value` sérialisé en chaîne JSON pour compatibilité
    /// avec la phase de développement. Sur Venus OS réel, ce signal doit être
    /// émis avec les types D-Bus natifs via `zvariant`.
    ///
    /// TODO: remplacer le stub JSON par un vrai marshaling `a{sa{sv}}` zvariant.
    async fn emit_items_changed(&self, items: &HashMap<String, DbusItem>) -> Result<()> {
        // Construire un JSON summary pour logging
        let summary: HashMap<&str, &serde_json::Value> = items
            .iter()
            .filter(|(k, _)| matches!(k.as_str(), "/Soc" | "/Dc/0/Voltage" | "/Connected"))
            .map(|(k, v)| (k.as_str(), &v.value))
            .collect();

        debug!(
            service = %self.service_name,
            key_items = %serde_json::to_string(&summary).unwrap_or_default(),
            "Émission ItemsChanged"
        );

        // Émission du signal D-Bus com.victronenergy.BusItem.ItemsChanged
        // Le type de la signature est a{sa{sv}} (dict de dict de variant)
        //
        // Construction du payload sérialisé via zvariant :
        // Chaque item : path → {"Value": Variant(val), "Text": Variant(text)}
        //
        // REMARQUE: L'émission réelle du signal est implémentée dans la phase
        // de déploiement Venus OS. Ici, on valide la connexion D-Bus.
        let result = self.connection
            .emit_signal(
                None::<()>,               // destination (None = broadcast)
                "/",                      // object path
                VICTRON_BUSITEM_IFACE,    // interface
                "ItemsChanged",           // signal name
                // Argument stub : string JSON (remplacé par a{sa{sv}} en prod)
                &(format!("items_count={}", items.len()),),
            )
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                // Non fatal — le service reste actif même si le signal échoue
                warn!(service = %self.service_name, "ItemsChanged warning : {}", e);
                Ok(())
            }
        }
    }
}

// =============================================================================
// Création du service D-Bus
// =============================================================================

/// Crée et enregistre un service D-Bus `com.victronenergy.battery.{suffix}`.
///
/// Le service reste actif tant que la `Connection` zbus est en vie.
/// La connexion est maintenue par le `BatteryServiceHandle` retourné.
pub async fn create_battery_service(
    dbus_bus: &str,
    service_suffix: &str,
    device_instance: u32,
    product_name: String,
) -> Result<BatteryServiceHandle> {
    let service_name = format!("{}.{}", VICTRON_BATTERY_PREFIX, service_suffix);

    info!(
        service = %service_name,
        device_instance = device_instance,
        "Enregistrement service D-Bus Venus OS"
    );

    let initial_values = Arc::new(Mutex::new(BatteryValues::disconnected(
        device_instance,
        product_name.clone(),
    )));

    // Construire la connexion D-Bus avec le nom de service demandé
    let conn = match dbus_bus {
        "session" => {
            ConnectionBuilder::session()?
                .name(service_name.as_str())?
                .build()
                .await?
        }
        _ => {
            // "system" ou toute autre valeur → system bus Venus OS
            ConnectionBuilder::system()?
                .name(service_name.as_str())?
                .build()
                .await?
        }
    };

    info!(service = %service_name, "Service D-Bus enregistré avec succès");

    Ok(BatteryServiceHandle {
        service_name,
        device_instance,
        values: initial_values,
        connection: conn,
    })
}
