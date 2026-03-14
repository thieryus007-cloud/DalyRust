//! Commandes de lecture implémentées pour le protocole Daly BMS.
//!
//! Chaque fonction correspond à un Data ID du protocole et retourne
//! une structure typée prête à être assemblée dans un [`BmsSnapshot`].

use crate::bus::DalyPort;
use crate::error::Result;
use crate::protocol::{
    DataId, decode_cell_voltage, decode_current, decode_soc, decode_temperature,
    decode_voltage, read_u16_be,
};
use crate::types::{
    BalanceFlags, CellTemperatures, CellVoltages, MosStatus, SocData, StatusInfo, SystemData,
};
use std::sync::Arc;
use tracing::trace;

/// Lit le statut pack : tension totale, courant, SOC (Data ID 0x90).
pub async fn get_pack_status(port: &Arc<DalyPort>, addr: u8) -> Result<SocData> {
    let frame = port.send_command(addr, DataId::PackStatus, [0u8; 8]).await?;
    let d = frame.data();
    Ok(SocData {
        voltage: decode_voltage(d, 0),
        current: decode_current(d, 2),
        soc:     decode_soc(d, 4),
    })
}

/// Lit les tensions min/max des cellules avec les numéros de cellule (0x91).
///
/// Retourne (min_voltage, min_cell_id, max_voltage, max_cell_id).
pub async fn get_cell_voltage_minmax(
    port: &Arc<DalyPort>,
    addr: u8,
) -> Result<(f32, u8, f32, u8)> {
    let frame = port
        .send_command(addr, DataId::CellVoltageMinMax, [0u8; 8])
        .await?;
    let d = frame.data();
    let max_v    = decode_cell_voltage(d, 0);
    let max_cell = d[2];
    let min_v    = decode_cell_voltage(d, 3);
    let min_cell = d[5];
    Ok((min_v, min_cell, max_v, max_cell))
}

/// Lit les températures min/max avec les numéros de capteur (0x92).
///
/// Retourne (min_temp, min_sensor, max_temp, max_sensor).
pub async fn get_temperature_minmax(
    port: &Arc<DalyPort>,
    addr: u8,
) -> Result<(f32, u8, f32, u8)> {
    let frame = port
        .send_command(addr, DataId::TemperatureMinMax, [0u8; 8])
        .await?;
    let d = frame.data();
    let max_t      = decode_temperature(d[0]);
    let max_sensor = d[1];
    let min_t      = decode_temperature(d[2]);
    let min_sensor = d[3];
    Ok((min_t, min_sensor, max_t, max_sensor))
}

/// Lit l'état des MOSFET, les cycles et la capacité résiduelle (0x93).
pub async fn get_mos_status(port: &Arc<DalyPort>, addr: u8) -> Result<MosStatus> {
    let frame = port.send_command(addr, DataId::MosStatus, [0u8; 8]).await?;
    let d = frame.data();
    Ok(MosStatus {
        charge_mos:           d[0] & 0x02 != 0,
        discharge_mos:        d[0] & 0x01 != 0,
        bms_life:             d[1],
        residual_capacity_mah: u32::from_be_bytes([d[4], d[5], d[6], d[7]]),
        charge_cycles:        read_u16_be(d, 2) as u32,
    })
}

/// Lit les informations de statut 1 : nombre de cellules, capteurs, états (0x94).
pub async fn get_status_info(port: &Arc<DalyPort>, addr: u8) -> Result<StatusInfo> {
    let frame = port.send_command(addr, DataId::StatusInfo1, [0u8; 8]).await?;
    let d = frame.data();
    Ok(StatusInfo {
        cell_count:       d[0],
        temp_sensor_count: d[1],
        charger_status:   d[2],
        load_status:      d[3],
        dio_states:       d[4],
        cycle_count:      read_u16_be(d, 5),
    })
}

/// Lit les tensions individuelles de toutes les cellules (0x95, multi-trames).
///
/// Le protocole Daly envoie 3 tensions par trame (uint16 BE, en millivolts).
/// Le nombre de trames = ceil(cell_count / 3).
/// Chaque trame successive incrémente automatiquement l'index de bloc.
pub async fn get_cell_voltages(
    port: &Arc<DalyPort>,
    addr: u8,
    cell_count: u8,
) -> Result<CellVoltages> {
    let frame_count = (cell_count as usize + 2) / 3;
    let mut voltages = Vec::with_capacity(cell_count as usize);

    for i in 0..frame_count {
        // La trame de requête précise le numéro de bloc dans data[0]
        let mut data = [0u8; 8];
        data[0] = (i + 1) as u8;
        let frame = port.send_command(addr, DataId::CellVoltages1, data).await?;
        let d = frame.data();

        // 3 cellules par trame, octets 1-2, 3-4, 5-6 (octet 0 = frame index)
        for j in 0..3 {
            let cell_idx = i * 3 + j;
            if cell_idx >= cell_count as usize {
                break;
            }
            let offset = 1 + j * 2;
            voltages.push(decode_cell_voltage(d, offset));
        }
        trace!(addr = format!("{:#04x}", addr), frame = i + 1, "tensions cellules lues");
    }

    Ok(CellVoltages { voltages })
}

/// Lit les températures individuelles de tous les capteurs (0x96, multi-trames).
///
/// 7 températures par trame (7 octets, encodage = valeur + 40).
pub async fn get_temperatures(
    port: &Arc<DalyPort>,
    addr: u8,
    sensor_count: u8,
) -> Result<CellTemperatures> {
    let frame_count = (sensor_count as usize + 6) / 7;
    let mut temperatures = Vec::with_capacity(sensor_count as usize);

    for i in 0..frame_count {
        let mut data = [0u8; 8];
        data[0] = (i + 1) as u8;
        let frame = port.send_command(addr, DataId::Temperatures, data).await?;
        let d = frame.data();

        for j in 0..7 {
            let sensor_idx = i * 7 + j;
            if sensor_idx >= sensor_count as usize {
                break;
            }
            temperatures.push(decode_temperature(d[j + 1]));
        }
    }

    Ok(CellTemperatures { temperatures })
}

/// Lit les flags d'équilibrage cellule par cellule (0x97).
///
/// 48 cellules max, encodées en bits little-endian sur 6 octets.
pub async fn get_balance_flags(
    port: &Arc<DalyPort>,
    addr: u8,
    cell_count: u8,
) -> Result<BalanceFlags> {
    let frame = port
        .send_command(addr, DataId::BalanceStatus, [0u8; 8])
        .await?;
    let d = frame.data();

    let mut flags = Vec::with_capacity(cell_count as usize);
    for i in 0..(cell_count as usize) {
        let byte_idx = i / 8;
        let bit_idx  = i % 8;
        if byte_idx < 6 {
            flags.push((d[byte_idx] >> bit_idx) & 1 != 0);
        } else {
            flags.push(false);
        }
    }

    Ok(BalanceFlags { flags })
}

/// Lit les drapeaux d'alarme/protection (0x98).
///
/// Retourne (charge_mos_en, discharge_mos_en, alarm_flags_7_bytes).
pub async fn get_alarm_flags(
    port: &Arc<DalyPort>,
    addr: u8,
) -> Result<(bool, bool, [u8; 7])> {
    let frame = port
        .send_command(addr, DataId::AlarmFlags, [0u8; 8])
        .await?;
    let d = frame.data();
    let charge_en    = d[0] & 0x02 != 0;
    let discharge_en = d[0] & 0x01 != 0;
    let mut alarm_bytes = [0u8; 7];
    alarm_bytes.copy_from_slice(&d[1..8]);
    Ok((charge_en, discharge_en, alarm_bytes))
}

// =============================================================================
// Parsing des alarmes
// =============================================================================

use crate::types::Alarms;

/// Convertit les 7 octets bruts d'alarme (0x98) en structure [`Alarms`].
///
/// Mapping basé sur la documentation Daly UART V1.21, page 6.
pub fn parse_alarm_flags(bytes: &[u8; 7]) -> Alarms {
    // Mapping basé sur documentation Daly UART V1.21, page 6.
    // Byte 0 : [bit0]=cell_OVP, [bit1]=cell_UVP, [bit2]=pack_OVP, [bit3]=pack_UVP
    // Byte 1 : [bit0]=charge_OTP, [bit1]=charge_UTP, [bit2]=disch_OTP, [bit3]=disch_UTP
    // Byte 2 : [bit0]=charge_OCP, [bit1]=disch_OCP
    // Byte 3 : [bit0]=cell_imbalance
    // Byte 5 : [bit5]=fuse_blown
    Alarms {
        high_voltage:             ((bytes[0] >> 0) | (bytes[0] >> 2)) & 1,
        low_voltage:              ((bytes[0] >> 1) | (bytes[0] >> 3)) & 1,
        low_cell_voltage:         (bytes[0] >> 1) & 1,
        high_charge_temperature:  (bytes[1] >> 0) & 1,
        low_charge_temperature:   (bytes[1] >> 1) & 1,
        high_temperature:         (bytes[1] >> 2) & 1,
        low_temperature:          (bytes[1] >> 3) & 1,
        high_charge_current:      (bytes[2] >> 0) & 1,
        high_discharge_current:   (bytes[2] >> 1) & 1,
        high_current:             ((bytes[2] >> 0) | (bytes[2] >> 1)) & 1,
        cell_imbalance:           (bytes[3] >> 0) & 1,
        fuse_blown:               (bytes[5] >> 5) & 1,
        low_soc:                  0, // calculé par l'AlertEngine logiciel
    }
}
