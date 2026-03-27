import serial
import time

PORT = 'COM5'
BAUDRATE = 9600

def calculate_crc(data):
    """Calcule le CRC16 Modbus pour un tableau d'octets"""
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 0x0001:
                crc = (crc >> 1) ^ 0xA001
            else:
                crc >>= 1
    return crc

def build_frame(addr, func, reg, value=None):
    """Construit une trame Modbus avec CRC"""
    if func == 0x03:  # Lecture
        data = bytes([addr, func, (reg >> 8) & 0xFF, reg & 0xFF, 0x00, 0x01])
    elif func == 0x06:  # Écriture
        data = bytes([addr, func, (reg >> 8) & 0xFF, reg & 0xFF, (value >> 8) & 0xFF, value & 0xFF])
    else:
        return None
    
    crc = calculate_crc(data)
    return data + bytes([crc & 0xFF, (crc >> 8) & 0xFF])

# Construction des trames avec la fonction
ADDR = 6

COMMANDS = {
    "État sources (0x004F)": build_frame(ADDR, 0x03, 0x004F),
    "État commutateur (0x0050)": build_frame(ADDR, 0x03, 0x0050),
    "Tension A Source I (0x0006)": build_frame(ADDR, 0x03, 0x0006),
    "Tension B Source I (0x0007)": build_frame(ADDR, 0x03, 0x0007),
    "Tension C Source I (0x0008)": build_frame(ADDR, 0x03, 0x0008),
    "Version logicielle (0x000C)": build_frame(ADDR, 0x03, 0x000C),
    "Adresse Modbus (0x0100)": build_frame(ADDR, 0x03, 0x0100),
    "Parité (0x000E)": build_frame(ADDR, 0x03, 0x000E),
}

WRITE_COMMANDS = {
    "Activer télécommande": build_frame(ADDR, 0x06, 0x2800, 0x0004),
    "Désactiver télécommande": build_frame(ADDR, 0x06, 0x2800, 0x0000),
    "Forcer double": build_frame(ADDR, 0x06, 0x2700, 0x00FF),
    "Forcer Source I": build_frame(ADDR, 0x06, 0x2700, 0x0000),
    "Forcer Source II": build_frame(ADDR, 0x06, 0x2700, 0x00AA),
}

print("=" * 70)
print("  TEST MODBUS RTU - CHINT ATS")
print(f"  Port: {PORT} | 9600 Even 8N1 | Adresse: {ADDR}")
print("=" * 70)

try:
    ser = serial.Serial(PORT, BAUDRATE, bytesize=8, parity='E', stopbits=1, timeout=1.5)
    time.sleep(0.3)
    print("✅ Port série ouvert\n")
    
    print("📖 LECTURES:")
    print("-" * 70)
    
    for name, frame in COMMANDS.items():
        print(f"\n🔍 {name}")
        print(f"   Trame: {frame.hex().upper()}")
        
        ser.write(frame)
        time.sleep(0.25)
        response = ser.read(256)
        
        if response and len(response) >= 5:
            print(f"   ✅ Réponse: {response.hex().upper()}")
            if response[1] == 0x03 and len(response) >= 5:
                value = (response[3] << 8) | response[4]
                print(f"   📊 Valeur: {value} (0x{value:04X})")
        else:
            print(f"   ❌ PAS DE RÉPONSE")
    
    print("\n" + "=" * 70)
    print("✍️ ÉCRITURES (nécessite télécommande activée au préalable)")
    print("-" * 70)
    
    for name, frame in WRITE_COMMANDS.items():
        print(f"\n🔧 {name}")
        print(f"   Trame: {frame.hex().upper()}")
        
        ser.write(frame)
        time.sleep(0.25)
        response = ser.read(256)
        
        if response:
            print(f"   ✅ Réponse: {response.hex().upper()}")
        else:
            print(f"   ❌ PAS DE RÉPONSE")
    
    ser.close()
    print("\n" + "=" * 70)
    print("✅ Test terminé")
    
except serial.SerialException as e:
    print(f"❌ Erreur port série: {e}")
except Exception as e:
    print(f"❌ Erreur: {e}")
