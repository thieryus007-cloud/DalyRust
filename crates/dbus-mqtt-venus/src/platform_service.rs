//! Service D-Bus `com.victronenergy.platform` (singleton).
//!
//! Service custom exposant l'état des opérations de backup/restore Pi5.
//!
//! ## Chemins D-Bus exposés
//!
//! ```text
//! /Backup/Status      — 0=idle, 1=running, 2=OK, 3=error
//! /Backup/LastRun     — timestamp Unix (secondes)
//! /Restore/Status     — 0=idle, 1=running, 2=OK, 3=error
//! /Restore/LastRun    — timestamp Unix (secondes)
//! /Connected
//! /ProductName
//! /ProductId
//! /DeviceInstance
//! /Mgmt/ProcessName
//! /Mgmt/ProcessVersion
//! /Mgmt/Connection
//! ```

use crate::types::PlatformPayload;
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

const VICTRON_PLATFORM_SERVICE: &str = "com.victronenergy.platform";

// =============================================================================
// Item D-Bus
// =============================================================================

#[derive(Debug, Clone)]
pub struct DbusItem {
    pub value: serde_json::Value,
    pub text:  String,
}

impl DbusItem {
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

/// État courant du service platform exposé sur D-Bus.
#[derive(Debug, Clone)]
pub struct PlatformValues {
    pub connected:       i32,
    pub backup_status:   i32,
    pub backup_last_run: i64,
    pub restore_status:  i32,
    pub restore_last_run: i64,
    pub product_name:    String,
    pub device_instance: u32,
    pub last_update:     Instant,
}

impl PlatformValues {
    pub fn disconnected(device_instance: u32, product_name: String) -> Self {
        Self {
            connected:        0,
            backup_status:    0,
            backup_last_run:  0,
            restore_status:   0,
            restore_last_run: 0,
            product_name,
            device_instance,
            last_update:      Instant::now(),
        }
    }

    pub fn from_payload(
        payload:         &PlatformPayload,
        device_instance: u32,
        product_name:    String,
    ) -> Self {
        Self {
            connected:        1,
            backup_status:    payload.backup.as_ref().map(|b| b.status).unwrap_or(0),
            backup_last_run:  payload.backup.as_ref().map(|b| b.last_run).unwrap_or(0),
            restore_status:   payload.restore.as_ref().map(|r| r.status).unwrap_or(0),
            restore_last_run: payload.restore.as_ref().map(|r| r.last_run).unwrap_or(0),
            product_name,
            device_instance,
            last_update:      Instant::now(),
        }
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

        // Backup Pi5
        m.insert("/Backup/Status".into(),   DbusItem::i32(self.backup_status));
        m.insert("/Backup/LastRun".into(),  DbusItem::i64(self.backup_last_run));

        // Restore Pi5
        m.insert("/Restore/Status".into(),  DbusItem::i32(self.restore_status));
        m.insert("/Restore/LastRun".into(), DbusItem::i64(self.restore_last_run));

        m
    }
}

// =============================================================================
// Interface D-Bus — objet racine `/`
// =============================================================================

struct PlatformRootIface {
    values: Arc<Mutex<PlatformValues>>,
}

#[zbus::interface(name = "com.victronenergy.BusItem")]
impl PlatformRootIface {
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
    values: Arc<Mutex<PlatformValues>>,
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

pub struct PlatformServiceHandle {
    pub service_name:    String,
    pub device_instance: u32,
    pub values:          Arc<Mutex<PlatformValues>>,
    connection:          Connection,
    pub product_name:    String,
}

impl PlatformServiceHandle {
    pub async fn update(&self, payload: &PlatformPayload) -> Result<()> {
        let new_values = PlatformValues::from_payload(
            payload,
            self.device_instance,
            self.product_name.clone(),
        );
        let items = new_values.to_items();
        { *self.values.lock().unwrap() = new_values; }
        self.emit_items_changed(&items).await?;
        debug!(service = %self.service_name, "D-Bus ItemsChanged platform émis");
        Ok(())
    }

    pub async fn set_disconnected(&self) -> Result<()> {
        let items = {
            let mut g = self.values.lock().unwrap();
            g.connected = 0;
            g.to_items()
        };
        warn!(service = %self.service_name, "Platform déconnectée — watchdog timeout");
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
        match PlatformRootIface::items_changed(&ctx, dict).await {
            Ok(_)  => { debug!(service = %self.service_name, "ItemsChanged platform émis"); Ok(()) }
            Err(e) => { warn!(service = %self.service_name, "ItemsChanged warning : {}", e); Ok(()) }
        }
    }
}

// =============================================================================
// Création du service
// =============================================================================

pub async fn create_platform_service(
    dbus_bus:        &str,
    device_instance: u32,
    product_name:    String,
) -> Result<PlatformServiceHandle> {
    let service_name = VICTRON_PLATFORM_SERVICE.to_string();

    info!(
        service = %service_name,
        device_instance = device_instance,
        "Enregistrement service D-Bus platform Venus OS"
    );

    let initial_values = Arc::new(Mutex::new(
        PlatformValues::disconnected(device_instance, product_name.clone())
    ));

    let root = PlatformRootIface { values: initial_values.clone() };

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
        "Service D-Bus platform enregistré ({} chemins + racine /)",
        leaf_paths.len()
    );

    Ok(PlatformServiceHandle {
        service_name,
        device_instance,
        values: initial_values,
        connection: conn,
        product_name,
    })
}
