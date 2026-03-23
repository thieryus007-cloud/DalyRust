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
//!
//! ## Refactoring bus unifié
//!
//! Cette version utilise `rs485_bus::SharedBus` + `rs485_bus::modbus_rtu`
//! au lieu de `tokio-modbus`. Le port série est partagé avec les BMS Daly
//! et le capteur PRALRAN sur un seul `/dev/ttyUSB0`.

use super::types::Et112Snapshot;
use crate::config::Et112DeviceConfig;
use chrono::Local;
use rs485_bus::{modbus_rtu, SharedBus};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Décode deux registres Modbus ET112 en INT32 signé (little-endian word order).
///
/// Le ET112 envoie le registre de poids faible (LSW) en premier, poids fort (MSW) en second.
/// Source : dbus-cgwacs Victron : `(int32_t)(registers[0] | registers[1] << 16)`
fn regs_to_i32(lo: u16, hi: u16) -> i32 {
    (lo as i32) | ((hi as i32) << 16)
}

/// Lance la boucle de polling ET112 sur le bus RS485 unifié.
///
/// # Paramètres
/// - `bus`           : bus RS485 partagé (même instance que le bus Daly BMS)
/// - `devices`       : liste des ET112 configurés (adresse + nom + ...)
/// - `poll_interval` : intervalle entre deux cycles complets
/// - `on_snapshot`   : callback appelé pour chaque snapshot valide
pub async fn run_et112_poll_loop<F>(
    bus: Arc<SharedBus>,
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
        count = devices.len(),
        "ET112 polling démarré (bus RS485 unifié)"
    );

    loop {
        for dev in &devices {
            let address = dev.parsed_address();
            match poll_device(&bus, address, dev).await {
                Ok(snap) => {
                    debug!(
                        addr  = format!("{:#04x}", address),
                        name  = %dev.name,
                        power_w = snap.power_w,
                        "ET112 snapshot OK"
                    );
                    on_snapshot(snap);
                }
                Err(e) => {
                    warn!(
                        addr = format!("{:#04x}", address),
                        name = %dev.name,
                        "ET112 erreur lecture : {:#}",
                        e
                    );
                }
            }
        }

        tokio::time::sleep(poll_interval).await;
    }
}

/// Interroge un ET112 et retourne un snapshot complet.
async fn poll_device(
    bus: &SharedBus,
    address: u8,
    dev: &Et112DeviceConfig,
) -> anyhow::Result<Et112Snapshot> {
    // ── Bloc 1 : 0x0000–0x000F → 16 registres ────────────────────────────────
    // Tension, courant, puissances (active/apparente/réactive), facteur de puissance,
    // (2 registres réservés), fréquence (INT16 au registre 0x000F)
    let req1 = modbus_rtu::build_fc04(address, 0x0000, 16);
    let resp1 = bus
        .transact(&req1, modbus_rtu::response_len(16))
        .await
        .map_err(|e| anyhow::anyhow!("ET112 {:#04x} bloc1: {}", address, e))?;
    let regs1 = modbus_rtu::parse_read_response(address, 0x04, &resp1)
        .map_err(|e| anyhow::anyhow!("ET112 {:#04x} parse bloc1: {}", address, e))?;

    if regs1.len() < 16 {
        anyhow::bail!(
            "ET112 {:#04x} bloc1 trop court ({} registres)",
            address,
            regs1.len()
        );
    }

    // ── Bloc 2 : 0x0010–0x0011 → 2 registres (énergie import, INT32) ─────────
    let req2 = modbus_rtu::build_fc04(address, 0x0010, 2);
    let resp2 = bus
        .transact(&req2, modbus_rtu::response_len(2))
        .await
        .map_err(|e| anyhow::anyhow!("ET112 {:#04x} bloc2: {}", address, e))?;
    let regs2 = modbus_rtu::parse_read_response(address, 0x04, &resp2)
        .map_err(|e| anyhow::anyhow!("ET112 {:#04x} parse bloc2: {}", address, e))?;

    if regs2.len() < 2 {
        anyhow::bail!(
            "ET112 {:#04x} bloc2 trop court ({} registres)",
            address,
            regs2.len()
        );
    }

    // ── Bloc 3 : 0x0020–0x0021 → 2 registres (énergie export, INT32) ─────────
    let req3 = modbus_rtu::build_fc04(address, 0x0020, 2);
    let resp3 = bus
        .transact(&req3, modbus_rtu::response_len(2))
        .await
        .map_err(|e| anyhow::anyhow!("ET112 {:#04x} bloc3: {}", address, e))?;
    let regs3 = modbus_rtu::parse_read_response(address, 0x04, &resp3)
        .map_err(|e| anyhow::anyhow!("ET112 {:#04x} parse bloc3: {}", address, e))?;

    if regs3.len() < 2 {
        anyhow::bail!(
            "ET112 {:#04x} bloc3 trop court ({} registres)",
            address,
            regs3.len()
        );
    }

    // ── Décodage INT32 little-endian word order ────────────────────────────────
    // Le ET112 stocke les valeurs 32 bits en « little-endian word order » :
    // index pair = LSW (poids faible), index impair = MSW (poids fort).
    // Source : dbus-cgwacs Victron.
    let voltage_v          = regs_to_i32(regs1[0],  regs1[1])  as f32 * 0.1;
    let current_a          = regs_to_i32(regs1[2],  regs1[3])  as f32 * 0.001;
    let power_w            = regs_to_i32(regs1[4],  regs1[5])  as f32 * 0.1;
    let apparent_power_va  = regs_to_i32(regs1[6],  regs1[7])  as f32 * 0.1;
    let reactive_power_var = regs_to_i32(regs1[8],  regs1[9])  as f32 * 0.1;
    let power_factor       = regs_to_i32(regs1[10], regs1[11]) as f32 * 0.001;
    // regs1[12..13] = phase_angle / réservé (non utilisé)
    // regs1[14] = réservé
    // regs1[15] (0x000F) = fréquence INT16 simple, facteur 0.1 → Hz
    let frequency_hz       = regs1[15] as i16 as f32 * 0.1;

    // Énergie : INT32 × 0.1 = kWh ; ×100 pour convertir en Wh
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
