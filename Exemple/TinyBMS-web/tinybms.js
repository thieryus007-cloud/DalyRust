const { SerialPort } = require('serialport');

// CARTE DES REGISTRES (Basée sur TinyBMS Communication Protocols Rev D)
// Cellules 1 à 16 (Reg 0-15)
const cellRegisters = Array.from({ length: 16 }, (_, i) => ({
    id: i, label: `Cell ${i + 1}`, unit: 'V', type: 'UINT16', scale: 0.0001, category: 'Live'
}));

const REGISTER_MAP = [
    // --- LIVE DATA (0-99) ---
    ...cellRegisters,
    { id: 36, label: 'Pack Voltage', unit: 'V', type: 'FLOAT', category: 'Live' },
    { id: 38, label: 'Pack Current', unit: 'A', type: 'FLOAT', category: 'Live' },
    { id: 40, label: 'Min Cell Voltage', unit: 'V', type: 'UINT16', scale: 0.001, category: 'Live' },
    { id: 41, label: 'Max Cell Voltage', unit: 'V', type: 'UINT16', scale: 0.001, category: 'Live' },
    { id: 42, label: 'Temp Sensor 1', unit: '°C', type: 'INT16', scale: 0.1, category: 'Live' },
    { id: 43, label: 'Temp Sensor 2', unit: '°C', type: 'INT16', scale: 0.1, category: 'Live' },
    { id: 45, label: 'State Of Health', unit: '%', type: 'UINT16', scale: 0.002, category: 'Stats' },
    { id: 46, label: 'State Of Charge', unit: '%', type: 'UINT32', scale: 0.000001, category: 'Live' },
    { id: 48, label: 'Internal Temp', unit: '°C', type: 'INT16', scale: 0.1, category: 'Live' },
    { id: 50, label: 'BMS Status', type: 'UINT16', category: 'Live' },
    { id: 52, label: 'Real Balancing', type: 'UINT16', category: 'Live' },

    // --- STATISTICS (100-199) ---
    { id: 100, label: 'Total Distance', unit: 'km', type: 'UINT32', scale: 0.01, category: 'Stats' },
    { id: 106, label: 'Over-Voltage Count', type: 'UINT16', category: 'Stats' },
    { id: 105, label: 'Under-Voltage Count', type: 'UINT16', category: 'Stats' },
    { id: 111, label: 'Charging Count', type: 'UINT16', category: 'Stats' },
    { id: 112, label: 'Full Charge Count', type: 'UINT16', category: 'Stats' },

    // --- SETTINGS (300-343) - Organisés par Groupes pour l'affichage ---
    
    // Groupe 1: Battery
    { id: 300, label: 'Fully Charged Voltage', unit: 'V', type: 'UINT16', scale: 0.001, category: 'Settings', group: 'battery' },
    { id: 301, label: 'Fully Discharged Voltage', unit: 'V', type: 'UINT16', scale: 0.001, category: 'Settings', group: 'battery' },
    { id: 306, label: 'Battery Capacity', unit: 'Ah', type: 'UINT16', scale: 0.01, category: 'Settings', group: 'battery' },
    { id: 307, label: 'Series Cells Count', unit: '', type: 'UINT16', category: 'Settings', group: 'battery' },
    { id: 322, label: 'Max Cycles Count', unit: '', type: 'UINT16', category: 'Settings', group: 'battery' },
    { id: 328, label: 'Manual SOC Set', unit: '%', type: 'UINT16', scale: 0.002, category: 'Settings', group: 'battery' },

    // Groupe 2: Safety
    { id: 315, label: 'Over-Voltage Cutoff', unit: 'V', type: 'UINT16', scale: 0.001, category: 'Settings', group: 'safety' },
    { id: 316, label: 'Under-Voltage Cutoff', unit: 'V', type: 'UINT16', scale: 0.001, category: 'Settings', group: 'safety' },
    { id: 317, label: 'Discharge Over-Current', unit: 'A', type: 'UINT16', scale: 1, category: 'Settings', group: 'safety' },
    { id: 318, label: 'Charge Over-Current', unit: 'A', type: 'UINT16', scale: 1, category: 'Settings', group: 'safety' },
    { id: 305, label: 'Peak Discharge Current', unit: 'A', type: 'UINT16', scale: 1, category: 'Settings', group: 'safety' },
    { id: 319, label: 'Over-Heat Cutoff', unit: '°C', type: 'INT16', scale: 1, category: 'Settings', group: 'safety' },
    { id: 320, label: 'Low Temp Charge Cutoff', unit: '°C', type: 'INT16', scale: 1, category: 'Settings', group: 'safety' },

    // Groupe 3: Balance
    { id: 303, label: 'Early Balancing Threshold', unit: 'V', type: 'UINT16', scale: 0.001, category: 'Settings', group: 'balance' },
    { id: 304, label: 'Charge Finished Current', unit: 'mA', type: 'UINT16', scale: 1, category: 'Settings', group: 'balance' },
    { id: 308, label: 'Allowed Disbalance', unit: 'mV', type: 'UINT16', scale: 1, category: 'Settings', group: 'balance' },
    { id: 321, label: 'Charge Restart Level', unit: '%', type: 'UINT16', category: 'Settings', group: 'balance' },
    { id: 332, label: 'Automatic Recovery', unit: 's', type: 'UINT16', category: 'Settings', group: 'balance' },

    // Groupe 4: Hardware
    { id: 310, label: 'Charger Startup Delay', unit: 's', type: 'UINT16', category: 'Settings', group: 'hardware' },
    { id: 311, label: 'Charger Disable Delay', unit: 's', type: 'UINT16', category: 'Settings', group: 'hardware' },
    { id: 312, label: 'Pulses Per Unit', unit: '', type: 'UINT32', category: 'Settings', group: 'hardware' },
    { id: 330, label: 'Charger Type', unit: '', type: 'UINT16', category: 'Settings', group: 'hardware' },
    { id: 340, label: 'Operation Mode', unit: '', type: 'UINT16', category: 'Settings', group: 'hardware' }, // 0=Dual, 1=Single
    { id: 343, label: 'Protocol', unit: '', type: 'UINT16', category: 'Settings', group: 'hardware' },

    // --- VERSION (500+) ---
    { id: 501, label: 'Firmware Version', type: 'UINT16', category: 'Version' }
];

class TinyBMS {
    constructor(path, baudRate = 115200) {
        this.path = path;
        this.baudRate = baudRate;
        this.port = null;
        this.isConnected = false;
    }

    async connect() {
        return new Promise((resolve, reject) => {
            this.port = new SerialPort({ path: this.path, baudRate: this.baudRate, autoOpen: false });
            this.port.open((err) => {
                if (err) return reject(err);
                this.isConnected = true;
                console.log(`Connected to TinyBMS on ${this.path}`);
                resolve();
            });
            this.port.on('error', (err) => {
                console.error('Serial Error:', err);
                this.isConnected = false;
            });
        });
    }

    /**
     * Configure le protocole de communication du TinyBMS
     * @param {number} protocolValue - 0 pour MODBUS (défaut), 1 pour ASCII
     * @returns {Promise<boolean>} true si la configuration a réussi
     */
    async setProtocol(protocolValue = 1) {
        if (!this.isConnected) {
            throw new Error("Cannot set protocol: not connected");
        }

        console.log(`Setting TinyBMS protocol to ${protocolValue === 1 ? 'ASCII' : 'MODBUS'}...`);

        try {
            const success = await this.writeRegister(343, protocolValue);
            if (success) {
                console.log(`Protocol successfully set to ${protocolValue === 1 ? 'ASCII' : 'MODBUS'}`);
                // Attendre un peu pour que le BMS applique le changement
                await new Promise(resolve => setTimeout(resolve, 500));
            } else {
                console.warn('Protocol write command sent but no confirmation received');
            }
            return success;
        } catch (error) {
            console.error('Failed to set protocol:', error.message);
            throw error;
        }
    }

    // Calcul CRC Modbus (Poly 0xA001)
    calculateCRC(buffer) {
        let crc = 0xFFFF;
        for (let pos = 0; pos < buffer.length; pos++) {
            crc ^= buffer[pos];
            for (let i = 8; i !== 0; i--) {
                if ((crc & 0x0001) !== 0) {
                    crc >>= 1;
                    crc ^= 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        return crc;
    }

    // Lecture par bloc (Fonction 0x03)
    // Note: IMPORTANT - Configuration byte order pour TinyBMS:
    // - ADRESSES: Little Endian (LSB first, MSB second)
    // - DONNÉES: Little Endian (LSB first, MSB second) - confirmé par test_tinybms.py
    // - CRC: Little Endian (LSB first, MSB second)
    readRegisterBlock(startAddr, count) {
        return new Promise((resolve, reject) => {
            if (!this.isConnected) return reject(new Error("Not connected"));

            // Vider le buffer série et attendre que ce soit terminé
            this.port.flush(async (err) => {
                if (err) {
                    console.warn('[TinyBMS] Flush error:', err.message);
                }

                // Attendre un peu après le flush pour s'assurer que le buffer est bien vidé
                await new Promise(r => setTimeout(r, 100));

                const cmd = [0xAA, 0x03, startAddr & 0xFF, (startAddr >> 8) & 0xFF, 0x00, count & 0xFF]; // Address LSB, MSB (Little Endian)
                const crc = this.calculateCRC(Buffer.from(cmd));
                const finalBuf = Buffer.from([...cmd, crc & 0xFF, (crc >> 8) & 0xFF]);

                let rxBuffer = Buffer.alloc(0);

                const onData = (chunk) => {
                    console.log(`[TinyBMS] Received ${chunk.length} bytes (total: ${rxBuffer.length + chunk.length})`);

                    // Accumuler les données reçues
                    rxBuffer = Buffer.concat([rxBuffer, chunk]);

                    // Vérifier si on a au moins le header
                    if (rxBuffer.length < 3) return;

                    // Vérification Header AA 03
                    if (rxBuffer[0] !== 0xAA || rxBuffer[1] !== 0x03) {
                        console.log(`[TinyBMS] Invalid header: ${rxBuffer[0].toString(16)} ${rxBuffer[1].toString(16)}`);
                        console.log(`[TinyBMS] Full buffer: ${rxBuffer.toString('hex')}`);
                        this.port.removeListener('data', onData);
                        reject(new Error("Invalid header"));
                        return;
                    }

                    const len = rxBuffer[2];
                    const expectedLen = 3 + len + 2;

                    console.log(`[TinyBMS] Frame progress: ${rxBuffer.length}/${expectedLen} bytes`);

                    if (rxBuffer.length < expectedLen) {
                        return; // Attendre plus de données
                    }

                    const payload = rxBuffer.slice(3, 3 + len);
                    this.port.removeListener('data', onData);
                    console.log(`[TinyBMS] Read successful, ${len} bytes payload, full frame: ${rxBuffer.slice(0, expectedLen).toString('hex')}`);
                    resolve(this.parseBlock(startAddr, payload));
                };

                this.port.on('data', onData);
                console.log(`[TinyBMS] Sending read command: addr=${startAddr}, count=${count}, bytes=${finalBuf.toString('hex')}`);
                this.port.write(finalBuf);

                setTimeout(() => {
                    this.port.removeListener('data', onData);
                    reject(new Error("Timeout Read"));
                }, 2000); // Augmenté de 800ms à 2000ms pour laisser plus de temps au BMS
            });
        });
    }

    // Ecriture (Fonction 0x0D - Write Individual Registers)
    // Format: AA 0D PL AddrLSB AddrMSB DataLSB DataMSB CRC_LSB CRC_MSB
    // Note: Adresse ET Données en Little Endian (confirmé par test_tinybms.py)
    writeRegister(regId, value) {
        return new Promise((resolve, reject) => {
            if (!this.isConnected) return reject(new Error("Not connected"));

            const def = REGISTER_MAP.find(r => r.id === regId);
            if (!def) return reject(new Error("Unknown register ID"));

            let rawValue = value;
            if (def.scale) rawValue = Math.round(value / def.scale);

            // Vider le buffer série AVANT l'écriture (comme test_tinybms.py)
            this.port.flush(async (err) => {
                if (err) {
                    console.warn('[TinyBMS] Flush error before write:', err.message);
                }

                // Attendre un peu après le flush
                await new Promise(r => setTimeout(r, 100));

                // Préparation données (2 octets) - Little Endian (LSB, MSB) comme test_tinybms.py
                const dataBytes = Buffer.alloc(2);
                if (def.type === 'INT16') dataBytes.writeInt16LE(rawValue);
                else dataBytes.writeUInt16LE(rawValue);

                // Header: AA 0D PL AddrLSB AddrMSB (Write Individual - comme test_tinybms.py)
                // PL = 0x04 (4 bytes: 2 pour l'adresse + 2 pour les données)
                const header = [0xAA, 0x0D, 0x04, regId & 0xFF, (regId >> 8) & 0xFF]; // Command 0x0D, Payload Length, Address LE
                const cmdNoCrc = Buffer.concat([Buffer.from(header), dataBytes]);
                const crc = this.calculateCRC(cmdNoCrc);
                const finalBuf = Buffer.concat([cmdNoCrc, Buffer.from([crc & 0xFF, (crc >> 8) & 0xFF])]);

                let rxBuffer = Buffer.alloc(0);

                const onData = (chunk) => {
                    console.log(`[TinyBMS] Write response chunk: ${chunk.length} bytes - hex: ${chunk.toString('hex')}`);

                    // Accumuler les données reçues
                    rxBuffer = Buffer.concat([rxBuffer, chunk]);

                    console.log(`[TinyBMS] Write accumulated buffer (${rxBuffer.length} bytes): ${rxBuffer.toString('hex')}`);

                    // Attendre au moins 5 bytes (taille minimale réponse ACK/NACK)
                    if (rxBuffer.length < 5) {
                        console.log(`[TinyBMS] Waiting for more data... (need at least 5 bytes)`);
                        return;
                    }

                    console.log(`[TinyBMS] Write full response: ${rxBuffer.toString('hex')}`);

                    // Réponse pour commande 0x0D: AA 01 0D (ACK) ou AA 00 0D ERROR (NACK)
                    if (rxBuffer[0] === 0xAA) {
                        this.port.removeListener('data', onData);

                        if (rxBuffer[1] === 0x01) {
                            // ACK - Vérifier que c'est bien pour la commande 0x0D
                            if (rxBuffer[2] === 0x0D) {
                                console.log(`[TinyBMS] ✅ Write ACK received for register ${regId}`);
                                resolve(true);
                            } else {
                                console.warn(`[TinyBMS] ACK received but for wrong command: 0x${rxBuffer[2].toString(16)}`);
                                resolve(true); // Accepter quand même
                            }
                        } else if (rxBuffer[1] === 0x00) {
                            // NACK - Le code d'erreur est à l'index 3
                            const errorCode = rxBuffer[3] || 0xFF;
                            const errorMessages = {
                                0x00: "Command error",
                                0x01: "CRC error",
                                0x02: "Invalid register address",
                                0x03: "Read-only register",
                                0x04: "Value out of range"
                            };
                            const errorMsg = errorMessages[errorCode] || "Unknown error";
                            console.error(`[TinyBMS] ❌ Write NACK for register ${regId}, error: ${errorMsg} (code: 0x${errorCode.toString(16)})`);
                            reject(new Error(`Write failed: ${errorMsg}`));
                        } else {
                            reject(new Error(`Invalid write response: ${rxBuffer.toString('hex')}`));
                        }
                    } else {
                        this.port.removeListener('data', onData);
                        reject(new Error(`Invalid write response header: ${rxBuffer.toString('hex')}`));
                    }
                };

                this.port.on('data', onData);
                console.log(`[TinyBMS] Sending write command: reg=${regId}, value=${rawValue}, bytes=${finalBuf.toString('hex')}`);
                this.port.write(finalBuf);

                setTimeout(() => {
                    this.port.removeListener('data', onData);
                    resolve(false);
                }, 2000); // Augmenté de 800ms à 2000ms
            });
        });
    }

    // Décodage générique selon la map
    parseBlock(startAddr, buffer) {
        const result = {};
        const count = buffer.length / 2; // 2 octets par registre

        for (let i = 0; i < count; i++) {
            const currentRegId = startAddr + i;
            const def = REGISTER_MAP.find(r => r.id === currentRegId);

            if (def) {
                let rawValue = 0;
                let byteOffset = i * 2;

                // DEBUG: Log les octets bruts pour le registre 306 (Battery Capacity)
                if (currentRegId === 306) {
                    const byte0 = buffer[byteOffset];
                    const byte1 = buffer[byteOffset + 1];
                    console.log(`[DEBUG] Reg 306 Battery Capacity - Raw bytes: [${byte0.toString(16).padStart(2, '0')}, ${byte1.toString(16).padStart(2, '0')}]`);
                    console.log(`[DEBUG] Reg 306 - If read as BE: ${buffer.readUInt16BE(byteOffset)} (0x${buffer.readUInt16BE(byteOffset).toString(16)})`);
                    console.log(`[DEBUG] Reg 306 - If read as LE: ${buffer.readUInt16LE(byteOffset)} (0x${buffer.readUInt16LE(byteOffset).toString(16)})`);
                }

                if (def.type === 'FLOAT') {
                    if (byteOffset + 4 <= buffer.length) rawValue = buffer.readFloatLE(byteOffset);
                } else if (def.type === 'UINT32') {
                    if (byteOffset + 4 <= buffer.length) rawValue = buffer.readUInt32LE(byteOffset);
                } else if (def.type === 'INT16') {
                    rawValue = buffer.readInt16LE(byteOffset);
                } else {
                    rawValue = buffer.readUInt16LE(byteOffset);
                }

                let finalValue = rawValue;
                if (def.scale) finalValue = rawValue * def.scale;

                // DEBUG: Log les valeurs pour le registre 306
                if (currentRegId === 306) {
                    console.log(`[DEBUG] Reg 306 - rawValue (LE): ${rawValue}, finalValue (after scale ${def.scale}): ${finalValue}`);
                }

                // On nettoie les flottants (ex: 3.90000001 -> 3.9)
                result[currentRegId] = {
                    id: def.id,
                    label: def.label,
                    value: parseFloat(finalValue.toFixed(4)),
                    unit: def.unit || '',
                    category: def.category,
                    group: def.group // Essentiel pour l'affichage par onglets
                };
            }
        }
        return result;
    }
}

module.exports = TinyBMS;
