//! Gestion du port série RS485 partagé entre plusieurs BMS.
//!
//! Le [`DalyPort`] encapsule un [`rs485_bus::SharedBus`] et implémente le
//! protocole Daly par-dessus (framing, checksum, gestion adresse).
//!
//! Le [`DalyBusManager`] coordonne plusieurs [`DalyBms`] sur le même bus.
//!
//! ## Bus unifié
//!
//! Pour partager le même port série avec d'autres drivers (ET112, PRALRAN…) :
//! ```rust,ignore
//! let port = DalyPort::open("/dev/ttyUSB0", 9600, 500)?;
//! let bus  = port.shared_bus();  // Arc<SharedBus> partageable
//! // passer bus à run_et112_poll_loop(), run_irradiance_poll_loop()…
//! ```

use crate::error::{DalyError, Result};
use crate::protocol::{DataId, RequestFrame, ResponseFrame, FRAME_LEN};
use rs485_bus::SharedBus;
use std::sync::Arc;
use tracing::{debug, trace, warn};

/// Timeout par défaut pour une réponse BMS.
pub const DEFAULT_TIMEOUT_MS: u64 = 500;

/// Délai minimum entre deux commandes sur le bus (50 ms selon doc Daly V1.21).
pub const INTER_FRAME_DELAY_MS: u64 = 50;

// =============================================================================
// DalyPort — protocole Daly par-dessus SharedBus
// =============================================================================

/// Port RS485 avec implémentation du protocole Daly.
///
/// Wraps un [`SharedBus`] et ajoute le framing Daly, la validation
/// des réponses (checksum, adresse, DataId) et la logique de fallback
/// (2ème trame si BMS maître répond en premier).
pub struct DalyPort {
    bus: Arc<SharedBus>,
    timeout_ms: u64,
}

impl DalyPort {
    /// Ouvre le port série et crée un `SharedBus` sous-jacent (8N1).
    ///
    /// Compatible avec le comportement précédent : `DalyPort::open(path, baud, timeout_ms)`.
    pub fn open(port_path: &str, baud: u32, timeout_ms: u64) -> Result<Arc<Self>> {
        let bus = SharedBus::open(
            port_path,
            baud,
            tokio_serial::Parity::None,
            INTER_FRAME_DELAY_MS,
            timeout_ms,
        )
        .map_err(DalyError::Other)?;

        Ok(Arc::new(Self { bus, timeout_ms }))
    }

    /// Crée un `DalyPort` à partir d'un `SharedBus` existant.
    ///
    /// Utilisé quand le bus est déjà ouvert et partagé avec d'autres drivers.
    pub fn from_bus(bus: Arc<SharedBus>, timeout_ms: u64) -> Arc<Self> {
        Arc::new(Self { bus, timeout_ms })
    }

    /// Retourne le `SharedBus` sous-jacent pour le partager avec d'autres drivers.
    ///
    /// ```rust,ignore
    /// let port = DalyPort::open("/dev/ttyUSB0", 9600, 500)?;
    /// let bus  = port.shared_bus();
    /// // Passer bus aux drivers ET112 / PRALRAN
    /// tokio::spawn(run_et112_poll_loop(bus, ...));
    /// ```
    pub fn shared_bus(&self) -> Arc<SharedBus> {
        self.bus.clone()
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
        let request = if cmd.is_write() {
            RequestFrame::new(bms_address, cmd, data)
        } else {
            RequestFrame::read(bms_address, cmd)
        };

        trace!(
            bms = format!("{:#04x}", bms_address),
            cmd = format!("{:#04x}", cmd as u8),
            "→ envoi trame"
        );

        let req_bytes = request.as_bytes();
        trace!(
            bms = format!("{:#04x}", bms_address),
            cmd = format!("{:#04x}", cmd as u8),
            raw = format!("{:02X?}", req_bytes),
            "→ envoi trame"
        );

        // Acquérir le verrou exclusif pour toute la transaction
        let mut guard = self.bus.acquire().await;

        guard.flush_rx().await;
        guard.write_all(req_bytes).await?; // std::io::Error → DalyError::Io via #[from]
        guard.inter_frame_delay().await;

        // Réception avec timeout
        let buf = match guard.read_exact_with_timeout(FRAME_LEN, self.timeout_ms).await {
            None => {
                // Timeout — tenter lecture partielle pour diagnostic
                let partial = guard.try_read_partial(32).await;
                if !partial.is_empty() {
                    warn!(
                        bms     = format!("{:#04x}", bms_address),
                        cmd     = format!("{:#04x}", cmd as u8),
                        partial = format!("{:02X?}", partial),
                        "Timeout — réponse partielle reçue (câblage A/B ?, baud rate ?)"
                    );
                } else {
                    warn!(
                        bms = format!("{:#04x}", bms_address),
                        cmd = format!("{:#04x}", cmd as u8),
                        "Timeout — aucun octet reçu (BMS hors tension ? câble débranché ?)"
                    );
                }
                return Err(DalyError::Timeout { bms_id: bms_address, cmd: cmd as u8 });
            }
            Some(Err(e)) => return Err(DalyError::Io(e)),
            Some(Ok(b)) => b,
        };

        trace!(
            bms = format!("{:#04x}", bms_address),
            cmd = format!("{:#04x}", cmd as u8),
            raw = format!("{:02X?}", &buf),
            "← réponse reçue"
        );

        let frame = ResponseFrame::parse(&buf)?;

        // Sur un bus RS485 partagé, BMS 0x01 (master) peut répondre en
        // premier même si la requête cible BMS 0x02. On tente alors de
        // lire une deuxième trame : BMS 0x02 peut répondre juste après.
        // Le verrou est toujours maintenu (même BusGuard).
        if frame.address() != bms_address {
            warn!(
                bms    = format!("{:#04x}", bms_address),
                actual = format!("{:#04x}", frame.address()),
                "Adresse inattendue — tentative lecture 2ème trame (bus partagé)"
            );
            if let Some(Ok(buf2)) = guard.read_exact_with_timeout(FRAME_LEN, self.timeout_ms).await {
                trace!(
                    bms = format!("{:#04x}", bms_address),
                    raw = format!("{:02X?}", &buf2),
                    "← 2ème trame reçue"
                );
                if let Ok(frame2) = ResponseFrame::parse(&buf2) {
                    if frame2.validate_for(bms_address, cmd).is_ok() {
                        debug!(
                            bms = format!("{:#04x}", bms_address),
                            "← 2ème trame OK (BMS répond après le master)"
                        );
                        return Ok(frame2);
                    }
                }
            }
            return Err(DalyError::UnexpectedAddress {
                expected: bms_address,
                actual:   frame.address(),
            });
        }

        frame.validate_for(bms_address, cmd)?;
        debug!(
            bms = format!("{:#04x}", bms_address),
            cmd = format!("{:#04x}", cmd as u8),
            "← réponse OK"
        );
        Ok(frame)
    }

    /// Envoie une commande et lit N trames de réponse successives.
    ///
    /// Utilisé pour les commandes multi-trames (0x95, 0x96) où le BMS envoie
    /// toutes les trames d'un coup après une seule requête.
    pub async fn send_command_multi(
        &self,
        bms_address: u8,
        cmd: DataId,
        n_frames: usize,
    ) -> Result<Vec<ResponseFrame>> {
        if n_frames == 0 {
            return Ok(Vec::new());
        }

        let request = RequestFrame::read(bms_address, cmd);

        let mut guard = self.bus.acquire().await;
        guard.flush_rx().await;

        let req_bytes = request.as_bytes();
        trace!(
            bms     = format!("{:#04x}", bms_address),
            cmd     = format!("{:#04x}", cmd as u8),
            n_frames,
            raw     = format!("{:02X?}", req_bytes),
            "→ envoi trame multi"
        );
        guard.write_all(req_bytes).await?;
        guard.inter_frame_delay().await;

        let mut frames = Vec::with_capacity(n_frames);
        for frame_idx in 0..n_frames {
            let buf = match guard.read_exact_with_timeout(FRAME_LEN, self.timeout_ms).await {
                None => {
                    warn!(
                        bms   = format!("{:#04x}", bms_address),
                        cmd   = format!("{:#04x}", cmd as u8),
                        frame = frame_idx,
                        "Timeout trame multi"
                    );
                    return Err(DalyError::Timeout { bms_id: bms_address, cmd: cmd as u8 });
                }
                Some(Err(e)) => return Err(DalyError::Io(e)),
                Some(Ok(b)) => b,
            };

            trace!(
                bms   = format!("{:#04x}", bms_address),
                cmd   = format!("{:#04x}", cmd as u8),
                frame = frame_idx,
                raw   = format!("{:02X?}", &buf),
                "← trame multi reçue"
            );

            let frame = ResponseFrame::parse(&buf)?;
            frame.validate_for(bms_address, cmd)?;
            frames.push(frame);
        }

        Ok(frames)
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
    /// Courant de charge maximal configuré (A) ; 0 = inconnu
    pub max_charge_current_a: f32,
    /// Courant de décharge maximal configuré (A) ; 0 = inconnu
    pub max_discharge_current_a: f32,
}

impl BmsConfig {
    pub fn new(address: u8) -> Self {
        Self {
            address,
            name: format!("BMS-{:#04x}", address),
            cell_count: 16,
            temp_sensor_count: 4,
            installed_capacity_ah: 100.0,
            max_charge_current_a: 0.0,
            max_discharge_current_a: 0.0,
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
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        found
    }
}
