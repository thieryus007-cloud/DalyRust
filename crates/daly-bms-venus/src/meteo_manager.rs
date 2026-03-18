//! Manager du service D-Bus météo — gère le service unique `com.victronenergy.meteo`.
//!
//! Reçoit les `MeteoMqttEvent` depuis `mqtt_source` (topic `santuario/meteo/venus`)
//! et maintient le service D-Bus avec watchdog + keepalive.

use crate::config::{MeteoConfig, VenusConfig};
use crate::meteo_service::{MeteoServiceHandle, create_meteo_service};
use crate::mqtt_source::MeteoMqttEvent;
use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info, warn};

pub struct MeteoManager {
    cfg:        VenusConfig,
    meteo_cfg:  MeteoConfig,
    service:    Option<MeteoServiceHandle>,
    rx:         mpsc::Receiver<MeteoMqttEvent>,
}

impl MeteoManager {
    pub fn new(
        cfg:       VenusConfig,
        meteo_cfg: MeteoConfig,
        rx:        mpsc::Receiver<MeteoMqttEvent>,
    ) -> Self {
        Self { cfg, meteo_cfg, service: None, rx }
    }

    pub async fn run(mut self) -> Result<()> {
        if !self.cfg.enabled {
            info!("Service météo D-Bus désactivé (enabled = false)");
            while self.rx.recv().await.is_some() {}
            return Ok(());
        }

        let watchdog_dur  = Duration::from_secs(self.cfg.watchdog_sec);
        let republish_dur = Duration::from_secs(self.cfg.republish_sec);
        let mut tick      = interval(republish_dur);

        info!(
            dbus_bus     = %self.cfg.dbus_bus,
            device_instance = self.meteo_cfg.device_instance,
            watchdog_sec = self.cfg.watchdog_sec,
            "MeteoManager démarré"
        );

        loop {
            tokio::select! {
                Some(evt) = self.rx.recv() => {
                    if let Err(e) = self.handle_event(evt).await {
                        error!("Erreur événement météo MQTT : {:#}", e);
                    }
                }
                _ = tick.tick() => {
                    self.republish_and_watchdog(watchdog_dur).await;
                }
            }
        }
    }

    async fn handle_event(&mut self, evt: MeteoMqttEvent) -> Result<()> {
        // Créer le service D-Bus au premier message reçu
        if self.service.is_none() {
            let svc = create_meteo_service(
                &self.cfg.dbus_bus,
                self.meteo_cfg.device_instance,
                self.meteo_cfg.product_name.clone(),
            )
            .await?;
            self.service = Some(svc);
        }

        if let Some(svc) = &self.service {
            svc.update(&evt.payload).await?;
        }
        Ok(())
    }

    async fn republish_and_watchdog(&self, watchdog_dur: Duration) {
        if let Some(svc) = &self.service {
            let last = { svc.values.lock().unwrap().last_update };
            if Instant::now().duration_since(last) > watchdog_dur {
                if let Err(e) = svc.set_disconnected().await {
                    warn!("Erreur watchdog météo : {:#}", e);
                }
            } else if let Err(e) = svc.republish().await {
                warn!("Erreur republication météo : {:#}", e);
            }
        }
    }
}
