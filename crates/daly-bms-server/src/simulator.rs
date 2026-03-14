//! Simulateur BMS — génère des snapshots réalistes sans matériel physique.
//!
//! Activé par le flag `--simulate` du serveur.
//! Simule 1 à N BMS avec des données qui varient dans le temps :
//! - SOC qui descend (décharge) ou monte (charge) selon un courant simulé
//! - Tension pack corrélée au SOC (courbe LiFePO4 réaliste)
//! - Cellules avec légères disparités (~10–30 mV de delta)
//! - Température qui monte légèrement sous charge
//! - Alarmes déclenchables selon les seuils (pour tester AlertEngine)
//!
//! ## Usage
//! ```bash
//! RUST_LOG=info cargo run --bin daly-bms-server -- --simulate
//! RUST_LOG=info cargo run --bin daly-bms-server -- --simulate --sim-bms 0x01,0x02
//! ```

use crate::config::AppConfig;
use crate::state::AppState;
use daly_bms_core::types::{
    Alarms, BmsSnapshot, DcData, HistoryData, InfoData, IoData, SystemData,
};
use chrono::Utc;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::info;

// =============================================================================
// État interne du simulateur
// =============================================================================

/// État physique simulé d'un seul BMS.
#[derive(Debug, Clone)]
struct SimBmsState {
    address:          u8,
    soc:              f32,       // % 0–100
    installed_ah:     f32,       // capacité nominale (Ah)
    current:          f32,       // A (négatif = décharge, positif = charge)
    temp_base:        f32,       // température de base (°C)
    cell_count:       u8,
    cell_offsets_mv:  Vec<f32>,  // offset par cellule (mV) — constant, simule déséquilibre
    charge_cycles:    u32,
    charge_mos:       bool,
    discharge_mos:    bool,
    tick:             u64,       // compteur de cycles
}

impl SimBmsState {
    fn new(address: u8, installed_ah: f32, cell_count: u8) -> Self {
        // Déséquilibre fixe par cellule : -15 à +15 mV (réaliste LiFePO4)
        let mut offsets = Vec::with_capacity(cell_count as usize);
        for i in 0..cell_count {
            // Pseudo-random déterministe basé sur l'adresse et l'index
            let seed = (address as f32 * 7.0 + i as f32 * 3.7) % 13.0;
            offsets.push((seed - 6.5) * 2.3); // -15 to +15 mV
        }

        Self {
            address,
            soc: 72.0 + (address as f32 * 5.0 % 20.0), // SOC initial différent par BMS
            installed_ah,
            current: -8.5 - (address as f32 * 1.2), // décharge légère
            temp_base: 22.0 + (address as f32 * 0.5),
            cell_count,
            cell_offsets_mv: offsets,
            charge_cycles: 140 + address as u32 * 12,
            charge_mos: true,
            discharge_mos: true,
            tick: 0,
        }
    }

    /// Avance la simulation d'un tick (1 seconde par défaut).
    fn tick(&mut self, dt_sec: f32) {
        self.tick += 1;

        // Courant varie lentement (± 2A sur 120s, sinus)
        let t = self.tick as f32;
        self.current = -8.5 - (self.address as f32 * 1.2)
            + 2.0 * (t / 60.0).sin()
            - 1.5 * (t / 30.0).cos();

        // SOC intègre le courant (Ah)
        let delta_ah = self.current * dt_sec / 3600.0;
        let delta_soc = delta_ah / self.installed_ah * 100.0;
        self.soc = (self.soc + delta_soc).clamp(5.0, 99.5);

        // Quand SOC atteint < 10%, simuler charge automatique
        if self.soc < 10.0 {
            self.current = 25.0; // charge
        }
        // Quand SOC atteint > 95%, retour en décharge
        if self.soc > 95.0 {
            self.current = -8.5;
            self.charge_cycles += 1;
        }
    }

    /// Tension pack LiFePO4 simulée à partir du SOC (courbe réaliste 16s).
    fn pack_voltage(&self) -> f32 {
        // Courbe LiFePO4 16 cellules : 44V (vide) → 58.4V (plein)
        // Approximation linéaire par morceaux
        let soc = self.soc.clamp(0.0, 100.0);
        let cell_v = if soc < 10.0 {
            2.80 + soc * 0.05              // 2.80 → 3.30V
        } else if soc < 30.0 {
            3.30 + (soc - 10.0) * 0.005   // 3.30 → 3.40V
        } else if soc < 80.0 {
            3.40 + (soc - 30.0) * 0.002   // 3.40 → 3.50V (plateau LiFePO4)
        } else {
            3.50 + (soc - 80.0) * 0.007   // 3.50 → 3.64V
        };
        // Correction charge/décharge (chute interne ~0.3V)
        let ir_drop = self.current * 0.015; // 15 mΩ par cellule
        (cell_v * self.cell_count as f32 - ir_drop * self.cell_count as f32)
            .clamp(44.0, 58.4)
    }

    /// Tension nominale par cellule (V) pour cette simulation.
    fn base_cell_voltage(&self) -> f32 {
        let pack_v = self.pack_voltage();
        pack_v / self.cell_count as f32
    }

    /// Températures simulées (légèrement corrélées au courant absolu).
    fn temperature(&self) -> f32 {
        let heat = self.current.abs() * 0.08; // ~0.8°C par 10A
        let ambient_variation = ((self.tick as f32 / 900.0).sin() * 1.5); // ±1.5°C sur 15 min
        (self.temp_base + heat + ambient_variation).clamp(15.0, 50.0)
    }

    /// Produit un BmsSnapshot complet.
    fn to_snapshot(&self) -> BmsSnapshot {
        let pack_v = self.pack_voltage();
        let base_cell_v = self.base_cell_voltage();
        let temp = self.temperature();

        // Tensions individuelles
        let mut voltages = BTreeMap::new();
        let mut min_v = f32::INFINITY;
        let mut max_v = f32::NEG_INFINITY;
        let mut min_cell = 1u8;
        let mut max_cell = 1u8;

        for i in 0..(self.cell_count as usize) {
            let v = base_cell_v + self.cell_offsets_mv[i] / 1000.0;
            let name = format!("Cell{}", i + 1);
            if v < min_v { min_v = v; min_cell = i as u8 + 1; }
            if v > max_v { max_v = v; max_cell = i as u8 + 1; }
            voltages.insert(name, (v * 1000.0).round() / 1000.0);
        }

        // Balances (toutes à 0 sauf si delta > 20 mV)
        let delta_mv = (max_v - min_v) * 1000.0;
        let mut balances = BTreeMap::new();
        for i in 0..(self.cell_count as usize) {
            let v = base_cell_v + self.cell_offsets_mv[i] / 1000.0;
            // Cellule en équilibrage si elle est la plus haute ET delta > 10mV
            let balancing = delta_mv > 10.0 && (v - min_v) * 1000.0 > 8.0;
            balances.insert(format!("Cell{}", i + 1), balancing as u8);
        }

        let capacity_ah = self.installed_ah * self.soc / 100.0;
        let consumed_ah = self.installed_ah - capacity_ah;
        let power = pack_v * self.current;

        // Temps restant (si en décharge)
        let time_to_go = if self.current < -0.5 {
            (capacity_ah / (-self.current) * 3600.0) as u32
        } else { 0 };

        // TimeToSoc simplifié
        let mut time_to_soc = BTreeMap::new();
        for soc_target in (0..=100u8).step_by(5) {
            let delta_soc = soc_target as f32 - self.soc;
            let delta_ah = self.installed_ah * delta_soc / 100.0;
            let seconds = if self.current.abs() < 0.1 { 0u32 } else {
                (delta_ah / self.current.abs() * 3600.0).abs() as u32
            };
            time_to_soc.insert(soc_target, seconds);
        }

        // Alarmes (toutes OK sauf si seuils franchis)
        let alarms = Alarms {
            low_soc:   u8::from(self.soc < 15.0),
            cell_imbalance: u8::from(delta_mv > 80.0),
            ..Default::default()
        };

        let system = SystemData {
            min_voltage_cell_id:  format!("C{}", min_cell),
            min_cell_voltage:     (min_v * 1000.0).round() / 1000.0,
            max_voltage_cell_id:  format!("C{}", max_cell),
            max_cell_voltage:     (max_v * 1000.0).round() / 1000.0,
            min_temperature_cell_id: "C1".into(),
            min_cell_temperature: temp - 1.5,
            max_temperature_cell_id: format!("C{}", self.cell_count),
            max_cell_temperature: temp,
            mos_temperature:      temp + 3.0,
            nr_of_modules_online:  1,
            nr_of_modules_offline: 0,
            nr_of_cells_per_battery: self.cell_count,
            nr_of_modules_blocking_charge:    u8::from(!self.charge_mos),
            nr_of_modules_blocking_discharge: u8::from(!self.discharge_mos),
        };

        BmsSnapshot {
            address:            self.address,
            timestamp:          Utc::now(),
            dc: DcData {
                power,
                voltage:     (pack_v * 100.0).round() / 100.0,
                current:     (self.current * 10.0).round() / 10.0,
                temperature: temp,
            },
            installed_capacity: self.installed_ah,
            consumed_amphours:  (consumed_ah * 100.0).round() / 100.0,
            capacity:           (capacity_ah * 100.0).round() / 100.0,
            soc:                (self.soc * 10.0).round() / 10.0,
            soh:                98.5,
            time_to_go,
            balancing:          balances.values().any(|&b| b != 0) as u8,
            system_switch:      1,
            alarms,
            info: InfoData {
                charge_request:         0,
                max_charge_voltage:     58.4,
                max_charge_current:     70.0,
                max_discharge_current:  120.0,
                max_charge_cell_voltage: 3.65,
                ..Default::default()
            },
            history: HistoryData {
                charge_cycles: self.charge_cycles,
                minimum_voltage: 44.5,
                maximum_voltage: 57.8,
                total_ah_drawn: (self.charge_cycles as f32 * self.installed_ah * 0.8),
            },
            system,
            voltages,
            balances,
            io: IoData {
                allow_to_charge:    self.charge_mos as u8,
                allow_to_discharge: self.discharge_mos as u8,
                allow_to_balance:   1,
                allow_to_heat:      0,
                external_relay:     0,
            },
            heating: 0,
            time_to_soc,
        }
    }
}

// =============================================================================
// Boucle de simulation
// =============================================================================

/// Démarre la boucle de simulation en remplacement du polling RS485.
///
/// Génère des snapshots réalistes et les injecte dans [`AppState`],
/// déclenchant ainsi les bridges (MQTT, InfluxDB, AlertEngine) et le WebSocket.
pub async fn run_simulator(state: AppState, cfg: AppConfig, addresses: Vec<u8>) {
    let interval_ms = cfg.serial.poll_interval_ms;
    let dt_sec = interval_ms as f32 / 1000.0;

    // Créer un état simulé par BMS
    let sim_states: Arc<Mutex<Vec<SimBmsState>>> = Arc::new(Mutex::new(
        addresses
            .iter()
            .enumerate()
            .map(|(i, &addr)| {
                let ah = 320.0 + i as f32 * 40.0; // 320, 360, 400… Ah
                SimBmsState::new(addr, ah, cfg.serial.default_cell_count)
            })
            .collect(),
    ));

    info!(
        bms_count = addresses.len(),
        interval_ms,
        "🎮 Simulateur BMS démarré"
    );
    // Afficher les adresses simulées
    for addr in &addresses {
        info!("  → BMS {:#04x} simulé", addr);
    }

    let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms));

    loop {
        ticker.tick().await;

        let snapshots: Vec<BmsSnapshot> = {
            let mut states = sim_states.lock().unwrap();
            states.iter_mut().map(|s| {
                s.tick(dt_sec);
                s.to_snapshot()
            }).collect()
        };

        for snap in snapshots {
            state.on_snapshot(snap).await;
        }
    }
}
