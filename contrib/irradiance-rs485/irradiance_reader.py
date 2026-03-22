#!/usr/bin/env python3
"""
Irradiance RS485 Modbus RTU → MQTT bridge
==========================================
Capteur : Solar Radiation Sensor (PRALRAN)
Protocol : Modbus RTU, FC=0x04, registre 0x0000 → irradiance W/m²
Port     : /dev/ttyUSB1 (FT232 FTDI USB-RS485)
Baud     : 9600, 8N1
Adresse  : 0x05 (configurée sur le capteur)

Publie sur MQTT topic : santuario/irradiance/raw
  payload = entier W/m² (ex: "423")

Topic bridgé vers NanoPi ? NON — interne Pi5 uniquement.
Le flow Node-RED souscrit et injecte la valeur dans santuario/meteo/venus.
"""

import logging
import struct
import time

import paho.mqtt.client as mqtt
import serial

# ─── Configuration ──────────────────────────────────────────────────────────

SERIAL_PORT    = '/dev/ttyUSB1'
BAUD_RATE      = 9600          # factory default capteur (docs §4)
MODBUS_ADDR    = 0x05          # adresse configurée sur le capteur
POLL_INTERVAL  = 5             # secondes entre deux lectures

MQTT_HOST      = 'localhost'   # broker Pi5 (Docker Mosquitto port 1883)
MQTT_PORT      = 1883
MQTT_TOPIC     = 'santuario/irradiance/raw'
MQTT_CLIENT_ID = 'irradiance-rs485'

# ─── Logging ────────────────────────────────────────────────────────────────

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s %(levelname)s %(message)s',
    datefmt='%Y-%m-%dT%H:%M:%S',
)
log = logging.getLogger(__name__)

# ─── Modbus RTU helpers ──────────────────────────────────────────────────────

def crc16_modbus(data: bytes) -> int:
    """CRC-16/Modbus (polynomial 0xA001, init 0xFFFF)."""
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            crc = (crc >> 1) ^ 0xA001 if (crc & 0x0001) else crc >> 1
    return crc


def build_fc04_request(slave_addr: int, register: int, count: int = 1) -> bytes:
    """Build a Modbus RTU FC=04 read-input-registers request frame."""
    frame = struct.pack('>BBHH', slave_addr, 0x04, register, count)
    crc   = crc16_modbus(frame)
    return frame + struct.pack('<H', crc)


def parse_fc04_response(data: bytes, slave_addr: int) -> int | None:
    """
    Parse a Modbus RTU FC=04 response.
    Expected: ADDR FC BYTE_COUNT DATA_HI DATA_LO CRC_LO CRC_HI  (7 bytes for 1 register)
    Returns register value (uint16) or None on error.
    """
    if len(data) < 5:
        log.warning('Réponse trop courte : %d octet(s)', len(data))
        return None
    if data[0] != slave_addr:
        log.warning('Adresse inattendue : got 0x%02X, expected 0x%02X', data[0], slave_addr)
        return None
    if data[1] == 0x84:  # FC=04 + error flag
        log.warning('Exception Modbus : code 0x%02X', data[2] if len(data) > 2 else 0)
        return None
    if data[1] != 0x04:
        log.warning('FC inattendu : 0x%02X', data[1])
        return None
    byte_count = data[2]
    if len(data) < 3 + byte_count:
        log.warning('Données incomplètes : %d/%d', len(data), 3 + byte_count)
        return None
    # First register = big-endian uint16
    value = struct.unpack('>H', data[3:5])[0]
    return value

# ─── Main loop ───────────────────────────────────────────────────────────────

def make_mqtt_client() -> mqtt.Client:
    """Create and connect a paho MQTT client (compatible paho 1.x and 2.x)."""
    try:
        # paho-mqtt >= 2.0
        client = mqtt.Client(mqtt.CallbackAPIVersion.VERSION1, MQTT_CLIENT_ID)
    except AttributeError:
        # paho-mqtt < 2.0
        client = mqtt.Client(MQTT_CLIENT_ID)

    def on_connect(c, userdata, flags, rc):
        if rc == 0:
            log.info('MQTT connecté à %s:%d', MQTT_HOST, MQTT_PORT)
        else:
            log.error('MQTT connexion refusée (code %d)', rc)

    client.on_connect = on_connect
    client.connect(MQTT_HOST, MQTT_PORT, keepalive=60)
    client.loop_start()
    return client


def read_irradiance(ser: serial.Serial) -> int | None:
    """Send Modbus request and return irradiance W/m² or None."""
    request = build_fc04_request(MODBUS_ADDR, 0x0000)
    ser.reset_input_buffer()
    ser.write(request)
    # FC=04, 1 register → response = 7 bytes
    response = ser.read(7)
    return parse_fc04_response(response, MODBUS_ADDR)


def main() -> None:
    mqtt_client = make_mqtt_client()
    request = build_fc04_request(MODBUS_ADDR, 0x0000)
    log.info('Trame Modbus RTU : %s', request.hex(' ').upper())

    consecutive_errors = 0

    while True:
        try:
            with serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=0.5) as ser:
                log.info('Port série ouvert : %s @ %d baud', SERIAL_PORT, BAUD_RATE)
                consecutive_errors = 0

                while True:
                    try:
                        value = read_irradiance(ser)
                        if value is not None:
                            mqtt_client.publish(
                                MQTT_TOPIC, str(value), qos=0, retain=True
                            )
                            log.info('Irradiance : %d W/m²', value)
                            consecutive_errors = 0
                        else:
                            consecutive_errors += 1
                            if consecutive_errors <= 3 or consecutive_errors % 12 == 0:
                                log.warning(
                                    'Capteur ne répond pas (erreur #%d)',
                                    consecutive_errors,
                                )
                    except serial.SerialException as e:
                        log.error('Erreur port série : %s', e)
                        break  # Sortir pour rouvrir le port

                    time.sleep(POLL_INTERVAL)

        except serial.SerialException as e:
            log.error('Impossible d\'ouvrir %s : %s', SERIAL_PORT, e)
            time.sleep(15)  # Attendre avant de réessayer


if __name__ == '__main__':
    main()
