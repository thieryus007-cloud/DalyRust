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
use crate::state::AppState;
use daly_bms_core::types::BmsSnapshot;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json::json;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

/// Démarre la tâche de publication MQTT en arrière-plan.
///
/// La tâche lit les derniers snapshots toutes les `publish_interval_sec`
/// secondes et publie sur les topics configurés.
pub async fn run_mqtt_bridge(state: AppState, cfg: MqttConfig) {
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

        let snapshots = state.latest_snapshots().await;
        for snap in &snapshots {
            if let Err(e) = publish_snapshot(&client, &cfg, snap).await {
                error!("MQTT publish erreur : {:?}", e);
            }
        }
    }
}

/// Publie un snapshot complet sur tous les topics d'un BMS.
async fn publish_snapshot(
    client: &AsyncClient,
    cfg: &MqttConfig,
    snap: &BmsSnapshot,
) -> anyhow::Result<()> {
    let prefix = format!("{}/{}", cfg.topic_prefix, snap.address);

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

/// Construit le payload au format dbus-mqtt-battery (Venus OS).
///
/// Compatible avec https://github.com/mr-manuel/venus-os_dbus-mqtt-battery
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
        "Capacity":           snap.capacity,
        "Soc":                snap.soc,
        "Soh":                snap.soh,
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
            "MinVoltageCellId":               snap.system.min_voltage_cell_id,
            "MinCellVoltage":                 snap.system.min_cell_voltage,
            "MaxVoltageCellId":               snap.system.max_voltage_cell_id,
            "MaxCellVoltage":                 snap.system.max_cell_voltage,
            "MinTemperatureCellId":           snap.system.min_temperature_cell_id,
            "MinCellTemperature":             snap.system.min_cell_temperature,
            "MaxTemperatureCellId":           snap.system.max_temperature_cell_id,
            "MaxCellTemperature":             snap.system.max_cell_temperature,
            "NrOfCellsPerBattery":            snap.system.nr_of_cells_per_battery,
            "NrOfModulesOnline":              snap.system.nr_of_modules_online,
            "NrOfModulesOffline":             snap.system.nr_of_modules_offline,
            "NrOfModulesBlockingCharge":      snap.system.nr_of_modules_blocking_charge,
            "NrOfModulesBlockingDischarge":   snap.system.nr_of_modules_blocking_discharge,
        },
        "Voltages":   snap.voltages,
        "Balances":   snap.balances,
        "Io": {
            "AllowToCharge":    snap.io.allow_to_charge,
            "AllowToDischarge": snap.io.allow_to_discharge,
            "AllowToBalance":   snap.io.allow_to_balance,
            "AllowToHeat":      snap.io.allow_to_heat,
            "ExternalRelay":    snap.io.external_relay,
        },
        "Heating":    snap.heating,
        "TimeToSoC":  snap.time_to_soc,
    })
}
