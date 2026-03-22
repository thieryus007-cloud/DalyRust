//! Source MQTT : abonnement au broker et parsing des payloads Venus OS.
//!
//! ## Topics surveillés
//!
//! - `{bms_prefix}/+/venus`  → batteries BMS Daly → `MqttEvent`
//! - `{heat_prefix}/+/venus` → capteurs température → `SensorMqttEvent`

use crate::config::MqttRef;
use crate::types::{GridPayload, HeatPayload, HeatpumpPayload, MeteoPayload, PlatformPayload, PvinverterPayload, SwitchPayload, VenusPayload};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

// =============================================================================
// Événements émis vers les managers
// =============================================================================

/// Événement batterie reçu depuis MQTT (topic BMS).
#[derive(Debug)]
pub struct MqttEvent {
    /// Index du BMS (déduit du topic : `prefix/1/venus` → `1`)
    pub mqtt_index: u8,
    /// Payload Venus OS parsé
    pub payload: VenusPayload,
}

/// Événement capteur température reçu depuis MQTT (topic heat).
#[derive(Debug)]
pub struct SensorMqttEvent {
    /// Index du capteur (déduit du topic : `prefix/1/venus` → `1`)
    pub mqtt_index: u8,
    /// Payload température parsé
    pub payload: HeatPayload,
}

/// Événement pompe à chaleur reçu depuis MQTT (topic heatpump).
#[derive(Debug)]
pub struct HeatpumpMqttEvent {
    /// Index (ex: `santuario/heatpump/1/venus` → `1`)
    pub mqtt_index: u8,
    /// Payload pompe à chaleur parsé
    pub payload: HeatpumpPayload,
}

/// Événement capteur météo reçu depuis MQTT (topic fixe `santuario/meteo/venus`).
#[derive(Debug)]
pub struct MeteoMqttEvent {
    /// Payload irradiance/météo parsé
    pub payload: MeteoPayload,
}

/// Événement switch/ATS reçu depuis MQTT (topic switch).
#[derive(Debug)]
pub struct SwitchMqttEvent {
    /// Index du switch (déduit du topic : `prefix/1/venus` → `1`)
    pub mqtt_index: u8,
    /// Payload switch parsé
    pub payload: SwitchPayload,
}

/// Événement compteur réseau/acload reçu depuis MQTT (topic grid).
#[derive(Debug)]
pub struct GridMqttEvent {
    /// Index du compteur (déduit du topic : `prefix/1/venus` → `1`)
    pub mqtt_index: u8,
    /// Payload grid/acload parsé
    pub payload: GridPayload,
}

/// Événement platform reçu depuis MQTT (topic fixe `santuario/platform/venus`).
#[derive(Debug)]
pub struct PlatformMqttEvent {
    /// Payload platform parsé
    pub payload: PlatformPayload,
}

/// Événement onduleur PV / compteur ET112 reçu depuis MQTT (topic pvinverter).
#[derive(Debug)]
pub struct PvinverterMqttEvent {
    /// Index (ex: `santuario/pvinverter/3/venus` → `3`)
    pub mqtt_index: u8,
    /// Payload pvinverter parsé
    pub payload: PvinverterPayload,
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
    let client_id = format!("dbus-mqtt-venus-{}", uuid_short());
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

// =============================================================================
// Source MQTT pour capteurs de température (santuario/heat/{n}/venus)
// =============================================================================

/// Démarre la source MQTT pour les capteurs de température.
///
/// S'abonne sur `{heat_prefix}/+/venus`, parse le `HeatPayload` et émet
/// des `SensorMqttEvent` vers le `SensorManager`.
pub async fn start_sensor_mqtt_source(
    cfg:        MqttRef,
    heat_prefix: String,
    tx:         mpsc::Sender<SensorMqttEvent>,
) {
    let subscribe_topic = format!("{}/+/venus", heat_prefix);

    info!(
        broker = %format!("{}:{}", cfg.host, cfg.port),
        topic  = %subscribe_topic,
        "Démarrage source MQTT capteurs température"
    );

    loop {
        match sensor_connect_and_run(&cfg, &subscribe_topic, &heat_prefix, &tx).await {
            Ok(()) => {
                warn!("MQTT source capteurs terminée de façon inattendue, reconnexion dans 5s");
            }
            Err(e) => {
                error!("MQTT source capteurs erreur : {:#}, reconnexion dans 5s", e);
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn sensor_connect_and_run(
    cfg:         &MqttRef,
    subscribe_topic: &str,
    heat_prefix: &str,
    tx:          &mpsc::Sender<SensorMqttEvent>,
) -> anyhow::Result<()> {
    let client_id = format!("dbus-mqtt-venus-sensors-{}", uuid_short());
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

    info!("MQTT capteurs connecté, abonnement sur '{}'", subscribe_topic);

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(pub_msg))) => {
                let topic = &pub_msg.topic;
                debug!(topic = %topic, "MQTT capteur message reçu");

                if let Some(idx) = extract_mqtt_index(topic, heat_prefix) {
                    match serde_json::from_slice::<HeatPayload>(&pub_msg.payload) {
                        Ok(payload) => {
                            let evt = SensorMqttEvent { mqtt_index: idx, payload };
                            if tx.send(evt).await.is_err() {
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            warn!(topic = %topic, "Échec parsing payload capteur: {}", e);
                        }
                    }
                }
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                info!("MQTT capteurs ConnAck reçu — broker connecté");
            }
            Ok(_) => {}
            Err(e) => {
                return Err(e.into());
            }
        }
    }
}

// =============================================================================
// Source MQTT pour pompes à chaleur (santuario/heatpump/{n}/venus)
// =============================================================================

/// Démarre la source MQTT pour les pompes à chaleur / chauffe-eau.
///
/// S'abonne sur `{heatpump_prefix}/+/venus`, parse le `HeatpumpPayload`
/// et émet des `HeatpumpMqttEvent` vers le `HeatpumpManager`.
pub async fn start_heatpump_mqtt_source(
    cfg:              MqttRef,
    heatpump_prefix:  String,
    tx:               mpsc::Sender<HeatpumpMqttEvent>,
) {
    let subscribe_topic = format!("{}/+/venus", heatpump_prefix);

    info!(
        broker = %format!("{}:{}", cfg.host, cfg.port),
        topic  = %subscribe_topic,
        "Démarrage source MQTT pompes à chaleur"
    );

    loop {
        match heatpump_connect_and_run(&cfg, &subscribe_topic, &heatpump_prefix, &tx).await {
            Ok(())  => warn!("MQTT source heatpump terminée de façon inattendue, reconnexion dans 5s"),
            Err(e)  => error!("MQTT source heatpump erreur : {:#}, reconnexion dans 5s", e),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn heatpump_connect_and_run(
    cfg:              &MqttRef,
    subscribe_topic:  &str,
    heatpump_prefix:  &str,
    tx:               &mpsc::Sender<HeatpumpMqttEvent>,
) -> anyhow::Result<()> {
    let client_id = format!("dbus-mqtt-venus-heatpump-{}", uuid_short());
    let mut opts = MqttOptions::new(client_id, &cfg.host, cfg.port);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);
    if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) { opts.set_credentials(u, p); }

    let (client, mut eventloop) = AsyncClient::new(opts, 64);
    client.subscribe(subscribe_topic, QoS::AtLeastOnce).await?;
    info!("MQTT heatpump connecté, abonnement sur '{}'", subscribe_topic);

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(msg))) => {
                let topic = &msg.topic;
                debug!(topic = %topic, "MQTT heatpump message reçu");
                if let Some(idx) = extract_mqtt_index(topic, heatpump_prefix) {
                    match serde_json::from_slice::<HeatpumpPayload>(&msg.payload) {
                        Ok(payload) => {
                            let evt = HeatpumpMqttEvent { mqtt_index: idx, payload };
                            if tx.send(evt).await.is_err() { return Ok(()); }
                        }
                        Err(e) => warn!(topic = %topic, "Échec parsing payload heatpump: {}", e),
                    }
                }
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => info!("MQTT heatpump ConnAck reçu"),
            Ok(_) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

// =============================================================================
// Source MQTT pour capteur météo (santuario/meteo/venus — topic fixe)
// =============================================================================

/// Démarre la source MQTT pour le capteur météo/irradiance.
///
/// S'abonne sur le topic FIXE `{meteo_topic}` (ex: `santuario/meteo/venus`),
/// parse le `MeteoPayload` et émet des `MeteoMqttEvent` vers le `MeteoManager`.
pub async fn start_meteo_mqtt_source(
    cfg:         MqttRef,
    meteo_topic: String,
    tx:          mpsc::Sender<MeteoMqttEvent>,
) {
    info!(
        broker = %format!("{}:{}", cfg.host, cfg.port),
        topic  = %meteo_topic,
        "Démarrage source MQTT capteur météo"
    );

    loop {
        match meteo_connect_and_run(&cfg, &meteo_topic, &tx).await {
            Ok(())  => warn!("MQTT source météo terminée de façon inattendue, reconnexion dans 5s"),
            Err(e)  => error!("MQTT source météo erreur : {:#}, reconnexion dans 5s", e),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn meteo_connect_and_run(
    cfg:         &MqttRef,
    meteo_topic: &str,
    tx:          &mpsc::Sender<MeteoMqttEvent>,
) -> anyhow::Result<()> {
    let client_id = format!("dbus-mqtt-venus-meteo-{}", uuid_short());
    let mut opts = MqttOptions::new(client_id, &cfg.host, cfg.port);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);
    if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) { opts.set_credentials(u, p); }

    let (client, mut eventloop) = AsyncClient::new(opts, 64);
    client.subscribe(meteo_topic, QoS::AtLeastOnce).await?;
    info!("MQTT météo connecté, abonnement sur '{}'", meteo_topic);

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(msg))) => {
                debug!(topic = %msg.topic, "MQTT météo message reçu");
                match serde_json::from_slice::<MeteoPayload>(&msg.payload) {
                    Ok(payload) => {
                        let evt = MeteoMqttEvent { payload };
                        if tx.send(evt).await.is_err() { return Ok(()); }
                    }
                    Err(e) => warn!(topic = %msg.topic, "Échec parsing payload météo: {}", e),
                }
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => info!("MQTT météo ConnAck reçu"),
            Ok(_) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

// =============================================================================
// Source MQTT pour switches/ATS (santuario/switch/{n}/venus)
// =============================================================================

/// Démarre la source MQTT pour les switches/ATS.
///
/// S'abonne sur `{switch_prefix}/+/venus`, parse le `SwitchPayload`
/// et émet des `SwitchMqttEvent` vers le `SwitchManager`.
pub async fn start_switch_mqtt_source(
    cfg:           MqttRef,
    switch_prefix: String,
    tx:            mpsc::Sender<SwitchMqttEvent>,
) {
    let subscribe_topic = format!("{}/+/venus", switch_prefix);

    info!(
        broker = %format!("{}:{}", cfg.host, cfg.port),
        topic  = %subscribe_topic,
        "Démarrage source MQTT switches/ATS"
    );

    loop {
        match switch_connect_and_run(&cfg, &subscribe_topic, &switch_prefix, &tx).await {
            Ok(())  => warn!("MQTT source switch terminée de façon inattendue, reconnexion dans 5s"),
            Err(e)  => error!("MQTT source switch erreur : {:#}, reconnexion dans 5s", e),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn switch_connect_and_run(
    cfg:           &MqttRef,
    subscribe_topic: &str,
    switch_prefix: &str,
    tx:            &mpsc::Sender<SwitchMqttEvent>,
) -> anyhow::Result<()> {
    let client_id = format!("dbus-mqtt-venus-switch-{}", uuid_short());
    let mut opts = MqttOptions::new(client_id, &cfg.host, cfg.port);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);
    if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) { opts.set_credentials(u, p); }

    let (client, mut eventloop) = AsyncClient::new(opts, 64);
    client.subscribe(subscribe_topic, QoS::AtLeastOnce).await?;
    info!("MQTT switch connecté, abonnement sur '{}'", subscribe_topic);

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(msg))) => {
                let topic = &msg.topic;
                debug!(topic = %topic, "MQTT switch message reçu");
                if let Some(idx) = extract_mqtt_index(topic, switch_prefix) {
                    match serde_json::from_slice::<SwitchPayload>(&msg.payload) {
                        Ok(payload) => {
                            let evt = SwitchMqttEvent { mqtt_index: idx, payload };
                            if tx.send(evt).await.is_err() { return Ok(()); }
                        }
                        Err(e) => warn!(topic = %topic, "Échec parsing payload switch: {}", e),
                    }
                }
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => info!("MQTT switch ConnAck reçu"),
            Ok(_) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

// =============================================================================
// Source MQTT pour compteurs réseau/acload (santuario/grid/{n}/venus)
// =============================================================================

/// Démarre la source MQTT pour les compteurs réseau/acload.
///
/// S'abonne sur `{grid_prefix}/+/venus`, parse le `GridPayload`
/// et émet des `GridMqttEvent` vers le `GridManager`.
pub async fn start_grid_mqtt_source(
    cfg:         MqttRef,
    grid_prefix: String,
    tx:          mpsc::Sender<GridMqttEvent>,
) {
    let subscribe_topic = format!("{}/+/venus", grid_prefix);

    info!(
        broker = %format!("{}:{}", cfg.host, cfg.port),
        topic  = %subscribe_topic,
        "Démarrage source MQTT compteurs réseau/acload"
    );

    loop {
        match grid_connect_and_run(&cfg, &subscribe_topic, &grid_prefix, &tx).await {
            Ok(())  => warn!("MQTT source grid terminée de façon inattendue, reconnexion dans 5s"),
            Err(e)  => error!("MQTT source grid erreur : {:#}, reconnexion dans 5s", e),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn grid_connect_and_run(
    cfg:         &MqttRef,
    subscribe_topic: &str,
    grid_prefix: &str,
    tx:          &mpsc::Sender<GridMqttEvent>,
) -> anyhow::Result<()> {
    let client_id = format!("dbus-mqtt-venus-grid-{}", uuid_short());
    let mut opts = MqttOptions::new(client_id, &cfg.host, cfg.port);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);
    if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) { opts.set_credentials(u, p); }

    let (client, mut eventloop) = AsyncClient::new(opts, 64);
    client.subscribe(subscribe_topic, QoS::AtLeastOnce).await?;
    info!("MQTT grid connecté, abonnement sur '{}'", subscribe_topic);

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(msg))) => {
                let topic = &msg.topic;
                debug!(topic = %topic, "MQTT grid message reçu");
                if let Some(idx) = extract_mqtt_index(topic, grid_prefix) {
                    match serde_json::from_slice::<GridPayload>(&msg.payload) {
                        Ok(payload) => {
                            let evt = GridMqttEvent { mqtt_index: idx, payload };
                            if tx.send(evt).await.is_err() { return Ok(()); }
                        }
                        Err(e) => warn!(topic = %topic, "Échec parsing payload grid: {}", e),
                    }
                }
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => info!("MQTT grid ConnAck reçu"),
            Ok(_) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

// =============================================================================
// Source MQTT pour platform (santuario/platform/venus — topic fixe)
// =============================================================================

/// Démarre la source MQTT pour le service platform/backup Pi5.
///
/// S'abonne sur le topic FIXE `{platform_topic}`, parse le `PlatformPayload`
/// et émet des `PlatformMqttEvent` vers le `PlatformManager`.
pub async fn start_platform_mqtt_source(
    cfg:            MqttRef,
    platform_topic: String,
    tx:             mpsc::Sender<PlatformMqttEvent>,
) {
    info!(
        broker = %format!("{}:{}", cfg.host, cfg.port),
        topic  = %platform_topic,
        "Démarrage source MQTT platform"
    );

    loop {
        match platform_connect_and_run(&cfg, &platform_topic, &tx).await {
            Ok(())  => warn!("MQTT source platform terminée de façon inattendue, reconnexion dans 5s"),
            Err(e)  => error!("MQTT source platform erreur : {:#}, reconnexion dans 5s", e),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn platform_connect_and_run(
    cfg:            &MqttRef,
    platform_topic: &str,
    tx:             &mpsc::Sender<PlatformMqttEvent>,
) -> anyhow::Result<()> {
    let client_id = format!("dbus-mqtt-venus-platform-{}", uuid_short());
    let mut opts = MqttOptions::new(client_id, &cfg.host, cfg.port);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);
    if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) { opts.set_credentials(u, p); }

    let (client, mut eventloop) = AsyncClient::new(opts, 64);
    client.subscribe(platform_topic, QoS::AtLeastOnce).await?;
    info!("MQTT platform connecté, abonnement sur '{}'", platform_topic);

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(msg))) => {
                debug!(topic = %msg.topic, "MQTT platform message reçu");
                match serde_json::from_slice::<PlatformPayload>(&msg.payload) {
                    Ok(payload) => {
                        let evt = PlatformMqttEvent { payload };
                        if tx.send(evt).await.is_err() { return Ok(()); }
                    }
                    Err(e) => warn!(topic = %msg.topic, "Échec parsing payload platform: {}", e),
                }
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => info!("MQTT platform ConnAck reçu"),
            Ok(_) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

// =============================================================================
// Source MQTT pour onduleurs PV / compteurs ET112 (santuario/pvinverter/{n}/venus)
// =============================================================================

/// Démarre la source MQTT pour les onduleurs PV / compteurs ET112.
///
/// S'abonne sur `{pvinverter_prefix}/+/venus`, parse le `PvinverterPayload`
/// et émet des `PvinverterMqttEvent` vers le `PvinverterManager`.
pub async fn start_pvinverter_mqtt_source(
    cfg:               MqttRef,
    pvinverter_prefix: String,
    tx:                mpsc::Sender<PvinverterMqttEvent>,
) {
    let subscribe_topic = format!("{}/+/venus", pvinverter_prefix);

    info!(
        broker = %format!("{}:{}", cfg.host, cfg.port),
        topic  = %subscribe_topic,
        "Démarrage source MQTT pvinverter/ET112"
    );

    loop {
        match pvinverter_connect_and_run(&cfg, &subscribe_topic, &pvinverter_prefix, &tx).await {
            Ok(())  => warn!("MQTT source pvinverter terminée de façon inattendue, reconnexion dans 5s"),
            Err(e)  => error!("MQTT source pvinverter erreur : {:#}, reconnexion dans 5s", e),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn pvinverter_connect_and_run(
    cfg:               &MqttRef,
    subscribe_topic:   &str,
    pvinverter_prefix: &str,
    tx:                &mpsc::Sender<PvinverterMqttEvent>,
) -> anyhow::Result<()> {
    let client_id = format!("dbus-mqtt-venus-pvinverter-{}", uuid_short());
    let mut opts = MqttOptions::new(client_id, &cfg.host, cfg.port);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);
    if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) { opts.set_credentials(u, p); }

    let (client, mut eventloop) = AsyncClient::new(opts, 64);
    client.subscribe(subscribe_topic, QoS::AtLeastOnce).await?;
    info!("MQTT pvinverter connecté, abonnement sur '{}'", subscribe_topic);

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(msg))) => {
                let topic = &msg.topic;
                debug!(topic = %topic, "MQTT pvinverter message reçu");
                if let Some(idx) = extract_mqtt_index(topic, pvinverter_prefix) {
                    match serde_json::from_slice::<PvinverterPayload>(&msg.payload) {
                        Ok(payload) => {
                            let evt = PvinverterMqttEvent { mqtt_index: idx, payload };
                            if tx.send(evt).await.is_err() { return Ok(()); }
                        }
                        Err(e) => warn!(topic = %topic, "Échec parsing payload pvinverter: {}", e),
                    }
                }
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => info!("MQTT pvinverter ConnAck reçu"),
            Ok(_) => {}
            Err(e) => return Err(e.into()),
        }
    }
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
        assert_eq!(extract_mqtt_index("santuario/heat/1/venus", "santuario/heat"), Some(1));
        assert_eq!(extract_mqtt_index("santuario/heat/2/venus", "santuario/heat"), Some(2));
    }
}
