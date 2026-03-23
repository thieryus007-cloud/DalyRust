//! Service D-Bus `com.victronenergy.pvinverter.{name}`
//! pour onduleurs PV et compteurs d'énergie AC (ET112 micro-inverseurs).
//!
//! Conforme au wiki Victron Venus OS — section PV Inverter :
//! <https://github.com/victronenergy/venus/wiki/dbus#pv-inverters>
//!
//! ## Chemins D-Bus exposés
//!
//! ```text
//! /Ac/Power              — W puissance AC totale
//! /Ac/Energy/Forward     — kWh énergie produite totale
//! /Ac/L1/Voltage         — V tension L1
//! /Ac/L1/Current         — A courant L1
//! /Ac/L1/Power           — W puissance L1
//! /Ac/L1/Energy/Forward  — kWh énergie L1
//! /StatusCode            — 7=Running
//! /ErrorCode             — 0=No Error
//! /Position              — 1=AC Output
//! /IsGenericEnergyMeter  — 1 (ET112 masquerade)
//! /AllowedRoles          — liste des rôles possibles (active le menu Setup Venus OS)
//! /Role                  — rôle courant (writable)
//! /CustomName            — nom personnalisé
//! /DeviceType            — 120 (Carlo Gavazzi ET112)
//! /FirmwareVersion       — version firmware
//! /Connected
//! /ProductName
//! /ProductId
//! /DeviceInstance
//! /Mgmt/ProcessName
//! /Mgmt/ProcessVersion
//! /Mgmt/Connection
//! ```

use crate::types::PvinverterPayload;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info, warn};
use zbus::{connection, object_server::SignalContext, Connection};
use zvariant::{Array, OwnedValue, Str, Value};

// =============================================================================
// Constantes
// =============================================================================

const VICTRON_PVINVERTER_PREFIX: &str = "com.victronenergy.pvinverter";

const ALLOWED_ROLES: &[&str] = &[
    "grid", "pvinverter", "genset", "acload", "evcharger", "heatpump",
];

// =============================================================================
// Item D-Bus
// =============================================================================

/// Valeur D-Bus typée — Clone-able sans dépendre de OwnedValue::Clone.
#[derive(Debug, Clone)]
pub enum DbusValueKind {
    F64(f64),
    I32(i32),
    U32(u32),
    Str(String),
    /// Tableau de strings D-Bus (signature "as") — ex: /AllowedRoles
    StringArray(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct DbusItem {
    pub kind: DbusValueKind,
    pub text: String,
}

impl DbusItem {
    pub fn f64(v: f64, unit: &str) -> Self {
        Self { kind: DbusValueKind::F64(v), text: format!("{:.2} {}", v, unit) }
    }
    pub fn i32(v: i32) -> Self {
        Self { kind: DbusValueKind::I32(v), text: v.to_string() }
    }
    pub fn str(v: &str) -> Self {
        Self { kind: DbusValueKind::Str(v.to_string()), text: v.to_string() }
    }
    pub fn u32(v: u32) -> Self {
        Self { kind: DbusValueKind::U32(v), text: v.to_string() }
    }
    pub fn string_array(strs: &[&str]) -> Self {
        Self {
            kind: DbusValueKind::StringArray(strs.iter().map(|s| (*s).to_string()).collect()),
            text: String::new(),
        }
    }
}

fn kind_to_owned(kind: &DbusValueKind) -> OwnedValue {
    match kind {
        DbusValueKind::F64(v)          => OwnedValue::from(*v),
        DbusValueKind::I32(v)          => OwnedValue::from(*v),
        DbusValueKind::U32(v)          => OwnedValue::from(*v),
        DbusValueKind::Str(s)          => OwnedValue::from(Str::from(s.clone())),
        DbusValueKind::StringArray(ss) => build_string_array_owned(ss),
    }
}

fn item_to_inner(item: &DbusItem) -> HashMap<String, OwnedValue> {
    let mut d = HashMap::new();
    d.insert("Value".to_string(), kind_to_owned(&item.kind));
    d.insert("Text".to_string(), OwnedValue::from(Str::from(item.text.clone())));
    d
}

type ItemsDict = HashMap<String, HashMap<String, OwnedValue>>;

// =============================================================================
// Construction d'un tableau de strings D-Bus
// =============================================================================

/// Construit un `OwnedValue` contenant un tableau D-Bus de strings (signature "as").
/// Utilisé pour `/AllowedRoles` qui active le menu "Setup" dans Venus OS GUI.
fn build_string_array_owned(strings: &[String]) -> OwnedValue {
    let arr = Array::from(strings.iter().map(String::as_str).collect::<Vec<&str>>());
    OwnedValue::try_from(Value::Array(arr))
        .unwrap_or_else(|_| OwnedValue::from(0i32))
}

// =============================================================================
// Valeurs courantes
// =============================================================================

/// État courant d'un service pvinverter exposé sur D-Bus.
#[derive(Debug, Clone)]
pub struct PvinverterValues {
    pub connected:               i32,
    pub power:                   f64,
    pub energy_forward:          f64,
    pub l1_voltage:              f64,
    pub l1_current:              f64,
    pub l1_power:                f64,
    pub l1_energy_forward:       f64,
    pub status_code:             i32,
    pub error_code:              i32,
    pub position:                i32,
    pub is_generic_energy_meter: i32,
    pub product_name:            String,
    pub custom_name:             String,
    /// Rôle courant — writable par Venus OS via menu Setup.
    pub role:                    String,
    pub device_instance:         u32,
    pub last_update:             Instant,
}

impl PvinverterValues {
    pub fn disconnected(device_instance: u32, product_name: String, custom_name: String) -> Self {
        Self {
            connected:               0,
            power:                   0.0,
            energy_forward:          0.0,
            l1_voltage:              0.0,
            l1_current:              0.0,
            l1_power:                0.0,
            l1_energy_forward:       0.0,
            status_code:             7,
            error_code:              0,
            position:                1,
            is_generic_energy_meter: 1,
            product_name,
            custom_name,
            role:                    "pvinverter".to_string(),
            device_instance,
            last_update:             Instant::now(),
        }
    }

    pub fn from_payload(
        payload:         &PvinverterPayload,
        device_instance: u32,
        product_name:    String,
        custom_name:     String,
        current_role:    String,
    ) -> Self {
        let l1 = payload.ac.l1.as_ref();
        Self {
            connected:               1,
            power:                   payload.ac.power,
            energy_forward:          payload.ac.energy.as_ref().map(|e| e.forward).unwrap_or(0.0),
            l1_voltage:              l1.map(|p| p.voltage).unwrap_or(0.0),
            l1_current:              l1.map(|p| p.current).unwrap_or(0.0),
            l1_power:                l1.map(|p| p.power).unwrap_or(payload.ac.power),
            l1_energy_forward:       l1.and_then(|p| p.energy.as_ref()).map(|e| e.forward).unwrap_or(0.0),
            status_code:             payload.status_code,
            error_code:              payload.error_code,
            position:                payload.position,
            is_generic_energy_meter: payload.is_generic_energy_meter,
            product_name:            payload.product_name.clone().unwrap_or(product_name),
            custom_name:             payload.custom_name.clone().unwrap_or(custom_name),
            role:                    current_role,
            device_instance,
            last_update:             Instant::now(),
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

        // Chemins supplémentaires présents dans le device cgwacs natif
        // → nécessaires pour que le menu "Setup" apparaisse dans Venus OS GUI
        m.insert("/DeviceType".into(),      DbusItem::i32(120));   // Carlo Gavazzi ET112
        m.insert("/FirmwareVersion".into(), DbusItem::str("4"));
        m.insert("/CustomName".into(),      DbusItem::str(&self.custom_name));

        // /AllowedRoles — tableau de strings "as" — condition du menu Setup dans Venus OS
        m.insert("/AllowedRoles".into(), DbusItem::string_array(ALLOWED_ROLES));

        // /Role — rôle courant, writable (Venus OS écrit via menu Setup)
        m.insert("/Role".into(), DbusItem::str(&self.role));

        // Metadata pvinverter
        m.insert("/StatusCode".into(),            DbusItem::i32(self.status_code));
        m.insert("/ErrorCode".into(),             DbusItem::i32(self.error_code));
        m.insert("/Position".into(),              DbusItem::i32(self.position));
        m.insert("/IsGenericEnergyMeter".into(),  DbusItem::i32(self.is_generic_energy_meter));

        // Totaux AC
        m.insert("/Ac/Power".into(),           DbusItem::f64(self.power,          "W"));
        m.insert("/Ac/Energy/Forward".into(),  DbusItem::f64(self.energy_forward, "kWh"));

        // Phase L1
        m.insert("/Ac/L1/Voltage".into(),        DbusItem::f64(self.l1_voltage,        "V"));
        m.insert("/Ac/L1/Current".into(),        DbusItem::f64(self.l1_current,        "A"));
        m.insert("/Ac/L1/Power".into(),          DbusItem::f64(self.l1_power,          "W"));
        m.insert("/Ac/L1/Energy/Forward".into(), DbusItem::f64(self.l1_energy_forward, "kWh"));

        m
    }
}

// =============================================================================
// Interface D-Bus — objet racine `/`
// =============================================================================

struct PvinverterRootIface {
    values: Arc<Mutex<PvinverterValues>>,
}

#[zbus::interface(name = "com.victronenergy.BusItem")]
impl PvinverterRootIface {
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
    path:       String,
    values:     Arc<Mutex<PvinverterValues>>,
    connection: Connection,
}

#[zbus::interface(name = "com.victronenergy.BusItem")]
impl BusItemLeaf {
    fn get_value(&self) -> OwnedValue {
        let guard = self.values.lock().unwrap();
        match guard.to_items().get(&self.path) {
            Some(item) => kind_to_owned(&item.kind),
            None       => OwnedValue::from(0i32),
        }
    }

    fn get_text(&self) -> String {
        let guard = self.values.lock().unwrap();
        guard.to_items().get(&self.path).map(|i| i.text.clone()).unwrap_or_default()
    }

    fn set_value(&self, val: zvariant::Value<'_>) -> i32 {
        match self.path.as_str() {
            "/Position" => {
                let new_pos: i32 = match &val {
                    zvariant::Value::I32(v) => *v,
                    zvariant::Value::U32(v) => *v as i32,
                    zvariant::Value::I64(v) => *v as i32,
                    zvariant::Value::U64(v) => *v as i32,
                    _ => return 1,
                };
                if !(0..=2).contains(&new_pos) {
                    return 1;
                }
                let items = {
                    let mut g = self.values.lock().unwrap();
                    g.position = new_pos;
                    info!(position = new_pos, "Position pvinverter mise à jour par Venus OS");
                    g.to_items()
                };
                self.emit_items_changed_sync(items);
                0
            }
            "/Role" => {
                let new_role = match &val {
                    zvariant::Value::Str(s) => s.to_string(),
                    _ => return 1,
                };
                if !ALLOWED_ROLES.contains(&new_role.as_str()) {
                    return 1;
                }
                let items = {
                    let mut g = self.values.lock().unwrap();
                    g.role = new_role.clone();
                    info!(role = %new_role, "Rôle pvinverter mis à jour par Venus OS");
                    g.to_items()
                };
                self.emit_items_changed_sync(items);
                0
            }
            _ => 1,
        }
    }
}

impl BusItemLeaf {
    fn emit_items_changed_sync(&self, items: HashMap<String, DbusItem>) {
        let conn = self.connection.clone();
        let dict: ItemsDict = items
            .iter()
            .map(|(p, i)| (p.clone(), item_to_inner(i)))
            .collect();
        tokio::spawn(async move {
            if let Ok(ctx) = SignalContext::new(&conn, "/") {
                let _ = PvinverterRootIface::items_changed(&ctx, dict).await;
            }
        });
    }
}

// =============================================================================
// Handle
// =============================================================================

pub struct PvinverterServiceHandle {
    pub service_name:    String,
    pub device_instance: u32,
    pub values:          Arc<Mutex<PvinverterValues>>,
    connection:          Connection,
    pub product_name:    String,
    pub custom_name:     String,
}

impl PvinverterServiceHandle {
    pub async fn update(&self, payload: &PvinverterPayload) -> Result<()> {
        let current_role = { self.values.lock().unwrap().role.clone() };
        let new_values = PvinverterValues::from_payload(
            payload,
            self.device_instance,
            self.product_name.clone(),
            self.custom_name.clone(),
            current_role,
        );
        let items = new_values.to_items();
        { *self.values.lock().unwrap() = new_values; }
        self.emit_items_changed(&items).await?;
        debug!(
            service = %self.service_name,
            power   = payload.ac.power,
            "D-Bus ItemsChanged pvinverter émis"
        );
        Ok(())
    }

    pub async fn set_disconnected(&self) -> Result<()> {
        let items = {
            let mut g = self.values.lock().unwrap();
            g.connected = 0;
            g.to_items()
        };
        warn!(service = %self.service_name, "PV Inverter déconnecté — watchdog timeout");
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
        match PvinverterRootIface::items_changed(&ctx, dict).await {
            Ok(_)  => { debug!(service = %self.service_name, "ItemsChanged pvinverter émis"); Ok(()) }
            Err(e) => { warn!(service = %self.service_name, "ItemsChanged warning : {}", e); Ok(()) }
        }
    }
}

// =============================================================================
// Création du service
// =============================================================================

/// Crée un service `com.victronenergy.pvinverter.{service_suffix}`.
pub async fn create_pvinverter_service(
    dbus_bus:        &str,
    service_suffix:  &str,
    device_instance: u32,
    product_name:    String,
    custom_name:     String,
) -> Result<PvinverterServiceHandle> {
    let service_name = format!("{}.{}", VICTRON_PVINVERTER_PREFIX, service_suffix);

    info!(
        service         = %service_name,
        device_instance = device_instance,
        "Enregistrement service D-Bus pvinverter Venus OS"
    );

    let initial_values = Arc::new(Mutex::new(
        PvinverterValues::disconnected(device_instance, product_name.clone(), custom_name.clone())
    ));

    let root = PvinverterRootIface { values: initial_values.clone() };

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
            .at(path.as_str(), BusItemLeaf {
                path:       path.clone(),
                values:     initial_values.clone(),
                connection: conn.clone(),
            })
            .await?;
    }

    info!(
        service = %service_name,
        paths   = leaf_paths.len(),
        "Service D-Bus pvinverter enregistré ({} chemins + racine /)",
        leaf_paths.len()
    );

    Ok(PvinverterServiceHandle {
        service_name,
        device_instance,
        values: initial_values,
        connection: conn,
        product_name,
        custom_name,
    })
}
