//! Boucle de polling Modbus RTU pour le compteur Carlo Gavazzi ET112.
//!
//! ## Registres lus (FC=04, FLOAT32 big-endian)
//!
//! | Adresse hex | Description           | Unité |
//! |-------------|----------------------|-------|
//! | 0x0000      | Tension L1           | V     |
//! | 0x0002      | Courant L1           | A     |
//! | 0x0004      | Puissance active     | W     |
//! | 0x0006      | Puissance apparente  | VA    |
//! | 0x0008      | Puissance réactive   | VAr   |
//! | 0x000A      | Facteur de puissance | —     |
//! | 0x000C      | Angle de phase       | °     |
//! | 0x000E      | Fréquence            | Hz    |
//! | 0x0010      | Énergie import       | Wh    |
//! | 0x0012      | Énergie export       | Wh    |

use super::types::Et112Snapshot;
use crate::config::Et112DeviceConfig;
use chrono::Local;
use std::time::Duration;
use tokio_modbus::client::rtu;
use tokio_modbus::prelude::*;
use tokio_serial::SerialStream;
use tracing::{debug, error, info, warn};

/// Convertit deux registres u16 (big-endian) en f32 IEEE 754.
fn regs_to_f32(hi: u16, lo: u16) -> f32 {
    f32::from_bits(((hi as u32) << 16) | (lo as u32))
}

/// Lance la boucle de polling ET112.
///
/// # Paramètres
/// - `port_path`     : chemin du port série (ex: `/dev/ttyUSB1`)
/// - `baud`          : vitesse en bauds (9600 par défaut pour ET112)
/// - `devices`       : liste des ET112 configurés (adresse + nom + ...)
/// - `poll_interval` : intervalle entre deux cycles complets
/// - `on_snapshot`   : callback appelé pour chaque snapshot valide
pub async fn run_et112_poll_loop<F>(
    port_path: String,
    baud: u32,
    devices: Vec<Et112DeviceConfig>,
    poll_interval: Duration,
    mut on_snapshot: F,
)
where
    F: FnMut(Et112Snapshot) + Send + 'static,
{
    if devices.is_empty() {
        info!("ET112 : aucun appareil configuré, polling désactivé");
        return;
    }

    info!(
        port = %port_path,
        baud,
        count = devices.len(),
        "ET112 polling démarré"
    );

    // Backoff exponentiel en cas d'erreur série
    let mut backoff_ms: u64 = 500;

    loop {
        match poll_cycle(&port_path, baud, &devices, &mut on_snapshot).await {
            Ok(()) => {
                backoff_ms = 500; // reset on success
            }
            Err(e) => {
                error!("ET112 erreur cycle polling : {:#}", e);
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms = (backoff_ms * 2).min(30_000);
                continue;
            }
        }
        tokio::time::sleep(poll_interval).await;
    }
}

/// Un cycle de polling : lit tous les registres pour chaque ET112 configuré.
async fn poll_cycle<F>(
    port_path: &str,
    baud: u32,
    devices: &[Et112DeviceConfig],
    on_snapshot: &mut F,
) -> anyhow::Result<()>
where
    F: FnMut(Et112Snapshot),
{
    for dev in devices {
        let address = dev.parsed_address();
        match poll_device(port_path, baud, address, dev).await {
            Ok(snap) => {
                debug!(
                    addr = address,
                    name = %dev.name,
                    power_w = snap.power_w,
                    "ET112 snapshot OK"
                );
                on_snapshot(snap);
            }
            Err(e) => {
                warn!(
                    addr = address,
                    name = %dev.name,
                    "ET112 erreur lecture : {:#}", e
                );
            }
        }
    }
    Ok(())
}

/// Ouvre le port série, interroge un ET112 et retourne un snapshot.
async fn poll_device(
    port_path: &str,
    baud: u32,
    address: u8,
    dev: &Et112DeviceConfig,
) -> anyhow::Result<Et112Snapshot> {
    let builder = tokio_serial::new(port_path, baud)
        .data_bits(tokio_serial::DataBits::Eight)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .timeout(Duration::from_millis(1500));

    let port = SerialStream::open(&builder)
        .map_err(|e| anyhow::anyhow!("Impossible d'ouvrir {} : {}", port_path, e))?;

    let mut ctx = rtu::attach_slave(port, Slave(address));

    // Lecture bloc 1 : 0x0000–0x000F → 16 registres (8 × FLOAT32)
    // voltage, current, power_active, power_apparent, power_reactive, pf, phase_angle, freq
    let regs1: Vec<u16> = ctx
        .read_input_registers(0x0000, 16)
        .await
        .map_err(|e| anyhow::anyhow!("ET112 addr={:#04x} FC04 read 0x0000: {}", address, e))?
        .map_err(|e| anyhow::anyhow!("ET112 addr={:#04x} exception code: {:?}", address, e))?;

    if regs1.len() < 16 {
        anyhow::bail!("ET112 addr={:#04x} réponse trop courte bloc 1 ({})", address, regs1.len());
    }

    // Lecture bloc 2 : 0x0010–0x0013 → 4 registres (2 × FLOAT32)
    // energy_import, energy_export
    let regs2: Vec<u16> = ctx
        .read_input_registers(0x0010, 4)
        .await
        .map_err(|e| anyhow::anyhow!("ET112 addr={:#04x} FC04 read 0x0010: {}", address, e))?
        .map_err(|e| anyhow::anyhow!("ET112 addr={:#04x} exception code: {:?}", address, e))?;

    if regs2.len() < 4 {
        anyhow::bail!("ET112 addr={:#04x} réponse trop courte bloc 2 ({})", address, regs2.len());
    }

    // Décodage FLOAT32 big-endian (2 registres par valeur)
    let voltage_v          = regs_to_f32(regs1[0],  regs1[1]);
    let current_a          = regs_to_f32(regs1[2],  regs1[3]);
    let power_w            = regs_to_f32(regs1[4],  regs1[5]);
    let apparent_power_va  = regs_to_f32(regs1[6],  regs1[7]);
    let reactive_power_var = regs_to_f32(regs1[8],  regs1[9]);
    let power_factor       = regs_to_f32(regs1[10], regs1[11]);
    // regs1[12..13] = phase_angle (non utilisé pour l'instant)
    let frequency_hz       = regs_to_f32(regs1[14], regs1[15]);

    let energy_import_wh   = regs_to_f32(regs2[0],  regs2[1]);
    let energy_export_wh   = regs_to_f32(regs2[2],  regs2[3]);

    Ok(Et112Snapshot {
        address,
        name: dev.name.clone(),
        timestamp: Local::now(),
        voltage_v,
        current_a,
        power_w,
        apparent_power_va,
        reactive_power_var,
        power_factor,
        frequency_hz,
        energy_import_wh,
        energy_export_wh,
    })
}
