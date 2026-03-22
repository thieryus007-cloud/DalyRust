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
use crate::state::AppState;
use daly_bms_core::types::BmsSnapshot;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

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

        // ── ET112 snapshots → topic pvinverter/{mqtt_index}/venus ────────────
        let et112_snaps = state.et112_latest_all().await;
        for snap in &et112_snaps {
            // Résoudre le mqtt_index depuis la config
            let idx = state.config.et112.devices
                .iter()
                .find(|d| d.parsed_address() == snap.address)
                .and_then(|d| d.mqtt_index)
                .unwrap_or(snap.address);
            if let Err(e) = publish_et112_snapshot(&client, &cfg, snap, idx).await {
                error!("MQTT publish ET112 erreur : {:?}", e);
            }
        }
    }
}

/// Publie un snapshot ET112 sur le topic `santuario/pvinverter/{idx}/venus`.
///
/// Format compatible `dbus-mqtt-venus` type pvinverter (com.victronenergy.pvinverter).
async fn publish_et112_snapshot(
    client: &AsyncClient,
    cfg: &MqttConfig,
    snap: &Et112Snapshot,
    mqtt_index: u8,
) -> anyhow::Result<()> {
    // Dériver le topic prefix pvinverter depuis le topic prefix BMS
    // "santuario/bms" → "santuario/pvinverter"
    let base = cfg.topic_prefix
        .rsplit_once('/')
        .map(|(prefix, _)| prefix)
        .unwrap_or("santuario");
    let topic = format!("{}/pvinverter/{}/venus", base, mqtt_index);

    let payload = json!({
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
            "Power":         snap.power_w,
            "Energy": {
                "Forward":   snap.energy_import_kwh(),
                "Reverse":   snap.energy_export_kwh()
            }
        },
        "StatusCode":           7,   // Running
        "ErrorCode":            0,   // No Error
        "Position":             1,   // AC output (micro-inverseurs sur AC output)
        "IsGenericEnergyMeter": 1,   // ET112 = generic energy meter
        "ProductName":          format!("ET112 addr={:#04x}", snap.address),
        "CustomName":           snap.name,
    });

    client
        .publish(&topic, QoS::AtLeastOnce, true, serde_json::to_vec(&payload)?)
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
