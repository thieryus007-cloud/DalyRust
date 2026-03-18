//! Service D-Bus `com.victronenergy.heatpump.{name}` pour pompes à chaleur
//! et chauffe-eau.
//!
//! Conforme au wiki Victron Venus OS — section Heatpump :
//! <https://github.com/victronenergy/venus/wiki/dbus#heatpump>
//!
//! ## Chemins D-Bus exposés
//!
//! ```text
//! /State              — état de la pompe à chaleur (enum Victron TBD)
//! /Temperature        — température eau courante °C (optionnel)
//! /TargetTemperature  — température eau cible °C (optionnel)
//! /Ac/Power           — puissance consommée W
//! /Ac/Energy/Forward  — énergie totale consommée kWh
//! /Position           — 0=AC Output, 1=AC Input
//! /Connected          — 0 ou 1
//! /ProductName
//! /ProductId
//! /DeviceInstance
//! /Mgmt/ProcessName
//! /Mgmt/ProcessVersion
//! /Mgmt/Connection
//! ```
//!
//! Utilisé pour :
//! - Chauffe-eau (avec sonde de température et contrôle cible)
//! - Pompe à chaleur LG (future intégration)

use crate::types::HeatpumpPayload;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info, warn};
use zbus::{connection, object_server::SignalContext, Connection};
use zvariant::{OwnedValue, Str};

// =============================================================================
// Constante
// =============================================================================

const VICTRON_HEATPUMP_PREFIX: &str = "com.victronenergy.heatpump";

// =============================================================================
// Item D-Bus
// =============================================================================

#[derive(Debug, Clone)]
pub struct DbusItem {
    pub value: serde_json::Value,
    pub text:  String,
}

impl DbusItem {
    pub fn f64(v: f64, unit: &str) -> Self {
        Self { value: serde_json::Value::from(v), text: format!("{:.1} {}", v, unit) }
    }
    pub fn i32(v: i32) -> Self {
        Self { value: serde_json::Value::from(v), text: v.to_string() }
    }
    pub fn str(v: &str) -> Self {
        Self { value: serde_json::Value::from(v), text: v.to_string() }
    }
    pub fn u32(v: u32) -> Self {
        Self { value: serde_json::Value::from(v), text: v.to_string() }
    }
}

fn json_to_owned(v: &serde_json::Value) -> OwnedValue {
    match v {
        serde_json::Value::Number(n) => {
            if n.is_f64() {
                OwnedValue::from(n.as_f64().unwrap_or(0.0))
            } else if n.is_u64() {
                OwnedValue::from(n.as_u64().unwrap_or(0) as u32)
            } else {
                let i = n.as_i64().unwrap_or(0);
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    OwnedValue::from(i as i32)
                } else {
                    OwnedValue::from(i)
                }
            }
        }
        serde_json::Value::String(s) => OwnedValue::from(Str::from(s.clone())),
        _ => OwnedValue::from(0i32),
    }
}

fn item_to_inner(item: &DbusItem) -> HashMap<String, OwnedValue> {
    let mut d = HashMap::new();
    d.insert("Value".to_string(), json_to_owned(&item.value));
    d.insert("Text".to_string(), OwnedValue::from(Str::from(item.text.clone())));
    d
}

type ItemsDict = HashMap<String, HashMap<String, OwnedValue>>;

// =============================================================================
// Valeurs courantes
// =============================================================================

/// État courant d'une pompe à chaleur / chauffe-eau exposé sur D-Bus.
#[derive(Debug, Clone)]
pub struct HeatpumpValues {
    pub connected:          i32,
    pub state:              i32,
    pub temperature:        Option<f64>,
    pub target_temperature: Option<f64>,
    pub ac_power:           f64,
    pub ac_energy_forward:  f64,
    pub position:           i32,
    pub product_name:       String,
    pub device_instance:    u32,
    pub last_update:        Instant,
}

impl HeatpumpValues {
    pub fn disconnected(device_instance: u32, product_name: String) -> Self {
        Self {
            connected:          0,
            state:              0,
            temperature:        None,
            target_temperature: None,
            ac_power:           0.0,
            ac_energy_forward:  0.0,
            position:           0,
            product_name,
            device_instance,
            last_update:        Instant::now(),
        }
    }

    pub fn from_payload(
        payload:         &HeatpumpPayload,
        device_instance: u32,
        product_name:    String,
    ) -> Self {
        let ac_power         = payload.ac.as_ref().map(|a| a.power).unwrap_or(0.0);
        let ac_energy_forward = payload.ac.as_ref()
            .and_then(|a| a.energy.as_ref())
            .map(|e| e.forward)
            .unwrap_or(0.0);

        Self {
            connected: 1,
            state: payload.state,
            temperature: payload.temperature,
            target_temperature: payload.target_temperature,
            ac_power,
            ac_energy_forward,
            position: payload.position,
            product_name,
            device_instance,
            last_update: Instant::now(),
        }
    }

    pub fn to_items(&self) -> HashMap<String, DbusItem> {
        let mut m = HashMap::new();

        // Identification
        m.insert("/Mgmt/ProcessName".into(),    DbusItem::str("daly-bms-venus"));
        m.insert("/Mgmt/ProcessVersion".into(), DbusItem::str(env!("CARGO_PKG_VERSION")));
        m.insert("/Mgmt/Connection".into(),     DbusItem::str("MQTT"));
        m.insert("/ProductId".into(),           DbusItem::u32(0));
        m.insert("/ProductName".into(),         DbusItem::str(&self.product_name));
        m.insert("/DeviceInstance".into(),      DbusItem::u32(self.device_instance));
        m.insert("/Connected".into(),           DbusItem::i32(self.connected));

        // Heatpump (chemins officiels wiki Victron)
        m.insert("/State".into(),    DbusItem::i32(self.state));
        m.insert("/Position".into(), DbusItem::i32(self.position));
        m.insert("/Ac/Power".into(),          DbusItem::f64(self.ac_power, "W"));
        m.insert("/Ac/Energy/Forward".into(), DbusItem::f64(self.ac_energy_forward, "kWh"));

        // Températures — toujours publiées (0.0 si absentes) pour que
        // Venus OS les connaisse dès GetItems()
        if let Some(t) = self.temperature {
            m.insert("/Temperature".into(), DbusItem::f64(t, "°C"));
        }
        if let Some(tt) = self.target_temperature {
            m.insert("/TargetTemperature".into(), DbusItem::f64(tt, "°C"));
        }

        m
    }
}

// =============================================================================
// Interface D-Bus — objet racine `/`
// =============================================================================

struct HeatpumpRootIface {
    values: Arc<Mutex<HeatpumpValues>>,
}

#[zbus::interface(name = "com.victronenergy.BusItem")]
impl HeatpumpRootIface {
    fn get_items(&self) -> ItemsDict {
        let guard = self.values.lock().unwrap();
        guard
            .to_items()
            .iter()
            .map(|(path, item)| (path.clone(), item_to_inner(item)))
            .collect()
    }

    fn get_value(&self) -> OwnedValue { OwnedValue::from(0i32) }
    fn get_text(&self) -> String { String::new() }
    fn set_value(&self, _val: zvariant::Value<'_>) -> i32 { 1 }

    #[zbus(signal)]
    async fn items_changed(
        ctx:   &SignalContext<'_>,
        items: ItemsDict,
    ) -> zbus::Result<()>;
}

// =============================================================================
// Interface D-Bus — objet feuille
// =============================================================================

struct BusItemLeaf {
    path:   String,
    values: Arc<Mutex<HeatpumpValues>>,
}

#[zbus::interface(name = "com.victronenergy.BusItem")]
impl BusItemLeaf {
    fn get_value(&self) -> OwnedValue {
        let guard = self.values.lock().unwrap();
        match guard.to_items().get(&self.path) {
            Some(item) => json_to_owned(&item.value),
            None       => OwnedValue::from(0i32),
        }
    }

    fn get_text(&self) -> String {
        let guard = self.values.lock().unwrap();
        guard.to_items().get(&self.path).map(|i| i.text.clone()).unwrap_or_default()
    }

    fn set_value(&self, _val: zvariant::Value<'_>) -> i32 { 1 }
}

// =============================================================================
// Handle
// =============================================================================

pub struct HeatpumpServiceHandle {
    pub service_name:    String,
    pub device_instance: u32,
    pub values:          Arc<Mutex<HeatpumpValues>>,
    connection:          Connection,
    pub product_name:    String,
}

impl HeatpumpServiceHandle {
    pub async fn update(&self, payload: &HeatpumpPayload) -> Result<()> {
        let new_values = HeatpumpValues::from_payload(
            payload,
            self.device_instance,
            self.product_name.clone(),
        );
        let items = new_values.to_items();
        { *self.values.lock().unwrap() = new_values; }
        self.emit_items_changed(&items).await?;
        debug!(service = %self.service_name, state = payload.state, "D-Bus ItemsChanged heatpump émis");
        Ok(())
    }

    pub async fn set_disconnected(&self) -> Result<()> {
        let items = {
            let mut g = self.values.lock().unwrap();
            g.connected = 0;
            g.to_items()
        };
        warn!(service = %self.service_name, "Heatpump déconnectée — watchdog timeout");
        self.emit_items_changed(&items).await
    }

    pub async fn republish(&self) -> Result<()> {
        let items = { self.values.lock().unwrap().to_items() };
        self.emit_items_changed(&items).await
    }

    async fn emit_items_changed(&self, items: &HashMap<String, DbusItem>) -> Result<()> {
        let dict: ItemsDict = items
            .iter()
            .map(|(p, i)| (p.clone(), item_to_inner(i)))
            .collect();
        let ctx = SignalContext::new(&self.connection, "/")?;
        match HeatpumpRootIface::items_changed(&ctx, dict).await {
            Ok(_)  => { debug!(service = %self.service_name, "ItemsChanged heatpump émis"); Ok(()) }
            Err(e) => { warn!(service = %self.service_name, "ItemsChanged warning : {}", e); Ok(()) }
        }
    }
}

// =============================================================================
// Création du service
// =============================================================================

pub async fn create_heatpump_service(
    dbus_bus:        &str,
    service_suffix:  &str,
    device_instance: u32,
    product_name:    String,
) -> Result<HeatpumpServiceHandle> {
    let service_name = format!("{}.{}", VICTRON_HEATPUMP_PREFIX, service_suffix);

    info!(
        service = %service_name,
        device_instance = device_instance,
        "Enregistrement service D-Bus heatpump Venus OS"
    );

    let initial_values = Arc::new(Mutex::new(
        HeatpumpValues::disconnected(device_instance, product_name.clone())
    ));

    let root = HeatpumpRootIface { values: initial_values.clone() };

    let builder = match dbus_bus {
        "session" => connection::Builder::session()?,
        _         => connection::Builder::system()?,
    };

    let conn = builder
        .name(service_name.as_str())?
        .serve_at("/", root)?
        .build()
        .await?;

    let leaf_paths: Vec<String> = {
        initial_values.lock().unwrap().to_items().into_keys().collect()
    };

    for path in &leaf_paths {
        conn.object_server()
            .at(path.as_str(), BusItemLeaf { path: path.clone(), values: initial_values.clone() })
            .await?;
    }

    info!(
        service = %service_name,
        paths   = leaf_paths.len(),
        "Service D-Bus heatpump enregistré ({} chemins + racine /)",
        leaf_paths.len()
    );

    Ok(HeatpumpServiceHandle {
        service_name,
        device_instance,
        values: initial_values,
        connection: conn,
        product_name,
    })
}
