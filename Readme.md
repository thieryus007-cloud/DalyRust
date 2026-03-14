# Daly-BMS — Rust Edition

**Version Rust complète** — mise à jour 14 mars 2026
Remplacement total de la stack Python/FastAPI par **Rust** (workspace multi-crates : `daly-bms-core` + `daly-bms-server` + `daly-bms-cli`).

> Infrastructure Docker **inchangée** (Mosquitto, InfluxDB, Grafana, Node-RED).
> Dashboard React **conservé** (compatible WebSocket Axum).
> Déploiement ultra-léger : **un seul binaire statique** (~12–18 Mo).

---

## Architecture globale

```
Pack A (0x01) ─┐
Pack B (0x02) ─┤── RS485/USB ── RPi CM5 ──[ daly-bms-server ]── Dashboard React
Pack C (0x03) ─┤                              (Axum natif)         WebSocket /ws/bms/stream
Pack D (0x04) ─┘                                    │
(jusqu'à 32)                          ┌─────────────┼─────────────┐
                                      ▼             ▼             ▼
                                 Mosquitto      InfluxDB      AlertEngine
                                 (MQTT)         (séries)      (SQLite)
                                      │
                                 dbus-mqtt-battery (Venus OS / NanoPi)
```

### Workspace Rust

| Crate / Binaire        | Rôle |
|------------------------|------|
| `daly-bms-core`        | Protocole UART, parsing trames, types (`BmsSnapshot`), bus partagé, commandes lecture/écriture, polling |
| `daly-bms-server`      | API Axum (REST + WebSocket) + ring buffer + bridges (MQTT, InfluxDB, Alertes) |
| `daly-bms-cli`         | Outil CLI de diagnostic et contrôle RS485 |

### Flux de données

```
BMS UART ──► daly_bms_core::poll_loop()
                    │
                    ▼  on_snapshot(snap)
             AppState::on_snapshot()
              ┌──────┴──────────────────────┐
              ▼                             ▼
       ring_buffer                  broadcast (WebSocket)
       (3600 snaps/BMS)         ┌────────┼──────────┐
                                ▼        ▼           ▼
                           MqttBridge InfluxBridge AlertEngine
                           (rumqttc)  (influxdb2)  (rusqlite)
```

---

## Gains vs version Python

| Métrique            | Python/FastAPI | Rust/Axum | Gain |
|---------------------|----------------|-----------|------|
| RAM au repos        | 150–300 Mo     | 10–35 Mo  | ÷5–10 |
| CPU polling         | base           | ÷3 à ÷5   |       |
| Latence WebSocket   | base           | ÷5–10     |       |
| Taille binaire      | 150 Mo (venv)  | 12–18 Mo  | ÷10  |
| Démarrage           | ~3 s           | < 150 ms  | ÷20  |
| Sécurité mémoire    | GC Python      | Ownership Rust | Zéro race condition |

---

## Structure du dépôt

```
Daly-BMS-Rust/
├── Cargo.toml                 ← Workspace Rust (résolver v2)
├── Config.toml                ← Fichier de configuration exemple (TOML)
├── Makefile                   ← Commandes build/test/deploy/docker
├── .env                       ← Variables Docker (InfluxDB, Grafana)
├── .gitignore
├── docker-compose.infra.yml   ← Infra Docker (Phase 1)
│
├── crates/
│   ├── daly-bms-core/         ← Bibliothèque protocole
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs       ← DalyError, Result<T>
│   │       ├── types.rs       ← BmsSnapshot, Alarms, SystemData…
│   │       ├── protocol.rs    ← DataId, RequestFrame, checksum
│   │       ├── bus.rs         ← DalyPort (Mutex), DalyBusManager
│   │       ├── commands.rs    ← get_pack_status, get_cell_voltages…
│   │       ├── write.rs       ← set_charge_mos, set_soc, reset_bms
│   │       └── poll.rs        ← poll_loop + backoff + retry
│   │
│   ├── daly-bms-server/       ← Serveur principal
│   │   └── src/
│   │       ├── main.rs        ← Entrypoint, init, spawn tasks
│   │       ├── config.rs      ← AppConfig (TOML → struct)
│   │       ├── state.rs       ← AppState, ring buffer, broadcast
│   │       ├── api/
│   │       │   ├── mod.rs     ← Router Axum (toutes les routes)
│   │       │   ├── system.rs  ← GET /api/v1/system/*
│   │       │   └── bms.rs     ← GET/POST /api/v1/bms/*, WebSocket
│   │       └── bridges/
│   │           ├── mod.rs
│   │           ├── mqtt.rs    ← rumqttc, topics, Venus OS payload
│   │           ├── influx.rs  ← influxdb2-client, batch write
│   │           └── alerts.rs  ← AlertEngine, SQLite, Telegram/SMTP
│   │
│   └── daly-bms-cli/          ← Outil CLI
│       └── src/main.rs        ← clap, sous-commandes
│
├── dashboard/                 ← SPA React (WebSocket /ws/bms/stream)
├── contrib/
│   ├── daly-bms.service       ← Service systemd
│   ├── nginx.conf             ← Reverse proxy nginx
│   ├── install-systemd.sh     ← Script d'installation
│   └── uninstall-systemd.sh
├── docker/                    ← Configs Docker (Mosquitto, Grafana…)
├── docs/
│   ├── Plan.md                ← Plan d'implémentation détaillé
│   ├── JSONData.json          ← Structure de données de référence
│   ├── Daly-UART_485-Communications-Protocol-V1.21-1.pdf
│   └── dalyModbusProtocol.xlsx
└── nanoPi/                    ← Config dbus-mqtt-battery (Venus OS)
    ├── config-bms1.ini
    ├── config-bms2.ini
    └── README.md
```

---

## Prérequis

| Composant | Version | Usage |
|-----------|---------|-------|
| Rust      | 1.80+   | Compilation |
| Docker    | 24+     | Infra (MQTT, InfluxDB, Grafana) |
| Docker Compose v2 | — | `make up` |
| Node.js   | 20 LTS  | Dashboard (dev uniquement) |
| cross     | dernière | Cross-compilation ARM (optionnel) |

**Matériel** : Raspberry Pi CM5 (ou Pi 4/5) + adaptateur USB/RS485
**OS** : Debian Bookworm / Ubuntu 24.04 (aarch64 ou x86_64)
**Permissions** : `sudo usermod -aG dialout $USER`

---

## Démarrage rapide

### Phase 1 — Infrastructure Docker (5 min)

```bash
cp .env.example .env          # adapter les tokens
make up                        # Mosquitto:1883 InfluxDB:8086 Grafana:3001 Node-RED:1880
make ps                        # vérifier l'état
```

### Phase 2 — Configuration

```bash
sudo mkdir -p /etc/daly-bms
sudo cp Config.toml /etc/daly-bms/config.toml
sudo nano /etc/daly-bms/config.toml   # adapter port série + adresses BMS
```

### Phase 3 — Compilation et Lancement

```bash
# Développement (local, sans cross-compile)
make run-debug

# Production sur le Pi (cross-compile)
make build-arm
make deploy PI_HOST=pi@192.168.1.100
```

### Phase 4 — Service systemd

```bash
make install        # copie le binaire + installe daly-bms.service
journalctl -u daly-bms -f
```

---

## API REST — Endpoints

### Système

| Méthode | Endpoint | Description |
|---------|----------|-------------|
| GET | `/api/v1/system/status` | État global (BMS online, polling, version) |
| GET | `/api/v1/config` | Configuration active (sans secrets) |
| GET | `/api/v1/discover` | Découverte live sur le bus RS485 |

### BMS — Lecture

| Méthode | Endpoint | Description |
|---------|----------|-------------|
| GET | `/api/v1/bms/{id}/status` | Snapshot complet (SOC, tension, courant…) |
| GET | `/api/v1/bms/{id}/cells` | Tensions individuelles + delta + équilibrage |
| GET | `/api/v1/bms/{id}/temperatures` | Températures par capteur |
| GET | `/api/v1/bms/{id}/alarms` | Flags d'alarme + `any_alarm` |
| GET | `/api/v1/bms/{id}/mos` | État MOS charge/décharge + cycles |
| GET | `/api/v1/bms/{id}/history` | Ring buffer (jusqu'à 3600 snapshots) |
| GET | `/api/v1/bms/{id}/history/summary` | Statistiques min/max/avg |
| GET | `/api/v1/bms/{id}/export/csv` | Export CSV du ring buffer |
| GET | `/api/v1/bms/compare` | Comparaison côte-à-côte de tous les BMS |

### BMS — Écriture (nécessite `api_key` si configurée)

| Méthode | Endpoint | Description |
|---------|----------|-------------|
| POST | `/api/v1/bms/{id}/mos` | Activer/désactiver MOS charge/décharge |
| POST | `/api/v1/bms/{id}/soc` | Calibrer SOC |
| POST | `/api/v1/bms/{id}/soc/full` | SOC → 100% |
| POST | `/api/v1/bms/{id}/soc/empty` | SOC → 0% |
| POST | `/api/v1/bms/{id}/reset` | Reset BMS (avec `confirm: true`) |

### WebSocket

| Endpoint | Description |
|----------|-------------|
| `/ws/bms/stream` | Tous les BMS, broadcast à chaque cycle |
| `/ws/bms/{id}/stream` | Un seul BMS |

---

## CLI

```bash
# Status complet
daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 status

# Tensions cellules
daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 cells --count 16

# Scanner le bus
daly-bms-cli --port /dev/ttyUSB0 discover --start 1 --end 10

# Polling continu
daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 poll --interval 2

# Activer MOS charge
daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 set-charge-mos --enable

# Calibrer SOC à 80%
daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 set-soc --value 80.0
```

---

## Commandes Make

```bash
make up            # Démarrer Docker
make down          # Arrêter Docker
make build         # Compiler (release, local)
make build-arm     # Cross-compiler pour aarch64
make run           # Lancer le serveur
make run-debug     # Debug (RUST_LOG=debug)
make test          # Tests unitaires
make lint          # Clippy
make fmt           # Format code
make check         # check + fmt + clippy
make deploy        # Cross-compile + deploy SSH sur le Pi
make install       # Installer service systemd
make doc           # Générer et ouvrir la doc Rust
```

---

## Protocole Daly implémenté

### Commandes de lecture

| Data ID | Description | Parsing |
|---------|-------------|---------|
| 0x90 | Tension pack, courant, SOC | uint16/10, offset 30000, uint16/10 |
| 0x91 | Min/max tension cellule + numéro | uint16/1000, octet index |
| 0x92 | Min/max température + capteur | byte-40, octet index |
| 0x93 | État MOS, cycles, capacité résiduelle | bits, uint16, uint32 |
| 0x94 | Nombre cellules, capteurs, état charge | octets |
| 0x95 | Tensions individuelles (3/trame) | uint16/1000, multi-trames |
| 0x96 | Températures individuelles (7/trame) | byte-40, multi-trames |
| 0x97 | Flags équilibrage (48 max) | bits little-endian |
| 0x98 | Alarmes protection (7 octets) | flags |

### Commandes d'écriture

| Data ID | Description |
|---------|-------------|
| 0xD9 | MOS décharge ON/OFF |
| 0xDA | MOS charge ON/OFF |
| 0x21 | Calibration SOC (×10, uint16 BE) |
| 0x00 | Reset BMS |

---

## Alertes configurables

| Règle | Seuil déclenchement | Hysteresis |
|-------|---------------------|------------|
| `cell_ovp` | > 3.60 V | -50 mV |
| `cell_uvp` | < 2.90 V | +50 mV |
| `cell_imbalance` | > 100 mV | -10 mV |
| `soc_low` | < 20% | +5% |
| `soc_critical` | < 10% | +2% |
| `temp_high` | > 45°C | -2°C |
| `high_current` | > 80 A | -5 A |

Notifications : Telegram Bot + SMTP email + journal SQLite.

---

## Dépannage

```bash
# Port série
ls -l /dev/ttyUSB* && groups $USER

# Logs service
journalctl -u daly-bms -f

# Test API
curl http://localhost:8000/api/v1/system/status | jq

# Test WebSocket
wscat -c ws://localhost:8000/ws/bms/stream

# Vérifier InfluxDB
make logs

# Niveau de logs augmenté
RUST_LOG=debug daly-bms-server
```

---

## Roadmap

- [x] Phase 0 : Structure workspace Rust (Cargo.toml, crates)
- [x] Phase 0 : Types de données (BmsSnapshot ↔ JSONData.json)
- [x] Phase 0 : Protocole UART + checksum + tests unitaires
- [x] Phase 0 : API Axum (toutes les routes définies)
- [x] Phase 0 : AppState + ring buffer + broadcast WebSocket
- [x] Phase 0 : Bridges (MQTT, InfluxDB, AlertEngine)
- [x] Phase 0 : CLI (clap, toutes les commandes)
- [x] Phase 1 : Infrastructure Docker
- [ ] Phase 2 : Port série réel + tests sur matériel BMS
- [ ] Phase 2 : Commandes d'écriture activées (MOS, SOC, reset)
- [ ] Phase 2 : Découverte auto + tests d'intégration
- [ ] Phase 3 : Docker complet (binaire Rust + nginx + dashboard)
- [ ] Phase 4 : Dashboard Rust natif (Leptos/Dioxus)
- [ ] Phase 4 : Support Venus OS natif via dbus

---

*Référence protocole : Daly UART/485 Communications Protocol V1.21*
*Runtime : [tokio-serial](https://docs.rs/tokio-serial/latest/tokio_serial/) — [Axum](https://docs.rs/axum/) — [rumqttc](https://docs.rs/rumqttc/)*
