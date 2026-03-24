//! Subscriber MQTT Tasmota — réception des topics natifs Tasmota.
//!
//! Topics surveillés (wildcards) :
//!   tele/+/SENSOR  → payload JSON avec bloc ENERGY (puissance, énergie, tension, courant)
//!   stat/+/POWER   → payload texte "ON" ou "OFF"
//!
//! Pour chaque message reçu, le device est identifié par son `tasmota_id`
//! (deuxième segment du topic), puis le snapshot est mis à jour et le
//! callback `on_snapshot` est appelé.

use crate::config::{MqttConfig, TasmotaDeviceConfig};
use super::types::TasmotaSnapshot;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};
use chrono::Local;

/// Boucle principale d'abonnement MQTT Tasmota.
///
/// Se reconnecte automatiquement après déconnexion (backoff 10 s).
/// Appelle `on_snapshot` pour chaque mesure reçue.
pub async fn run_tasmota_mqtt_loop<F>(
    devices: Vec<TasmotaDeviceConfig>,
    mqtt_cfg: MqttConfig,
    mut on_snapshot: F,
)
where
    F: FnMut(TasmotaSnapshot) + Send + 'static,
{
    if devices.is_empty() {
        return;
    }

    info!(count = devices.len(), host = %mqtt_cfg.host, "Démarrage Tasmota MQTT subscriber");

    // État inter-messages : état relais (POWER) et dernier snapshot énergie (SENSOR)
    let mut power_states:    HashMap<u8, bool>             = HashMap::new();
    let mut energy_cache:    HashMap<u8, TasmotaSnapshot>  = HashMap::new();

    loop {
        let mut opts = MqttOptions::new(
            format!("daly-bms-tasmota-{}", uuid::Uuid::new_v4()),
            &mqtt_cfg.host,
            mqtt_cfg.port,
        );
        opts.set_keep_alive(Duration::from_secs(30));

        if let (Some(user), Some(pass)) = (&mqtt_cfg.username, &mqtt_cfg.password) {
            opts.set_credentials(user, pass);
        }

        let (client, mut eventloop) = AsyncClient::new(opts, 128);

        // Abonnement aux deux wildcards Tasmota
        let _ = client.subscribe("tele/+/SENSOR", QoS::AtMostOnce).await;
        let _ = client.subscribe("stat/+/POWER",  QoS::AtMostOnce).await;

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    info!("Tasmota MQTT connecté (broker {}:{})", mqtt_cfg.host, mqtt_cfg.port);
                }

                Ok(Event::Incoming(Incoming::Publish(msg))) => {
                    let topic   = msg.topic.as_str();
                    let payload = match std::str::from_utf8(&msg.payload) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    // Découper le topic : tele/{device_name}/SENSOR ou stat/{device_name}/POWER
                    let parts: Vec<&str> = topic.split('/').collect();
                    if parts.len() != 3 { continue; }
                    let device_name = parts[1];
                    let msg_type    = parts[2];

                    // Trouver la config du device par son tasmota_id
                    let dev_cfg = match devices.iter().find(|d| d.tasmota_id == device_name) {
                        Some(d) => d,
                        None    => continue,
                    };
                    let id = dev_cfg.id;

                    match msg_type {
                        "POWER" => {
                            let on = payload.trim().eq_ignore_ascii_case("ON");
                            power_states.insert(id, on);
                            // Si un snapshot énergie existe déjà, mettre à jour et notifier
                            if let Some(snap) = energy_cache.get_mut(&id) {
                                snap.power_on  = on;
                                snap.timestamp = Local::now();
                                on_snapshot(snap.clone());
                            }
                        }

                        "SENSOR" => {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) {
                                let energy   = &json["ENERGY"];
                                let power_on = power_states.get(&id).copied().unwrap_or(true);

                                let snap = TasmotaSnapshot {
                                    id,
                                    name:                  dev_cfg.name.clone(),
                                    tasmota_id:            dev_cfg.tasmota_id.clone(),
                                    timestamp:             Local::now(),
                                    power_on,
                                    power_w:               energy["Power"]        .as_f64().unwrap_or(0.0) as f32,
                                    voltage_v:             energy["Voltage"]      .as_f64().unwrap_or(0.0) as f32,
                                    current_a:             energy["Current"]      .as_f64().unwrap_or(0.0) as f32,
                                    apparent_power_va:     energy["ApparentPower"].as_f64().unwrap_or(0.0) as f32,
                                    power_factor:          energy["Factor"]       .as_f64().unwrap_or(0.0) as f32,
                                    energy_today_kwh:      energy["Today"]        .as_f64().unwrap_or(0.0) as f32,
                                    energy_yesterday_kwh:  energy["Yesterday"]    .as_f64().unwrap_or(0.0) as f32,
                                    energy_total_kwh:      energy["Total"]        .as_f64().unwrap_or(0.0) as f32,
                                    rssi:                  json["Wifi"]["RSSI"]   .as_i64().map(|v| v as i32),
                                };
                                energy_cache.insert(id, snap.clone());
                                on_snapshot(snap);
                            }
                        }

                        _ => {}
                    }
                }

                Ok(_) => {}

                Err(e) => {
                    warn!("Tasmota MQTT erreur : {:?}", e);
                    break; // sortir pour reconnecter
                }
            }
        }

        warn!("Tasmota MQTT déconnecté — reconnexion dans 10s");
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
