from pymodbus.client import ModbusSerialClient

# Configuration
client = ModbusSerialClient(
    port='COM5',
    baudrate=9600,
    bytesize=8,
    parity='E',
    stopbits=1,
    timeout=2
)

if client.connect():
    print("✅ Connecté à COM5")
    
    # Lecture état sources (0x004F)
    result = client.read_holding_registers(0x004F, 1, slave=6)
    
    if not result.isError():
        print(f"📊 État sources: 0x{result.registers[0]:04X} ({result.registers[0]})")
    else:
        print(f"❌ Erreur: {result}")
    
    client.close()
else:
    print("❌ Échec connexion COM5")
