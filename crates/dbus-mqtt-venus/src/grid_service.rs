//! Service D-Bus `com.victronenergy.grid.{name}` ou `com.victronenergy.acload.{name}`
//! pour compteurs d'énergie AC (réseau, consommation).
//!
//! Conforme au wiki Victron Venus OS — section Grid/ACload meter :
//! <https://github.com/victronenergy/venus/wiki/dbus#grid-and-acload-and-genset-meter>
//!
//! ## Chemins D-Bus exposés
//!
//! ```text
//! /Ac/L1/Current         — A AC
//! /Ac/L1/Energy/Forward  — kWh consommés (import)
//! /Ac/L1/Energy/Reverse  — kWh injectés (export)
//! /Ac/L1/Power           — W (puissance réelle)
//! /Ac/L1/Voltage         — V AC
//! /Ac/L2/...             — Phase 2 (même structure, enregistrée à 0.0 si monophasé)
//! /Ac/L3/...             — Phase 3 (même structure, enregistrée à 0.0 si monophasé)
//! /DeviceType            — type de compteur (340 = generic energy meter)
//! /IsGenericEnergyMeter  — 1 si masquerade en genset/acload
//! /Connected
//! /ProductName
//! /ProductId
//! /DeviceInstance
//! /Mgmt/ProcessName
//! /Mgmt/ProcessVersion
//! /Mgmt/Connection
//! ```

use crate::types::{GridPayload, GridPhasePayload};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info, warn};
use zbus::{connection, object_server::SignalContext, Connection};
use zvariant::{OwnedValue, Str};

// =============================================================================
// Constantes
// =============================================================================

const VICTRON_GRID_PREFIX:   &str = "com.victronenergy.grid";
const VICTRON_ACLOAD_PREFIX: &str = "com.victronenergy.acload";

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
        Self { value: serde_json::Value::from(v), text: format!("{:.2} {}", v, unit) }
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
// Valeurs par phase
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct PhaseValues {
    pub voltage:        f64,
    pub current:        f64,
    pub power:          f64,
    pub energy_forward: f64,
    pub energy_reverse: f64,
}

impl PhaseValues {
    fn from_payload(p: &GridPhasePayload) -> Self {
        Self {
            voltage:        p.voltage,
            current:        p.current,
            power:          p.power,
            energy_forward: p.energy.as_ref().map(|e| e.forward).unwrap_or(0.0),
            energy_reverse: p.energy.as_ref().map(|e| e.reverse).unwrap_or(0.0),
        }
    }
}

// =============================================================================
// Valeurs courantes
// =============================================================================

/// État courant d'un compteur réseau/acload exposé sur D-Bus.
#[derive(Debug, Clone)]
pub struct GridValues {
    pub connected:              i32,
    pub l1:                     PhaseValues,
    pub l2:                     PhaseValues,
    pub l3:                     PhaseValues,
    pub device_type:            i32,
    pub is_generic_energy_meter: i32,
    pub product_name:           String,
    pub device_instance:        u32,
    pub last_update:            Instant,
}

impl GridValues {
    pub fn disconnected(device_instance: u32, product_name: String) -> Self {
        Self {
            connected:              0,
            l1:                     PhaseValues::default(),
            l2:                     PhaseValues::default(),
            l3:                     PhaseValues::default(),
            device_type:            340,
            is_generic_energy_meter: 0,
            product_name,
            device_instance,
            last_update:            Instant::now(),
        }
    }

    pub fn from_payload(
        payload:         &GridPayload,
        device_instance: u32,
        product_name:    String,
    ) -> Self {
        let empty = GridPhasePayload::default();
        Self {
            connected: 1,
            l1: PhaseValues::from_payload(payload.ac.l1.as_ref().unwrap_or(&empty)),
            l2: PhaseValues::from_payload(payload.ac.l2.as_ref().unwrap_or(&empty)),
            l3: PhaseValues::from_payload(payload.ac.l3.as_ref().unwrap_or(&empty)),
            device_type:            payload.device_type,
            is_generic_energy_meter: payload.is_generic_energy_meter,
            product_name,
            device_instance,
            last_update:            Instant::now(),
        }
    }

    fn phase_items(items: &mut HashMap<String, DbusItem>, prefix: &str, ph: &PhaseValues) {
        items.insert(format!("{}/Voltage", prefix),        DbusItem::f64(ph.voltage,        "V"));
        items.insert(format!("{}/Current", prefix),        DbusItem::f64(ph.current,        "A"));
        items.insert(format!("{}/Power", prefix),          DbusItem::f64(ph.power,          "W"));
        items.insert(format!("{}/Energy/Forward", prefix), DbusItem::f64(ph.energy_forward, "kWh"));
        items.insert(format!("{}/Energy/Reverse", prefix), DbusItem::f64(ph.energy_reverse, "kWh"));
    }

    pub fn to_items(&self) -> HashMap<String, DbusItem> {
        let mut m = HashMap::new();

        // Identification
        m.insert("/Mgmt/ProcessName".into(),    DbusItem::str("dbus-mqtt-venus"));
        m.insert("/Mgmt/ProcessVersion".into(), DbusItem::str(env!("CARGO_PKG_VERSION")));
        m.insert("/Mgmt/Connection".into(),     DbusItem::str("MQTT"));
        m.insert("/ProductId".into(),           DbusItem::u32(0));
        m.insert("/ProductName".into(),         DbusItem::str(&self.product_name));
        m.insert("/DeviceInstance".into(),      DbusItem::u32(self.device_instance));
        m.insert("/Connected".into(),           DbusItem::i32(self.connected));

        // Metadata compteur
        m.insert("/DeviceType".into(),            DbusItem::i32(self.device_type));
        m.insert("/IsGenericEnergyMeter".into(),  DbusItem::i32(self.is_generic_energy_meter));

        // Phases — toujours toutes les 3 présentes (0.0 si monophasé ou non reçu)
        // Obligatoire : les chemins doivent être enregistrés dès le démarrage (GetValue)
        Self::phase_items(&mut m, "/Ac/L1", &self.l1);
        Self::phase_items(&mut m, "/Ac/L2", &self.l2);
        Self::phase_items(&mut m, "/Ac/L3", &self.l3);

        m
    }
}

// =============================================================================
// Interface D-Bus — objet racine `/`
// =============================================================================

struct GridRootIface {
    values: Arc<Mutex<GridValues>>,
}

#[zbus::interface(name = "com.victronenergy.BusItem")]
impl GridRootIface {
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
    values: Arc<Mutex<GridValues>>,
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

pub struct GridServiceHandle {
    pub service_name:    String,
    pub device_instance: u32,
    pub values:          Arc<Mutex<GridValues>>,
    connection:          Connection,
    pub product_name:    String,
}

impl GridServiceHandle {
    pub async fn update(&self, payload: &GridPayload) -> Result<()> {
        let new_values = GridValues::from_payload(
            payload,
            self.device_instance,
            self.product_name.clone(),
        );
        let items = new_values.to_items();
        { *self.values.lock().unwrap() = new_values; }
        self.emit_items_changed(&items).await?;
        debug!(
            service = %self.service_name,
            power_l1 = payload.ac.l1.as_ref().map(|p| p.power).unwrap_or(0.0),
            "D-Bus ItemsChanged grid/acload émis"
        );
        Ok(())
    }

    pub async fn set_disconnected(&self) -> Result<()> {
        let items = {
            let mut g = self.values.lock().unwrap();
            g.connected = 0;
            g.to_items()
        };
        warn!(service = %self.service_name, "Grid/acload déconnecté — watchdog timeout");
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
        match GridRootIface::items_changed(&ctx, dict).await {
            Ok(_)  => { debug!(service = %self.service_name, "ItemsChanged grid émis"); Ok(()) }
            Err(e) => { warn!(service = %self.service_name, "ItemsChanged warning : {}", e); Ok(()) }
        }
    }
}

// =============================================================================
// Création du service
// =============================================================================

/// Crée un service grid ou acload selon `service_type` ("grid" ou "acload").
pub async fn create_grid_service(
    dbus_bus:        &str,
    service_suffix:  &str,
    device_instance: u32,
    product_name:    String,
    service_type:    &str,
) -> Result<GridServiceHandle> {
    let prefix = if service_type == "acload" {
        VICTRON_ACLOAD_PREFIX
    } else {
        VICTRON_GRID_PREFIX
    };
    let service_name = format!("{}.{}", prefix, service_suffix);

    info!(
        service = %service_name,
        device_instance = device_instance,
        service_type    = service_type,
        "Enregistrement service D-Bus grid/acload Venus OS"
    );

    let initial_values = Arc::new(Mutex::new(
        GridValues::disconnected(device_instance, product_name.clone())
    ));

    let root = GridRootIface { values: initial_values.clone() };

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
        "Service D-Bus grid/acload enregistré ({} chemins + racine /)",
        leaf_paths.len()
    );

    Ok(GridServiceHandle {
        service_name,
        device_instance,
        values: initial_values,
        connection: conn,
        product_name,
    })
}
