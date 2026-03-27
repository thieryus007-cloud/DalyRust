from flask import Flask, render_template_string, jsonify, request
import serial
import time
import threading

app = Flask(__name__)

# Configuration série
PORT = 'COM5'
BAUDRATE = 9600
ser = None
lock = threading.Lock()

HTML = """
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>CHINT ATS - Contrôle Modbus</title>
    <style>
        body { font-family: monospace; background: #1a2a3a; color: #eee; padding: 20px; }
        .container { max-width: 800px; margin: 0 auto; background: #1e2a32; border-radius: 16px; padding: 20px; }
        button { background: #2c3e50; border: none; color: white; padding: 10px 20px; border-radius: 8px; cursor: pointer; margin: 5px; font-size: 14px; }
        button:hover { background: #ffaa44; color: #1e2a32; }
        .connected { background: #1f6d3a; padding: 10px; border-radius: 8px; }
        .disconnected { background: #7f2a1f; padding: 10px; border-radius: 8px; }
        .log-area { background: #0a1219; border-radius: 8px; padding: 10px; height: 250px; overflow-y: auto; font-size: 12px; font-family: monospace; }
        .result { background: #0f1a1f; padding: 15px; border-radius: 8px; margin: 10px 0; text-align: center; font-size: 18px; }
        .flex { display: flex; flex-wrap: wrap; gap: 10px; margin: 15px 0; justify-content: center; }
        input { background: #0f1a1f; border: 1px solid #3a5a6a; color: #eee; padding: 8px; border-radius: 6px; width: 80px; text-align: center; }
    </style>
</head>
<body>
<div class="container">
    <h2>⚡ CHINT ATS - Modbus RTU</h2>
    <div id="status" class="disconnected">🔌 Déconnecté</div>
    
    <div class="flex">
        <button onclick="connect()">🔌 Connecter</button>
        <button onclick="disconnect()">⛔ Déconnecter</button>
        <button onclick="sendCmd('read_power')">📡 État sources</button>
        <button onclick="sendCmd('read_switch')">🔀 État commutateur</button>
        <button onclick="sendCmd('read_voltage')">📊 Tension A Source I</button>
        <button onclick="sendCmd('remote_on')">📡 Activer télécommande</button>
        <button onclick="sendCmd('remote_off')">🔒 Désactiver télécommande</button>
        <button onclick="sendCmd('force_double')">⏹️ Forcer double</button>
        <button onclick="sendCmd('force_source1')">🔵 Forcer Source I</button>
        <button onclick="sendCmd('force_source2')">🟠 Forcer Source II</button>
    </div>
    
    <div id="result" class="result">-- En attente --</div>
    
    <div class="log-area" id="log">
        <div>✨ Prêt - Cliquez sur Connecter</div>
    </div>
    <button onclick="clearLog()">🗑️ Effacer journal</button>
</div>

<script>
    async function apiCall(endpoint) {
        try {
            const resp = await fetch('/api/' + endpoint);
            return await resp.json();
        } catch(e) {
            addLog('Erreur: ' + e.message, 'error');
        }
    }
    
    function addLog(msg, type = 'info') {
        const logDiv = document.getElementById('log');
        const entry = document.createElement('div');
        const time = new Date().toLocaleTimeString();
        let prefix = 'ℹ️';
        if (type === 'error') prefix = '❌';
        if (type === 'success') prefix = '✅';
        entry.innerHTML = `[${time}] ${prefix} ${msg}`;
        logDiv.appendChild(entry);
        entry.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
    
    async function connect() {
        const result = await apiCall('connect');
        if (result.success) {
            document.getElementById('status').className = 'connected';
            document.getElementById('status').innerHTML = '✅ Connecté sur COM5 | 9600 Even | Adresse 6';
            addLog('Connecté!', 'success');
        } else {
            addLog('Échec: ' + result.error, 'error');
        }
    }
    
    async function disconnect() {
        const result = await apiCall('disconnect');
        document.getElementById('status').className = 'disconnected';
        document.getElementById('status').innerHTML = '🔌 Déconnecté';
        addLog('Déconnecté');
    }
    
    async function sendCmd(cmd) {
        const result = await apiCall(cmd);
        if (result.success) {
            document.getElementById('result').innerHTML = result.message;
            addLog(result.message, 'success');
        } else {
            addLog('Erreur: ' + result.error, 'error');
        }
    }
    
    function clearLog() {
        document.getElementById('log').innerHTML = '<div>Journal effacé</div>';
    }
</script>
</body>
</html>
"""

def send_frame(frame_bytes):
    """Envoie une trame brute et retourne la réponse"""
    with lock:
        ser.write(frame_bytes)
        time.sleep(0.05)
        response = ser.read(256)
        return response

@app.route('/')
def index():
    return render_template_string(HTML)

@app.route('/api/connect')
def api_connect():
    global ser
    try:
        ser = serial.Serial(
            port=PORT,
            baudrate=BAUDRATE,
            bytesize=8,
            parity='E',
            stopbits=1,
            timeout=2
        )
        return jsonify({'success': True})
    except Exception as e:
        return jsonify({'success': False, 'error': str(e)})

@app.route('/api/disconnect')
def api_disconnect():
    global ser
    if ser:
        ser.close()
        ser = None
    return jsonify({'success': True})

@app.route('/api/read_power')
def api_read_power():
    try:
        frame = bytes([0x06, 0x03, 0x00, 0x4F, 0x00, 0x01, 0xB4, 0x6A])
        resp = send_frame(frame)
        if len(resp) >= 5:
            val = (resp[3] << 8) | resp[4]
            return jsonify({'success': True, 'message': f'État sources = 0x{val:04X} ({val})'})
        return jsonify({'success': False, 'error': 'Pas de réponse'})
    except Exception as e:
        return jsonify({'success': False, 'error': str(e)})

@app.route('/api/read_switch')
def api_read_switch():
    try:
        frame = bytes([0x06, 0x03, 0x00, 0x50, 0x00, 0x01, 0x44, 0xBE])
        resp = send_frame(frame)
        if len(resp) >= 5:
            val = (resp[3] << 8) | resp[4]
            return jsonify({'success': True, 'message': f'État commutateur = 0x{val:04X} ({val})'})
        return jsonify({'success': False, 'error': 'Pas de réponse'})
    except Exception as e:
        return jsonify({'success': False, 'error': str(e)})

@app.route('/api/read_voltage')
def api_read_voltage():
    try:
        frame = bytes([0x06, 0x03, 0x00, 0x06, 0x00, 0x01, 0x25, 0xF4])
        resp = send_frame(frame)
        if len(resp) >= 5:
            val = (resp[3] << 8) | resp[4]
            return jsonify({'success': True, 'message': f'Tension Phase A Source I = {val} V'})
        return jsonify({'success': False, 'error': 'Pas de réponse'})
    except Exception as e:
        return jsonify({'success': False, 'error': str(e)})

@app.route('/api/remote_on')
def api_remote_on():
    try:
        frame = bytes([0x06, 0x06, 0x28, 0x00, 0x00, 0x04, 0x49, 0x14])
        resp = send_frame(frame)
        if len(resp) > 0:
            return jsonify({'success': True, 'message': '✅ Télécommande activée'})
        return jsonify({'success': False, 'error': 'Pas de réponse'})
    except Exception as e:
        return jsonify({'success': False, 'error': str(e)})

@app.route('/api/remote_off')
def api_remote_off():
    try:
        frame = bytes([0x06, 0x06, 0x28, 0x00, 0x00, 0x00, 0x48, 0xD4])
        resp = send_frame(frame)
        if len(resp) > 0:
            return jsonify({'success': True, 'message': '🔒 Télécommande désactivée'})
        return jsonify({'success': False, 'error': 'Pas de réponse'})
    except Exception as e:
        return jsonify({'success': False, 'error': str(e)})

@app.route('/api/force_double')
def api_force_double():
    try:
        frame = bytes([0x06, 0x06, 0x27, 0x00, 0x00, 0xFF, 0x83, 0x91])
        resp = send_frame(frame)
        if len(resp) > 0:
            return jsonify({'success': True, 'message': '⏹️ Forçage double déclenché'})
        return jsonify({'success': False, 'error': 'Pas de réponse'})
    except Exception as e:
        return jsonify({'success': False, 'error': str(e)})

@app.route('/api/force_source1')
def api_force_source1():
    try:
        frame = bytes([0x06, 0x06, 0x27, 0x00, 0x00, 0x00, 0x43, 0xD1])
        resp = send_frame(frame)
        if len(resp) > 0:
            return jsonify({'success': True, 'message': '🔵 Forçage Source I'})
        return jsonify({'success': False, 'error': 'Pas de réponse'})
    except Exception as e:
        return jsonify({'success': False, 'error': str(e)})

@app.route('/api/force_source2')
def api_force_source2():
    try:
        frame = bytes([0x06, 0x06, 0x27, 0x00, 0x00, 0xAA, 0xC3, 0x98])
        resp = send_frame(frame)
        if len(resp) > 0:
            return jsonify({'success': True, 'message': '🟠 Forçage Source II'})
        return jsonify({'success': False, 'error': 'Pas de réponse'})
    except Exception as e:
        return jsonify({'success': False, 'error': str(e)})

if __name__ == '__main__':
    print("=" * 50)
    print("  Serveur CHINT ATS - Interface Web")
    print("=" * 50)
    print("  Port série: COM5 | 9600 Even | Adresse 6")
    print("  Ouvrez http://localhost:5000 dans votre navigateur")
    print("=" * 50)
    app.run(host='localhost', port=5000, debug=False)
