//! Manager des services D-Bus pvinverter — orchestre N onduleurs PV / compteurs ET112.
//!
//! Reçoit les `PvinverterMqttEvent` depuis `mqtt_source` et les route vers
//! le `PvinverterServiceHandle` correspondant.
//! Topic MQTT : `santuario/pvinverter/{n}/venus`
//! Service D-Bus : `com.victronenergy.pvinverter.{prefix}_{n}`

use crate::config::{PvinverterRef, VenusConfig};
use crate::mqtt_source::PvinverterMqttEvent;
use crate::pvinverter_service::{PvinverterServiceHandle, create_pvinverter_service};
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info, warn};

pub struct PvinverterManager {
    cfg:              VenusConfig,
    pvinverter_refs:  Vec<PvinverterRef>,
    services:         HashMap<u8, PvinverterServiceHandle>,
    rx:               mpsc::Receiver<PvinverterMqttEvent>,
}

impl PvinverterManager {
    pub fn new(
        cfg:             VenusConfig,
        pvinverter_refs: Vec<PvinverterRef>,
        rx:              mpsc::Receiver<PvinverterMqttEvent>,
    ) -> Self {
        Self { cfg, pvinverter_refs, services: HashMap::new(), rx }
    }

    pub async fn run(mut self) -> Result<()> {
        if !self.cfg.enabled {
            info!("Service pvinverter D-Bus désactivé (enabled = false)");
            while self.rx.recv().await.is_some() {}
            return Ok(());
        }

        let watchdog_dur  = Duration::from_secs(self.cfg.watchdog_sec);
        let republish_dur = Duration::from_secs(self.cfg.republish_sec);
        let mut tick      = interval(republish_dur);

        info!(
            dbus_bus      = %self.cfg.dbus_bus,
            pvinverters   = self.pvinverter_refs.len(),
            watchdog_sec  = self.cfg.watchdog_sec,
            "PvinverterManager démarré"
        );

        loop {
            tokio::select! {
                Some(evt) = self.rx.recv() => {
                    if let Err(e) = self.handle_event(evt).await {
                        error!("Erreur événement pvinverter MQTT : {:#}", e);
                    }
                }
                _ = tick.tick() => {
                    self.republish_and_watchdog(watchdog_dur).await;
                }
            }
        }
    }

    async fn handle_event(&mut self, evt: PvinverterMqttEvent) -> Result<()> {
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

    async fn create_service(&self, idx: u8) -> Result<PvinverterServiceHandle> {
        let suffix          = format!("{}_{}", self.cfg.service_prefix, idx);
        let device_instance = self.device_instance_for(idx);
        let product_name    = self.product_name_for(idx);
        let custom_name     = self.custom_name_for(idx);
        create_pvinverter_service(
            &self.cfg.dbus_bus,
            &suffix,
            device_instance,
            product_name,
            custom_name,
        ).await
    }

    fn device_instance_for(&self, idx: u8) -> u32 {
        for (pos, p) in self.pvinverter_refs.iter().enumerate() {
            let pi = p.mqtt_index.unwrap_or((pos + 1) as u8);
            if pi == idx { return p.device_instance.unwrap_or(pi as u32); }
        }
        idx as u32
    }

    fn product_name_for(&self, idx: u8) -> String {
        for (pos, p) in self.pvinverter_refs.iter().enumerate() {
            let pi = p.mqtt_index.unwrap_or((pos + 1) as u8);
            if pi == idx { if let Some(n) = &p.name { return n.clone(); } }
        }
        format!("PV Inverter {}", idx)
    }

    fn custom_name_for(&self, idx: u8) -> String {
        for (pos, p) in self.pvinverter_refs.iter().enumerate() {
            let pi = p.mqtt_index.unwrap_or((pos + 1) as u8);
            if pi == idx {
                if let Some(cn) = &p.custom_name { return cn.clone(); }
                if let Some(n)  = &p.name        { return n.clone(); }
            }
        }
        format!("PV Inverter {}", idx)
    }

    async fn republish_and_watchdog(&self, watchdog_dur: Duration) {
        let now = Instant::now();
        for (idx, svc) in &self.services {
            let last = { svc.values.lock().unwrap().last_update };
            if now.duration_since(last) > watchdog_dur {
                if let Err(e) = svc.set_disconnected().await {
                    warn!(index = idx, "Erreur watchdog pvinverter : {:#}", e);
                }
            } else if let Err(e) = svc.republish().await {
                warn!(index = idx, "Erreur republication pvinverter : {:#}", e);
            }
        }
    }
}
