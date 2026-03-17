//! Format des trames UART Daly BMS et calcul du checksum.
//!
//! ## Format d'une trame (13 octets)
//!
//! ```text
//! [0]  0xA5        Start flag
//! [1]  Adresse     PC→BMS : 0x3F + board_number  |  BMS→PC : board_number
//! [2]  Data ID     Commande (0x90, 0x91, …)
//! [3]  0x08        Longueur du champ Data (toujours 8)
//! [4-11] Data      8 octets (requête : réservés 0x00 ; réponse : valeurs)
//! [12] Checksum    Somme des octets [0–11] & 0xFF
//! ```
//!
//! ## Adressage multi-BMS (protocole Daly V1.21 §2.1)
//!
//! Le BMS filtre les requêtes sur `byte[1]` (son adresse PC écoutée).
//! Le board number N correspond à `byte[1] = 0x3F + N` dans la requête PC.
//!
//! ```text
//! Board 1  →  requête byte[1] = 0x40,  réponse byte[1] = 0x01
//! Board 2  →  requête byte[1] = 0x41,  réponse byte[1] = 0x02
//! Board N  →  requête byte[1] = 0x3F+N, réponse byte[1] = N
//! ```
//!
//! Les 8 octets de données sont réservés (0x00) dans la direction PC→BMS.

/// Longueur totale d'une trame Daly (requête ou réponse simple).
pub const FRAME_LEN: usize = 13;

/// Base de l'adresse PC : `PC_BASE + board_number` = adresse écoutée par ce BMS.
pub const PC_BASE: u8 = 0x3F;

/// Adresse PC pour le board 1 (valeur historique, conservée pour compatibilité).
pub const PC_ADDRESS: u8 = 0x40;

/// Calcule l'adresse PC à mettre dans `byte[1]` pour le board/BMS `addr`.
///
/// ```
/// use daly_bms_core::protocol::pc_address_for;
/// assert_eq!(pc_address_for(1), 0x40);
/// assert_eq!(pc_address_for(2), 0x41);
/// ```
#[inline]
pub fn pc_address_for(bms_addr: u8) -> u8 {
    PC_BASE.wrapping_add(bms_addr)
}

/// Octet de démarrage de trame.
pub const START_FLAG: u8 = 0xA5;

/// Longueur du champ Data (toujours 8 pour Daly UART V1.21).
pub const DATA_LEN: u8 = 0x08;

// =============================================================================
// Data IDs
// =============================================================================

/// Identifiants de commandes du protocole Daly UART/RS485.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DataId {
    /// Tension totale, courant, SOC (page 5 doc Daly)
    PackStatus        = 0x90,
    /// Tension min/max des cellules avec numéro de cellule
    CellVoltageMinMax = 0x91,
    /// Températures min/max avec numéro de capteur
    TemperatureMinMax = 0x92,
    /// État MOS charge/décharge, cycles, capacité résiduelle
    MosStatus         = 0x93,
    /// Status information 1 : nombre de cellules/capteurs, état
    StatusInfo1       = 0x94,
    /// Tensions cellules bloc 1 (cellules 1-3)
    CellVoltages1     = 0x95,
    /// Températures individuelles des capteurs
    Temperatures      = 0x96,
    /// Flags d'équilibrage cellule par cellule (48 max, bits little-endian)
    BalanceStatus     = 0x97,
    /// Drapeaux d'alarme/protection (7 octets)
    AlarmFlags        = 0x98,

    // ── Commandes d'écriture ──────────────────────────────────────────────────
    /// Reset BMS
    Reset             = 0x00,
    /// Calibration SOC (uint16 BE × 10 à l'offset 4)
    SetSoc            = 0x21,
    /// Commande MOS décharge (0x01 = on, 0x00 = off)
    SetDischargeMos   = 0xD9,
    /// Commande MOS charge (0x01 = on, 0x00 = off)
    SetChargeMos      = 0xDA,
}

impl DataId {
    /// Convertit un octet brut en DataId. Retourne `None` si inconnu.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x90 => Some(Self::PackStatus),
            0x91 => Some(Self::CellVoltageMinMax),
            0x92 => Some(Self::TemperatureMinMax),
            0x93 => Some(Self::MosStatus),
            0x94 => Some(Self::StatusInfo1),
            0x95 => Some(Self::CellVoltages1),
            0x96 => Some(Self::Temperatures),
            0x97 => Some(Self::BalanceStatus),
            0x98 => Some(Self::AlarmFlags),
            0x00 => Some(Self::Reset),
            0x21 => Some(Self::SetSoc),
            0xD9 => Some(Self::SetDischargeMos),
            0xDA => Some(Self::SetChargeMos),
            _    => None,
        }
    }

    /// `true` si c'est une commande d'écriture (dangereux sans confirmation).
    pub fn is_write(self) -> bool {
        matches!(self, Self::Reset | Self::SetSoc | Self::SetDischargeMos | Self::SetChargeMos)
    }
}

// =============================================================================
// Trame de requête
// =============================================================================

/// Trame de requête PC → BMS (12 octets + checksum = 13).
#[derive(Debug, Clone, Copy)]
pub struct RequestFrame {
    pub bytes: [u8; FRAME_LEN],
}

impl RequestFrame {
    /// Construit une trame de requête avec les 8 octets de données spécifiés.
    ///
    /// Protocole Daly V1.21 §2.3.1 :
    /// - `byte[1]` = `0x3F + bms_address` (adresse PC écoutée par ce BMS)
    /// - `data[0..7]` = réservés (0x00) sauf pour certaines commandes d'écriture
    pub fn new(bms_address: u8, cmd: DataId, data: [u8; 8]) -> Self {
        let mut bytes = [0u8; FRAME_LEN];
        bytes[0] = START_FLAG;
        bytes[1] = pc_address_for(bms_address);  // 0x3F + bms_address
        bytes[2] = cmd as u8;
        bytes[3] = DATA_LEN;
        bytes[4..12].copy_from_slice(&data);
        bytes[12] = checksum(&bytes[..12]);
        Self { bytes }
    }

    /// Trame de lecture standard : data[0..7] = 0x00 (réservé).
    ///
    /// `A5 [0x3F+addr] <CMD> 08 00 00 00 00 00 00 00 00 <CS>`
    pub fn read(bms_address: u8, cmd: DataId) -> Self {
        Self::new(bms_address, cmd, [0u8; 8])
    }

    /// Trame d'écriture avec 1 octet de payload dans data[0].
    pub fn write_byte(bms_address: u8, cmd: DataId, value: u8) -> Self {
        let mut data = [0u8; 8];
        data[0] = value;
        Self::new(bms_address, cmd, data)
    }

    /// Trame d'écriture SOC : valeur en % × 10, uint16 BE dans data[6..7].
    ///
    /// Protocole 0x21 : bytes [4-9] = date/time (zéros), bytes [10-11] = SOC
    /// → data[6..7] dans le payload de 8 octets (conforme python-daly-bms).
    pub fn write_soc(bms_address: u8, soc_percent: f32) -> Self {
        let raw = (soc_percent * 10.0) as u16;
        let mut data = [0u8; 8];
        data[6] = (raw >> 8) as u8;
        data[7] = (raw & 0xFF) as u8;
        Self::new(bms_address, DataId::SetSoc, data)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

// =============================================================================
// Trame de réponse
// =============================================================================

/// Trame de réponse BMS → PC (13 octets).
#[derive(Debug, Clone, Copy)]
pub struct ResponseFrame {
    pub bytes: [u8; FRAME_LEN],
}

impl ResponseFrame {
    /// Parse une tranche d'octets en ResponseFrame.
    ///
    /// Valide : start flag, longueur, checksum.
    pub fn parse(raw: &[u8]) -> crate::error::Result<Self> {
        use crate::error::DalyError;

        if raw.len() < FRAME_LEN {
            return Err(DalyError::InvalidFrame {
                len: raw.len(),
                reason: "trame trop courte",
            });
        }

        if raw[0] != START_FLAG {
            return Err(DalyError::InvalidStartFlag(raw[0]));
        }

        let expected = checksum(&raw[..12]);
        let actual   = raw[12];
        if expected != actual {
            return Err(DalyError::Checksum { expected, actual });
        }

        let mut bytes = [0u8; FRAME_LEN];
        bytes.copy_from_slice(&raw[..FRAME_LEN]);
        Ok(Self { bytes })
    }

    /// Adresse BMS dans la réponse (octet [1]).
    pub fn address(&self) -> u8 { self.bytes[1] }

    /// Data ID dans la réponse (octet [2]).
    pub fn data_id(&self) -> u8 { self.bytes[2] }

    /// Les 8 octets de données (octets [4–11]).
    pub fn data(&self) -> &[u8; 8] {
        self.bytes[4..12].try_into().expect("slice de 8 octets")
    }

    /// Valide que la réponse correspond à la requête (adresse + cmd).
    pub fn validate_for(
        &self,
        expected_address: u8,
        expected_cmd: DataId,
    ) -> crate::error::Result<()> {
        use crate::error::DalyError;

        if self.address() != expected_address {
            return Err(DalyError::UnexpectedAddress {
                expected: expected_address,
                actual:   self.address(),
            });
        }
        if self.data_id() != expected_cmd as u8 {
            return Err(DalyError::UnexpectedDataId {
                expected: expected_cmd as u8,
                actual:   self.data_id(),
            });
        }
        Ok(())
    }
}

// =============================================================================
// Checksum
// =============================================================================

/// Calcule le checksum Daly : somme des `n` premiers octets, modulo 256.
///
/// ```
/// use daly_bms_core::protocol::checksum;
/// let frame = [0xA5u8, 0x40, 0x90, 0x08, 0,0,0,0, 0,0,0,0];
/// assert_eq!(checksum(&frame), 0x7D);
/// ```
pub fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().map(|&b| b as u32).sum::<u32>() as u8
}

// =============================================================================
// Utilitaires de décodage
// =============================================================================

/// Lit un uint16 big-endian à l'offset donné dans une slice.
#[inline]
pub fn read_u16_be(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([data[offset], data[offset + 1]])
}

/// Tension Daly : uint16 BE / 10 → Volts.
#[inline]
pub fn decode_voltage(data: &[u8], offset: usize) -> f32 {
    read_u16_be(data, offset) as f32 / 10.0
}

/// Courant Daly : (uint16 BE − 30 000) / 10 → Ampères.
/// Positif = charge, négatif = décharge.
#[inline]
pub fn decode_current(data: &[u8], offset: usize) -> f32 {
    (read_u16_be(data, offset) as i32 - 30_000) as f32 / 10.0
}

/// SOC Daly : uint16 BE / 10 → %.
#[inline]
pub fn decode_soc(data: &[u8], offset: usize) -> f32 {
    read_u16_be(data, offset) as f32 / 10.0
}

/// Tension de cellule Daly : uint16 BE / 1000 → Volts.
#[inline]
pub fn decode_cell_voltage(data: &[u8], offset: usize) -> f32 {
    read_u16_be(data, offset) as f32 / 1000.0
}

/// Température Daly : octet - 40 → °C.
#[inline]
pub fn decode_temperature(raw: u8) -> f32 {
    raw as f32 - 40.0
}

// =============================================================================
// Tests unitaires
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_pack_status() {
        // Requête standard 0x90 → adresse 0x01
        let req = RequestFrame::read(0x01, DataId::PackStatus);
        // Vérifier que le dernier octet est correct
        let expected = checksum(&req.bytes[..12]);
        assert_eq!(req.bytes[12], expected);
    }

    #[test]
    fn test_decode_voltage() {
        // 0x14, 0x8D = 5261 → 526.1 V... (pack 48V = ~520 raw)
        // Exemple réel : 52.53 V = 525 raw = 0x02, 0x0D
        let data = [0x02u8, 0x0D, 0, 0, 0, 0, 0, 0];
        assert!((decode_voltage(&data, 0) - 52.5).abs() < 0.1);
    }

    #[test]
    fn test_decode_current_discharge() {
        // -1.6 A → (30000 - 16) = 29984 = 0x75, 0x20
        let raw: u16 = 30_000 - 16; // 29984
        let bytes = raw.to_be_bytes();
        let data = [bytes[0], bytes[1], 0, 0, 0, 0, 0, 0];
        let current = decode_current(&data, 0);
        assert!((current - (-1.6)).abs() < 0.01);
    }

    #[test]
    fn test_decode_cell_voltage() {
        // 3405 mV → 0x0D, 0x4D
        let mv: u16 = 3405;
        let bytes = mv.to_be_bytes();
        let data = [bytes[0], bytes[1], 0, 0, 0, 0, 0, 0];
        assert!((decode_cell_voltage(&data, 0) - 3.405).abs() < 0.001);
    }

    #[test]
    fn test_decode_temperature() {
        // 24 °C → 64 raw
        assert!((decode_temperature(64) - 24.0).abs() < 0.01);
        // -5 °C → 35 raw
        assert!((decode_temperature(35) - (-5.0)).abs() < 0.01);
    }

    #[test]
    fn test_request_frame_checksum() {
        // BMS board 1 : byte[1] = 0x3F+1 = 0x40, data = 0x00
        // checksum = 0xA5 + 0x40 + 0x90 + 0x08 = 0x17D → 0x7D
        let frame = RequestFrame::read(0x01, DataId::PackStatus);
        assert_eq!(frame.bytes[0], START_FLAG);
        assert_eq!(frame.bytes[1], 0x40);   // pc_address_for(1) = 0x40
        assert_eq!(frame.bytes[2], 0x90);
        assert_eq!(frame.bytes[3], DATA_LEN);
        assert_eq!(frame.bytes[4], 0x00);   // data[0] réservé = 0x00
        assert_eq!(frame.bytes[12], 0x7D);

        // BMS board 2 : byte[1] = 0x3F+2 = 0x41, data = 0x00
        // checksum = 0xA5 + 0x41 + 0x90 + 0x08 = 0x17E → 0x7E
        let frame2 = RequestFrame::read(0x02, DataId::PackStatus);
        assert_eq!(frame2.bytes[1], 0x41);  // pc_address_for(2) = 0x41
        assert_eq!(frame2.bytes[4], 0x00);  // data[0] réservé = 0x00
        assert_eq!(frame2.bytes[12], 0x7E);
    }

    #[test]
    fn test_pc_address_for() {
        assert_eq!(pc_address_for(1), 0x40);
        assert_eq!(pc_address_for(2), 0x41);
        assert_eq!(pc_address_for(3), 0x42);
    }
}
