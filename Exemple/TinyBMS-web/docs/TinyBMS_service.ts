import { TinyBmsRegisterDef } from "../types";

// -- CRC16-MODBUS Implementation (Poly 0x8005, Reversed 0xA001) --
const CRC_TABLE = new Uint16Array([
    0x0000, 0xC0C1, 0xC181, 0x0140, 0xC301, 0x03C0, 0x0280, 0xC241,
    0xC601, 0x06C0, 0x0780, 0xC741, 0x0500, 0xC5C1, 0xC481, 0x0440,
    0xCC01, 0x0CC0, 0x0D80, 0xCD41, 0x0F00, 0xCFC1, 0xCE81, 0x0E40,
    0x0A00, 0xCAC1, 0xCB81, 0x0B40, 0xC901, 0x09C0, 0x0880, 0xC841,
    0xD801, 0x18C0, 0x1980, 0xD941, 0x1B00, 0xDBC1, 0xDA81, 0x1A40,
    0x1E00, 0xDEC1, 0xDF81, 0x1F40, 0xDD01, 0x1DC0, 0x1C80, 0xDC41,
    0x1400, 0xD4C1, 0xD581, 0x1540, 0xD701, 0x17C0, 0x1680, 0xD641,
    0xD201, 0x12C0, 0x1380, 0xD341, 0x1100, 0xD1C1, 0xD081, 0x1040,
    0xF001, 0x30C0, 0x3180, 0xF141, 0x3300, 0xF3C1, 0xF281, 0x3240,
    0x3600, 0xF6C1, 0xF781, 0x3740, 0xF501, 0x35C0, 0x3480, 0xF441,
    0x3C00, 0xFCC1, 0xFD81, 0x3D40, 0xFF01, 0x3FC0, 0x3E80, 0xFE41,
    0xFA01, 0x3AC0, 0x3B80, 0xFB41, 0x3900, 0xF9C1, 0xF881, 0x3840,
    0x2800, 0xE8C1, 0xE981, 0x2940, 0xEB01, 0x2BC0, 0x2A80, 0xEA41,
    0xEE01, 0x2EC0, 0x2F80, 0xEF41, 0x2D00, 0xEDC1, 0xEC81, 0x2C40,
    0xE401, 0x24C0, 0x2580, 0xE541, 0x2700, 0xE7C1, 0xE681, 0x2640,
    0x2200, 0xE2C1, 0xE381, 0x2340, 0xE101, 0x21C0, 0x2080, 0xE041,
    0xA001, 0x60C0, 0x6180, 0xA141, 0x6300, 0xA3C1, 0xA281, 0x6240,
    0x6600, 0xA6C1, 0xA781, 0x6740, 0xA501, 0x65C0, 0x6480, 0xA441,
    0x6C00, 0xACC1, 0xAD81, 0x6D40, 0xAF01, 0x6FC0, 0x6E80, 0xAE41,
    0xAA01, 0x6AC0, 0x6B80, 0xAB41, 0x6900, 0xA9C1, 0xA881, 0x6840,
    0x7800, 0xB8C1, 0xB981, 0x7940, 0xBB01, 0x7BC0, 0x7A80, 0xBA41,
    0xBE01, 0x7EC0, 0x7F80, 0xBF41, 0x7D00, 0xBDC1, 0xBC81, 0x7C40,
    0xB401, 0x74C0, 0x7580, 0xB541, 0x7700, 0xB7C1, 0xB681, 0x7640,
    0x7200, 0xB2C1, 0xB381, 0x7340, 0xB101, 0x71C0, 0x7080, 0xB041,
    0x5000, 0x90C1, 0x9181, 0x5140, 0x9301, 0x53C0, 0x5280, 0x9241,
    0x9601, 0x56C0, 0x5780, 0x9741, 0x5500, 0x95C1, 0x9481, 0x5440,
    0x9C01, 0x5CC0, 0x5D80, 0x9D41, 0x5F00, 0x9FC1, 0x9E81, 0x5E40,
    0x5A00, 0x9AC1, 0x9B81, 0x5B40, 0x9901, 0x59C0, 0x5880, 0x9841,
    0x8801, 0x48C0, 0x4980, 0x8941, 0x4B00, 0x8BC1, 0x8A81, 0x4A40,
    0x4E00, 0x8EC1, 0x8F81, 0x4F40, 0x8D01, 0x4DC0, 0x4C80, 0x8C41,
    0x4400, 0x84C1, 0x8581, 0x4540, 0x8701, 0x47C0, 0x4680, 0x8641,
    0x8201, 0x42C0, 0x4380, 0x8341, 0x4100, 0x81C1, 0x8081, 0x4040
]);

function calculateCRC(data: Uint8Array | number[]): number {
    let crc = 0xFFFF;
    for (let i = 0; i < data.length; i++) {
        const tmp = (data[i] ^ crc) & 0xFF;
        crc = (crc >> 8) ^ CRC_TABLE[tmp];
    }
    return crc;
}

// -- Commands --

// Protocol: AA 03 ADDR_MSB ADDR_LSB 00 COUNT CRC_L CRC_H (Read Block Modbus)
// Note: Address uses Big Endian (MSB first, LSB second) per TinyBMS documentation Rev D
export function buildReadRegisterCommand(startAddr: number, count: number): Uint8Array {
    const buf = [
        0xAA,
        0x03,
        (startAddr >> 8) & 0xFF, // Address MSB (Big Endian)
        startAddr & 0xFF,        // Address LSB
        0x00,
        count & 0xFF
    ];
    const crc = calculateCRC(buf);
    buf.push(crc & 0xFF);
    buf.push((crc >> 8) & 0xFF);
    return new Uint8Array(buf);
}

// Protocol: AA 10 ADDR_MSB ADDR_LSB 00 01 02 DATA_MSB DATA_LSB CRC_L CRC_H (Write 1 Register Modbus)
// Note: Address uses Big Endian (MSB, LSB) per TinyBMS documentation Rev D
// Note: Data uses Big Endian (MSB, LSB) per MODBUS standard
export function buildWriteRegisterCommand(addr: number, value: number): Uint8Array {
    const buf = [
        0xAA,
        0x10,
        (addr >> 8) & 0xFF,  // Address MSB (Big Endian)
        addr & 0xFF,         // Address LSB
        0x00,
        0x01,                // Quantity of registers (1)
        0x02,                // Byte count (2)
        (value >> 8) & 0xFF, // Data MSB (Big Endian for MODBUS)
        value & 0xFF         // Data LSB
    ];
    const crc = calculateCRC(buf);
    buf.push(crc & 0xFF);
    buf.push((crc >> 8) & 0xFF);
    return new Uint8Array(buf);
}

// -- Parsing --

export interface ParseResult {
    valid: boolean;
    cmd?: number;
    startAddr?: number;
    // We need to infer this from context/queue usually, but simple parsing returns data
    data?: Uint8Array;
    consumed: number;
}

export function parseFrame(buffer: Uint8Array): ParseResult {
    // Min frame size is ~6 bytes (ACK) to ~N bytes.
    if (buffer.length < 6) return { valid: false, consumed: 0 };

    if (buffer[0] !== 0xAA) {
        // Scan for start byte
        const startIdx = buffer.indexOf(0xAA);
        if (startIdx === -1) return { valid: false, consumed: buffer.length }; // Trash all
        return { valid: false, consumed: startIdx }; // Trash up to AA
    }

    const cmd = buffer[1];
    
    // Modbus Read Response (0x03)
    // Structure: AA 03 LENGTH [DATA...] CRC_L CRC_H
    if (cmd === 0x03) {
        const payloadLen = buffer[2];
        const totalLen = 3 + payloadLen + 2; // Header(2) + Len(1) + Payload + CRC(2)
        
        if (buffer.length < totalLen) return { valid: false, consumed: 0 }; // Wait for more data

        // Check CRC
        const frameData = buffer.slice(0, totalLen - 2);
        const receivedCrc = buffer[totalLen - 2] | (buffer[totalLen - 1] << 8);
        const calcCrc = calculateCRC(frameData);

        if (receivedCrc !== calcCrc) {
            console.warn("CRC Error", receivedCrc.toString(16), calcCrc.toString(16));
            return { valid: false, consumed: 1 }; // Invalid frame, skip start byte and retry
        }

        return { 
            valid: true, 
            cmd: cmd, 
            data: buffer.slice(3, 3 + payloadLen), 
            consumed: totalLen 
        };
    }
    
    // Write response (0x10) 
    // Structure: AA 10 ADDR_H ADDR_L 00 CNT CRC_L CRC_H (8 bytes)
    if (cmd === 0x10) {
        if (buffer.length < 8) return { valid: false, consumed: 0 };
        const totalLen = 8;
        const frameData = buffer.slice(0, totalLen - 2);
        const receivedCrc = buffer[totalLen - 2] | (buffer[totalLen - 1] << 8);
        
        if (calculateCRC(frameData) === receivedCrc) {
             return { valid: true, cmd: cmd, consumed: totalLen };
        }
    }

    // Fallback: If buffer is huge and we haven't matched, move forward 1 byte to retry
    if (buffer.length > 300) return { valid: false, consumed: 1 };
    return { valid: false, consumed: 0 };
}

// -- Register Map (Based on Rev D PDF) --

// Helper to generate cell registers 0-15
const cellRegisters: TinyBmsRegisterDef[] = Array.from({ length: 16 }, (_, i) => ({
    id: i,
    label: `Cell ${i + 1} Voltage`,
    unit: 'V',
    type: 'UINT16',
    access: 'R',
    [cite_start]scale: 0.0001, // 0.1mV resolution (e.g., 38720 -> 3.8720 V) [cite: 32, 702]
    category: 'Live'
}));

export const TINY_BMS_REGISTERS: TinyBmsRegisterDef[] = [
    [cite_start]// -- Live Data (Reg 0-99) [cite: 702] --
    ...cellRegisters,
    // 16-31 Reserved
    { id: 32, label: 'Lifetime Counter', unit: 's', type: 'UINT32', access: 'R', category: 'Stats' },
    { id: 34, label: 'Estimated Time Left', unit: 's', type: 'UINT32', access: 'R', category: 'Stats' },
    { id: 36, label: 'Pack Voltage', unit: 'V', type: 'FLOAT', access: 'R', category: 'Live' },
    { id: 38, label: 'Pack Current', unit: 'A', type: 'FLOAT', access: 'R', category: 'Live' },
    { id: 40, label: 'Min Cell Voltage', unit: 'V', type: 'UINT16', access: 'R', scale: 0.001, category: 'Live' }, // 1mV res
    { id: 41, label: 'Max Cell Voltage', unit: 'V', type: 'UINT16', access: 'R', scale: 0.001, category: 'Live' }, // 1mV res
    { id: 42, label: 'Ext Temp Sensor 1', unit: '°C', type: 'INT16', access: 'R', scale: 0.1, category: 'Live' },
    { id: 43, label: 'Ext Temp Sensor 2', unit: '°C', type: 'INT16', access: 'R', scale: 0.1, category: 'Live' },
    { id: 44, label: 'Distance Left', unit: 'km', type: 'UINT16', access: 'R', scale: 1, category: 'Stats' },
    { id: 45, label: 'State Of Health', unit: '%', type: 'UINT16', access: 'R', scale: 0.002, category: 'Stats' }, // 0.002% res
    { id: 46, label: 'State Of Charge', unit: '%', type: 'UINT32', access: 'R', scale: 0.000001, category: 'Live' }, // High res
    { id: 48, label: 'Internal Temp', unit: '°C', type: 'INT16', access: 'R', scale: 0.1, category: 'Live' },
    // 49 Reserved
    { id: 50, label: 'BMS Status', type: 'UINT16', access: 'R', category: 'Live' }, // Enum: 0x91 Charging, 0x92 Full, 0x93 Discharging, etc.
    { id: 51, label: 'Balancing Decision', type: 'UINT16', access: 'R', category: 'Live' }, // Bitmask
    { id: 52, label: 'Real Balancing', type: 'UINT16', access: 'R', category: 'Live' }, // Bitmask
    { id: 53, label: 'Detected Cells Count', type: 'UINT16', access: 'R', category: 'Live' },
    { id: 54, label: 'Speed', unit: 'km/h', type: 'FLOAT', access: 'R', category: 'Stats' },

    [cite_start]// -- Statistics Data (Reg 100-199) [cite: 703, 709] --
    // 100 Reserved in Rev D
    { id: 101, label: 'Total Distance', unit: 'km', type: 'UINT32', access: 'R', scale: 0.01, category: 'Stats' },
    { id: 102, label: 'Max Discharge Current', unit: 'A', type: 'UINT16', access: 'R', scale: 0.1, category: 'Stats' }, // 100mA res
    { id: 103, label: 'Max Charge Current', unit: 'A', type: 'UINT16', access: 'R', scale: 0.1, category: 'Stats' }, // 100mA res
    { id: 104, label: 'Max Cell Diff', unit: 'V', type: 'UINT16', access: 'R', scale: 0.0001, category: 'Stats' }, // 0.1mV res
    { id: 105, label: 'Under-Voltage Count', type: 'UINT16', access: 'R', category: 'Stats' },
    { id: 106, label: 'Over-Voltage Count', type: 'UINT16', access: 'R', category: 'Stats' },
    { id: 107, label: 'Discharge Over-Current Count', type: 'UINT16', access: 'R', category: 'Stats' },
    { id: 108, label: 'Charge Over-Current Count', type: 'UINT16', access: 'R', category: 'Stats' },
    { id: 109, label: 'Over-Heat Count', type: 'UINT16', access: 'R', category: 'Stats' },
    // 110 Reserved
    { id: 111, label: 'Charging Count', type: 'UINT16', access: 'R', category: 'Stats' },
    { id: 112, label: 'Full Charge Count', type: 'UINT16', access: 'R', category: 'Stats' },
    { id: 113, label: 'Min/Max Pack Temp', type: 'UINT16', access: 'R', category: 'Stats' }, // Packed: LSB=Min(Int8), MSB=Max(Int8)
    { id: 114, label: 'Last Reset/Wakeup Event', type: 'UINT16', access: 'R', category: 'Stats' }, // Packed: LSB=ResetID, MSB=WakeupID
    // 115 Reserved
    { id: 116, label: 'Stats Last Cleared Timestamp', unit: 's', type: 'UINT32', access: 'R', category: 'Stats' },

    [cite_start]// -- Settings (Reg 300-399) [cite: 715, 721] --
    { id: 300, label: 'Fully Charged Voltage', unit: 'V', type: 'UINT16', access: 'R/W', scale: 0.001, min: 1.2, max: 4.5, category: 'Settings' },
    { id: 301, label: 'Fully Discharged Voltage', unit: 'V', type: 'UINT16', access: 'R/W', scale: 0.001, min: 1.0, max: 3.5, category: 'Settings' },
    // 302 Reserved
    { id: 303, label: 'Early Balancing Threshold', unit: 'V', type: 'UINT16', access: 'R/W', scale: 0.001, category: 'Settings' },
    { id: 304, label: 'Charge Finished Current', unit: 'mA', type: 'UINT16', access: 'R/W', scale: 1, category: 'Settings' }, // 1mA res
    { id: 305, label: 'Peak Discharge Current', unit: 'A', type: 'UINT16', access: 'R/W', scale: 1, category: 'Settings' },
    { id: 306, label: 'Battery Capacity', unit: 'Ah', type: 'UINT16', access: 'R/W', scale: 0.01, category: 'Settings' },
    { id: 307, label: 'Number Of Series Cells', type: 'UINT16', access: 'R/W', min: 4, max: 16, category: 'Settings' },
    { id: 308, label: 'Allowed Disbalance', unit: 'mV', type: 'UINT16', access: 'R/W', scale: 1, category: 'Settings' },
    // 309 Reserved
    { id: 310, label: 'Charger Startup Delay', unit: 's', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 311, label: 'Charger Disable Delay', unit: 's', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 312, label: 'Pulses Per Unit', type: 'UINT32', access: 'R/W', category: 'Settings' },
    // 313 Reserved
    { id: 314, label: 'Distance Unit Name', type: 'UINT16', access: 'R/W', category: 'Settings' }, // 1=m, 2=km, 3=ft, 4=mi, 5=yd
    { id: 315, label: 'Over-Voltage Cutoff', unit: 'V', type: 'UINT16', access: 'R/W', scale: 0.001, category: 'Settings' },
    { id: 316, label: 'Under-Voltage Cutoff', unit: 'V', type: 'UINT16', access: 'R/W', scale: 0.001, category: 'Settings' },
    { id: 317, label: 'Discharge Over-Current', unit: 'A', type: 'UINT16', access: 'R/W', scale: 1, category: 'Settings' },
    { id: 318, label: 'Charge Over-Current', unit: 'A', type: 'UINT16', access: 'R/W', scale: 1, category: 'Settings' },
    { id: 319, label: 'Over-Heat Cutoff', unit: '°C', type: 'INT16', access: 'R/W', scale: 1, category: 'Settings' },
    { id: 320, label: 'Low Temp Charger Cutoff', unit: '°C', type: 'INT16', access: 'R/W', scale: 1, category: 'Settings' },
    { id: 321, label: 'Charge Restart Level', unit: '%', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 322, label: 'Battery Max Cycles', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 323, label: 'SOH Setting', unit: '%', type: 'UINT16', access: 'R/W', scale: 0.002, category: 'Settings' },
    // 324-327 Reserved
    { id: 328, label: 'SOC Setting', unit: '%', type: 'UINT16', access: 'R/W', scale: 0.002, category: 'Settings' },
    { id: 329, label: 'Configuration Bits', type: 'UINT16', access: 'R/W', category: 'Settings' }, // Inv Current, Disable Load/Chg, Switch Diag, En Chg Restart
    { id: 330, label: 'Charger Type / Dischg Timeout', type: 'UINT16', access: 'R/W', category: 'Settings' }, // Packed LSB/MSB
    { id: 331, label: 'Load Switch Type', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 332, label: 'Automatic Recovery', unit: 's', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 333, label: 'Charger Switch Type', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 334, label: 'Ignition Input', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 335, label: 'Charger Detection Input', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 336, label: 'Speed Sensor Input', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 337, label: 'Precharge Pin', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 338, label: 'Precharge Duration', type: 'UINT16', access: 'R/W', category: 'Settings' }, // Enum 0-7
    { id: 339, label: 'Temp Sensor Type', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 340, label: 'BMS Operation Mode', type: 'UINT16', access: 'R/W', category: 'Settings' }, // 0=Dual Port, 1=Single Port
    { id: 341, label: 'Single Port Switch Type', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 342, label: 'Broadcast Time', type: 'UINT16', access: 'R/W', category: 'Settings' },
    { id: 343, label: 'Protocol', type: 'UINT16', access: 'R/W', category: 'Settings' },
    
    [cite_start]// -- Version Info (Reg 500+) [cite: 724] --
    { id: 500, label: 'Hardware Version', type: 'UINT16', access: 'R', category: 'Version' }, // LSB=HW, MSB=HW Changes
    { id: 501, label: 'Public Firmware Ver', type: 'UINT16', access: 'R', category: 'Version' }, // LSB=Ver, MSB=BPT/BCS
    { id: 502, label: 'Internal Firmware Ver', type: 'UINT16', access: 'R', category: 'Version' },
    { id: 503, label: 'Bootloader Version', type: 'UINT16', access: 'R', category: 'Version' }, // LSB=Bootloader, MSB=Profile
    { id: 506, label: 'Serial Number Start', type: 'UINT16', access: 'R', category: 'Version' }, // Starts at 506 (96 bits)
];

// Helper to get polling commands to refresh the UI
// We use blocks to minimize traffic.
export const getPollCommands = (): { start: number, count: number }[] => [
    { start: 0, count: 57 },   // Live Data (0 - 56)
    { start: 101, count: 17 }, // Statistics (101 - 117)
    { start: 300, count: 44 }  // Settings (300 - 343)
];
