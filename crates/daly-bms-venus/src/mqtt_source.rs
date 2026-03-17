//! Source MQTT : abonnement au broker et parsing des payloads Venus OS.
//!
//! S'abonne sur `{prefix}/+/venus` et émet des événements `MqttEvent`
//! portant l'`mqtt_index` (déduit du topic) et le `VenusPayload` parsé.

use crate::config::MqttRef;
use crate::types::VenusPayload;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

// =============================================================================
// Événement émis vers le manager
// =============================================================================

/// Événement reçu depuis MQTT.
#[derive(Debug)]
pub struct MqttEvent {
    /// Index du BMS (déduit du topic : `prefix/1/venus` → `1`)
    pub mqtt_index: u8,
    /// Payload Venus OS parsé
    pub payload: VenusPayload,
}

// =============================================================================
// Tâche MQTT
// =============================================================================

/// Démarre la tâche MQTT et retourne un channel de réception.
///
/// Écoute sur `{cfg.topic_prefix}/+/venus` et envoie les événements parsés.
pub async fn start_mqtt_source(
    cfg: MqttRef,
    tx: mpsc::Sender<MqttEvent>,
) {
    let topic_prefix = cfg.topic_prefix.clone();
    let subscribe_topic = format!("{}/+/venus", topic_prefix);

    info!(
        broker = %format!("{}:{}", cfg.host, cfg.port),
        topic = %subscribe_topic,
        "Démarrage source MQTT Venus"
    );

    loop {
        match connect_and_run(&cfg, &subscribe_topic, &topic_prefix, &tx).await {
            Ok(()) => {
                // Connexion fermée proprement (ne devrait pas arriver)
                warn!("MQTT source terminée de façon inattendue, reconnexion dans 5s");
            }
            Err(e) => {
                error!("MQTT source erreur : {:#}, reconnexion dans 5s", e);
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn connect_and_run(
    cfg: &MqttRef,
    subscribe_topic: &str,
    topic_prefix: &str,
    tx: &mpsc::Sender<MqttEvent>,
) -> anyhow::Result<()> {
    let client_id = format!("daly-bms-venus-{}", uuid_short());
    let mut opts = MqttOptions::new(client_id, &cfg.host, cfg.port);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);

    if let (Some(user), Some(pass)) = (&cfg.username, &cfg.password) {
        opts.set_credentials(user, pass);
    }

    let (client, mut eventloop) = AsyncClient::new(opts, 64);

    client
        .subscribe(subscribe_topic, QoS::AtLeastOnce)
        .await?;

    info!("MQTT connecté, abonnement sur '{}'", subscribe_topic);

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(pub_msg))) => {
                let topic = &pub_msg.topic;
                debug!(topic = %topic, "MQTT message reçu");

                if let Some(idx) = extract_mqtt_index(topic, topic_prefix) {
                    match serde_json::from_slice::<VenusPayload>(&pub_msg.payload) {
                        Ok(payload) => {
                            let evt = MqttEvent { mqtt_index: idx, payload };
                            if tx.send(evt).await.is_err() {
                                // Le manager a été arrêté
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            warn!(topic = %topic, "Échec parsing payload Venus: {}", e);
                        }
                    }
                }
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                info!("MQTT ConnAck reçu — broker connecté");
            }
            Ok(_) => {}
            Err(e) => {
                return Err(e.into());
            }
        }
    }
}

/// Extrait l'`mqtt_index` depuis un topic de la forme `{prefix}/{index}/venus`.
///
/// `extract_mqtt_index("santuario/bms/2/venus", "santuario/bms")` → `Some(2)`
fn extract_mqtt_index(topic: &str, prefix: &str) -> Option<u8> {
    // Le topic a la forme: {prefix}/{index}/venus
    let rest = topic.strip_prefix(prefix)?.strip_prefix('/')?;
    let (index_str, suffix) = rest.split_once('/')?;
    if suffix != "venus" {
        return None;
    }
    index_str.parse::<u8>().ok()
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{:08x}", t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_index() {
        assert_eq!(extract_mqtt_index("santuario/bms/1/venus", "santuario/bms"), Some(1));
        assert_eq!(extract_mqtt_index("santuario/bms/2/venus", "santuario/bms"), Some(2));
        assert_eq!(extract_mqtt_index("santuario/bms/2/status", "santuario/bms"), None);
        assert_eq!(extract_mqtt_index("other/1/venus", "santuario/bms"), None);
        assert_eq!(extract_mqtt_index("santuario/bms/255/venus", "santuario/bms"), Some(255));
    }
}
