const express = require('express');
const http = require('http');
const { Server } = require('socket.io');
const TinyBMS = require('./tinybms');
const BmsSimulator = require('./simulator');
const { SerialPort } = require('serialport');

const app = express();
const server = http.createServer(app);
const io = new Server(server);

app.use(express.static('public'));
app.use(express.json());

const bms = new TinyBMS('');
const simulator = new BmsSimulator();

let currentMode = 'DISCONNECTED';
let pollTimer = null; // Used for simulation interval
let isPolling = false; // Flag for real polling loop

// Helper function to send logs to clients
function sendLog(message, type = 'info') {
    console.log(`[${type.toUpperCase()}] ${message}`);
    io.emit('log', { message, type });
}

// --- ROUTES API ---
app.get('/api/ports', async (req, res) => {
    const ports = await SerialPort.list();
    // Option "SIMULATION" ajoutée en tête
    const result = [{ path: 'SIMULATION', manufacturer: 'Virtual Device' }, ...ports];
    res.json(result);
});

app.post('/api/connect', async (req, res) => {
    const { path, protocol } = req.body; // protocol: 0=MODBUS, 1=ASCII
    stopAll();

    if (path === 'SIMULATION') {
        currentMode = 'SIMULATION';
        startSimulation();
        sendLog('Starting simulation mode', 'system');
        res.json({ success: true, mode: 'SIMULATION' });
        io.emit('status-change', { mode: 'SIMULATION' });
    } else {
        bms.path = path;
        try {
            if (bms.isConnected && bms.port && bms.port.isOpen) {
                await new Promise(resolve => bms.port.close(resolve));
            }
            sendLog(`Connecting to BMS on ${path}...`, 'info');
            await bms.connect();

            // Configuration du protocole si spécifié (par défaut ASCII = 1)
            const selectedProtocol = protocol !== undefined ? parseInt(protocol) : 1;
            const protocolName = selectedProtocol === 1 ? 'ASCII' : 'MODBUS';
            sendLog(`Configuring protocol to ${protocolName}...`, 'info');

            try {
                await bms.setProtocol(selectedProtocol);
                sendLog(`Protocol configured to ${protocolName}`, 'success');
            } catch (protocolError) {
                sendLog(`Protocol configuration warning: ${protocolError.message}`, 'warning');
                // Continue même si la configuration du protocole échoue
                // (le BMS pourrait déjà être sur le bon protocole)
            }

            currentMode = 'CONNECTED';
            startRealPolling();
            sendLog(`BMS connected successfully on ${path}`, 'success');
            res.json({ success: true, mode: 'CONNECTED', protocol: selectedProtocol });
            io.emit('status-change', { mode: 'CONNECTED' });
        } catch (e) {
            sendLog(`Connection failed: ${e.message}`, 'error');
            res.status(500).json({ error: e.message });
        }
    }
});

app.post('/api/write-batch', async (req, res) => {
    const { changes } = req.body;
    if (currentMode === 'SIMULATION') {
        sendLog(`Writing ${changes.length} register(s) in simulation`, 'info');
        changes.forEach(c => simulator.writeSetting(c.id, parseFloat(c.value)));
        io.emit('bms-settings', simulator.getSettings());
        res.json({ success: true });
    } else if (currentMode === 'CONNECTED') {
        try {
            // Pause polling temporarily if needed, or rely on single-threaded nature of JS
            // Ideally, we should queue this or handle it within the polling loop logic to avoid conflicts
            // For now, we'll just execute. Since we use 'await' in polling loop, this might interleave.
            // A safer way is to stop polling, write, resume.

            // Stop polling loop logic
            isPolling = false;

            // Wait a bit for current poll to finish (naive approach)
            await new Promise(r => setTimeout(r, 200));

            sendLog(`Writing ${changes.length} register(s) to BMS...`, 'info');
            for (const c of changes) {
                await bms.writeRegister(parseInt(c.id), parseFloat(c.value));
                sendLog(`Register ${c.id} written: ${c.value}`, 'success');
                // Attendre que le BMS enregistre en EEPROM (critique !)
                await new Promise(r => setTimeout(r, 500));
            }

            // IMPORTANT: Attendre encore avant de relire pour laisser le temps au BMS
            // d'écrire les valeurs en mémoire flash (EEPROM)
            sendLog('Waiting for BMS to save to EEPROM...', 'info');
            await new Promise(r => setTimeout(r, 1000));

            // Relire les settings pour confirmer et mettre à jour l'interface
            sendLog('Reading back settings to verify...', 'info');
            const settings = await bms.readRegisterBlock(300, 45);
            io.emit('bms-settings', settings);

            // Resume polling
            if (currentMode === 'CONNECTED') {
                startRealPolling();
            }

            sendLog(`All registers written successfully`, 'success');
            res.json({ success: true });
        } catch (e) {
            sendLog(`Write error: ${e.message}`, 'error');
            // Resume polling even on error
            if (currentMode === 'CONNECTED') {
                startRealPolling();
            }
            res.status(500).json({ error: e.message });
        }
    } else {
        sendLog('Write failed: Not connected', 'error');
        res.status(400).json({ error: "Not connected" });
    }
});

function stopAll() {
    if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
    }
    isPolling = false; // Stops the recursive real polling loop
    if (currentMode !== 'DISCONNECTED') {
        sendLog('Stopping connection', 'info');
    }
    currentMode = 'DISCONNECTED';
    io.emit('status-change', { mode: 'DISCONNECTED' });
}

function startSimulation() {
    let cycle = 0;
    // Simulation is fast and sync, setInterval is fine here
    pollTimer = setInterval(() => {
        simulator.tick();
        io.emit('bms-live', simulator.getLiveData());
        // Envoi Settings/Stats tous les 5 cycles
        if (cycle % 5 === 0) {
            io.emit('bms-settings', simulator.getSettings());
            io.emit('bms-stats', simulator.getSettings()); // Note: simulator.getSettings() used for stats in original code? Kept as is.
        }
        cycle++;
    }, 1000);
}

async function startRealPolling() {
    if (isPolling) return; // Already polling
    isPolling = true;
    let cycle = 0;

    const pollLoop = async () => {
        if (!isPolling || currentMode !== 'CONNECTED') return;

        try {
            // Lecture des données live (registres 0-56)
            const live = await bms.readRegisterBlock(0, 57);
            io.emit('bms-live', live);

            // Délai plus long entre les lectures pour laisser le BMS respirer
            await new Promise(resolve => setTimeout(resolve, 500));

            // Lecture des stats et settings tous les 5 cycles
            if (cycle % 5 === 0) {
                try {
                    const stats = await bms.readRegisterBlock(100, 20);
                    io.emit('bms-stats', stats);

                    // Délai avant la prochaine lecture
                    await new Promise(resolve => setTimeout(resolve, 500));
                } catch (statsError) {
                    // Les statistiques peuvent ne pas être disponibles sur tous les BMS
                    console.warn("Stats read failed (may not be supported):", statsError.message);
                    // Délai même en cas d'erreur
                    await new Promise(resolve => setTimeout(resolve, 500));
                }
            }

            // Lecture Settings (300-343)
            if (cycle % 5 === 0) {
                const settings = await bms.readRegisterBlock(300, 45);
                io.emit('bms-settings', settings);
            }
            cycle++;
        } catch (e) {
            console.error("Polling error:", e.message || e);
            // Optional: Check if error is fatal (disconnect)
        }

        // Schedule next poll only after this one completes
        // Augmentation du délai entre les cycles complets de 1s à 3s
        if (isPolling && currentMode === 'CONNECTED') {
            setTimeout(pollLoop, 3000);
        }
    };

    pollLoop();
}

const PORT = process.env.PORT || 3000;
server.listen(PORT, () => console.log(`Server on http://localhost:${PORT}`));
