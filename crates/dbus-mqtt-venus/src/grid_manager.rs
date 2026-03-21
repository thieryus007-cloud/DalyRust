//! Manager des services D-Bus grid/acload — orchestre N compteurs réseau.
//!
//! Reçoit les `GridMqttEvent` depuis `mqtt_source` et les route vers
//! le `GridServiceHandle` correspondant.
//! Topic MQTT : `santuario/grid/{n}/venus`
//! Service D-Bus : `com.victronenergy.grid.{prefix}_{n}`
//!            ou : `com.victronenergy.acload.{prefix}_{n}`

use crate::config::{GridRef, VenusConfig};
use crate::grid_service::{GridServiceHandle, create_grid_service};
use crate::mqtt_source::GridMqttEvent;
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info, warn};

pub struct GridManager {
    cfg:       VenusConfig,
    grid_refs: Vec<GridRef>,
    services:  HashMap<u8, GridServiceHandle>,
    rx:        mpsc::Receiver<GridMqttEvent>,
}

impl GridManager {
    pub fn new(
        cfg:       VenusConfig,
        grid_refs: Vec<GridRef>,
        rx:        mpsc::Receiver<GridMqttEvent>,
    ) -> Self {
        Self { cfg, grid_refs, services: HashMap::new(), rx }
    }

    pub async fn run(mut self) -> Result<()> {
        if !self.cfg.enabled {
            info!("Service grid D-Bus désactivé (enabled = false)");
            while self.rx.recv().await.is_some() {}
            return Ok(());
        }

        let watchdog_dur  = Duration::from_secs(self.cfg.watchdog_sec);
        let republish_dur = Duration::from_secs(self.cfg.republish_sec);
        let mut tick      = interval(republish_dur);

        info!(
            dbus_bus     = %self.cfg.dbus_bus,
            grids        = self.grid_refs.len(),
            watchdog_sec = self.cfg.watchdog_sec,
            "GridManager démarré"
        );

        loop {
            tokio::select! {
                Some(evt) = self.rx.recv() => {
                    if let Err(e) = self.handle_event(evt).await {
                        error!("Erreur événement grid MQTT : {:#}", e);
                    }
                }
                _ = tick.tick() => {
                    self.republish_and_watchdog(watchdog_dur).await;
                }
            }
        }
    }

    async fn handle_event(&mut self, evt: GridMqttEvent) -> Result<()> {
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

    async fn create_service(&self, idx: u8) -> Result<GridServiceHandle> {
        let suffix          = format!("{}_{}", self.cfg.service_prefix, idx);
        let device_instance = self.device_instance_for(idx);
        let product_name    = self.product_name_for(idx);
        let service_type    = self.service_type_for(idx);
        create_grid_service(
            &self.cfg.dbus_bus,
            &suffix,
            device_instance,
            product_name,
            &service_type,
        ).await
    }

    fn device_instance_for(&self, idx: u8) -> u32 {
        for (pos, g) in self.grid_refs.iter().enumerate() {
            let gi = g.mqtt_index.unwrap_or((pos + 1) as u8);
            if gi == idx { return g.device_instance.unwrap_or(gi as u32); }
        }
        idx as u32
    }

    fn product_name_for(&self, idx: u8) -> String {
        for (pos, g) in self.grid_refs.iter().enumerate() {
            let gi = g.mqtt_index.unwrap_or((pos + 1) as u8);
            if gi == idx { if let Some(n) = &g.name { return n.clone(); } }
        }
        format!("Energy Meter {}", idx)
    }

    fn service_type_for(&self, idx: u8) -> String {
        for (pos, g) in self.grid_refs.iter().enumerate() {
            let gi = g.mqtt_index.unwrap_or((pos + 1) as u8);
            if gi == idx {
                if let Some(t) = &g.service_type { return t.clone(); }
            }
        }
        "grid".to_string()
    }

    async fn republish_and_watchdog(&self, watchdog_dur: Duration) {
        let now = Instant::now();
        for (idx, svc) in &self.services {
            let last = { svc.values.lock().unwrap().last_update };
            if now.duration_since(last) > watchdog_dur {
                if let Err(e) = svc.set_disconnected().await {
                    warn!(index = idx, "Erreur watchdog grid : {:#}", e);
                }
            } else if let Err(e) = svc.republish().await {
                warn!(index = idx, "Erreur republication grid : {:#}", e);
            }
        }
    }
}
