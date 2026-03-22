//! Service D-Bus `com.victronenergy.temperature.{name}` pour capteurs de température.
//!
//! Conforme au wiki Victron Venus OS — section Temperatures :
//! <https://github.com/victronenergy/venus/wiki/dbus#temperatures>
//!
//! ## Chemins D-Bus exposés
//!
//! ```text
//! /Temperature      — °C
//! /TemperatureType  — 0=battery 1=fridge 2=generic 3=Room 4=Outdoor 5=WaterHeater 6=Freezer
//! /CustomName       — nom libre (ex: "Température Extérieure")
//! /Humidity         — % humidité relative (optionnel)
//! /Pressure         — hPa (optionnel)
//! /Status           — 0=OK, 1=Disconnected
//! /Connected        — 0 ou 1
//! /ProductName      — ex: "Temperature Sensor"
//! /ProductId        — 0
//! /DeviceInstance   — instance unique Venus OS / VRM
//! /Mgmt/ProcessName
//! /Mgmt/ProcessVersion
//! /Mgmt/Connection
//! ```

use crate::types::HeatPayload;
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

const VICTRON_TEMPERATURE_PREFIX: &str = "com.victronenergy.temperature";

// =============================================================================
// Item D-Bus — paire (valeur, texte)
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
// Valeurs courantes d'un capteur
// =============================================================================

/// État courant d'un capteur de température exposé sur D-Bus.
#[derive(Debug, Clone)]
pub struct SensorValues {
    pub connected:        i32,
    pub temperature:      f64,
    pub temperature_type: i32,
    pub humidity:         Option<f64>,
    pub pressure:         Option<f64>,
    pub custom_name:      String,
    pub product_name:     String,
    pub device_instance:  u32,
    /// Timestamp de la dernière mise à jour (watchdog)
    pub last_update:      Instant,
}

impl SensorValues {
    /// État initial déconnecté.
    pub fn disconnected(
        device_instance:  u32,
        product_name:     String,
        custom_name:      String,
        temperature_type: i32,
    ) -> Self {
        Self {
            connected:        0,
            temperature:      0.0,
            temperature_type,
            humidity:         None,
            pressure:         None,
            custom_name,
            product_name,
            device_instance,
            last_update:      Instant::now(),
        }
    }

    /// Met à jour depuis un payload MQTT ; le `temperature_type` de la config
    /// a priorité sur celui du payload (0 = non défini côté payload).
    pub fn from_payload(
        payload:          &HeatPayload,
        device_instance:  u32,
        product_name:     String,
        custom_name:      String,
        default_type:     i32,
    ) -> Self {
        // La config a priorité; sinon on prend la valeur du payload
        let temperature_type = if default_type != 0 {
            default_type
        } else {
            payload.temperature_type
        };

        Self {
            connected: 1,
            temperature: payload.temperature,
            temperature_type,
            humidity: payload.humidity,
            pressure: payload.pressure,
            custom_name: payload.custom_name.clone().unwrap_or(custom_name),
            product_name,
            device_instance,
            last_update: Instant::now(),
        }
    }

    /// Construit le dictionnaire complet des items D-Bus.
    pub fn to_items(&self) -> HashMap<String, DbusItem> {
        let mut m = HashMap::new();

        // Identification (commun à tous les services Victron)
        m.insert("/Mgmt/ProcessName".into(),    DbusItem::str("dbus-mqtt-venus"));
        m.insert("/Mgmt/ProcessVersion".into(), DbusItem::str(env!("CARGO_PKG_VERSION")));
        m.insert("/Mgmt/Connection".into(),     DbusItem::str("MQTT"));
        m.insert("/ProductId".into(),           DbusItem::u32(0));
        m.insert("/ProductName".into(),         DbusItem::str(&self.product_name));
        m.insert("/DeviceInstance".into(),      DbusItem::u32(self.device_instance));
        m.insert("/Connected".into(),           DbusItem::i32(self.connected));

        // Status : 0=OK, 1=Disconnected
        let status = if self.connected == 1 { 0 } else { 1 };
        m.insert("/Status".into(), DbusItem::i32(status));

        // Données température (chemins officiels Venus OS wiki)
        m.insert("/Temperature".into(),     DbusItem::f64(self.temperature, "°C"));
        m.insert("/TemperatureType".into(), DbusItem::i32(self.temperature_type));
        m.insert("/CustomName".into(),      DbusItem::str(&self.custom_name));

        // Toujours enregistrés (0.0 si absent) pour que le chemin D-Bus existe
        // dès la création du service et soit interrogeable immédiatement.
        m.insert("/Humidity".into(), DbusItem::f64(self.humidity.unwrap_or(0.0), "%"));
        m.insert("/Pressure".into(), DbusItem::f64(self.pressure.unwrap_or(0.0), "hPa"));

        m
    }
}

// =============================================================================
// Interface D-Bus — objet racine `/`
// =============================================================================

struct TemperatureRootIface {
    values: Arc<Mutex<SensorValues>>,
}

#[zbus::interface(name = "com.victronenergy.BusItem")]
impl TemperatureRootIface {
    fn get_items(&self) -> ItemsDict {
        let guard = self.values.lock().unwrap();
        guard
            .to_items()
            .iter()
            .map(|(path, item)| (path.clone(), item_to_inner(item)))
            .collect()
    }

    fn get_value(&self) -> OwnedValue {
        OwnedValue::from(0i32)
    }

    fn get_text(&self) -> String {
        String::new()
    }

    fn set_value(&self, _val: zvariant::Value<'_>) -> i32 {
        1
    }

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
    values: Arc<Mutex<SensorValues>>,
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
        guard
            .to_items()
            .get(&self.path)
            .map(|i| i.text.clone())
            .unwrap_or_default()
    }

    fn set_value(&self, _val: zvariant::Value<'_>) -> i32 {
        1 // lecture seule
    }
}

// =============================================================================
// Handle vers un service actif
// =============================================================================

/// Référence à un service D-Bus temperature actif.
pub struct SensorServiceHandle {
    pub service_name:     String,
    pub device_instance:  u32,
    pub values:           Arc<Mutex<SensorValues>>,
    /// La connexion maintient le service D-Bus vivant.
    connection:           Connection,
    /// Type par défaut (de la config [[sensors]])
    pub default_type:     i32,
    /// Nom produit (pour re-créer SensorValues depuis payload)
    pub product_name:     String,
    /// Nom personnalisé par défaut
    pub custom_name:      String,
}

impl SensorServiceHandle {
    /// Met à jour les valeurs et émet `ItemsChanged`.
    pub async fn update(&self, payload: &HeatPayload) -> Result<()> {
        let new_values = SensorValues::from_payload(
            payload,
            self.device_instance,
            self.product_name.clone(),
            self.custom_name.clone(),
            self.default_type,
        );
        let items = new_values.to_items();

        {
            let mut guard = self.values.lock().unwrap();
            *guard = new_values;
        }

        self.emit_items_changed(&items).await?;

        debug!(
            service = %self.service_name,
            temperature = %payload.temperature,
            "D-Bus ItemsChanged température émis"
        );

        Ok(())
    }

    /// Marque le capteur comme déconnecté (timeout watchdog).
    pub async fn set_disconnected(&self) -> Result<()> {
        let items = {
            let mut guard = self.values.lock().unwrap();
            guard.connected = 0;
            guard.to_items()
        };
        warn!(service = %self.service_name, "Capteur déconnecté — watchdog timeout");
        self.emit_items_changed(&items).await
    }

    /// Republication forcée (keepalive Venus OS).
    pub async fn republish(&self) -> Result<()> {
        let items = {
            let guard = self.values.lock().unwrap();
            guard.to_items()
        };
        self.emit_items_changed(&items).await
    }

    async fn emit_items_changed(&self, items: &HashMap<String, DbusItem>) -> Result<()> {
        let dict: ItemsDict = items
            .iter()
            .map(|(path, item)| (path.clone(), item_to_inner(item)))
            .collect();

        let ctx = SignalContext::new(&self.connection, "/")?;

        match TemperatureRootIface::items_changed(&ctx, dict).await {
            Ok(_) => {
                debug!(
                    service = %self.service_name,
                    count = items.len(),
                    "ItemsChanged(a{{sa{{sv}}}}) température émis"
                );
                Ok(())
            }
            Err(e) => {
                warn!(service = %self.service_name, "ItemsChanged warning : {}", e);
                Ok(())
            }
        }
    }
}

// =============================================================================
// Création du service D-Bus
// =============================================================================

/// Crée et enregistre un service D-Bus `com.victronenergy.temperature.{suffix}`.
pub async fn create_temperature_service(
    dbus_bus:        &str,
    service_suffix:  &str,
    device_instance: u32,
    product_name:    String,
    custom_name:     String,
    default_type:    i32,
) -> Result<SensorServiceHandle> {
    let service_name = format!("{}.{}", VICTRON_TEMPERATURE_PREFIX, service_suffix);

    info!(
        service = %service_name,
        device_instance = device_instance,
        temperature_type = default_type,
        "Enregistrement service D-Bus température Venus OS"
    );

    let initial_values = Arc::new(Mutex::new(SensorValues::disconnected(
        device_instance,
        product_name.clone(),
        custom_name.clone(),
        default_type,
    )));

    let root = TemperatureRootIface { values: initial_values.clone() };

    let builder = match dbus_bus {
        "session" => connection::Builder::session()?,
        _         => connection::Builder::system()?,
    };

    let conn = builder
        .name(service_name.as_str())?
        .serve_at("/", root)?
        .build()
        .await?;

    // Enregistrer un objet feuille par chemin métrique
    let leaf_paths: Vec<String> = {
        let guard = initial_values.lock().unwrap();
        guard.to_items().into_keys().collect()
    };

    for path in &leaf_paths {
        let leaf = BusItemLeaf {
            path:   path.clone(),
            values: initial_values.clone(),
        };
        conn.object_server().at(path.as_str(), leaf).await?;
    }

    info!(
        service = %service_name,
        paths = leaf_paths.len(),
        "Service D-Bus température enregistré ({} chemins + racine /)",
        leaf_paths.len()
    );

    Ok(SensorServiceHandle {
        service_name,
        device_instance,
        values: initial_values,
        connection: conn,
        default_type,
        product_name,
        custom_name,
    })
}
