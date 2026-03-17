//! État partagé de l'application (AppState).
//!
//! [`AppState`] est clonable et partagé via `Arc` entre toutes les tâches Tokio
//! et les handlers Axum.

use crate::config::AppConfig;
use daly_bms_core::bus::DalyPort;
use daly_bms_core::types::BmsSnapshot;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

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
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let (ws_tx, _) = broadcast::channel(WS_BROADCAST_CAPACITY);
        let addresses = config.bms_addresses();
        let ring_size = config.serial.ring_buffer_size;

        let mut buffers = BTreeMap::new();
        for addr in &addresses {
            buffers.insert(*addr, BmsRingBuffer::new(ring_size));
        }

        Self {
            config: Arc::new(config),
            buffers: Arc::new(RwLock::new(buffers)),
            ws_tx,
            polling_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            port: Arc::new(RwLock::new(None)),
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
}
