from flask import Flask, render_template_string, jsonify
import serial
import time
import threading

app = Flask(__name__)

PORT = 'COM5'
BAUDRATE = 9600
ADDR = 6
ser = None

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
    if func == 0x03:
        data = bytes([ADDR, func, (reg >> 8) & 0xFF, reg & 0xFF, 0x00, 0x01])
    elif func == 0x06:
        data = bytes([ADDR, func, (reg >> 8) & 0xFF, reg & 0xFF, (value >> 8) & 0xFF, value & 0xFF])
    else:
        return None
    crc = calculate_crc(data)
    return data + bytes([crc & 0xFF, (crc >> 8) & 0xFF])

def read_register(reg):
    global ser
    if ser is None or not ser.is_open:
        return None
    try:
        frame = build_frame(0x03, reg)
        ser.write(frame)
        time.sleep(0.15)
        resp = ser.read(256)
        if resp and len(resp) >= 5 and resp[1] == 0x03:
            return (resp[3] << 8) | resp[4]
        return None
    except:
        return None

def write_register(reg, value):
    global ser
    if ser is None or not ser.is_open:
        return False
    try:
        frame = build_frame(0x06, reg, value)
        ser.write(frame)
        time.sleep(0.15)
        resp = ser.read(256)
        return resp is not None and len(resp) > 0
    except:
        return False

# Page HTML simple
HTML = """
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>CHINT ATS</title>
    <style>
        body { font-family: monospace; background: #1a2a3a; color: #eee; padding: 20px; }
        .container { max-width: 800px; margin: 0 auto; }
        .card { background: #1e2a32; border-radius: 12px; padding: 20px; margin-bottom: 20px; }
        button { background: #3b82f6; border: none; color: white; padding: 10px 20px; margin: 5px; cursor: pointer; border-radius: 8px; }
        button:hover { background: #2563eb; }
        .data { font-size: 24px; font-weight: bold; color: #fbbf24; }
        .log { background: #0f172a; padding: 10px; height: 200px; overflow-y: auto; font-size: 12px; }
        .flex { display: flex; flex-wrap: wrap; gap: 10px; }
        .status { padding: 10px; border-radius: 8px; margin-bottom: 15px; }
        .connected { background: #1f6d3a; }
        .disconnected { background: #7f2a1f; }
    </style>
</head>
<body>
<div class="container">
    <h1>⚡ CHINT ATS</h1>
    <div id="status" class="status disconnected">🔌 Déconnecté</div>
    
    <div class="card">
        <h3>📊 Données</h3>
        <div id="data">Chargement...</div>
    </div>
    
    <div class="card">
        <h3>🎮 Commandes</h3>
        <div class="flex">
            <button onclick="sendCmd('remote_on')">📡 Activer télécommande</button>
            <button onclick="sendCmd('remote_off')">🔒 Désactiver télécommande</button>
            <button onclick="sendCmd('force_double')">⏹️ Forcer double</button>
            <button onclick="sendCmd('force_source1')">🔵 Forcer Source I</button>
            <button onclick="sendCmd('force_source2')">🟠 Forcer Source II</button>
        </div>
        <p style="font-size:12px; color:#fbbf24;">⚠️ Activez d'abord la télécommande</p>
    </div>
    
    <div class="card">
        <h3>📋 Journal</h3>
        <div id="log" class="log"></div>
        <button onclick="clearLog()">Effacer</button>
    </div>
</div>

<script>
    function addLog(msg, type='info') {
        const log = document.getElementById('log');
        const time = new Date().toLocaleTimeString();
        const icon = type === 'error' ? '❌' : (type === 'success' ? '✅' : 'ℹ️');
        log.innerHTML += `<div>[${time}] ${icon} ${msg}</div>`;
        log.scrollTop = log.scrollHeight;
        while(log.children.length > 50) log.removeChild(log.firstChild);
    }
    
    function clearLog() {
        document.getElementById('log').innerHTML = '';
        addLog('Journal effacé');
    }
    
    async function refresh() {
        try {
            const resp = await fetch('/api/read_all');
            const data = await resp.json();
            if (data.success) {
                document.getElementById('status').className = 'status connected';
                document.getElementById('status').innerHTML = '✅ Connecté';
                let html = '<div class="flex">';
                for (const [key, val] of Object.entries(data.values)) {
                    html += `<div><strong>${key}</strong><br><span class="data">${val}</span></div>`;
                }
                html += '</div>';
                document.getElementById('data').innerHTML = html;
                addLog('Données actualisées', 'success');
            } else {
                document.getElementById('status').className = 'status disconnected';
                document.getElementById('status').innerHTML = '🔌 ' + (data.error || 'Erreur');
            }
        } catch(e) {
            addLog('Erreur: ' + e.message, 'error');
        }
    }
    
    async function sendCmd(cmd) {
        addLog('Envoi: ' + cmd);
        try {
            const resp = await fetch('/api/' + cmd);
            const data = await resp.json();
            if (data.success) {
                addLog('✅ ' + (data.message || 'OK'), 'success');
                setTimeout(refresh, 500);
            } else {
                addLog('❌ ' + (data.error || 'Échec'), 'error');
            }
        } catch(e) {
            addLog('❌ Erreur: ' + e.message, 'error');
        }
    }
    
    setInterval(refresh, 5000);
    refresh();
</script>
</body>
</html>
"""

@app.route('/')
def index():
    return render_template_string(HTML)

@app.route('/api/read_all')
def read_all():
    global ser
    
    # Connexion si nécessaire
    if ser is None or not ser.is_open:
        try:
            ser = serial.Serial(PORT, BAUDRATE, bytesize=8, parity='E', stopbits=1, timeout=1)
            time.sleep(0.3)
        except Exception as e:
            return jsonify({'success': False, 'error': str(e)})
    
    values = {}
    
    # Lecture de tous les registres
    regs = [
        (0x0006, "Tension A Source I", "V"),
        (0x0007, "Tension B Source I", "V"),
        (0x0008, "Tension C Source I", "V"),
        (0x0009, "Tension A Source II", "V"),
        (0x000A, "Tension B Source II", "V"),
        (0x000B, "Tension C Source II", "V"),
        (0x004F, "État sources", ""),
        (0x0050, "État commutateur", ""),
        (0x0015, "Nb commutations Source I", ""),
        (0x0016, "Nb commutations Source II", ""),
        (0x0017, "Temps fonctionnement", "h"),
        (0x000C, "Version logicielle", ""),
    ]
    
    for reg, name, unit in regs:
        val = read_register(reg)
        if val is not None:
            if reg == 0x000C:
                values[name] = f"{val/100:.2f}"
            elif reg == 0x0017:
                values[name] = f"{val} {unit}"
            elif unit:
                values[name] = f"{val} {unit}"
            else:
                values[name] = f"0x{val:04X} ({val})"
        else:
            values[name] = "Erreur"
    
    return jsonify({'success': True, 'values': values})

@app.route('/api/remote_on')
def remote_on():
    success = write_register(0x2800, 0x0004)
    return jsonify({'success': success, 'message': 'Télécommande activée'})

@app.route('/api/remote_off')
def remote_off():
    success = write_register(0x2800, 0x0000)
    return jsonify({'success': success, 'message': 'Télécommande désactivée'})

@app.route('/api/force_double')
def force_double():
    success = write_register(0x2700, 0x00FF)
    return jsonify({'success': success, 'message': 'Forçage double'})

@app.route('/api/force_source1')
def force_source1():
    success = write_register(0x2700, 0x0000)
    return jsonify({'success': success, 'message': 'Forçage Source I'})

@app.route('/api/force_source2')
def force_source2():
    success = write_register(0x2700, 0x00AA)
    return jsonify({'success': success, 'message': 'Forçage Source II'})

if __name__ == '__main__':
    print("=" * 50)
    print("  CHINT ATS - Interface Web")
    print("  Port: COM5 | 9600 Even | Adresse 6")
    print("  Ouvrez http://localhost:5000")
    print("=" * 50)
    app.run(host='localhost', port=5000, debug=False)
