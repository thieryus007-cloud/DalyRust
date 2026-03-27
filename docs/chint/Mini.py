import serial
import time

# Configuration
PORT = 'COM5'
BAUDRATE = 9600
TIMEOUT = 2

# Trame qui fonctionne avec QModbusExplorer (adresse 6, état sources)
# 06 03 00 4F 00 01 B4 6A
FRAME = bytes([0x06, 0x03, 0x00, 0x4F, 0x00, 0x01, 0xB4, 0x6A])

try:
    # Ouverture du port série
    ser = serial.Serial(
        port=PORT,
        baudrate=BAUDRATE,
        bytesize=8,
        parity='E',
        stopbits=1,
        timeout=TIMEOUT
    )
    
    print(f"✅ Port {PORT} ouvert avec 9600 Even 8N1")
    
    # Attente du silence T3.5
    time.sleep(0.05)
    
    # Envoi de la trame
    print(f"📤 Envoi: {FRAME.hex().upper()}")
    ser.write(FRAME)
    
    # Attente de la réponse
    response = ser.read(256)
    
    if response:
        print(f"📥 Réponse reçue ({len(response)} octets): {response.hex().upper()}")
    else:
        print("⏱️ TIMEOUT - Pas de réponse")
    
    ser.close()
    
except serial.SerialException as e:
    print(f"❌ Erreur port série: {e}")
except Exception as e:
    print(f"❌ Erreur: {e}")

# python mini.py
# ✅ Port COM5 ouvert avec 9600 Even 8N1
# 📤 Envoi: 0603004F0001B46A
# 📥 Réponse reçue (7 octets): 0603020015CC4B
