//! Manager des services D-Bus heatpump — orchestre N pompes à chaleur.
//!
//! Reçoit les `HeatpumpMqttEvent` depuis `mqtt_source` et les route vers
//! le `HeatpumpServiceHandle` correspondant.
//! Topic MQTT : `santuario/heatpump/{n}/venus`
//! Service D-Bus : `com.victronenergy.heatpump.{prefix}_{n}`

use crate::config::{HeatpumpRef, VenusConfig};
use crate::heatpump_service::{HeatpumpServiceHandle, create_heatpump_service};
use crate::mqtt_source::HeatpumpMqttEvent;
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info, warn};

pub struct HeatpumpManager {
    cfg:           VenusConfig,
    heatpump_refs: Vec<HeatpumpRef>,
    services:      HashMap<u8, HeatpumpServiceHandle>,
    rx:            mpsc::Receiver<HeatpumpMqttEvent>,
}

impl HeatpumpManager {
    pub fn new(
        cfg:           VenusConfig,
        heatpump_refs: Vec<HeatpumpRef>,
        rx:            mpsc::Receiver<HeatpumpMqttEvent>,
    ) -> Self {
        Self { cfg, heatpump_refs, services: HashMap::new(), rx }
    }

    pub async fn run(mut self) -> Result<()> {
        if !self.cfg.enabled {
            info!("Service heatpump D-Bus désactivé (enabled = false)");
            while self.rx.recv().await.is_some() {}
            return Ok(());
        }

        let watchdog_dur  = Duration::from_secs(self.cfg.watchdog_sec);
        let republish_dur = Duration::from_secs(self.cfg.republish_sec);
        let mut tick      = interval(republish_dur);

        info!(
            dbus_bus     = %self.cfg.dbus_bus,
            heatpumps    = self.heatpump_refs.len(),
            watchdog_sec = self.cfg.watchdog_sec,
            "HeatpumpManager démarré"
        );

        loop {
            tokio::select! {
                Some(evt) = self.rx.recv() => {
                    if let Err(e) = self.handle_event(evt).await {
                        error!("Erreur événement heatpump MQTT : {:#}", e);
                    }
                }
                _ = tick.tick() => {
                    self.republish_and_watchdog(watchdog_dur).await;
                }
            }
        }
    }

    async fn handle_event(&mut self, evt: HeatpumpMqttEvent) -> Result<()> {
        let idx = evt.mqtt_index;
        if !self.services.contains_key(&idx) {
            let handle = self.create_service(idx).await?;
            self.services.insert(idx, handle);
        }
        if let Some(svc) = self.services.get(&idx) {
            svc.update(&evt.payload).await?;
        }
        Ok(())
    }

    async fn create_service(&self, idx: u8) -> Result<HeatpumpServiceHandle> {
        let suffix          = format!("{}_{}", self.cfg.service_prefix, idx);
        let device_instance = self.device_instance_for(idx);
        let product_name    = self.product_name_for(idx);
        create_heatpump_service(&self.cfg.dbus_bus, &suffix, device_instance, product_name).await
    }

    fn device_instance_for(&self, idx: u8) -> u32 {
        for (pos, h) in self.heatpump_refs.iter().enumerate() {
            let hi = h.mqtt_index.unwrap_or((pos + 1) as u8);
            if hi == idx { return h.device_instance.unwrap_or(hi as u32); }
        }
        idx as u32
    }

    fn product_name_for(&self, idx: u8) -> String {
        for (pos, h) in self.heatpump_refs.iter().enumerate() {
            let hi = h.mqtt_index.unwrap_or((pos + 1) as u8);
            if hi == idx { if let Some(n) = &h.name { return n.clone(); } }
        }
        format!("Heat Pump {}", idx)
    }

    async fn republish_and_watchdog(&self, watchdog_dur: Duration) {
        let now = Instant::now();
        for (idx, svc) in &self.services {
            let last = { svc.values.lock().unwrap().last_update };
            if now.duration_since(last) > watchdog_dur {
                if let Err(e) = svc.set_disconnected().await {
                    warn!(index = idx, "Erreur watchdog heatpump : {:#}", e);
                }
            } else if let Err(e) = svc.republish().await {
                warn!(index = idx, "Erreur republication heatpump : {:#}", e);
            }
        }
    }
}
