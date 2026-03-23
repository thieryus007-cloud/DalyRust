//! Framing Modbus RTU pur Rust — CRC16, FC03, FC04, FC06.
//!
//! Implémentation minimale et autonome sans dépendance à `tokio-modbus`.
//! Couvre les besoins du projet :
//! - FC04 (Read Input Registers) → ET112, PRALRAN
//! - FC03 (Read Holding Registers) → CHINT ATS (futur)
//! - FC06 (Write Single Register) → CHINT ATS (futur)

// =============================================================================
// CRC-16/Modbus
// =============================================================================

/// Calcule le CRC-16/Modbus (polynôme 0xA001, init 0xFFFF, LSB first).
///
/// La trame Modbus RTU se termine par [CRC_LO, CRC_HI] (little-endian).
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

// =============================================================================
// Constructeurs de requêtes
// =============================================================================

/// FC04 — Read Input Registers (8 octets).
///
/// ```text
/// [ADDR][0x04][REG_HI][REG_LO][COUNT_HI][COUNT_LO][CRC_LO][CRC_HI]
/// ```
pub fn build_fc04(addr: u8, reg_start: u16, count: u16) -> [u8; 8] {
    let mut frame = [
        addr,
        0x04,
        (reg_start >> 8) as u8,
        reg_start as u8,
        (count >> 8) as u8,
        count as u8,
        0,
        0,
    ];
    let crc = crc16(&frame[..6]);
    frame[6] = crc as u8;
    frame[7] = (crc >> 8) as u8;
    frame
}

/// FC03 — Read Holding Registers (8 octets).
///
/// ```text
/// [ADDR][0x03][REG_HI][REG_LO][COUNT_HI][COUNT_LO][CRC_LO][CRC_HI]
/// ```
pub fn build_fc03(addr: u8, reg_start: u16, count: u16) -> [u8; 8] {
    let mut frame = [
        addr,
        0x03,
        (reg_start >> 8) as u8,
        reg_start as u8,
        (count >> 8) as u8,
        count as u8,
        0,
        0,
    ];
    let crc = crc16(&frame[..6]);
    frame[6] = crc as u8;
    frame[7] = (crc >> 8) as u8;
    frame
}

/// FC06 — Write Single Register (8 octets).
///
/// ```text
/// [ADDR][0x06][REG_HI][REG_LO][VAL_HI][VAL_LO][CRC_LO][CRC_HI]
/// ```
pub fn build_fc06(addr: u8, reg: u16, value: u16) -> [u8; 8] {
    let mut frame = [
        addr,
        0x06,
        (reg >> 8) as u8,
        reg as u8,
        (value >> 8) as u8,
        value as u8,
        0,
        0,
    ];
    let crc = crc16(&frame[..6]);
    frame[6] = crc as u8;
    frame[7] = (crc >> 8) as u8;
    frame
}

// =============================================================================
// Longueur de réponse
// =============================================================================

/// Longueur attendue de la réponse FC03/FC04 pour `count` registres.
///
/// Format : `addr(1) + fc(1) + byte_count(1) + data(count×2) + crc(2)`
pub fn response_len(count: u16) -> usize {
    5 + (count as usize) * 2
}

// =============================================================================
// Parseur de réponse
// =============================================================================

/// Valide et décode la réponse à une requête FC03/FC04.
///
/// Vérifie : adresse, function code, byte_count, CRC.
/// Retourne les registres comme `Vec<u16>` (big-endian dans la trame Modbus,
/// index 0 = premier registre demandé).
///
/// # Paramètres
/// - `addr` : adresse esclave attendue
/// - `fc`   : function code attendu (0x03 ou 0x04)
/// - `buf`  : octets bruts reçus (longueur = `response_len(count)`)
pub fn parse_read_response(addr: u8, fc: u8, buf: &[u8]) -> anyhow::Result<Vec<u16>> {
    if buf.len() < 5 {
        anyhow::bail!(
            "Réponse Modbus trop courte ({} octets, minimum 5)",
            buf.len()
        );
    }

    // Adresse
    if buf[0] != addr {
        anyhow::bail!(
            "Adresse inattendue : attendu {:#04x}, reçu {:#04x}",
            addr,
            buf[0]
        );
    }

    // Function code (ou exception)
    if buf[1] == fc | 0x80 {
        let exc = buf.get(2).copied().unwrap_or(0);
        anyhow::bail!("Exception Modbus : FC {:#04x}, code {:#04x}", fc, exc);
    }
    if buf[1] != fc {
        anyhow::bail!(
            "Function code inattendu : attendu {:#04x}, reçu {:#04x}",
            fc,
            buf[1]
        );
    }

    // Byte count
    let byte_count = buf[2] as usize;
    let expected_data_bytes = buf.len().saturating_sub(5); // addr+fc+bc+crc_lo+crc_hi
    if byte_count != expected_data_bytes {
        anyhow::bail!(
            "byte_count={} incohérent avec longueur buffer={} (attendu {})",
            byte_count,
            buf.len(),
            expected_data_bytes
        );
    }

    // CRC
    let crc_calc = crc16(&buf[..buf.len() - 2]);
    let crc_recv = (buf[buf.len() - 2] as u16) | ((buf[buf.len() - 1] as u16) << 8);
    if crc_recv != crc_calc {
        anyhow::bail!(
            "CRC invalide : reçu {:#06x}, calculé {:#06x}",
            crc_recv,
            crc_calc
        );
    }

    // Extraction registres (big-endian u16 dans la trame Modbus)
    let data = &buf[3..buf.len() - 2];
    let regs = data
        .chunks(2)
        .map(|c| ((c[0] as u16) << 8) | c[1] as u16)
        .collect();

    Ok(regs)
}

// =============================================================================
// Tests unitaires
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_known_value() {
        // Exemple Modbus RTU standard : requête FC03 addr=1 reg=0 count=1
        // Requête : 01 03 00 00 00 01 → CRC = 0x840A (lo=0x0A, hi=0x84)
        let data = [0x01u8, 0x03, 0x00, 0x00, 0x00, 0x01];
        let crc = crc16(&data);
        assert_eq!(crc, 0x840A, "CRC Modbus RTU incorrect");
    }

    #[test]
    fn test_build_fc04_pralran() {
        // FC04 addr=0x05 reg=0x0000 count=1
        let frame = build_fc04(0x05, 0x0000, 1);
        assert_eq!(frame[0], 0x05);
        assert_eq!(frame[1], 0x04);
        assert_eq!(frame[2], 0x00);
        assert_eq!(frame[3], 0x00);
        assert_eq!(frame[4], 0x00);
        assert_eq!(frame[5], 0x01);
        // CRC doit être valide
        let crc = crc16(&frame[..6]);
        assert_eq!(frame[6], crc as u8);
        assert_eq!(frame[7], (crc >> 8) as u8);
    }

    #[test]
    fn test_response_len() {
        assert_eq!(response_len(1), 7);   // 5 + 2×1
        assert_eq!(response_len(2), 9);   // 5 + 2×2
        assert_eq!(response_len(16), 37); // 5 + 2×16
    }

    #[test]
    fn test_parse_read_response_valid() {
        // Simuler réponse FC04 addr=0x05, 1 registre, valeur=334 (irradiance)
        let value: u16 = 334;
        let data_hi = (value >> 8) as u8;
        let data_lo = value as u8;
        let raw = [0x05u8, 0x04, 0x02, data_hi, data_lo, 0x00, 0x00];
        // Calculer et corriger le CRC
        let mut buf = raw;
        let crc = crc16(&buf[..5]);
        buf[5] = crc as u8;
        buf[6] = (crc >> 8) as u8;

        let regs = parse_read_response(0x05, 0x04, &buf).unwrap();
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0], 334);
    }

    #[test]
    fn test_parse_read_response_bad_crc() {
        let buf = [0x05u8, 0x04, 0x02, 0x01, 0x4E, 0xFF, 0xFF]; // CRC invalide
        assert!(parse_read_response(0x05, 0x04, &buf).is_err());
    }
}
