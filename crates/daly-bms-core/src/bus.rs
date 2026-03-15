//! Gestion du port série RS485 partagé entre plusieurs BMS.
//!
//! Le [`DalyPort`] encapsule le port série avec un Mutex pour garantir
//! qu'une seule commande est en cours à tout moment (bus RS485 half-duplex).
//!
//! Le [`DalyBusManager`] coordonne plusieurs [`DalyBms`] sur le même bus.

use crate::error::{DalyError, Result};
use crate::protocol::{DataId, RequestFrame, ResponseFrame, FRAME_LEN};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, trace, warn};

/// Timeout par défaut pour une réponse BMS.
pub const DEFAULT_TIMEOUT_MS: u64 = 500;

/// Délai minimum entre deux commandes sur le bus (50 ms selon doc Daly).
pub const INTER_FRAME_DELAY_MS: u64 = 50;

// =============================================================================
// DalyPort — port série avec accès exclusif
// =============================================================================

/// Port série RS485 asynchrone, sécurisé par un Mutex.
///
/// Utilise `tokio-serial` (wrapper autour de `tokio::io`).
pub struct DalyPort {
    inner: Mutex<tokio_serial::SerialStream>,
    timeout_ms: u64,
}

impl DalyPort {
    /// Ouvre le port série avec les paramètres spécifiés.
    pub fn open(port_path: &str, baud: u32, timeout_ms: u64) -> Result<Arc<Self>> {
        use tokio_serial::SerialPortBuilderExt;

        let stream = tokio_serial::new(port_path, baud)
            .data_bits(tokio_serial::DataBits::Eight)
            .stop_bits(tokio_serial::StopBits::One)
            .parity(tokio_serial::Parity::None)
            .flow_control(tokio_serial::FlowControl::None)
            .open_native_async()?;

        Ok(Arc::new(Self {
            inner: Mutex::new(stream),
            timeout_ms,
        }))
    }

    /// Envoie une commande et attend la réponse correspondante.
    ///
    /// Applique un délai inter-trame après l'envoi.
    /// Valide le checksum, l'adresse et le Data ID de la réponse.
    pub async fn send_command(
        &self,
        bms_address: u8,
        cmd: DataId,
        data: [u8; 8],
    ) -> Result<ResponseFrame> {
        let request = RequestFrame::new(bms_address, cmd, data);
        trace!(
            bms = format!("{:#04x}", bms_address),
            cmd = format!("{:#04x}", cmd as u8),
            "→ envoi trame"
        );

        let mut port = self.inner.lock().await;

        // Vider le buffer de réception avant l'envoi
        let _ = Self::flush_input(&mut *port).await;

        // Envoi
        let req_bytes = request.as_bytes();
        trace!(
            bms = format!("{:#04x}", bms_address),
            cmd = format!("{:#04x}", cmd as u8),
            raw = format!("{:02X?}", req_bytes),
            "→ envoi trame"
        );
        port.write_all(req_bytes).await?;
        port.flush().await?;

        // Délai inter-trame (laisser le temps à l'adaptateur RS485 de basculer en RX)
        tokio::time::sleep(Duration::from_millis(INTER_FRAME_DELAY_MS)).await;

        // Réception avec timeout
        let mut buf = [0u8; FRAME_LEN];
        let read_result = timeout(
            Duration::from_millis(self.timeout_ms),
            port.read_exact(&mut buf),
        )
        .await;

        match read_result {
            Err(_elapsed) => {
                // Tenter de lire des octets partiels pour le diagnostic
                let mut partial = [0u8; 32];
                let n = timeout(
                    Duration::from_millis(50),
                    port.read(&mut partial),
                )
                .await
                .ok()
                .and_then(|r| r.ok())
                .unwrap_or(0);

                if n > 0 {
                    warn!(
                        bms = format!("{:#04x}", bms_address),
                        cmd = format!("{:#04x}", cmd as u8),
                        partial = format!("{:02X?}", &partial[..n]),
                        "Timeout — réponse partielle reçue (câblage A/B ?, baud rate ?)"
                    );
                } else {
                    warn!(
                        bms = format!("{:#04x}", bms_address),
                        cmd = format!("{:#04x}", cmd as u8),
                        "Timeout — aucun octet reçu (BMS hors tension ? câble débranché ? mauvais port COM ?)"
                    );
                }
                Err(DalyError::Timeout { bms_id: bms_address, cmd: cmd as u8 })
            }
            Ok(Err(e)) => Err(e.into()),
            Ok(Ok(_)) => {
                trace!(
                    bms = format!("{:#04x}", bms_address),
                    cmd = format!("{:#04x}", cmd as u8),
                    raw = format!("{:02X?}", &buf),
                    "← réponse reçue"
                );
                let frame = ResponseFrame::parse(&buf)?;
                frame.validate_for(bms_address, cmd)?;
                debug!(
                    bms = format!("{:#04x}", bms_address),
                    cmd = format!("{:#04x}", cmd as u8),
                    "← réponse OK"
                );
                Ok(frame)
            }
        }
    }

    /// Vide le buffer de réception (lecture non-bloquante avec timeout court).
    async fn flush_input(port: &mut tokio_serial::SerialStream) -> std::io::Result<()> {
        let mut tmp = [0u8; 256];
        let _ = timeout(Duration::from_millis(10), port.read(&mut tmp)).await;
        Ok(())
    }
}

// =============================================================================
// DalyBusManager — orchestrateur multi-BMS
// =============================================================================

/// Configuration d'un BMS sur le bus.
#[derive(Debug, Clone)]
pub struct BmsConfig {
    pub address: u8,
    pub name: String,
    pub cell_count: u8,
    pub temp_sensor_count: u8,
    pub installed_capacity_ah: f32,
}

impl BmsConfig {
    pub fn new(address: u8) -> Self {
        Self {
            address,
            name: format!("BMS-{:#04x}", address),
            cell_count: 16,
            temp_sensor_count: 4,
            installed_capacity_ah: 100.0,
        }
    }
}

/// Gestionnaire du bus RS485 partagé entre plusieurs BMS.
///
/// Coordonne le polling séquentiel de chaque BMS et distribue les snapshots
/// via un canal broadcast.
pub struct DalyBusManager {
    pub port: Arc<DalyPort>,
    pub devices: Vec<BmsConfig>,
}

impl DalyBusManager {
    /// Crée un nouveau gestionnaire avec le port et la liste de BMS configurés.
    pub fn new(port: Arc<DalyPort>, devices: Vec<BmsConfig>) -> Self {
        Self { port, devices }
    }

    /// Découverte automatique des BMS sur une plage d'adresses.
    ///
    /// Interroge chaque adresse avec la commande 0x90 (PackStatus).
    /// Retourne les adresses qui ont répondu.
    pub async fn discover(&self, start: u8, end: u8) -> Vec<u8> {
        let mut found = Vec::new();
        for addr in start..=end {
            match self
                .port
                .send_command(addr, DataId::PackStatus, [0u8; 8])
                .await
            {
                Ok(_) => {
                    tracing::info!("Découverte : BMS {:#04x} trouvé", addr);
                    found.push(addr);
                }
                Err(DalyError::Timeout { .. }) | Err(DalyError::NotFound(_)) => {}
                Err(e) => {
                    tracing::debug!("Découverte {:#04x} : erreur {:?}", addr, e);
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        found
    }
}
