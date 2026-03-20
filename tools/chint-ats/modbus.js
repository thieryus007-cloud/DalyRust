'use strict';

// =============================================================================
// Modbus RTU — Utilitaires CHINT NXZBN-63S/2DT
// Protocole : Modbus-RTU, adresse esclave défaut = 3, 9600,E,8,1
// Référence : MANUEL_ATSE_RS485.md / CHINT V1.1
// =============================================================================

// -----------------------------------------------------------------------------
// CRC-16 Modbus (polynôme 0xA001, octet bas en premier dans la trame)
// -----------------------------------------------------------------------------
function crc16(buffer) {
    let crc = 0xFFFF;
    for (let i = 0; i < buffer.length; i++) {
        crc ^= buffer[i];
        for (let j = 0; j < 8; j++) {
            if (crc & 0x0001) {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    return crc; // low byte first when appended to frame
}

function appendCrc(buf) {
    const c = crc16(buf);
    return Buffer.concat([buf, Buffer.from([c & 0xFF, (c >> 8) & 0xFF])]);
}

function checkCrc(buf) {
    if (buf.length < 4) return false;
    const payload = buf.slice(0, -2);
    const received = buf[buf.length - 2] | (buf[buf.length - 1] << 8);
    return crc16(payload) === received;
}

// -----------------------------------------------------------------------------
// Construction de trames
// -----------------------------------------------------------------------------

/** FC 03 — Lecture de N registres */
function buildReadFrame(slaveAddr, startReg, count) {
    const buf = Buffer.from([
        slaveAddr,
        0x03,
        (startReg >> 8) & 0xFF, startReg & 0xFF,
        (count   >> 8) & 0xFF, count   & 0xFF,
    ]);
    return appendCrc(buf);
}

/** FC 06 — Écriture d'un registre */
function buildWriteFrame(slaveAddr, reg, value) {
    const buf = Buffer.from([
        slaveAddr,
        0x06,
        (reg   >> 8) & 0xFF, reg   & 0xFF,
        (value >> 8) & 0xFF, value & 0xFF,
    ]);
    return appendCrc(buf);
}

/** Trame hexadécimale brute (string "XX XX XX...") → Buffer */
function hexToFrame(hexStr) {
    const bytes = hexStr.trim().split(/\s+/).map(b => parseInt(b, 16));
    if (bytes.some(isNaN)) throw new Error('Trame hex invalide');
    return Buffer.from(bytes);
}

// -----------------------------------------------------------------------------
// Parsing des réponses
// -----------------------------------------------------------------------------

function parseReadResponse(buf, expectedCount) {
    if (!checkCrc(buf)) return { error: 'CRC invalide' };
    if (buf[1] & 0x80)  return { error: `Exception Modbus ${buf[2].toString(16).toUpperCase()}` };
    if (buf[1] !== 0x03) return { error: `Code fonction inattendu: ${buf[1]}` };

    const byteCount = buf[2];
    if (byteCount !== expectedCount * 2) return { error: `Longueur inattendue: ${byteCount}` };

    const registers = [];
    for (let i = 0; i < expectedCount; i++) {
        registers.push((buf[3 + i * 2] << 8) | buf[4 + i * 2]);
    }
    return { registers };
}

function parseWriteResponse(buf) {
    if (!checkCrc(buf)) return { error: 'CRC invalide' };
    if (buf[1] & 0x80)  return { error: `Exception Modbus ${buf[2].toString(16).toUpperCase()}` };
    if (buf[1] !== 0x06) return { error: `Code fonction inattendu: ${buf[1]}` };
    const reg   = (buf[2] << 8) | buf[3];
    const value = (buf[4] << 8) | buf[5];
    return { reg, value };
}

// -----------------------------------------------------------------------------
// Interprétation des registres CHINT NXZBN
// Référence : §4.1-§4.4 MANUEL_ATSE_RS485.md
// -----------------------------------------------------------------------------

/**
 * Registres mesures (lus en bloc 0x0006, count=8)
 * Adresses inférées depuis la doc (§4.1 tronqué) :
 *   0x0006 UA alim I, 0x0007 UB alim I, 0x0008 UC alim I
 *   0x0009 UA alim II, 0x000A UB alim II, 0x000B UC alim II
 *   0x000C Fréquence alim I (×0.01 Hz), 0x000D Fréquence alim II (×0.01 Hz)
 *
 * ⚠ Les adresses 0x0008-0x000D sont à VÉRIFIER sur l'appareil réel
 *   (utiliser le scan de registres pour confirmer)
 */
function interpretMeasures(regs) {
    return {
        ua1: regs[0],         // V — alimentation I phase A
        ub1: regs[1],         // V — alimentation I phase B
        uc1: regs[2],         // V — alimentation I phase C (à vérifier)
        ua2: regs[3],         // V — alimentation II phase A (à vérifier)
        ub2: regs[4],         // V — alimentation II phase B (à vérifier)
        uc2: regs[5],         // V — alimentation II phase C (à vérifier)
        freq1: regs[6] / 100, // Hz — fréquence alim I (à vérifier)
        freq2: regs[7] / 100, // Hz — fréquence alim II (à vérifier)
    };
}

/**
 * Registre 0x004F — État alimentation I & II (§4.3)
 * Les définitions exactes des bits sont dans §4.3 (non détaillé dans la doc).
 * Interprétation commune pour CHINT NXZBN :
 *   Byte haut (octet fort) = état alim II
 *   Byte bas  (octet faible) = état alim I
 *   Bit 0 : présence tension
 *   Bit 1 : sous-tension
 *   Bit 2 : surtension
 *   Bit 3 : défaut fréquence
 * ⚠ À VÉRIFIER sur l'appareil réel — brancher l'oscilloscope ou lire le PDF
 */
function interpretPowerState(raw) {
    const alim1 = raw & 0xFF;
    const alim2 = (raw >> 8) & 0xFF;
    return {
        raw,
        alim1: {
            present:      !!(alim1 & 0x01),
            undervoltage: !!(alim1 & 0x02),
            overvoltage:  !!(alim1 & 0x04),
            freqFault:    !!(alim1 & 0x08),
            ok: alim1 === 0x01,  // présente, pas de défaut
        },
        alim2: {
            present:      !!(alim2 & 0x01),
            undervoltage: !!(alim2 & 0x02),
            overvoltage:  !!(alim2 & 0x04),
            freqFault:    !!(alim2 & 0x08),
            ok: alim2 === 0x01,
        },
    };
}

/**
 * Registre 0x0050 — État commutateur (§4.4)
 * Valeurs communes pour NXZBN :
 *   0x0000 : Position I  (alim I active)
 *   0x0001 : Position 0  (double ouverture)
 *   0x0002 : Position II (alim II active)
 * ⚠ À VÉRIFIER — peut différer selon version firmware
 */
function interpretSwitchState(raw) {
    const labels = {
        0x0000: { pos: 'I',  label: 'Alimentation I',          color: 'green' },
        0x0001: { pos: '0',  label: 'Double ouverture',         color: 'orange' },
        0x0002: { pos: 'II', label: 'Alimentation II',          color: 'blue' },
    };
    return {
        raw,
        ...(labels[raw] || { pos: '?', label: `Inconnu (0x${raw.toString(16)})`, color: 'red' }),
    };
}

/**
 * Registre 0x0101 — Débit en bauds
 */
function interpretBaud(raw) {
    return [4800, 9600, 19200, 38400][raw] || raw;
}

// -----------------------------------------------------------------------------
// Trames de commande prêtes à l'emploi (bytes exacts du doc + CRC vérifiés)
// Référence : §4.5 et §4.6
// -----------------------------------------------------------------------------
const COMMANDS = {
    enterRemote:     Buffer.from([0x03, 0x06, 0x28, 0x00, 0x00, 0x04, 0x80, 0x4B]),
    exitRemote:      Buffer.from([0x03, 0x06, 0x28, 0x00, 0x00, 0x00, 0x81, 0x88]),
    restoreDefaults: Buffer.from([0x03, 0x06, 0x28, 0x00, 0x00, 0x02, 0x00, 0x49]),
    clearHistory:    Buffer.from([0x03, 0x06, 0x28, 0x00, 0x00, 0x01, 0x40, 0x48]),
    forceAlim1:      appendCrc(Buffer.from([0x03, 0x06, 0x27, 0x00, 0x00, 0x00])),
    forceAlim2:      appendCrc(Buffer.from([0x03, 0x06, 0x27, 0x00, 0x00, 0xAA])),
    forceOpen:       Buffer.from([0x03, 0x06, 0x27, 0x00, 0x00, 0xFF, 0xC2, 0xDC]),
};

// -----------------------------------------------------------------------------
// Export
// -----------------------------------------------------------------------------
module.exports = {
    crc16,
    checkCrc,
    buildReadFrame,
    buildWriteFrame,
    hexToFrame,
    parseReadResponse,
    parseWriteResponse,
    interpretMeasures,
    interpretPowerState,
    interpretSwitchState,
    interpretBaud,
    COMMANDS,

    // Adresses registres de référence
    REG: {
        UA1:            0x0006,
        MEASURES_START: 0x0006,
        MEASURES_COUNT: 8,
        PARITY:         0x000E,
        POWER_STATE:    0x004F,
        SWITCH_STATE:   0x0050,
        MODBUS_ADDR:    0x0100,
        BAUD_RATE:      0x0101,
        UV_THRESHOLD:   0x2065,
        MODE_SELECT:    0x206D,
        FORCE_TRANSFER: 0x2700,
        COMMAND:        0x2800,
    },
};
