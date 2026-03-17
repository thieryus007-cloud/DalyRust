//! Commandes d'écriture sécurisées pour le BMS Daly.
//!
//! Toutes les commandes d'écriture :
//! 1. Vérifient que le mode read-only n'est pas activé.
//! 2. Envoient la commande et attendent la confirmation du BMS.
//! 3. Effectuent une lecture de vérification post-écriture.
//!
//! ## Commandes disponibles
//! - [`set_discharge_mos`] — activer/désactiver le MOSFET de décharge (0xD9)
//! - [`set_charge_mos`]    — activer/désactiver le MOSFET de charge (0xDA)
//! - [`set_soc`]           — calibrer le SOC (0x21)
//! - [`reset_bms`]         — réinitialiser le BMS (0x00)

use crate::bus::DalyPort;
use crate::error::{DalyError, Result};
use crate::protocol::DataId;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// Activer ou désactiver le MOSFET de décharge (Data ID 0xD9).
///
/// `enable = true` → MOS ON ; `enable = false` → MOS OFF.
pub async fn set_discharge_mos(
    port: &Arc<DalyPort>,
    addr: u8,
    enable: bool,
    read_only: bool,
) -> Result<()> {
    if read_only {
        return Err(DalyError::ReadOnly);
    }
    let payload = u8::from(enable);
    info!(
        bms = format!("{:#04x}", addr),
        "set_discharge_mos → {}",
        if enable { "ON" } else { "OFF" }
    );
    port.send_command(addr, DataId::SetDischargeMos, payload_to_data(payload))
        .await?;
    // Vérification : lire l'état MOS et confirmer
    tokio::time::sleep(Duration::from_millis(200)).await;
    let mos = crate::commands::get_mos_status(port, addr).await?;
    if mos.discharge_mos != enable {
        warn!(bms = format!("{:#04x}", addr), "Vérification set_discharge_mos échouée");
        return Err(DalyError::VerifyFailed { bms_id: addr, cmd: DataId::SetDischargeMos as u8 });
    }
    Ok(())
}

/// Activer ou désactiver le MOSFET de charge (Data ID 0xDA).
pub async fn set_charge_mos(
    port: &Arc<DalyPort>,
    addr: u8,
    enable: bool,
    read_only: bool,
) -> Result<()> {
    if read_only {
        return Err(DalyError::ReadOnly);
    }
    let payload = u8::from(enable);
    info!(
        bms = format!("{:#04x}", addr),
        "set_charge_mos → {}",
        if enable { "ON" } else { "OFF" }
    );
    port.send_command(addr, DataId::SetChargeMos, payload_to_data(payload))
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let mos = crate::commands::get_mos_status(port, addr).await?;
    if mos.charge_mos != enable {
        warn!(bms = format!("{:#04x}", addr), "Vérification set_charge_mos échouée");
        return Err(DalyError::VerifyFailed { bms_id: addr, cmd: DataId::SetChargeMos as u8 });
    }
    Ok(())
}

/// Calibrer le SOC à la valeur indiquée en % (Data ID 0x21).
///
/// La valeur est encodée en uint16 BE × 10 à l'offset 4 de la trame.
pub async fn set_soc(
    port: &Arc<DalyPort>,
    addr: u8,
    soc_percent: f32,
    read_only: bool,
) -> Result<()> {
    if read_only {
        return Err(DalyError::ReadOnly);
    }
    if !(0.0..=100.0).contains(&soc_percent) {
        return Err(anyhow::anyhow!("SOC hors plage [0, 100] : {}", soc_percent).into());
    }
    info!(bms = format!("{:#04x}", addr), "set_soc → {:.1}%", soc_percent);
    let raw = (soc_percent * 10.0) as u16;
    let mut data = [0u8; 8];
    data[0] = (raw >> 8) as u8;
    data[1] = (raw & 0xFF) as u8;
    port.send_command(addr, DataId::SetSoc, data).await?;
    Ok(())
}

/// Réinitialiser le BMS (Data ID 0x00). ⚠️ Utiliser avec précaution.
pub async fn reset_bms(port: &Arc<DalyPort>, addr: u8, read_only: bool) -> Result<()> {
    if read_only {
        return Err(DalyError::ReadOnly);
    }
    warn!(bms = format!("{:#04x}", addr), "RESET BMS demandé !");
    port.send_command(addr, DataId::Reset, [0u8; 8]).await?;
    Ok(())
}

// =============================================================================
// Utilitaire interne
// =============================================================================

/// Crée un tableau data[8] avec `value` dans data[0], reste à zéro.
fn payload_to_data(value: u8) -> [u8; 8] {
    let mut data = [0u8; 8];
    data[0] = value;
    data
}
