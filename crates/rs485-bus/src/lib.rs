//! # rs485-bus
//!
//! Bus RS485 partagé entre plusieurs drivers (Daly BMS, Modbus RTU).
//!
//! ## Principe
//!
//! Un seul `Arc<SharedBus>` est ouvert pour tout le projet. Chaque driver
//! (Daly BMS, ET112, PRALRAN…) acquiert le verrou via `SharedBus::acquire()`
//! ou utilise la méthode de convenance `SharedBus::transact()`.
//!
//! Le Mutex interne garantit qu'une seule transaction est en cours à tout
//! moment sur le bus half-duplex RS485.
//!
//! ## Modules
//! - [`modbus_rtu`] — framing Modbus RTU pur Rust (CRC16, FC03, FC04, FC06)

pub mod modbus_rtu;

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tracing::trace;

/// Timeout du flush RX (drain des octets résiduels avant émission).
const FLUSH_TIMEOUT_MS: u64 = 10;

// =============================================================================
// SharedBus
// =============================================================================

/// Port série RS485 partagé entre tous les drivers du projet.
///
/// Accès exclusif garanti par un `Mutex<SerialStream>`.
/// Créer une seule instance et la distribuer via `Arc::clone()`.
pub struct SharedBus {
    inner: Mutex<tokio_serial::SerialStream>,
    /// Délai TX→RX après émission (commutation RS485 half-duplex).
    pub inter_frame_ms: u64,
    /// Timeout de réception par défaut pour `transact()`.
    pub timeout_ms: u64,
}

impl SharedBus {
    /// Ouvre le port série et crée le bus partagé.
    ///
    /// # Paramètres
    /// - `port_path`      : ex `/dev/ttyUSB0`
    /// - `baud`           : 9600 pour tous les appareils de ce projet
    /// - `parity`         : `tokio_serial::Parity::None` pour 8N1
    /// - `inter_frame_ms` : délai TX→RX en ms (50 ms recommandé pour Daly BMS)
    /// - `timeout_ms`     : timeout de réception par défaut
    pub fn open(
        port_path: &str,
        baud: u32,
        parity: tokio_serial::Parity,
        inter_frame_ms: u64,
        timeout_ms: u64,
    ) -> anyhow::Result<Arc<Self>> {
        use tokio_serial::SerialPortBuilderExt;

        let stream = tokio_serial::new(port_path, baud)
            .data_bits(tokio_serial::DataBits::Eight)
            .stop_bits(tokio_serial::StopBits::One)
            .parity(parity)
            .flow_control(tokio_serial::FlowControl::None)
            .open_native_async()
            .map_err(|e| anyhow::anyhow!("Impossible d'ouvrir {} : {}", port_path, e))?;

        Ok(Arc::new(Self {
            inner: Mutex::new(stream),
            inter_frame_ms,
            timeout_ms,
        }))
    }

    /// Acquiert le verrou exclusif et retourne un [`BusGuard`].
    ///
    /// Le garde libère le verrou automatiquement à la fin du scope.
    /// Permet des transactions multi-étapes (ex : Daly 2ème trame de secours).
    pub async fn acquire(&self) -> BusGuard<'_> {
        BusGuard {
            port: self.inner.lock().await,
            inter_frame_ms: self.inter_frame_ms,
            timeout_ms: self.timeout_ms,
        }
    }

    /// Transaction simple : flush + TX + délai inter-trame + RX exact.
    ///
    /// Convient pour tous les appareils Modbus RTU simple (1 requête / 1 réponse).
    /// Pour des transactions complexes (Daly multi-trame), utiliser [`acquire()`].
    pub async fn transact(&self, tx: &[u8], resp_len: usize) -> anyhow::Result<Vec<u8>> {
        let mut guard = self.acquire().await;
        guard.flush_rx().await;
        guard.write_all(tx).await
            .map_err(|e| anyhow::anyhow!("TX erreur : {}", e))?;
        guard.inter_frame_delay().await;
        guard
            .read_exact_timed(resp_len)
            .await
            .ok_or_else(|| anyhow::anyhow!("Timeout ({} ms) — aucune réponse", self.timeout_ms))?
            .map_err(|e| anyhow::anyhow!("RX erreur : {}", e))
    }
}

// =============================================================================
// BusGuard
// =============================================================================

/// Garde sur le bus RS485 — verrou Mutex maintenu pour la durée de la transaction.
///
/// Obtenu via [`SharedBus::acquire()`].
/// Libère le verrou automatiquement (Drop).
pub struct BusGuard<'a> {
    port: tokio::sync::MutexGuard<'a, tokio_serial::SerialStream>,
    pub inter_frame_ms: u64,
    pub timeout_ms: u64,
}

impl<'a> BusGuard<'a> {
    /// Vide le buffer RX (drain les octets résiduels d'une transaction précédente).
    pub async fn flush_rx(&mut self) {
        let mut tmp = [0u8; 256];
        let _ = tokio::time::timeout(
            Duration::from_millis(FLUSH_TIMEOUT_MS),
            self.port.read(&mut tmp),
        )
        .await;
    }

    /// Écrit tous les octets dans le port + flush.
    pub async fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        trace!(raw = format!("{:02X?}", data), "→ TX");
        self.port.write_all(data).await?;
        self.port.flush().await
    }

    /// Délai de commutation TX→RX (inter-trame).
    pub async fn inter_frame_delay(&self) {
        tokio::time::sleep(Duration::from_millis(self.inter_frame_ms)).await;
    }

    /// Lit exactement `n` octets avec le timeout par défaut du bus.
    ///
    /// Retourne `None` sur timeout, `Some(Err)` sur erreur I/O, `Some(Ok(buf))` sur succès.
    pub async fn read_exact_timed(&mut self, n: usize) -> Option<std::io::Result<Vec<u8>>> {
        self.read_exact_with_timeout(n, self.timeout_ms).await
    }

    /// Lit exactement `n` octets avec un timeout explicite (ms).
    ///
    /// Retourne `None` sur timeout, `Some(Err)` sur erreur I/O, `Some(Ok(buf))` sur succès.
    pub async fn read_exact_with_timeout(
        &mut self,
        n: usize,
        timeout_ms: u64,
    ) -> Option<std::io::Result<Vec<u8>>> {
        let mut buf = vec![0u8; n];
        match tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            self.port.read_exact(&mut buf),
        )
        .await
        {
            Ok(Ok(_)) => {
                trace!(raw = format!("{:02X?}", buf), "← RX");
                Some(Ok(buf))
            }
            Ok(Err(e)) => Some(Err(e)),
            Err(_elapsed) => None,
        }
    }

    /// Tente de lire jusqu'à `max` octets disponibles (pour diagnostic timeout).
    ///
    /// Utilise un timeout court (50 ms). Retourne les octets lus (peut être vide).
    pub async fn try_read_partial(&mut self, max: usize) -> Vec<u8> {
        let mut tmp = vec![0u8; max];
        match tokio::time::timeout(
            Duration::from_millis(50),
            self.port.read(&mut tmp),
        )
        .await
        {
            Ok(Ok(n)) if n > 0 => tmp[..n].to_vec(),
            _ => vec![],
        }
    }
}
