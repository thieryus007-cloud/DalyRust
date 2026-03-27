from flask import Flask, render_template_string, jsonify
import serial
import time
import threading

app = Flask(__name__)

# Configuration
PORT = 'COM5'
BAUDRATE = 9600
ADDRESS = 6
ser = None
lock = threading.Lock()

def calculate_crc(data):
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 0x0001:
                crc = (crc >> 1) ^ 0xA001
            else:
                crc >>= 1
    return crc

def build_frame(func, reg, value=None):
    data = bytes([ADDRESS, func, (reg >> 8) & 0xFF, reg & 0xFF])
    if func == 0x03:
        data += bytes([0x00, 0x01])
    elif func == 0x06:
        data += bytes([(value >> 8) & 0xFF, value & 0xFF])
    crc = calculate_crc(data)
    return data + bytes([crc & 0xFF, (crc >> 8) & 0xFF])

def send_frame(frame):
    with lock:
        ser.write(frame)
        time.sleep(0.15)
        return ser.read(256)

def read_register(reg):
    if ser is None:
        return None
    try:
        frame = build_frame(0x03, reg)
        resp = send_frame(frame)
        if resp and len(resp) >= 5 and resp[1] == 0x03:
            return (resp[3] << 8) | resp[4]
        return None
    except:
        return None

def write_register(reg, value):
    if ser is None:
        return False
    try:
        frame = build_frame(0x06, reg, value)
        resp = send_frame(frame)
        return resp is not None and len(resp) > 0
    except:
        return False

# Page HTML intégrée
HTML = """
<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CHINT ATS - Supervision</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: 'Segoe UI', system-ui; background: linear-gradient(135deg, #0f172a, #1e293b); color: #f1f5f9; padding: 24px; min-height: 100vh; }
        .container { max-width: 1400px; margin: 0 auto; }
        .header { background: rgba(30, 41, 59, 0.8); backdrop-filter: blur(10px); border-radius: 24px; padding: 24px 32px; margin-bottom: 24px; border: 1px solid rgba(255,255,255,0.1); }
        .header h1 { font-size: 28px; background: linear-gradient(135deg, #fbbf24, #f59e0b); -webkit-background-clip: text; -webkit-text-fill-color: transparent; margin-bottom: 8px; }
        .status-bar { display: flex; justify-content: space-between; align-items: center; margin-top: 16px; padding-top: 16px; border-top: 1px solid rgba(255,255,255,0.1); }
        .status { display: flex; align-items: center; gap: 12px; }
        .led { width: 12px; height: 12px; border-radius: 50%; background: #ef4444; transition: all 0.3s; }
        .led.connected { background: #22c55e; box-shadow: 0 0 8px #22c55e; }
        .btn { background: linear-gradient(135deg, #3b82f6, #2563eb); border: none; color: white; padding: 10px 24px; border-radius: 40px; font-weight: 600; cursor: pointer; transition: 0.2s; }
        .btn:hover { transform: scale(1.02); }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(350px, 1fr)); gap: 24px; margin-bottom: 24px; }
        .card { background: rgba(30, 41, 59, 0.8); backdrop-filter: blur(10px); border-radius: 20px; padding: 20px; border: 1px solid rgba(255,255,255,0.1); }
        .card-title { font-size: 16px; font-weight: 600; color: #94a3b8; margin-bottom: 16px; padding-bottom: 12px; border-bottom: 1px solid rgba(255,255,255,0.1); display: flex; align-items: center; gap: 8px; }
        .data-row { display: flex; justify-content: space-between; padding: 10px 0; border-bottom: 1px solid rgba(255,255,255,0.05); }
        .data-label { color: #94a3b8; font-size: 13px; }
        .data-value { font-weight: 600; font-family: monospace; font-size: 16px; }
        .badge { padding: 4px 12px; border-radius: 20px; font-size: 12px; font-weight: 500; }
        .badge-normal { background: rgba(34,197,94,0.2); color: #4ade80; }
        .badge-warning { background: rgba(245,158,11,0.2); color: #fbbf24; }
        .badge-danger { background: rgba(239,68,68,0.2); color: #f87171; }
        .action-buttons { display: flex; flex-wrap: wrap; gap: 12px; margin-top: 16px; }
        .action-btn { background: rgba(59,130,246,0.2); border: 1px solid rgba(59,130,246,0.5); color: #60a5fa; padding: 8px 16px; border-radius: 40px; font-size: 12px; cursor: pointer; transition: 0.2s; }
        .action-btn:hover { background: #3b82f6; color: white; }
        .logs-card { background: rgba(15, 23, 42, 0.9); border-radius: 20px; padding: 20px; }
        .logs-area { background: #0f172a; border-radius: 12px; padding: 12px; height: 180px; overflow-y: auto; font-family: monospace; font-size: 11px; }
        .log-entry { padding: 4px 0; border-bottom: 1px solid rgba(255,255,255,0.05); color: #94a3b8; }
        .log-entry.success { color: #4ade80; }
        .log-entry.error { color: #f87171; }
        .clear-log { margin-top: 12px; background: none; border: 1px solid rgba(255,255,255,0.2); color: #94a3b8; padding: 6px 12px; border-radius: 8px; cursor: pointer; }
        footer { text-align: center; margin-top: 24px; font-size: 12px; color: #475569; }
        .flex-between { display: flex; justify-content: space-between; align-items: center; }
    </style>
</head>
<body>
<div class="container">
    <div class="header">
        <h1>⚡ CHINT ATS - Supervision</h1>
        <div class="status-bar">
            <div class="status">
                <div class="led" id="led"></div>
                <div id="statusText">Déconnecté</div>
                <div style="color:#64748b;">| Adresse 6 | 9600 Even</div>
            </div>
            <button class="btn" id="connectBtn" onclick="connectSerial()">🔌 Connecter</button>
            <button class="btn" id="refreshBtn" onclick="refreshAll()">🔄 Actualiser</button>
        </div>
    </div>

    <div class="grid">
        <div class="card">
            <div class="card-title"><span>🔵</span> Source I - Tensions</div>
            <div class="data-row"><span class="data-label">Phase A (L1-N)</span><span class="data-value" id="v1a">--- V</span></div>
            <div class="data-row"><span class="data-label">Phase B (L2-N)</span><span class="data-value" id="v1b">--- V</span></div>
            <div class="data-row"><span class="data-label">Phase C (L3-N)</span><span class="data-value" id="v1c">--- V</span></div>
            <div class="data-row"><span class="data-label">Fréquence</span><span class="data-value" id="f1">--- Hz</span></div>
        </div>

        <div class="card">
            <div class="card-title"><span>🟠</span> Source II - Tensions</div>
            <div class="data-row"><span class="data-label">Phase A (L1-N)</span><span class="data-value" id="v2a">--- V</span></div>
            <div class="data-row"><span class="data-label">Phase B (L2-N)</span><span class="data-value" id="v2b">--- V</span></div>
            <div class="data-row"><span class="data-label">Phase C (L3-N)</span><span class="data-value" id="v2c">--- V</span></div>
            <div class="data-row"><span class="data-label">Fréquence</span><span class="data-value" id="f2">--- Hz</span></div>
        </div>

        <div class="card">
            <div class="card-title"><span>📊</span> État des sources</div>
            <div class="data-row"><span class="data-label">Source I - A</span><span class="data-value" id="s1a">---</span></div>
            <div class="data-row"><span class="data-label">Source I - B</span><span class="data-value" id="s1b">---</span></div>
            <div class="data-row"><span class="data-label">Source I - C</span><span class="data-value" id="s1c">---</span></div>
            <div class="data-row"><span class="data-label">Source II - A</span><span class="data-value" id="s2a">---</span></div>
            <div class="data-row"><span class="data-label">Source II - B</span><span class="data-value" id="s2b">---</span></div>
            <div class="data-row"><span class="data-label">Source II - C</span><span class="data-value" id="s2c">---</span></div>
        </div>

        <div class="card">
            <div class="card-title"><span>🔀</span> État du commutateur</div>
            <div class="data-row"><span class="data-label">Source I</span><span class="data-value" id="sw1">---</span></div>
            <div class="data-row"><span class="data-label">Source II</span><span class="data-value" id="sw2">---</span></div>
            <div class="data-row"><span class="data-label">Position double</span><span class="data-value" id="swMid">---</span></div>
            <div class="data-row"><span class="data-label">Mode</span><span class="data-value" id="swMode">---</span></div>
            <div class="data-row"><span class="data-label">Télécommande</span><span class="data-value" id="swRemote">---</span></div>
        </div>

        <div class="card">
            <div class="card-title"><span>📈</span> Statistiques</div>
            <div class="data-row"><span class="data-label">Commutations Source I</span><span class="data-value" id="cnt1">---</span></div>
            <div class="data-row"><span class="data-label">Commutations Source II</span><span class="data-value" id="cnt2">---</span></div>
            <div class="data-row"><span class="data-label">Temps fonctionnement</span><span class="data-value" id="runtime">--- h</span></div>
            <div class="data-row"><span class="data-label">Version logicielle</span><span class="data-value" id="swVer">---</span></div>
        </div>

        <div class="card">
            <div class="card-title"><span>🎮</span> Commandes</div>
            <div class="action-buttons">
                <button class="action-btn" onclick="sendCmd('remote_on')">📡 Activer télécommande</button>
                <button class="action-btn" onclick="sendCmd('remote_off')">🔒 Désactiver télécommande</button>
                <button class="action-btn" onclick="sendCmd('force_source1')">🔵 Forcer Source I</button>
                <button class="action-btn" onclick="sendCmd('force_source2')">🟠 Forcer Source II</button>
                <button class="action-btn" onclick="sendCmd('force_double')">⏹️ Forcer double</button>
            </div>
            <div class="data-row" style="margin-top: 12px;">
                <span class="data-label">⚠️ Astuce</span>
                <span class="data-value" style="font-size: 11px;">Activer télécommande avant forçage</span>
            </div>
        </div>
    </div>

    <div class="logs-card">
        <div class="flex-between">
            <div class="card-title" style="border: none;">📋 Journal</div>
            <button class="clear-log" onclick="clearLog()">Effacer</button>
        </div>
        <div class="logs-area" id="logs">
            <div class="log-entry">✨ Cliquez sur "Connecter" pour démarrer</div>
        </div>
    </div>
    <footer>CHINT ATS · Modbus RTU · Données temps réel</footer>
</div>

<script>
    let connected = false;

    function addLog(msg, type = 'info') {
        const logs = document.getElementById('logs');
        const div = document.createElement('div');
        div.className = `log-entry ${type}`;
        div.innerHTML = `[${new Date().toLocaleTimeString()}] ${type === 'error' ? '❌' : type === 'success' ? '✅' : 'ℹ️'} ${msg}`;
        logs.appendChild(div);
        div.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
        while (logs.children.length > 100) logs.removeChild(logs.firstChild);
    }

    function clearLog() { document.getElementById('logs').innerHTML = '<div class="log-entry">📋 Journal effacé</div>'; }

    async function apiCall(endpoint) {
        try {
            const resp = await fetch('/api/' + endpoint);
            return await resp.json();
        } catch(e) { addLog('Erreur: ' + e.message, 'error'); return null; }
    }

    async function connectSerial() {
        addLog('Connexion au port série...');
        const res = await apiCall('connect');
        if (res && res.success) {
            connected = true;
            document.getElementById('led').className = 'led connected';
            document.getElementById('statusText').innerHTML = 'Connecté';
            addLog('✅ Connecté sur COM5', 'success');
            refreshAll();
        } else {
            addLog('❌ Échec connexion: ' + (res?.error || 'inconnu'), 'error');
        }
    }

    async function refreshAll() {
        if (!connected) { addLog('Connectez-vous d\'abord', 'error'); return; }
        addLog('Lecture des données...');
        
        const endpoints = [
            'voltages', 'power_status', 'switch_status', 'counts', 'runtime', 'sw_version', 'frequency'
        ];
        
        for (const ep of endpoints) {
            const res = await apiCall(ep);
            if (res && res.success && res.values) {
                for (const [k, v] of Object.entries(res.values)) {
                    const el = document.getElementById(k);
                    if (el) el.innerHTML = v;
                }
            }
        }
        addLog('Données actualisées', 'success');
    }

    async function sendCmd(cmd) {
        if (!connected) { addLog('Connectez-vous d\'abord', 'error'); return; }
        addLog(`Commande: ${cmd}`);
        const res = await apiCall(cmd);
        if (res && res.success) addLog(`✅ ${res.message}`, 'success');
        else addLog(`❌ Échec ${cmd}`, 'error');
        setTimeout(() => refreshAll(), 500);
    }
</script>
</body>
</html>
"""

# API Routes
@app.route('/')
def index():
    return render_template_string(HTML)

@app.route('/api/connect')
def api_connect():
    global ser
    try:
        if ser:
            ser.close()
        ser = serial.Serial(PORT, BAUDRATE, bytesize=8, parity='E', stopbits=1, timeout=1.5)
        time.sleep(0.3)
        return jsonify({'success': True})
    except Exception as e:
        return jsonify({'success': False, 'error': str(e)})

@app.route('/api/voltages')
def api_voltages():
    v1a = read_register(0x0006)
    v1b = read_register(0x0007)
    v1c = read_register(0x0008)
    v2a = read_register(0x0009)
    v2b = read_register(0x000A)
    v2c = read_register(0x000B)
    if None in [v1a, v1b, v1c, v2a, v2b, v2c]:
        return jsonify({'success': False})
    return jsonify({'success': True, 'values': {
        'v1a': f'{v1a} V', 'v1b': f'{v1b} V', 'v1c': f'{v1c} V',
        'v2a': f'{v2a} V', 'v2b': f'{v2b} V', 'v2c': f'{v2c} V'
    }})

@app.route('/api/frequency')
def api_frequency():
    val = read_register(0x000D)
    if val is None:
        return jsonify({'success': False})
    return jsonify({'success': True, 'values': {'f1': f'{(val>>8)&0xFF} Hz', 'f2': f'{val&0xFF} Hz'}})

@app.route('/api/power_status')
def api_power_status():
    val = read_register(0x004F)
    if val is None:
        return jsonify({'success': False})
    def decode(bit):
        s = (val >> bit) & 0x03
        return {0: '✅ Normal', 1: '⚠️ Sous-tension', 2: '⚠️ Surtension'}.get(s, '❌ Erreur')
    return jsonify({'success': True, 'values': {
        's1a': decode(8), 's1b': decode(10), 's1c': decode(12),
        's2a': decode(0), 's2b': decode(2), 's2c': decode(4)
    }})

@app.route('/api/switch_status')
def api_switch_status():
    val = read_register(0x0050)
    if val is None:
        return jsonify({'success': False})
    return jsonify({'success': True, 'values': {
        'sw1': '✅ Fermé' if (val & 0x02) else '⭕ Ouvert',
        'sw2': '✅ Fermé' if (val & 0x04) else '⭕ Ouvert',
        'swMid': '⚠️ Oui' if (val & 0x08) else '⭕ Non',
        'swMode': '🤖 Auto' if (val & 0x01) else '👆 Manuel',
        'swRemote': '📡 Activé' if (val & 0x0100) else '🔒 Désactivé'
    }})

@app.route('/api/counts')
def api_counts():
    c1 = read_register(0x0015)
    c2 = read_register(0x0016)
    if c1 is None or c2 is None:
        return jsonify({'success': False})
    return jsonify({'success': True, 'values': {'cnt1': str(c1), 'cnt2': str(c2)}})

@app.route('/api/runtime')
def api_runtime():
    val = read_register(0x0017)
    if val is None:
        return jsonify({'success': False})
    return jsonify({'success': True, 'values': {'runtime': f'{val} h'}})

@app.route('/api/sw_version')
def api_sw_version():
    val = read_register(0x000C)
    if val is None:
        return jsonify({'success': False})
    return jsonify({'success': True, 'values': {'swVer': f'{(val/100):.2f}'}})

@app.route('/api/remote_on')
def api_remote_on():
    success = write_register(0x2800, 0x0004)
    return jsonify({'success': success, 'message': 'Télécommande activée'})

@app.route('/api/remote_off')
def api_remote_off():
    success = write_register(0x2800, 0x0000)
    return jsonify({'success': success, 'message': 'Télécommande désactivée'})

@app.route('/api/force_source1')
def api_force_source1():
    success = write_register(0x2700, 0x0000)
    return jsonify({'success': success, 'message': 'Forçage Source I'})

@app.route('/api/force_source2')
def api_force_source2():
    success = write_register(0x2700, 0x00AA)
    return jsonify({'success': success, 'message': 'Forçage Source II'})

@app.route('/api/force_double')
def api_force_double():
    success = write_register(0x2700, 0x00FF)
    return jsonify({'success': success, 'message': 'Forçage double déclenché'})

if __name__ == '__main__':
    print("=" * 50)
    print("  CHINT ATS - Interface Web")
    print("  Port: COM5 | 9600 Even | Adresse 6")
    print("  Ouvrez http://localhost:5000")
    print("=" * 50)
    app.run(host='localhost', port=5000, debug=False)
