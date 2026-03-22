//! Boucle de polling Modbus RTU pour le compteur Carlo Gavazzi ET112.
//!
//! ## Registres lus (FC=04, INT32 little-endian word order)
//!
//! Source : dbus-cgwacs Victron (Em112Commands), confirmé par documentation officielle.
//! Format : INT32 signé, premier registre = LSW, second registre = MSW.
//!
//! | Adresse hex | Description           | Unité | Facteur |
//! |-------------|----------------------|-------|---------|
//! | 0x0000      | Tension L1           | V     | ×0.1    |
//! | 0x0002      | Courant L1           | A     | ×0.001  |
//! | 0x0004      | Puissance active     | W     | ×0.1    |
//! | 0x0006      | Puissance apparente  | VA    | ×0.1    |
//! | 0x0008      | Puissance réactive   | VAr   | ×0.1    |
//! | 0x000A      | Facteur de puissance | —     | ×0.001  |
//! | 0x000F      | Fréquence            | Hz    | ×0.1 (INT16 simple) |
//! | 0x0010      | Énergie import       | kWh   | ×0.1    |
//! | 0x0020      | Énergie export       | kWh   | ×0.1    |

use super::types::Et112Snapshot;
use crate::config::Et112DeviceConfig;
use chrono::Local;
use std::time::Duration;
use tokio_modbus::client::rtu;
use tokio_modbus::prelude::*;
use tokio_serial::SerialStream;
use tracing::{debug, error, info, warn};

/// Décode deux registres Modbus ET112 en INT32 signé (little-endian word order).
///
/// Le ET112 envoie le registre de poids faible (LSW) en premier, poids fort (MSW) en second.
/// Source : dbus-cgwacs Victron : `(int32_t)(registers[0] | registers[1] << 16)`
fn regs_to_i32(lo: u16, hi: u16) -> i32 {
    (lo as i32) | ((hi as i32) << 16)
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

    // Lecture bloc 1 : 0x0000–0x000F → 16 registres
    // voltage, current, power_active, power_apparent, power_reactive, pf, (reserved×2), freq(INT16)
    let regs1: Vec<u16> = ctx
        .read_input_registers(0x0000, 16)
        .await
        .map_err(|e| anyhow::anyhow!("ET112 addr={:#04x} FC04 read 0x0000: {}", address, e))?
        .map_err(|e| anyhow::anyhow!("ET112 addr={:#04x} exception code: {:?}", address, e))?;

    if regs1.len() < 16 {
        anyhow::bail!("ET112 addr={:#04x} réponse trop courte bloc 1 ({})", address, regs1.len());
    }

    // Lecture bloc 2 : 0x0010–0x0011 → 2 registres (énergie import, INT32)
    let regs2: Vec<u16> = ctx
        .read_input_registers(0x0010, 2)
        .await
        .map_err(|e| anyhow::anyhow!("ET112 addr={:#04x} FC04 read 0x0010: {}", address, e))?
        .map_err(|e| anyhow::anyhow!("ET112 addr={:#04x} exception code: {:?}", address, e))?;

    if regs2.len() < 2 {
        anyhow::bail!("ET112 addr={:#04x} réponse trop courte bloc 2 ({})", address, regs2.len());
    }

    // Lecture bloc 3 : 0x0020–0x0021 → 2 registres (énergie export, INT32)
    let regs3: Vec<u16> = ctx
        .read_input_registers(0x0020, 2)
        .await
        .map_err(|e| anyhow::anyhow!("ET112 addr={:#04x} FC04 read 0x0020: {}", address, e))?
        .map_err(|e| anyhow::anyhow!("ET112 addr={:#04x} exception code: {:?}", address, e))?;

    if regs3.len() < 2 {
        anyhow::bail!("ET112 addr={:#04x} réponse trop courte bloc 3 ({})", address, regs3.len());
    }

    // Décodage INT32 little-endian (LSW first) avec facteurs d'échelle Victron cgwacs
    let voltage_v          = regs_to_i32(regs1[0],  regs1[1])  as f32 * 0.1;
    let current_a          = regs_to_i32(regs1[2],  regs1[3])  as f32 * 0.001;
    let power_w            = regs_to_i32(regs1[4],  regs1[5])  as f32 * 0.1;
    let apparent_power_va  = regs_to_i32(regs1[6],  regs1[7])  as f32 * 0.1;
    let reactive_power_var = regs_to_i32(regs1[8],  regs1[9])  as f32 * 0.1;
    let power_factor       = regs_to_i32(regs1[10], regs1[11]) as f32 * 0.001;
    // regs1[12..13] = phase_angle / reserved (non utilisé)
    // regs1[15] (0x000F) = fréquence INT16, scale 0.1 → Hz
    let frequency_hz       = regs1[15] as i16 as f32 * 0.1;

    // Énergie : INT32 × 0.1 = kWh ; ×100 → Wh pour le champ energy_*_wh
    let energy_import_wh   = regs_to_i32(regs2[0], regs2[1]) as f32 * 100.0;
    let energy_export_wh   = regs_to_i32(regs3[0], regs3[1]) as f32 * 100.0;

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
