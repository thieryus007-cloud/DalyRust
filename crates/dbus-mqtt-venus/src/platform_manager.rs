//! Manager du service D-Bus platform (singleton).
//!
//! Reçoit les `PlatformMqttEvent` depuis `mqtt_source` et met à jour
//! le `PlatformServiceHandle` unique (pas d'index — service singleton).
//! Topic MQTT fixe : `santuario/platform/venus`
//! Service D-Bus  : `com.victronenergy.platform`

use crate::config::{PlatformConfig, VenusConfig};
use crate::mqtt_source::PlatformMqttEvent;
use crate::platform_service::{PlatformServiceHandle, create_platform_service};
use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info, warn};

pub struct PlatformManager {
    cfg:            VenusConfig,
    platform_cfg:   PlatformConfig,
    service:        Option<PlatformServiceHandle>,
    rx:             mpsc::Receiver<PlatformMqttEvent>,
}

impl PlatformManager {
    pub fn new(
        cfg:          VenusConfig,
        platform_cfg: PlatformConfig,
        rx:           mpsc::Receiver<PlatformMqttEvent>,
    ) -> Self {
        Self { cfg, platform_cfg, service: None, rx }
    }

    pub async fn run(mut self) -> Result<()> {
        if !self.cfg.enabled || !self.platform_cfg.enabled {
            info!("Service platform D-Bus désactivé");
            while self.rx.recv().await.is_some() {}
            return Ok(());
        }

        let watchdog_dur  = Duration::from_secs(self.cfg.watchdog_sec);
        let republish_dur = Duration::from_secs(self.cfg.republish_sec);
        let mut tick      = interval(republish_dur);

        info!(
            dbus_bus     = %self.cfg.dbus_bus,
            topic        = %self.platform_cfg.topic,
            watchdog_sec = self.cfg.watchdog_sec,
            "PlatformManager démarré"
        );

        loop {
            tokio::select! {
                Some(evt) = self.rx.recv() => {
                    if let Err(e) = self.handle_event(evt).await {
                        error!("Erreur événement platform MQTT : {:#}", e);
                    }
                }
                _ = tick.tick() => {
                    self.republish_and_watchdog(watchdog_dur).await;
                }
            }
        }
    }

    async fn handle_event(&mut self, evt: PlatformMqttEvent) -> Result<()> {
        if self.service.is_none() {
            let svc = create_platform_service(
                &self.cfg.dbus_bus,
                self.platform_cfg.device_instance,
                self.platform_cfg.product_name.clone(),
            ).await?;
            self.service = Some(svc);
        }
        if let Some(svc) = &self.service {
            svc.update(&evt.payload).await?;
        }
        Ok(())
    }

    async fn republish_and_watchdog(&self, watchdog_dur: Duration) {
        let Some(svc) = &self.service else { return };
        let last = { svc.values.lock().unwrap().last_update };
        let now  = Instant::now();
        if now.duration_since(last) > watchdog_dur {
            if let Err(e) = svc.set_disconnected().await {
                warn!("Erreur watchdog platform : {:#}", e);
            }
        } else if let Err(e) = svc.republish().await {
            warn!("Erreur republication platform : {:#}", e);
        }
    }
}
