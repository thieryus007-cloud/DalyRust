//! Manager des services D-Bus switch — orchestre N switches/ATS.
//!
//! Reçoit les `SwitchMqttEvent` depuis `mqtt_source` et les route vers
//! le `SwitchServiceHandle` correspondant.
//! Topic MQTT : `santuario/switch/{n}/venus`
//! Service D-Bus : `com.victronenergy.switch.{prefix}_{n}`

use crate::config::{SwitchRef, VenusConfig};
use crate::mqtt_source::SwitchMqttEvent;
use crate::switch_service::{SwitchServiceHandle, create_switch_service};
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info, warn};

pub struct SwitchManager {
    cfg:         VenusConfig,
    switch_refs: Vec<SwitchRef>,
    services:    HashMap<u8, SwitchServiceHandle>,
    rx:          mpsc::Receiver<SwitchMqttEvent>,
}

impl SwitchManager {
    pub fn new(
        cfg:         VenusConfig,
        switch_refs: Vec<SwitchRef>,
        rx:          mpsc::Receiver<SwitchMqttEvent>,
    ) -> Self {
        Self { cfg, switch_refs, services: HashMap::new(), rx }
    }

    pub async fn run(mut self) -> Result<()> {
        if !self.cfg.enabled {
            info!("Service switch D-Bus désactivé (enabled = false)");
            while self.rx.recv().await.is_some() {}
            return Ok(());
        }

        let watchdog_dur  = Duration::from_secs(self.cfg.watchdog_sec);
        let republish_dur = Duration::from_secs(self.cfg.republish_sec);
        let mut tick      = interval(republish_dur);

        info!(
            dbus_bus     = %self.cfg.dbus_bus,
            switches     = self.switch_refs.len(),
            watchdog_sec = self.cfg.watchdog_sec,
            "SwitchManager démarré"
        );

        loop {
            tokio::select! {
                Some(evt) = self.rx.recv() => {
                    if let Err(e) = self.handle_event(evt).await {
                        error!("Erreur événement switch MQTT : {:#}", e);
                    }
                }
                _ = tick.tick() => {
                    self.republish_and_watchdog(watchdog_dur).await;
                }
            }
        }
    }

    async fn handle_event(&mut self, evt: SwitchMqttEvent) -> Result<()> {
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

    async fn create_service(&self, idx: u8) -> Result<SwitchServiceHandle> {
        let suffix          = format!("{}_{}", self.cfg.service_prefix, idx);
        let device_instance = self.device_instance_for(idx);
        let product_name    = self.product_name_for(idx);
        create_switch_service(&self.cfg.dbus_bus, &suffix, device_instance, product_name).await
    }

    fn device_instance_for(&self, idx: u8) -> u32 {
        for (pos, s) in self.switch_refs.iter().enumerate() {
            let si = s.mqtt_index.unwrap_or((pos + 1) as u8);
            if si == idx { return s.device_instance.unwrap_or(si as u32); }
        }
        idx as u32
    }

    fn product_name_for(&self, idx: u8) -> String {
        for (pos, s) in self.switch_refs.iter().enumerate() {
            let si = s.mqtt_index.unwrap_or((pos + 1) as u8);
            if si == idx { if let Some(n) = &s.name { return n.clone(); } }
        }
        format!("Switch {}", idx)
    }

    async fn republish_and_watchdog(&self, watchdog_dur: Duration) {
        let now = Instant::now();
        for (idx, svc) in &self.services {
            let last = { svc.values.lock().unwrap().last_update };
            if now.duration_since(last) > watchdog_dur {
                if let Err(e) = svc.set_disconnected().await {
                    warn!(index = idx, "Erreur watchdog switch : {:#}", e);
                }
            } else if let Err(e) = svc.republish().await {
                warn!(index = idx, "Erreur republication switch : {:#}", e);
            }
        }
    }
}
