//! Bridge MQTT — publication périodique vers Mosquitto.
//!
//! ## Topics publiés
//!
//! ```text
//! {prefix}/{bms_id}/soc          → "56.4"
//! {prefix}/{bms_id}/voltage      → "52.53"
//! {prefix}/{bms_id}/current      → "-1.60"
//! {prefix}/{bms_id}/power        → "-84.0"
//! {prefix}/{bms_id}/status       → JSON complet
//! {prefix}/{bms_id}/cells        → JSON tensions
//! {prefix}/{bms_id}/alarms       → JSON alarmes
//! {prefix}/{bms_id}/venus        → JSON format dbus-mqtt-battery (si activé)
//! ```

use crate::config::MqttConfig;
use crate::et112::Et112Snapshot;
use crate::state::{AppState, VenusMppt, VenusSmartShunt, VenusTemperature};
use crate::tasmota::TasmotaSnapshot;
use chrono::Utc;
use daly_bms_core::types::BmsSnapshot;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Démarre la tâche de publication MQTT en arrière-plan.
///
/// `addr_map` : table adresse RS485 → identifiant de topic (ex: 0x28 → "1").
/// Permet d'aligner les topics sur la configuration `dbus-mqttbattery` du NanoPi
/// (santuario/bms/1/venus, santuario/bms/2/venus, …).
/// Si l'adresse n'est pas dans la map, on publie avec l'adresse décimale brute.
pub async fn run_mqtt_bridge(state: AppState, cfg: MqttConfig, addr_map: HashMap<u8, String>) {
    if !cfg.enabled {
        info!("MQTT bridge désactivé (enabled = false)");
        return;
    }

    info!(host = %cfg.host, port = cfg.port, "Démarrage MQTT bridge");

    let mut opts = MqttOptions::new(
        format!("daly-bms-{}", uuid::Uuid::new_v4()),
        &cfg.host,
        cfg.port,
    );
    opts.set_keep_alive(Duration::from_secs(30));

    if let (Some(user), Some(pass)) = (&cfg.username, &cfg.password) {
        opts.set_credentials(user, pass);
    }

    let (client, mut eventloop) = AsyncClient::new(opts, 128);

    // Spawner la boucle d'événements MQTT (requis pour rumqttc async)
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(_) => {}
                Err(e) => {
                    warn!("MQTT eventloop erreur : {:?}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    let mut ticker = interval(Duration::from_secs_f64(cfg.publish_interval_sec.max(1.0)));

    loop {
        ticker.tick().await;

        // ── BMS snapshots ─────────────────────────────────────────────────────
        let snapshots = state.latest_snapshots().await;
        for snap in &snapshots {
            let topic_id = addr_map
                .get(&snap.address)
                .cloned()
                .unwrap_or_else(|| snap.address.to_string());
            if let Err(e) = publish_snapshot(&client, &cfg, snap, &topic_id).await {
                error!("MQTT publish BMS erreur : {:?}", e);
            }
        }

        // ── ET112 snapshots → topic {service_type}/{mqtt_index}/venus ──────
        let et112_snaps = state.et112_latest_all().await;
        for snap in &et112_snaps {
            // Résoudre le mqtt_index, position et service_type depuis la config
            let dev_cfg = state.config.et112.devices
                .iter()
                .find(|d| d.parsed_address() == snap.address);
            let idx          = dev_cfg.and_then(|d| d.mqtt_index).unwrap_or(snap.address);
            let position     = dev_cfg.map(|d| d.position).unwrap_or(1);
            let service_type = dev_cfg.map(|d| d.service_type.as_str()).unwrap_or("pvinverter");
            if let Err(e) = publish_et112_snapshot(&client, &cfg, snap, idx, position, service_type).await {
                error!("MQTT publish ET112 erreur : {:?}", e);
            }
        }

        // ── Irradiance PRALRAN → santuario/irradiance/raw ────────────────────
        // Même topic que l'ancien irradiance_reader.py → Node-RED inchangé.
        if let Some(snap) = state.latest_irradiance().await {
            if let Err(e) = publish_irradiance(&client, &cfg, snap.irradiance_wm2).await {
                error!("MQTT publish irradiance erreur : {:?}", e);
            }
        }

        // ── Tasmota → forward Venus OS switch/acload si mqtt_index défini ──
        let tasmota_snaps = state.tasmota_latest_all().await;
        for snap in &tasmota_snaps {
            let dev_cfg = state.config.tasmota.devices
                .iter()
                .find(|d| d.id == snap.id);
            if let Some(dev) = dev_cfg {
                if let Some(idx) = dev.mqtt_index {
                    let svc = dev.service_type.as_str();
                    if let Err(e) = publish_tasmota_snapshot(&client, &cfg, snap, idx, svc).await {
                        error!("MQTT publish Tasmota erreur : {:?}", e);
                    }
                }
            }
        }
    }
}

/// Publie la valeur d'irradiance sur `santuario/irradiance/raw` (retain=true).
///
/// Même format que l'ancien `irradiance_reader.py` — entier W/m² en string.
async fn publish_irradiance(
    client: &AsyncClient,
    cfg: &MqttConfig,
    irradiance_wm2: f32,
) -> anyhow::Result<()> {
    let base = cfg.topic_prefix
        .rsplit_once('/')
        .map(|(prefix, _)| prefix)
        .unwrap_or("santuario");
    let topic = format!("{}/irradiance/raw", base);
    let payload = format!("{:.0}", irradiance_wm2);
    client
        .publish(&topic, QoS::AtLeastOnce, true, payload)
        .await?;
    Ok(())
}

/// Publie un snapshot ET112 sur le topic `santuario/{service_type}/{idx}/venus`.
///
/// service_type = "pvinverter" → topic pvinverter/{idx}/venus  (PvinverterPayload)
/// service_type = "acload"     → topic grid/{idx}/venus        (GridPayload)
/// service_type = "heatpump"   → topic heatpump/{idx}/venus    (HeatpumpPayload)
async fn publish_et112_snapshot(
    client: &AsyncClient,
    cfg: &MqttConfig,
    snap: &Et112Snapshot,
    mqtt_index: u8,
    position: u8,
    service_type: &str,
) -> anyhow::Result<()> {
    let base = cfg.topic_prefix
        .rsplit_once('/')
        .map(|(prefix, _)| prefix)
        .unwrap_or("santuario");

    let topic_prefix = match service_type {
        "acload"   => "grid",
        "heatpump" => "heatpump",
        _          => "pvinverter",
    };
    let topic = format!("{}/{}/{}/venus", base, topic_prefix, mqtt_index);

    let payload = if service_type == "heatpump" {
        // HeatpumpPayload — l'ET112 mesure la consommation AC de la PAC
        json!({
            "Ac": {
                "Power":  snap.power_w,
                "Energy": { "Forward": snap.energy_import_kwh() }
            },
            "Position":    position,   // 1=AC Output
            "State":       0,          // 0=Off/unknown (l'ET112 ne connaît pas l'état)
            "ProductName": snap.name,
            "CustomName":  snap.name,
        })
    } else {
        // PvinverterPayload / GridPayload — format complet L1
        json!({
            "Ac": {
                "L1": {
                    "Voltage": snap.voltage_v,
                    "Current": snap.current_a,
                    "Power":   snap.power_w,
                    "Energy": {
                        "Forward": snap.energy_import_kwh(),
                        "Reverse": snap.energy_export_kwh()
                    }
                },
                "Power":  snap.power_w,
                "Energy": {
                    "Forward": snap.energy_import_kwh(),
                    "Reverse": snap.energy_export_kwh()
                }
            },
            "StatusCode":           7,   // Running
            "ErrorCode":            0,   // No Error
            "Position":             position,
            "IsGenericEnergyMeter": 1,
            "ProductName":          snap.name,
            "CustomName":           snap.name,
        })
    };

    client
        .publish(&topic, QoS::AtLeastOnce, true, serde_json::to_vec(&payload)?)
        .await?;

    Ok(())
}

/// Publie un snapshot Tasmota vers Venus OS.
///
/// service_type = "switch" → topic `santuario/switch/{idx}/venus`  (SwitchPayload)
/// service_type = "acload" → topic `santuario/grid/{idx}/venus`    (GridPayload)
async fn publish_tasmota_snapshot(
    client: &AsyncClient,
    cfg: &MqttConfig,
    snap: &TasmotaSnapshot,
    mqtt_index: u8,
    service_type: &str,
) -> anyhow::Result<()> {
    let base = cfg.topic_prefix
        .rsplit_once('/')
        .map(|(prefix, _)| prefix)
        .unwrap_or("santuario");

    let (topic_prefix, payload) = if service_type == "acload" {
        let topic = format!("{}/grid/{}/venus", base, mqtt_index);
        let p = json!({
            "Ac/L1/Power":   snap.power_w,
            "Ac/L1/Voltage": snap.voltage_v,
            "Ac/L1/Current": snap.current_a,
            "ProductName":   snap.name,
            "CustomName":    snap.name,
        });
        (topic, p)
    } else {
        // switch (défaut)
        let topic = format!("{}/switch/{}/venus", base, mqtt_index);
        let p = json!({
            "State":       if snap.power_on { 1 } else { 0 },
            "Position":    1,
            "ProductName": snap.name,
            "CustomName":  snap.name,
        });
        (topic, p)
    };

    client
        .publish(&topic_prefix, QoS::AtLeastOnce, true, serde_json::to_vec(&payload)?)
        .await?;

    Ok(())
}

/// Publie un snapshot complet sur tous les topics d'un BMS.
///
/// `topic_id` : identifiant résolu (ex: "1" pour 0x28, "2" pour 0x29).
async fn publish_snapshot(
    client: &AsyncClient,
    cfg: &MqttConfig,
    snap: &BmsSnapshot,
    topic_id: &str,
) -> anyhow::Result<()> {
    let prefix = format!("{}/{}", cfg.topic_prefix, topic_id);

    // Scalaires
    publish_str(client, &format!("{}/soc",     prefix), &format!("{:.1}", snap.soc)).await;
    publish_str(client, &format!("{}/voltage", prefix), &format!("{:.2}", snap.dc.voltage)).await;
    publish_str(client, &format!("{}/current", prefix), &format!("{:.1}", snap.dc.current)).await;
    publish_str(client, &format!("{}/power",   prefix), &format!("{:.1}", snap.dc.power)).await;

    // JSON status complet
    let status_json = serde_json::to_string(snap)?;
    client
        .publish(format!("{}/status", prefix), QoS::AtLeastOnce, true, status_json)
        .await?;

    // JSON cellules
    let cells_json = serde_json::to_string(&snap.voltages)?;
    client
        .publish(format!("{}/cells", prefix), QoS::AtLeastOnce, false, cells_json)
        .await?;

    // JSON alarmes
    let alarms_json = serde_json::to_string(&snap.alarms)?;
    client
        .publish(format!("{}/alarms", prefix), QoS::AtLeastOnce, false, alarms_json)
        .await?;

    // Format Venus OS (dbus-mqtt-battery)
    let venus_payload = build_venus_payload(snap);
    let venus_json = serde_json::to_string(&venus_payload)?;
    client
        .publish(format!("{}/venus", prefix), QoS::AtLeastOnce, true, venus_json)
        .await?;

    Ok(())
}

async fn publish_str(client: &AsyncClient, topic: &str, value: &str) {
    let _ = client
        .publish(topic, QoS::AtLeastOnce, false, value.to_string())
        .await;
}

/// Extrait le numéro entier d'un identifiant de cellule ("C3" → 3, "Cell3" → 3).
fn cell_id_to_int(id: &str) -> u32 {
    id.trim_start_matches("Cell")
      .trim_start_matches('C')
      .parse()
      .unwrap_or(0)
}

/// Construit le payload au format dbus-mqtt-battery (Venus OS).
///
/// Compatible avec https://github.com/mr-manuel/venus-os_dbus-mqtt-battery
///
/// IMPORTANT : seuls les champs reconnus par dbus-mqtt-battery sont inclus.
/// Les champs inconnus (Voltages/sum, Balances, TimeToSoC, Soh, Heating) provoquent
/// une exception Python dans le callback MQTT → first_data_received reste False → timeout.
fn build_venus_payload(snap: &BmsSnapshot) -> serde_json::Value {
    json!({
        "Dc": {
            "Power":       snap.dc.power,
            "Voltage":     snap.dc.voltage,
            "Current":     snap.dc.current,
            "Temperature": snap.dc.temperature,
        },
        "InstalledCapacity":  snap.installed_capacity,
        "ConsumedAmphours":   snap.consumed_amphours,
        "Capacity":           snap.bms_reported_capacity_ah,
        "Soc":                snap.soc,
        "TimeToGo":           snap.time_to_go,
        "Balancing":          snap.balancing,
        "SystemSwitch":       snap.system_switch,
        "Alarms": {
            "LowVoltage":             snap.alarms.low_voltage,
            "HighVoltage":            snap.alarms.high_voltage,
            "LowSoc":                 snap.alarms.low_soc,
            "HighChargeCurrent":      snap.alarms.high_charge_current,
            "HighDischargeCurrent":   snap.alarms.high_discharge_current,
            "HighCurrent":            snap.alarms.high_current,
            "CellImbalance":          snap.alarms.cell_imbalance,
            "HighChargeTemperature":  snap.alarms.high_charge_temperature,
            "LowChargeTemperature":   snap.alarms.low_charge_temperature,
            "LowCellVoltage":         snap.alarms.low_cell_voltage,
            "LowTemperature":         snap.alarms.low_temperature,
            "HighTemperature":        snap.alarms.high_temperature,
            "FuseBlown":              snap.alarms.fuse_blown,
        },
        "System": {
            // Entiers 1-based requis par dbus-mqtt-battery
            "MinVoltageCellId":               cell_id_to_int(&snap.system.min_voltage_cell_id),
            "MinCellVoltage":                 snap.system.min_cell_voltage,
            "MaxVoltageCellId":               cell_id_to_int(&snap.system.max_voltage_cell_id),
            "MaxCellVoltage":                 snap.system.max_cell_voltage,
            "MinTemperatureCellId":           cell_id_to_int(&snap.system.min_temperature_cell_id),
            "MinCellTemperature":             snap.system.min_cell_temperature,
            "MaxTemperatureCellId":           cell_id_to_int(&snap.system.max_temperature_cell_id),
            "MaxCellTemperature":             snap.system.max_cell_temperature,
            "NrOfCellsPerBattery":            snap.system.nr_of_cells_per_battery,
            "NrOfModulesOnline":              snap.system.nr_of_modules_online,
            "NrOfModulesOffline":             snap.system.nr_of_modules_offline,
            "NrOfModulesBlockingCharge":      snap.system.nr_of_modules_blocking_charge,
            "NrOfModulesBlockingDischarge":   snap.system.nr_of_modules_blocking_discharge,
        },
        // AllowToCharge / AllowToDischarge volontairement figés à 1 :
        // on ne veut pas que Venus OS (systemcalc) transmette ces signaux aux MPPT.
        "Io": {
            "AllowToCharge":    1,
            "AllowToDischarge": 1,
            "AllowToBalance":   snap.io.allow_to_balance,
            "ExternalRelay":    snap.io.external_relay,
        },
    })
}

// =============================================================================
// MQTT Subscriber — Réception des données Venus OS
// =============================================================================

/// Démarre un abonnement MQTT pour recevoir les données Venus OS.
///
/// Cette tâche écoute les topics :
/// - `santuario/meteo/venus` → MPPT SolarCharger (puissance, production kWh)
/// - `santuario/heat/*/venus` → Capteurs de température
/// - `santuario/heatpump/*/venus` → PAC/Chauffe-eau (optionnel)
/// - `santuario/system/venus` → SmartShunt (SOC, tension, courant)
pub async fn start_venus_mqtt_subscriber(state: AppState, cfg: MqttConfig) {
    if !cfg.enabled {
        return;
    }

    let mut opts = MqttOptions::new(
        format!("daly-bms-venus-sub-{}", uuid::Uuid::new_v4()),
        &cfg.host,
        cfg.port,
    );
    opts.set_keep_alive(Duration::from_secs(30));

    if let (Some(user), Some(pass)) = (&cfg.username, &cfg.password) {
        opts.set_credentials(user, pass);
    }

    let (client, mut eventloop) = AsyncClient::new(opts, 128);

    // S'abonner aux topics Venus OS
    let topics = vec![
        ("santuario/meteo/venus", QoS::AtLeastOnce),
        ("santuario/heat/+/venus", QoS::AtLeastOnce),
        ("santuario/heatpump/+/venus", QoS::AtLeastOnce),
        ("santuario/system/venus", QoS::AtLeastOnce),
    ];

    for (topic, qos) in &topics {
        if let Err(e) = client.subscribe(*topic, *qos).await {
            warn!("MQTT subscribe erreur pour {}: {:?}", topic, e);
        } else {
            debug!("MQTT abonné à {}", topic);
        }
    }

    // Boucle de réception
    loop {
        match eventloop.poll().await {
            Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(p))) => {
                let topic = &p.topic;
                let payload = std::str::from_utf8(&p.payload).unwrap_or("");

                debug!("MQTT reçu {} = {}", topic, payload);

                // Parser le payload JSON
                if let Ok(json) = serde_json::from_str::<Value>(payload) {
                    if topic == "santuario/meteo/venus" {
                        handle_meteo_topic(&state, &json).await;
                    } else if topic.starts_with("santuario/heat/") && topic.ends_with("/venus") {
                        handle_temperature_topic(&state, &json).await;
                    } else if topic.starts_with("santuario/heatpump/") && topic.ends_with("/venus") {
                        // Optionnel : traiter les heatpumps
                        debug!("Heatpump topic reçu : {}", topic);
                    } else if topic == "santuario/system/venus" {
                        handle_system_topic(&state, &json).await;
                    }
                }
            }
            Ok(rumqttc::Event::Outgoing(_)) => {
                // Ignorer les ACK d'envoi
            }
            Err(e) => {
                warn!("MQTT eventloop erreur : {:?}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            _ => {}
        }
    }
}

/// Traite le topic `santuario/meteo/venus`
/// Payload : { "Irradiance": 750, "TodaysYield": 12.5, "MpptPower": 2500 }
async fn handle_meteo_topic(state: &AppState, json: &Value) {
    if let Some(yield_kwh) = json.get("TodaysYield").and_then(|v| v.as_f64()).map(|v| v as f32) {
        // Power peut venir de MpptPower ou calculée depuis irradiance (fallback)
        let irradiance = json.get("Irradiance").and_then(|v| v.as_f64()).map(|v| v as f32).unwrap_or(0.0);
        let mppt_power = json.get("MpptPower").and_then(|v| v.as_f64()).map(|v| v as f32);

        let mppt = VenusMppt {
            instance: 0,
            name: "MPPT SolarCharger".to_string(),
            power_w: mppt_power.or(if irradiance > 0.0 { Some(irradiance) } else { None }),
            yield_today_kwh: Some(yield_kwh),
            max_power_today_w: None,
            timestamp: Utc::now(),
        };
        state.on_venus_mppt(mppt).await;
    }
}

/// Traite les topics `santuario/heat/*/venus`
/// Payload : { "Temperature": 15.3, "TemperatureType": 4, "Humidity": 65 }
async fn handle_temperature_topic(state: &AppState, json: &Value) {
    if let Some(temp_c) = json.get("Temperature").and_then(|v| v.as_f64()).map(|v| v as f32) {
        let instance = json
            .get("DeviceInstance")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;

        let humidity = json.get("Humidity").and_then(|v| v.as_f64()).map(|v| v as f32);
        let pressure = json.get("Pressure").and_then(|v| v.as_f64()).map(|v| v as f32);

        let temp_type = json
            .get("TemperatureType")
            .and_then(|v| v.as_u64())
            .and_then(|t| match t {
                0 => Some("Battery"),
                1 => Some("Fridge"),
                2 => Some("Generic"),
                3 => Some("Room"),
                4 => Some("Outdoor"),
                5 => Some("WaterHeater"),
                6 => Some("Freezer"),
                _ => None,
            })
            .unwrap_or("Generic")
            .to_string();

        let temp = VenusTemperature {
            instance,
            name: format!("Temperature {}", instance),
            temp_c: Some(temp_c),
            humidity_percent: humidity,
            pressure_mbar: pressure,
            temp_type,
            connected: true,
            timestamp: Utc::now(),
        };

        state.on_venus_temperature(temp).await;
    }
}

/// Traite le topic `santuario/system/venus`
/// Payload : SmartShunt data { "Soc": 75.2, "Voltage": 48.32, "Current": 5.5, ... }
async fn handle_system_topic(state: &AppState, json: &Value) {
    if let Some(soc) = json.get("Soc").and_then(|v| v.as_f64()).map(|v| v as f32) {
        let voltage = json.get("Voltage").and_then(|v| v.as_f64()).map(|v| v as f32);
        let current = json.get("Current").and_then(|v| v.as_f64()).map(|v| v as f32);
        let power = json.get("Power").and_then(|v| v.as_f64()).map(|v| v as f32);
        let energy_in = json.get("EnergyIn").and_then(|v| v.as_f64()).map(|v| v as f32);
        let energy_out = json.get("EnergyOut").and_then(|v| v.as_f64()).map(|v| v as f32);

        let shunt = VenusSmartShunt {
            soc_percent: Some(soc),
            voltage_v: voltage,
            current_a: current,
            power_w: power,
            energy_in_kwh: energy_in.map(|e| e / 1000.0),
            energy_out_kwh: energy_out.map(|e| e / 1000.0),
            timestamp: Utc::now(),
        };

        state.on_venus_smartshunt(shunt).await;
    }
}
