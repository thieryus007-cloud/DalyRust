'use strict';

// =============================================================================
// CHINT NXZBN-63S/2DT ATS — Serveur Node.js
// Modbus RTU sur RS485, COM5 (Windows) ou /dev/ttyUSB0 (Pi5/Linux)
// HTTP + WebSocket → dashboard temps réel
// =============================================================================

const express    = require('express');
const http       = require('http');
const { WebSocketServer } = require('ws');
const { SerialPort }      = require('serialport');
const path       = require('path');
const modbus     = require('./modbus');

// ---------------------------------------------------------------------------
// Configuration par défaut (modifiable via l'interface web)
// ---------------------------------------------------------------------------
const DEFAULT_CONFIG = {
    port:        'COM5',     // Windows : COM5, Linux Pi5 : /dev/ttyUSB0
    baudRate:    9600,
    parity:      'even',
    dataBits:    8,
    stopBits:    1,
    slaveAddr:   3,
    pollIntervalMs: 1000,    // intervalle polling automatique
};

// ---------------------------------------------------------------------------
// État global de l'application
// ---------------------------------------------------------------------------
let serial       = null;   // instance SerialPort active
let pollTimer    = null;   // setInterval polling
let config       = { ...DEFAULT_CONFIG };
let rxBuffer     = Buffer.alloc(0);
let pendingReq   = null;   // { resolve, reject, expectedLen, timer }
let requestQueue = [];     // file de requêtes en attente
let processing   = false;  // verrou half-duplex

// ---------------------------------------------------------------------------
// Express + WebSocket
// ---------------------------------------------------------------------------
const app    = express();
const server = http.createServer(app);
const wss    = new WebSocketServer({ server });

app.use(express.json());
app.use(express.static(path.join(__dirname, 'public')));

// ---------------------------------------------------------------------------
// WebSocket — broadcast vers tous les clients connectés
// ---------------------------------------------------------------------------
function broadcast(type, data) {
    const msg = JSON.stringify({ type, data, ts: Date.now() });
    wss.clients.forEach(ws => {
        if (ws.readyState === ws.OPEN) ws.send(msg);
    });
}

function broadcastLog(direction, hexStr, decoded) {
    broadcast('frame', { direction, hex: hexStr, decoded });
}

function broadcastState(state) {
    broadcast('state', state);
}

function broadcastError(msg) {
    broadcast('error', { message: msg });
}

// ---------------------------------------------------------------------------
// Serialport — gestion de la connexion
// ---------------------------------------------------------------------------
function bufToHex(buf) {
    return Array.from(buf).map(b => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
}

function openPort(cfg) {
    return new Promise((resolve, reject) => {
        if (serial && serial.isOpen) {
            serial.close(() => {});
        }
        serial = new SerialPort({
            path:     cfg.port,
            baudRate: cfg.baudRate,
            parity:   cfg.parity,
            dataBits: cfg.dataBits,
            stopBits: cfg.stopBits,
            autoOpen: false,
        });

        serial.on('data', onData);
        serial.on('error', e => broadcastError('Erreur port série : ' + e.message));
        serial.on('close', () => {
            broadcast('status', { connected: false });
            stopPolling();
        });

        serial.open(err => {
            if (err) return reject(err);
            broadcast('status', { connected: true, config: cfg });
            resolve();
        });
    });
}

function closePort() {
    stopPolling();
    if (serial && serial.isOpen) serial.close();
    serial = null;
    broadcast('status', { connected: false });
}

// ---------------------------------------------------------------------------
// Réception des données série
// ---------------------------------------------------------------------------
function onData(chunk) {
    rxBuffer = Buffer.concat([rxBuffer, chunk]);

    if (!pendingReq) {
        rxBuffer = Buffer.alloc(0);
        return;
    }

    if (rxBuffer.length >= pendingReq.expectedLen) {
        const frame = rxBuffer.slice(0, pendingReq.expectedLen);
        rxBuffer = rxBuffer.slice(pendingReq.expectedLen);
        clearTimeout(pendingReq.timer);
        const req = pendingReq;
        pendingReq = null;
        broadcastLog('RX', bufToHex(frame), null);
        req.resolve(frame);
        processNextRequest();
    }
}

// ---------------------------------------------------------------------------
// File de requêtes série (half-duplex : une à la fois)
// ---------------------------------------------------------------------------
function sendRequest(frame, expectedResponseLen, timeoutMs = 500) {
    return new Promise((resolve, reject) => {
        requestQueue.push({ frame, expectedResponseLen, timeoutMs, resolve, reject });
        if (!processing) processNextRequest();
    });
}

function processNextRequest() {
    if (requestQueue.length === 0) { processing = false; return; }
    if (!serial || !serial.isOpen)  { processing = false; return; }

    processing = true;
    const req = requestQueue.shift();
    rxBuffer   = Buffer.alloc(0);

    pendingReq = {
        expectedLen: req.expectedResponseLen,
        resolve: req.resolve,
        reject:  req.reject,
        timer:   setTimeout(() => {
            pendingReq = null;
            req.reject(new Error('Timeout RS485'));
            processNextRequest();
        }, req.timeoutMs),
    };

    broadcastLog('TX', bufToHex(req.frame), null);
    serial.write(req.frame);
}

// ---------------------------------------------------------------------------
// Longueur de réponse attendue
// ---------------------------------------------------------------------------
function readResponseLen(count) {
    return 3 + count * 2 + 2; // addr(1) + fc(1) + byteCount(1) + data(n*2) + crc(2)
}
const WRITE_RESPONSE_LEN = 8; // addr(1) + fc(1) + reg(2) + val(2) + crc(2)

// ---------------------------------------------------------------------------
// Fonctions de lecture Modbus
// ---------------------------------------------------------------------------
async function readRegisters(startReg, count) {
    const frame = modbus.buildReadFrame(config.slaveAddr, startReg, count);
    const resp  = await sendRequest(frame, readResponseLen(count));
    return modbus.parseReadResponse(resp, count);
}

async function writeRegister(reg, value) {
    const frame = modbus.buildWriteFrame(config.slaveAddr, reg, value);
    const resp  = await sendRequest(frame, WRITE_RESPONSE_LEN);
    return modbus.parseWriteResponse(resp);
}

async function sendRawCommand(cmdBuffer) {
    // Commandes d'écriture (FC 06) → réponse = écho 8 octets
    const resp = await sendRequest(cmdBuffer, WRITE_RESPONSE_LEN);
    return modbus.parseWriteResponse(resp);
}

// ---------------------------------------------------------------------------
// Polling automatique
// ---------------------------------------------------------------------------
let lastMeasures     = null;
let lastPowerState   = null;
let lastSwitchState  = null;
let lastConfig       = null;
let pollCount        = 0;

async function poll() {
    if (!serial || !serial.isOpen) return;

    try {
        // Lecture tensions + fréquences (0x0006 × 8 registres)
        const r1 = await readRegisters(modbus.REG.MEASURES_START, modbus.REG.MEASURES_COUNT);
        if (!r1.error) {
            lastMeasures = modbus.interpretMeasures(r1.registers);
        }

        // Lecture état alimentation + commutateur (0x004F × 2)
        const r2 = await readRegisters(modbus.REG.POWER_STATE, 2);
        if (!r2.error) {
            lastPowerState  = modbus.interpretPowerState(r2.registers[0]);
            lastSwitchState = modbus.interpretSwitchState(r2.registers[1]);
        }

        // Lecture config Modbus toutes les 10 cycles
        if (pollCount % 10 === 0) {
            const r3 = await readRegisters(modbus.REG.MODBUS_ADDR, 2);
            if (!r3.error) {
                lastConfig = {
                    slaveAddr: r3.registers[0],
                    baudRate:  modbus.interpretBaud(r3.registers[1]),
                };
            }
        }

        pollCount++;

        broadcastState({
            measures:    lastMeasures,
            powerState:  lastPowerState,
            switchState: lastSwitchState,
            deviceConfig: lastConfig,
            pollCount,
        });

    } catch (e) {
        broadcastError('Erreur polling : ' + e.message);
    }
}

function startPolling() {
    if (pollTimer) clearInterval(pollTimer);
    pollCount = 0;
    pollTimer = setInterval(poll, config.pollIntervalMs);
    broadcast('status', { polling: true, intervalMs: config.pollIntervalMs });
}

function stopPolling() {
    if (pollTimer) { clearInterval(pollTimer); pollTimer = null; }
    broadcast('status', { polling: false });
}

// ---------------------------------------------------------------------------
// REST API
// ---------------------------------------------------------------------------

/** GET /api/ports — lister les ports COM disponibles */
app.get('/api/ports', async (req, res) => {
    try {
        const ports = await SerialPort.list();
        res.json(ports.map(p => ({
            path:         p.path,
            manufacturer: p.manufacturer || '',
            description:  p.friendlyName || p.pnpId || '',
        })));
    } catch (e) {
        res.status(500).json({ error: e.message });
    }
});

/** POST /api/connect — ouvrir le port */
app.post('/api/connect', async (req, res) => {
    const cfg = {
        port:           req.body.port      || config.port,
        baudRate:       req.body.baudRate  || config.baudRate,
        parity:         req.body.parity    || config.parity,
        dataBits:       req.body.dataBits  || config.dataBits,
        stopBits:       req.body.stopBits  || config.stopBits,
        slaveAddr:      req.body.slaveAddr || config.slaveAddr,
        pollIntervalMs: req.body.pollIntervalMs || config.pollIntervalMs,
    };
    try {
        config = { ...config, ...cfg };
        await openPort(cfg);
        if (req.body.autoPolling !== false) startPolling();
        res.json({ ok: true, config });
    } catch (e) {
        res.status(500).json({ error: e.message });
    }
});

/** POST /api/disconnect */
app.post('/api/disconnect', (req, res) => {
    closePort();
    res.json({ ok: true });
});

/** POST /api/polling/start */
app.post('/api/polling/start', (req, res) => {
    if (req.body.intervalMs) config.pollIntervalMs = req.body.intervalMs;
    startPolling();
    res.json({ ok: true });
});

/** POST /api/polling/stop */
app.post('/api/polling/stop', (req, res) => {
    stopPolling();
    res.json({ ok: true });
});

/** POST /api/read — lecture manuelle d'un registre */
app.post('/api/read', async (req, res) => {
    const { address, count = 1 } = req.body;
    if (address === undefined) return res.status(400).json({ error: 'address manquant' });
    try {
        const r = await readRegisters(parseInt(address), parseInt(count));
        if (r.error) return res.status(502).json({ error: r.error });
        res.json({
            address: `0x${parseInt(address).toString(16).padStart(4, '0').toUpperCase()}`,
            registers: r.registers.map((v, i) => ({
                offset: i,
                address: `0x${(parseInt(address) + i).toString(16).padStart(4, '0').toUpperCase()}`,
                dec: v,
                hex: `0x${v.toString(16).padStart(4, '0').toUpperCase()}`,
                bin: v.toString(2).padStart(16, '0'),
            })),
        });
    } catch (e) {
        res.status(502).json({ error: e.message });
    }
});

/** POST /api/write — écriture manuelle d'un registre */
app.post('/api/write', async (req, res) => {
    const { address, value } = req.body;
    if (address === undefined || value === undefined)
        return res.status(400).json({ error: 'address et value requis' });
    try {
        const r = await writeRegister(parseInt(address), parseInt(value));
        if (r.error) return res.status(502).json({ error: r.error });
        res.json({ ok: true, reg: r.reg, value: r.value });
    } catch (e) {
        res.status(502).json({ error: e.message });
    }
});

/** POST /api/command — commande nommée */
app.post('/api/command', async (req, res) => {
    const { name } = req.body;
    const cmd = modbus.COMMANDS[name];
    if (!cmd) return res.status(400).json({ error: `Commande inconnue: ${name}` });
    try {
        const r = await sendRawCommand(cmd);
        if (r.error) return res.status(502).json({ error: r.error });
        res.json({ ok: true, command: name });
        broadcast('command', { name, ok: true });
    } catch (e) {
        res.status(502).json({ error: e.message });
        broadcast('command', { name, ok: false, error: e.message });
    }
});

/** POST /api/frame — envoi d'une trame hexadécimale brute */
app.post('/api/frame', async (req, res) => {
    const { hex, expectedLen } = req.body;
    if (!hex) return res.status(400).json({ error: 'hex requis' });
    try {
        const frame = modbus.hexToFrame(hex);
        const expLen = parseInt(expectedLen) || WRITE_RESPONSE_LEN;
        const resp = await sendRequest(frame, expLen);
        res.json({
            ok:          true,
            responseHex: Array.from(resp).map(b => b.toString(16).padStart(2, '0').toUpperCase()).join(' '),
            crcOk:       modbus.checkCrc(resp),
        });
    } catch (e) {
        res.status(502).json({ error: e.message });
    }
});

/** POST /api/scan — scan d'une plage de registres */
app.post('/api/scan', async (req, res) => {
    const { from = 0, to = 0x0100, step = 8 } = req.body;
    if (!serial || !serial.isOpen)
        return res.status(400).json({ error: 'Port non connecté' });

    stopPolling(); // suspendre polling pendant le scan

    const results = [];
    try {
        for (let addr = parseInt(from); addr <= parseInt(to); addr += parseInt(step)) {
            const count = Math.min(parseInt(step), parseInt(to) - addr + 1);
            try {
                const r = await readRegisters(addr, count);
                if (!r.error) {
                    r.registers.forEach((v, i) => {
                        results.push({
                            address: `0x${(addr + i).toString(16).padStart(4, '0').toUpperCase()}`,
                            dec: v, hex: `0x${v.toString(16).padStart(4, '0').toUpperCase()}`,
                        });
                    });
                }
            } catch (_) { /* timeout sur cette plage, continuer */ }
            broadcast('scanProgress', { addr, to: parseInt(to) });
        }
        res.json({ results });
    } catch (e) {
        res.status(502).json({ error: e.message });
    } finally {
        if (serial && serial.isOpen) startPolling();
    }
});

/** GET /api/status — état courant */
app.get('/api/status', (req, res) => {
    res.json({
        connected:   !!(serial && serial.isOpen),
        polling:     !!pollTimer,
        config,
        measures:    lastMeasures,
        powerState:  lastPowerState,
        switchState: lastSwitchState,
        deviceConfig: lastConfig,
    });
});

// ---------------------------------------------------------------------------
// Démarrage
// ---------------------------------------------------------------------------
const PORT = process.env.PORT || 3000;
server.listen(PORT, () => {
    console.log(`
╔══════════════════════════════════════════════════════╗
║  CHINT NXZBN-63S/2DT — Dashboard ATS                ║
║  http://localhost:${PORT}                              ║
║  Port RS485 défaut : ${DEFAULT_CONFIG.port} (modifiable)          ║
╚══════════════════════════════════════════════════════╝
`);
});
