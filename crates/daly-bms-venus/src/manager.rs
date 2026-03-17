//! Manager des services D-Bus — orchestre N services batterie.
//!
//! Reçoit les événements MQTT depuis `mqtt_source` et les route vers le
//! `BatteryServiceHandle` correspondant. Gère la création dynamique des
//! services D-Bus et le watchdog de déconnexion.

use crate::battery_service::{BatteryServiceHandle, create_battery_service};
use crate::config::{BmsRef, VenusConfig};
use crate::mqtt_source::MqttEvent;
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info, warn};

// =============================================================================
// Manager
// =============================================================================

/// Gestionnaire des services D-Bus Venus OS pour tous les BMS.
pub struct BatteryManager {
    cfg:      VenusConfig,
    bms_refs: Vec<BmsRef>,
    services: HashMap<u8, BatteryServiceHandle>,
    rx:       mpsc::Receiver<MqttEvent>,
}

impl BatteryManager {
    pub fn new(cfg: VenusConfig, bms_refs: Vec<BmsRef>, rx: mpsc::Receiver<MqttEvent>) -> Self {
        Self {
            cfg,
            bms_refs,
            services: HashMap::new(),
            rx,
        }
    }

    /// Boucle principale : traite les événements MQTT et le watchdog.
    pub async fn run(mut self) -> Result<()> {
        if !self.cfg.enabled {
            info!("Service Venus D-Bus désactivé (enabled = false)");
            while self.rx.recv().await.is_some() {}
            return Ok(());
        }

        let watchdog_dur  = Duration::from_secs(self.cfg.watchdog_sec);
        let republish_dur = Duration::from_secs(self.cfg.republish_sec);
        let mut republish_tick = interval(republish_dur);

        info!(
            dbus_bus     = %self.cfg.dbus_bus,
            prefix       = %self.cfg.service_prefix,
            watchdog_sec = self.cfg.watchdog_sec,
            "BatteryManager démarré"
        );

        loop {
            tokio::select! {
                Some(evt) = self.rx.recv() => {
                    if let Err(e) = self.handle_mqtt_event(evt).await {
                        error!("Erreur traitement événement MQTT : {:#}", e);
                    }
                }

                _ = republish_tick.tick() => {
                    self.republish_and_watchdog(watchdog_dur).await;
                }
            }
        }
    }

    /// Traite un événement MQTT : crée le service si besoin, puis met à jour.
    async fn handle_mqtt_event(&mut self, evt: MqttEvent) -> Result<()> {
        let idx = evt.mqtt_index;

        if !self.services.contains_key(&idx) {
            let handle = self.create_service_for_index(idx).await?;
            self.services.insert(idx, handle);
        }

        let product_name = self.product_name_for_index(idx);

        if let Some(svc) = self.services.get(&idx) {
            svc.update(&evt.payload, &product_name).await?;
        }

        Ok(())
    }

    /// Crée un service D-Bus pour un mqtt_index donné.
    async fn create_service_for_index(&self, idx: u8) -> Result<BatteryServiceHandle> {
        let service_suffix  = format!("{}_{}", self.cfg.service_prefix, idx);
        let device_instance = self.device_instance_for_index(idx);
        let product_name    = self.product_name_for_index(idx);

        create_battery_service(
            &self.cfg.dbus_bus,
            &service_suffix,
            device_instance,
            product_name,
        )
        .await
    }

    /// Retourne le `DeviceInstance` D-Bus pour un index MQTT.
    fn device_instance_for_index(&self, idx: u8) -> u32 {
        for (pos, bms) in self.bms_refs.iter().enumerate() {
            let bms_idx = bms.mqtt_index.unwrap_or((pos + 1) as u8);
            if bms_idx == idx {
                return bms_idx as u32;
            }
        }
        idx as u32
    }

    /// Retourne le nom produit pour un index MQTT.
    fn product_name_for_index(&self, idx: u8) -> String {
        for (pos, bms) in self.bms_refs.iter().enumerate() {
            let bms_idx = bms.mqtt_index.unwrap_or((pos + 1) as u8);
            if bms_idx == idx {
                if let Some(name) = &bms.name {
                    return name.clone();
                }
            }
        }
        format!("Daly BMS {}", idx)
    }

    /// Republication forcée (keepalive Venus OS) et vérification watchdog.
    async fn republish_and_watchdog(&self, watchdog_dur: Duration) {
        let now = Instant::now();

        for (idx, svc) in &self.services {
            let last_update = {
                let guard = svc.values.lock().unwrap();
                guard.last_update
            };

            if now.duration_since(last_update) > watchdog_dur {
                if let Err(e) = svc.set_disconnected().await {
                    warn!(index = idx, "Erreur watchdog disconnect : {:#}", e);
                }
            } else if let Err(e) = svc.republish().await {
                warn!(index = idx, "Erreur republication keepalive : {:#}", e);
            }
        }
    }
}
