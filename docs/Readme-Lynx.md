 ## Deux approches principales :
	1	Lecture directe I²C (Python simple, pour monitoring local/Home Assistant).
	2	Intégration complète dans Venus OS (recommandée si ton Pi 5 tourne Venus OS) via le projet dbus-lynx-distributor (affiche les fusibles dans le menu GX, VRM, alarmes, etc.).
# Matériel nécessaire (commun aux deux)
	•	Câble RJ10 coupé ou adaptateur RJ10 vers fils.
	•	Alimentation 5V stable pour le Pin 1 (jaune) → buck converter 12/24/48V → 5V ou sortie 5V du Pi (attention au courant).
	•	Level shifter bidirectionnel 3.3V ↔ 5V (obligatoire ! Le Pi 5 est en 3.3V, le Lynx en 5V).
	◦	Exemple : module 4 canaux bi-directionnel (AliExpress/Amazon ~2-5€) ou FT232H (voir ci-dessous).
	•	Résistances pull-up 4,7 kΩ ou 10 kΩ sur SDA et SCL vers +5V.
	•	Adresse I²C : Par défaut 0x08 (jumper A).
Pinout rappel :
	•	Pin 1 Jaune → 5V
	•	Pin 2 Vert → SDA
	•	Pin 3 Rouge → SCL
	•	Pin 4 Noir → GND
# 1. Lecture directe I²C avec Python (simple et rapide)
Câblage sur Raspberry Pi 5
	•	SDA (Pin 2 vert) → GPIO 2 (pin 3) via level shifter (côté 3.3V)
	•	SCL (Pin 3 rouge) → GPIO 3 (pin 5) via level shifter
	•	5V et GND du Lynx depuis une source externe stable (ou 5V du Pi si faible consommation)
Active I²C :
sudo raspi-config → Interface Options → I2C → Enable
sudo reboot
Code Python simple
Installe les libs :
sudo apt update
sudo apt install python3-smbus i2c-tools
Script exemple (lynx_fuses.py) :
import smbus2
import time

BUS = 1
ADDRESS = 0x08  # Change selon jumper (0x08 à 0x0B)

bus = smbus2.SMBus(BUS)

def read_fuses():
    try:
        status = bus.read_byte(ADDRESS)
        print(f"Status byte: 0x{status:02X} (b{status:08b})")
        
        no_power = (status & 0b00000010) != 0
        fuse1 = (status & 0b00010000) != 0
        fuse2 = (status & 0b00100000) != 0
        fuse3 = (status & 0b01000000) != 0
        fuse4 = (status & 0b10000000) != 0
        
        print("Bus alimenté :", not no_power)
        print("Fuse 1 :", "SAUTÉ" if fuse1 else "OK")
        print("Fuse 2 :", "SAUTÉ" if fuse2 else "OK")
        print("Fuse 3 :", "SAUTÉ" if fuse3 else "OK")
        print("Fuse 4 :", "SAUTÉ" if fuse4 else "OK")
        
        return status
    except Exception as e:
        print("Erreur I2C:", e)

while True:
    read_fuses()
    time.sleep(5)
Lance-le avec python3 lynx_fuses.py. Tu peux l’intégrer dans Home Assistant via MQTT, Node-RED, etc.
# 2. Intégration complète dans Venus OS (meilleure option)
Utilise le repo twam/dbus-lynx-distributor : il émule un Lynx BMS partiel sur le D-Bus → les fusibles apparaissent dans le Remote Console, VRM, avec alarmes.
Hardware recommandé
	•	Adaptateur FT232H (câble C232HM-EDHSL-0 ou module FT232H).
	•	Connecte SDA/SCL via level shifter si besoin (le 3.3V variant fonctionne souvent).
	•	Branche sur un port USB du Pi 5.
Installation
	1	Installe Venus OS sur ton Raspberry Pi 5 (image officielle Victron).
	2	Active SSH.
	3	Clone le repo : git clone https://github.com/twam/dbus-lynx-distributor.git
	4	cd dbus-lynx-distributor
	5	
	6	Copie et édite la config (config.sample.ini → config.ini).
	7	Exécute ./install.sh.
	8	Redémarre le service ou le Pi.
Les données apparaissent comme un “Battery” virtuel avec les infos Distributor.
Tu peux chaîner plusieurs Distributors (adresses différentes).
Conseils & précautions
	•	Level shifting : Ne connecte jamais directement les 5V du Lynx sur les GPIO du Pi 5 → risque de destruction.
	•	Teste d’abord avec i2cdetect -y 1 pour voir l’adresse 0x08.
	•	Les LEDs du Distributor s’allument dès que tu fournis du 5V.
	•	Pour plusieurs Lynx : configure les jumpers internes (A/B/C/D).
