//! État partagé de l'application (AppState).
//!
//! [`AppState`] est clonable et partagé via `Arc` entre toutes les tâches Tokio
//! et les handlers Axum.

use crate::config::AppConfig;
use crate::et112::Et112Snapshot;
use crate::irradiance::IrradianceSnapshot;
use crate::tasmota::TasmotaSnapshot;
use daly_bms_core::bus::DalyPort;
use daly_bms_core::types::BmsSnapshot;
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, RwLock};

// =============================================================================
// Buffer de logs en mémoire (pour l'interface web)
// =============================================================================

/// Une entrée de log capturée depuis tracing.
#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// Ring buffer de logs partagé (200 entrées max).
pub type LogBuffer = Arc<Mutex<VecDeque<LogEntry>>>;

/// Capacité du canal broadcast WebSocket.
const WS_BROADCAST_CAPACITY: usize = 128;

// =============================================================================
// Ring buffer par BMS
// =============================================================================

/// Ring buffer de snapshots en mémoire pour un BMS.
pub struct BmsRingBuffer {
    pub buffer: VecDeque<BmsSnapshot>,
    pub capacity: usize,
}

impl BmsRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, snap: BmsSnapshot) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(snap);
    }

    pub fn latest(&self) -> Option<&BmsSnapshot> {
        self.buffer.back()
    }
}

// =============================================================================
// AppState
// =============================================================================

// =============================================================================
// Ring buffer ET112
// =============================================================================

/// Ring buffer de snapshots ET112 pour un compteur.
pub struct Et112RingBuffer {
    pub buffer: VecDeque<Et112Snapshot>,
    pub capacity: usize,
}

impl Et112RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, snap: Et112Snapshot) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(snap);
    }

    pub fn latest(&self) -> Option<&Et112Snapshot> {
        self.buffer.back()
    }
}

// =============================================================================
// Ring buffer Tasmota
// =============================================================================

/// Ring buffer de snapshots Tasmota pour une prise.
pub struct TasmotaRingBuffer {
    pub buffer:   VecDeque<TasmotaSnapshot>,
    pub capacity: usize,
}

impl TasmotaRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer:   VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, snap: TasmotaSnapshot) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(snap);
    }

    pub fn latest(&self) -> Option<&TasmotaSnapshot> {
        self.buffer.back()
    }
}

/// État global partagé de l'application.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,

    /// Ring buffers indexés par adresse BMS.
    pub buffers: Arc<RwLock<BTreeMap<u8, BmsRingBuffer>>>,

    /// Canal broadcast pour le WebSocket (tous BMS confondus).
    pub ws_tx: broadcast::Sender<Arc<Vec<BmsSnapshot>>>,

    /// Indicateur polling actif.
    pub polling_active: Arc<std::sync::atomic::AtomicBool>,

    /// Port série partagé — None en mode simulateur.
    /// Partagé avec le poll_loop via le Mutex interne de DalyPort.
    pub port: Arc<RwLock<Option<Arc<DalyPort>>>>,

    /// Buffer de logs pour l'interface web.
    pub log_buffer: LogBuffer,

    /// Ring buffers ET112 indexés par adresse Modbus.
    pub et112_buffers: Arc<RwLock<BTreeMap<u8, Et112RingBuffer>>>,

    /// Dernière mesure du capteur d'irradiance PRALRAN (None si non configuré).
    pub irradiance_value: Arc<RwLock<Option<IrradianceSnapshot>>>,

    /// Ring buffers Tasmota indexés par id de device.
    pub tasmota_buffers: Arc<RwLock<BTreeMap<u8, TasmotaRingBuffer>>>,

    /// Production solaire totale aujourd'hui en kWh (MPPT + delta ET112 micro-onduleurs).
    /// Publiée par Node-RED via POST /api/v1/solar/mppt-yield.
    pub mppt_yield_kwh: Arc<RwLock<f32>>,

    /// Puissance MPPT instantanée totale en W (somme de tous les chargeurs solaires).
    /// Publiée par Node-RED via POST /api/v1/solar/mppt-yield.
    pub mppt_power_w: Arc<RwLock<f32>>,

    /// Puissance solaire totale en W = MPPT 273+289 + PV Inverter ET112 (VRM).
    /// Source unique : Solar_power.json Node-RED (via POST solar_total_w).
    pub solar_total_w: Arc<RwLock<f32>>,

    /// Puissance consommée par la maison en W (ESS AC output consumption).
    /// Source : N/c0619ab9929a/system/0/Ac/ConsumptionOnOutput/L1/Power via VRM → Node-RED.
    pub house_power_w: Arc<RwLock<f32>>,
}

impl AppState {
    pub fn new(config: AppConfig, log_buffer: LogBuffer) -> Self {
        let (ws_tx, _) = broadcast::channel(WS_BROADCAST_CAPACITY);
        let addresses = config.bms_addresses();
        let ring_size = config.serial.ring_buffer_size;

        let mut buffers = BTreeMap::new();
        for addr in &addresses {
            buffers.insert(*addr, BmsRingBuffer::new(ring_size));
        }

        // Pré-allouer les ring buffers ET112
        let et112_ring_size = config.et112.ring_buffer_size;
        let mut et112_buffers = BTreeMap::new();
        for dev in &config.et112.devices {
            et112_buffers.insert(dev.parsed_address(), Et112RingBuffer::new(et112_ring_size));
        }

        // Pré-allouer les ring buffers Tasmota
        let tasmota_ring_size = config.tasmota.ring_buffer_size;
        let mut tasmota_buffers = BTreeMap::new();
        for dev in &config.tasmota.devices {
            tasmota_buffers.insert(dev.id, TasmotaRingBuffer::new(tasmota_ring_size));
        }

        Self {
            config: Arc::new(config),
            buffers: Arc::new(RwLock::new(buffers)),
            ws_tx,
            polling_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            port: Arc::new(RwLock::new(None)),
            log_buffer,
            et112_buffers: Arc::new(RwLock::new(et112_buffers)),
            irradiance_value: Arc::new(RwLock::new(None)),
            tasmota_buffers: Arc::new(RwLock::new(tasmota_buffers)),
            mppt_yield_kwh: Arc::new(RwLock::new(0.0)),
            mppt_power_w:   Arc::new(RwLock::new(0.0)),
            solar_total_w:  Arc::new(RwLock::new(0.0)),
            house_power_w:  Arc::new(RwLock::new(0.0)),
        }
    }

    /// Enregistre le port série ouvert (mode hardware uniquement).
    pub async fn set_port(&self, port: Arc<DalyPort>) {
        *self.port.write().await = Some(port);
    }

    /// Enregistre un nouveau snapshot dans le ring buffer et broadcast WebSocket.
    pub async fn on_snapshot(&self, snap: BmsSnapshot) {
        {
            let mut buffers = self.buffers.write().await;
            buffers
                .entry(snap.address)
                .or_insert_with(|| BmsRingBuffer::new(self.config.serial.ring_buffer_size))
                .push(snap.clone());
        }
        // Broadcast : construire la liste de tous les derniers snapshots
        let latest = self.latest_snapshots().await;
        let _ = self.ws_tx.send(Arc::new(latest));
    }

    /// Retourne le dernier snapshot de chaque BMS.
    pub async fn latest_snapshots(&self) -> Vec<BmsSnapshot> {
        let buffers = self.buffers.read().await;
        buffers.values()
            .filter_map(|b| b.latest().cloned())
            .collect()
    }

    /// Retourne le dernier snapshot d'un BMS spécifique.
    pub async fn latest_for(&self, addr: u8) -> Option<BmsSnapshot> {
        let buffers = self.buffers.read().await;
        buffers.get(&addr)?.latest().cloned()
    }

    /// Retourne les `n` derniers snapshots d'un BMS (pour historique).
    pub async fn history_for(&self, addr: u8, limit: usize) -> Vec<BmsSnapshot> {
        let buffers = self.buffers.read().await;
        if let Some(buf) = buffers.get(&addr) {
            buf.buffer.iter().rev().take(limit).cloned().collect()
        } else {
            vec![]
        }
    }

    /// S'abonne au canal broadcast WebSocket.
    pub fn subscribe_ws(&self) -> broadcast::Receiver<Arc<Vec<BmsSnapshot>>> {
        self.ws_tx.subscribe()
    }

    /// Enregistre un snapshot ET112 dans le ring buffer correspondant.
    pub async fn on_et112_snapshot(&self, snap: Et112Snapshot) {
        let mut buffers = self.et112_buffers.write().await;
        buffers
            .entry(snap.address)
            .or_insert_with(|| Et112RingBuffer::new(self.config.et112.ring_buffer_size))
            .push(snap);
    }

    /// Retourne le dernier snapshot ET112 pour une adresse donnée.
    pub async fn et112_latest_for(&self, addr: u8) -> Option<Et112Snapshot> {
        let buffers = self.et112_buffers.read().await;
        buffers.get(&addr)?.latest().cloned()
    }

    /// Retourne les `n` derniers snapshots ET112 (pour historique).
    pub async fn et112_history_for(&self, addr: u8, limit: usize) -> Vec<Et112Snapshot> {
        let buffers = self.et112_buffers.read().await;
        if let Some(buf) = buffers.get(&addr) {
            buf.buffer.iter().rev().take(limit).cloned().collect()
        } else {
            vec![]
        }
    }

    /// Retourne tous les derniers snapshots ET112.
    pub async fn et112_latest_all(&self) -> Vec<Et112Snapshot> {
        let buffers = self.et112_buffers.read().await;
        buffers.values().filter_map(|b| b.latest().cloned()).collect()
    }

    /// Enregistre la dernière mesure du capteur d'irradiance.
    pub async fn on_irradiance_snapshot(&self, snap: IrradianceSnapshot) {
        *self.irradiance_value.write().await = Some(snap);
    }

    /// Retourne la dernière mesure d'irradiance (None si jamais reçue).
    pub async fn latest_irradiance(&self) -> Option<IrradianceSnapshot> {
        self.irradiance_value.read().await.clone()
    }

    /// Enregistre un snapshot Tasmota dans le ring buffer correspondant.
    pub async fn on_tasmota_snapshot(&self, snap: TasmotaSnapshot) {
        let mut buffers = self.tasmota_buffers.write().await;
        buffers
            .entry(snap.id)
            .or_insert_with(|| TasmotaRingBuffer::new(self.config.tasmota.ring_buffer_size))
            .push(snap);
    }

    /// Retourne le dernier snapshot Tasmota pour un id donné.
    pub async fn tasmota_latest_for(&self, id: u8) -> Option<TasmotaSnapshot> {
        let buffers = self.tasmota_buffers.read().await;
        buffers.get(&id)?.latest().cloned()
    }

    /// Retourne les `n` derniers snapshots Tasmota (pour historique).
    pub async fn tasmota_history_for(&self, id: u8, limit: usize) -> Vec<TasmotaSnapshot> {
        let buffers = self.tasmota_buffers.read().await;
        if let Some(buf) = buffers.get(&id) {
            buf.buffer.iter().rev().take(limit).cloned().collect()
        } else {
            vec![]
        }
    }

    /// Retourne tous les derniers snapshots Tasmota.
    pub async fn tasmota_latest_all(&self) -> Vec<TasmotaSnapshot> {
        let buffers = self.tasmota_buffers.read().await;
        buffers.values().filter_map(|b| b.latest().cloned()).collect()
    }
}
