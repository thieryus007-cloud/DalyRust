# DalyBMS — Rust Edition — Documentation Complète & Plan de Mise en Œuvre

**Version Rust complète** (Axum + daly-bms-core) — mise à jour 14 mars 2026  
Remplacement total de la stack Python/FastAPI par **Rust** (daly-bms-core + daly-bms-server Axum).  
Infrastructure Docker inchangée (Mosquitto, InfluxDB, Grafana, Node-RED).  
Dashboard React conservé (compatible WebSocket).  
Déploiement ultra-léger : **un seul binaire statique** (~12–18 Mo).

Pack A (BMS 0x01) ──┐ Pack B (BMS 0x02) ──┤ Pack C (BMS 0x03) ──┼── RS485/USB ── RPi CM5 ── daly-bms-server (Axum) ── Dashboard React Pack D (BMS 0x04) ──┘ [natif] [natif] (jusqu’à 32) │ ├── Mosquitto ─┐ ├── InfluxDB ├─ Docker (Phase 1) ├── Grafana │ ├── Node-RED └─ ├── Alertes (Telegram/Email) └── Venus OS (dbus-mqtt-battery)
**Gains majeurs par rapport à la version Python** :
- RAM : 10–35 Mo au lieu de 150–300 Mo
- CPU polling : ÷3 à ÷5
- Latence WebSocket/API : ÷5–10
- Binaire unique statique (cross-compile aarch64 facile)
- Sécurité mémoire (ownership Rust → zéro risque sur port série)
- Démarrage < 150 ms

## Architecture Rust

| Crate / Binaire              | Rôle |
|------------------------------|------|
| `daly-bms-core`              | Protocole UART, parsing trames, types, CommandQueue sécurisée, multi-BMS, verify post-écriture |
| `daly-bms-server`            | Axum (API REST + WebSocket) + polling + bridges (MQTT, Influx, Alertes) |
| `dashboard/`                 | SPA React (inchangée) — WebSocket `/ws/bms/stream` |
| `daly-bms-cli` (optionnel)   | Outil CLI pour tests/debug |

**Flux de données** :
BMS UART ──► daly_bms_core::poll_loop() │ ▼ on_snapshot(snaps) ┌──────┴───────┐ │ │ ▼ ▼ state.snapshots Bridges (tokio tasks parallèles) ring buffer ├── AlertEngine → rusqlite + Telegram/Email ├── MqttPublisher → rumqttc ├── InfluxWriter → influxdb2-client └── WebSocket broadcast (Axum)
## Plan de Mise en Œuvre (étapes recommandées)

Tree

DalyRust/                  ← racine du dépôt
├── Cargo.toml
├── crates/
│   ├── daly-bms-core/
│   ├── daly-bms-server/
│   └── daly-bms-cli/
├── dashboard/
├── contrib/               
│   ├── daly-bms.service
│   ├── nginx.conf.example
│   └── install-systemd.sh   (optionnel, script d’installation du service)
├── docker-compose.infra.yml
├── Makefile
├── config.toml
└── README.md

### Phase 0 — Préparation (1–2 jours)
1. Cloner le dépôt Rust (futur repo ou branche `rust-edition`)
2. Installer Rust : `rustup default stable`
3. Ajouter target ARM : `rustup target add aarch64-unknown-linux-gnu`
4. Copier `.env.docker.example` → `.env.docker` (identique à la version Python)

### Phase 1 — Infrastructure Docker (inchangée — 5 min)
```bash
make up                  # Mosquitto + InfluxDB + Grafana + Node-RED
make ps                  # vérifier ports 1883, 8086, 3001, 1880
Phase 2 — Compilation & Installation du binaire Rust (15–30 min)
cargo build --release --target aarch64-unknown-linux-gnu   # ou sans target si sur le Pi
sudo cp target/aarch64-unknown-linux-gnu/release/daly-bms-server /usr/local/bin/
sudo cp target/aarch64-unknown-linux-gnu/release/daly-bms-cli   /usr/local/bin/  # optionnel
Phase 3 — Configuration (TOML au lieu de .env)
sudo mkdir -p /etc/daly-bms
sudo cp config.example.toml /etc/daly-bms/config.toml
sudo nano /etc/daly-bms/config.toml
Exemple clé de config.toml :
[serial]
port = "/dev/ttyUSB1"
baud = 9600
poll_interval_ms = 1000

[api]
bind = "0.0.0.0:8000"
api_key = ""                  # vide = pas d'auth (recommandé : mettre une clé en prod)

[mqtt]
enabled = true
host = "localhost"
port = 1883
prefix = "santuario/bms"

[influxdb]
enabled = true
url = "http://localhost:8086"
token = "..."                 # depuis .env.docker
org = "santuario"
bucket = "daly_bms"

[alerts]
db_path = "/var/lib/daly-bms/alerts.db"
telegram_token = ""
telegram_chat_id = ""
Phase 4 — Service systemd
sudo cp contrib/daly-bms.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now daly-bms
journalctl -u daly-bms -f
Phase 5 — Dashboard & Tests
	•	Dashboard React → http://dalybms.local/ (nginx inchangé)
	•	Tester WebSocket : wscat -c ws://localhost:8000/ws/bms/stream
	•	Tester API : curl http://localhost:8000/api/v1/system/status
Infrastructure Docker Complète (Phase 1 — inchangée)
Services identiques à la version Python :
	•	Mosquitto : 1883 + WebSocket 9001
	•	InfluxDB : 8086 (buckets daly_bms + daly_bms_1m)
	•	Grafana : 3001 (dashboard pré-provisionné)
	•	Node-RED : 1880
Commandes Make :
make up / down / restart / logs / reset
Phase 2 future : tout en Docker (binaire Rust + dashboard statique + nginx dans un seul compose).
Prérequis
	•	Matériel : Raspberry Pi CM5 (ou Pi 4/5) + adaptateur USB/RS485
	•	OS : Debian Bookworm / Ubuntu 24.04 (aarch64)
	•	Rust 1.80+
	•	Docker 24+ + Compose v2
	•	Node.js 20 (uniquement pour dev dashboard)
	•	Permissions : utilisateur dans groupe dialout
Développement Local
make up
cargo run --bin daly-bms-server --release   # ou sans --release pour debug
cd dashboard && npm run dev                 # proxy vers :8000
Ajout de BMS / Découverte
	•	Auto-discovery au démarrage (configurable)
	•	Endpoint /api/v1/discover
	•	Support natif jusqu’à 32 BMS (adresses 0x01–0xFF)
Alertes & Sécurité
	•	Moteur Rust avec hysteresis + journal SQLite
	•	Commandes écriture sécurisées (CommandQueue + verify post-écriture)
	•	Mode read-only possible via config
	•	API key + rate-limiting (middleware Tower)
Dépannage Rapide
	•	Port série : ls -l /dev/ttyUSB* + groups
	•	Logs : journalctl -u daly-bms -f
	•	Influx : make logs ou console InfluxDB
	•	WebSocket : wscat ou dashboard
	•	Cargo : RUST_LOG=debug cargo run
Roadmap Future
	•	Phase 2 : Docker complet (binaire Rust + nginx + dashboard)
	•	Dashboard full Rust (Leptos ou Dioxus)
	•	CLI avancée + PyO3 binding (migration progressive)
	•	Support Venus OS via dbus-mqtt-battery (recommandé)


