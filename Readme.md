# Daly-BMS — Rust Edition

**Version Rust complète** — mise à jour 17 mars 2026
Remplacement total de la stack Python/FastAPI par **Rust** (workspace multi-crates : `daly-bms-core` + `daly-bms-server` + `daly-bms-cli` + `daly-bms-probe` + `santuario-venus-bridge`).

> Dashboard intégré **SSR Rust** (Askama + ECharts) — aucun npm, aucun React.
> Infrastructure Docker **inchangée** (Mosquitto, InfluxDB, Grafana, Node-RED).
> Déploiement ultra-léger : **un seul binaire statique** (~12–18 Mo).
> Compatible **Windows** (testé) et **Linux/aarch64** (Raspberry Pi).

---

**Matériel de production** : Raspberry Pi Compute Module 5 Wireless, 4 Go RAM, 32 Go eMMC, Raspberry Pi OS Lite (64-bit)

---

## Architecture globale

### Vue d'ensemble infrastructure

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Raspberry Pi 5 CM  (Master)                      │
│                                                                     │
│  RS485 Bus 1 ── BMS Pack A (0x01, 360Ah)                           │
│  RS485 Bus 1 ── BMS Pack B (0x02, 320Ah)  ──► daly-bms-server     │
│  RS485 Bus 2 ── [TODO] Irradiance / Météo  ──► daly-bms-solar      │
│  RS485 Bus 3 ── [TODO] ATS (bascule réseau/Victron/grid)           │
│  API LG Cloud ─ FAIT PAC chauffe-eau     ──► daly-bms-heatpump   │
│  API LG Cloud ─ [TODO] PAC climatisation   ──► daly-bms-heatpump   │
│                                │                                    │
│                         Mosquitto :1883                             │
│                                │                                    │
│    ┌──────────────┬────────────┴──────────┬────────────────────┐   │
│    ▼              ▼                       ▼                    ▼   │
│ InfluxDB       AlertEngine           Node-RED              Dashboard│
│ :8086          (SQLite)              :1880 (Pi5)            SSR     │
│    │                                                               │
│ Grafana :3001                                                       │
└────────────────────────────────┬────────────────────────────────────┘
                                 │ MQTT
                                 ▼
┌────────────────────────────────────────────────────────────────────┐
│                  NanoPi Neo3  (Venus OS — D-Bus bridge)            │
│                                                                    │
│   santuario-venus-bridge ◄── MQTT santuario/bms/{n}/venus         │
│   (+ solar, heat, ats)  ◄── MQTT santuario/solar/{n}/venus        │
│                         ◄── MQTT santuario/heat/{n}/venus         │
│                         ◄── MQTT santuario/ats/venus              │
│                │                                                   │
│                ▼  D-Bus (zbus / pur Rust)                          │
│   com.victronenergy.battery.*        (BMS × 2)                    │
│   com.victronenergy.meteo.*          [TODO]                        │
│   com.victronenergy.temperature.*   Fait                        │
│   com.victronenergy.grid.*          [TODO] ATS                    │
│                │                                                   │
│                ▼                                                   │
│   systemcalc-py ── VRM Portal ── Venus GUI                        │
│   hub4-control  (DVCC charge/discharge)                           │
└────────────────────────────────────────────────────────────────────┘
```

> **Adresses BMS production** : `0x01` (BMS-360Ah) et `0x02` (BMS-320Ah)
> **Validé en production** sur RPi5 au 17 mars 2026 — données confirmées dans InfluxDB.
> **Pi5 = master** : tous les capteurs RS485 y sont connectés, le NanoPi reste dédié Venus OS / D-Bus.

### Flux MQTT par domaine

| Topic MQTT | Source (Pi5) | Bridge NanoPi | Cible D-Bus Venus |
|---|---|---|---|
| `santuario/bms/{n}/venus` | `daly-bms-server` ✅ | `santuario-venus-bridge` ✅ | `com.victronenergy.battery.{n}` |
| `santuario/solar/{n}/venus` | `santuario-solar` 🔜 | `santuario-venus-bridge` 🔜 | `com.victronenergy.meteo.{n}` |
| `santuario/meteo/venus` | `santuario-solar` 🔜 | `santuario-venus-bridge` 🔜 | `com.victronenergy.meteo` |
| `santuario/heat/{n}/venus` | `santuario-heatpump` ✅ | `santuario-venus-bridge` ✅ | `com.victronenergy.temperature.{n}` |
| `santuario/ats/venus` | `santuario-ats` 🔜 | `santuario-venus-bridge` 🔜 | `com.victronenergy.grid` |

> `santuario-venus-bridge` est le **seul binaire sur le NanoPi** — il souscrit à tous les topics
> et enregistre tous les services D-Bus. Un seul processus, ~5–8 Mo RAM.

### Rôle de chaque service

| Service | Hôte | Port | Rôle |
|---------|------|------|------|
| **daly-bms-server** | Pi5 | 8080 | Serveur principal Rust : polling RS485, REST API, WebSocket, Dashboard SSR |
| **Mosquitto** | Pi5 | 1883 (MQTT), 9001 (WS) | Broker MQTT — relaye toutes les données capteurs vers Venus OS et Node-RED |
| **InfluxDB** | Pi5 | 8086 | Base de données séries temporelles — stockage 30 jours de métriques |
| **Grafana** | Pi5 | 3001 | Visualisation — dashboards temps réel + historique (provisionné automatiquement) |
| **Node-RED** | Pi5 | 1880 | Automatisation — flows MQTT, alertes, webhooks (migré NanoPi → Pi5) |
| **santuario-venus-bridge** | NanoPi | — | Bridge MQTT → D-Bus Venus OS (Rust pur, zbus) — unique binaire sur NanoPi, enregistre tous les capteurs sur Venus |

> **Note architecture** : Le Pi5 est le **master** de tous les capteurs RS485 et API cloud.
> Le NanoPi reste dédié à Venus OS et héberge uniquement `santuario-venus-bridge` (Rust statique musl, ~5 Mo).
> Node-RED a été migré du NanoPi vers le Pi5 pour consolider l'infrastructure.

---

## Flux de données détaillé

### BMS (implémenté)

```
BMS UART  ──► daly_bms_core::poll_loop()   ← mode hardware (RS485/USB)
Simulateur ──► run_simulator()              ← mode --simulate (sans matériel)
                    │
                    ▼  on_snapshot(snap)
             AppState::on_snapshot()
              ┌──────┴──────────────────────────┐
              ▼                                 ▼
       ring_buffer                      broadcast (tokio)
       (3600 snaps/BMS)         ┌────────┬──────┴──────┬───────────┐
                                ▼        ▼              ▼           ▼
                           MqttBridge InfluxBridge AlertEngine WebSocket
                           (rumqttc)  (influxdb2)  (rusqlite)  (/ws/bms/*)
                               │           │
                               ▼           ▼
                          Mosquitto     InfluxDB
                               │           │
             santuario-venus-bridge   Grafana
             com.victronenergy.battery.*
                  (Venus OS / NanoPi)
```

### Capteurs à venir (architecture cible)

```
RS485 Bus 2 ──► [TODO] daly-bms-solar::poll_loop()
                              │
                         MqttBridge ──► santuario/solar/{n}/venus
                                               │
                         santuario-venus-bridge (extension solar)
                              com.victronenergy.meteo.*

LG ThinQ API ──► [TODO] daly-bms-heatpump::lg_cloud_poll()
(PAC chauffe-eau + clim)      │
                         MqttBridge ──► santuario/heat/{n}/venus
                                               │
                         santuario-venus-bridge (extension heat)
                              com.victronenergy.temperature.*

RS485 Bus 3 ──► [TODO] daly-bms-ats::poll_loop()
                              │
                         MqttBridge ──► santuario/ats/venus
                         + commandes ◄── (bascule maison/grid/Victron)
                                               │
                         santuario-venus-bridge (extension ats)
                              com.victronenergy.grid.*
```

---

## Workspace Rust

### Convention de nommage

| Préfixe | Scope | Exemples |
|---|---|---|
| `daly-bms-*` | Spécifique au protocole / matériel Daly | `daly-bms-core`, `daly-bms-cli`, `daly-bms-probe` |
| `santuario-*` | Services du projet (indépendants du matériel) | `santuario-bms`, `santuario-solar`, `santuario-venus-bridge` |

> `daly-bms-core` garde son nom : c'est une bibliothèque liée à la **marque et au protocole**.
> Tous les nouveaux **services** (binaires) adoptent le préfixe `santuario-`.

### Crates du workspace

| Crate / Binaire | Hôte | Statut | Rôle |
|---|---|---|---|
| `daly-bms-core` | — | ✅ Production | Lib : protocole UART Daly, parsing trames, types (`BmsSnapshot`), polling |
| `daly-bms-server` | Pi5 | ✅ Production | Binaire Pi5 : API Axum (REST + WebSocket) + Dashboard SSR + bridges (MQTT, InfluxDB, Alertes) |
| `santuario-venus-bridge` | NanoPi | ✅ Production | Binaire NanoPi : MQTT → D-Bus Venus OS (zbus pur Rust) — tous les services `com.victronenergy.*` |
| `daly-bms-cli` | Pi5 | ✅ Stable | Outil CLI de diagnostic et contrôle RS485 |
| `daly-bms-probe` | Pi5 | ✅ Stable | Outil diagnostic bas niveau — trames brutes, test 3 variantes d'adressage |
| `santuario-core` | — | 🔜 TODO | Lib : traits partagés `DevicePoller`, `VenusPayload`, `MqttPublisher` |
| `santuario-solar` | Pi5 | 🔜 TODO | Binaire Pi5 : polling RS485 irradiance + météo → MQTT |
| `santuario-heatpump` | Pi5 |✅ | Binaire Pi5 : API LG ThinQ (PAC chauffe-eau + clim) → MQTT |
| `santuario-ats` | Pi5 | 🔜 TODO | Binaire Pi5 : ATS RS485 (bascule maison/grid/Victron) → MQTT |

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
├── Cargo.toml                 ← Workspace Rust (résolver v2, Rust 1.88+)
├── Cargo.lock
├── Config.toml                ← Configuration principale (TOML)
├── Config.docker.toml         ← Configuration Docker (hostnames internes)
├── Makefile                   ← Commandes build/test/deploy/docker
├── Dockerfile                 ← Image Docker multi-stage (builder + runtime)
├── docker-compose.yml         ← Stack complète (serveur + infra)
├── docker-compose.infra.yml   ← Infra seule (Mosquitto, InfluxDB, Grafana, Node-RED)
├── .env.docker                ← Template variables d'environnement (à copier en .env)
├── .env                       ← Variables secrètes Docker (gitignored)
├── .gitignore
│
├── crates/
│   ├── daly-bms-core/         ← Bibliothèque protocole
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs       ← DalyError, Result<T>
│   │       ├── types.rs       ← BmsSnapshot, Alarms, SystemData…
│   │       ├── protocol.rs    ← DataId, RequestFrame, checksum + 7 tests unitaires
│   │       ├── bus.rs         ← DalyPort (Mutex), DalyBusManager
│   │       ├── commands.rs    ← get_pack_status, get_cell_voltages…
│   │       ├── write.rs       ← set_charge_mos, set_soc, reset_bms
│   │       └── poll.rs        ← poll_loop + backoff + retry
│   │
│   ├── daly-bms-server/       ← Serveur principal
│   │   └── src/
│   │       ├── main.rs        ← Entrypoint, CLI flags (--simulate, --port, --bms…)
│   │       ├── config.rs      ← AppConfig (TOML → struct), per-BMS config
│   │       ├── state.rs       ← AppState, ring buffer, broadcast channel
│   │       ├── simulator.rs   ← Simulateur BMS (physique LiFePO4, sans matériel)
│   │       ├── autodetect.rs  ← Détection automatique port série + adresses BMS
│   │       ├── api/
│   │       │   ├── mod.rs     ← Router Axum (toutes les routes)
│   │       │   ├── system.rs  ← GET /api/v1/system/*
│   │       │   └── bms.rs     ← GET/POST /api/v1/bms/*, WebSocket
│   │       ├── bridges/
│   │       │   ├── mod.rs
│   │       │   ├── mqtt.rs    ← rumqttc, topics, Venus OS payload
│   │       │   ├── influx.rs  ← influxdb2-client, batch write
│   │       │   └── alerts.rs  ← AlertEngine, SQLite, Telegram/SMTP
│   │       └── dashboard/
│   │           ├── mod.rs     ← Routes /dashboard, templates Askama
│   │           └── charts.rs  ← Génération JSON ECharts (boxplot, séries…)
│   │
│   ├── santuario-venus-bridge/ ← Binaire NanoPi : MQTT → D-Bus (tous devices)
│   │   └── src/
│   │       ├── main.rs         ← Entrypoint, tokio runtime
│   │       ├── config.rs       ← VenusConfig (TOML)
│   │       ├── types.rs        ← VenusPayload (serde_json)
│   │       ├── mqtt_source.rs  ← Subscriber rumqttc (tous topics santuario/*)
│   │       ├── battery_service.rs ← com.victronenergy.battery.* (zbus)
│   │       └── manager.rs      ← Orchestration, watchdog, keepalive
│   │   [à venir]
│   │       ├── solar_service.rs   ← com.victronenergy.meteo.*
│   │       ├── heat_service.rs    ← com.victronenergy.temperature.*
│   │       └── ats_service.rs     ← com.victronenergy.grid.*
│   │
│   ├── daly-bms-cli/          ← Outil CLI
│   │   └── src/main.rs        ← clap, sous-commandes
│   │
│   └── daly-bms-probe/        ← Outil diagnostic bas niveau
│       └── src/main.rs        ← Trames brutes, test 3 variantes d'adressage
│
├── contrib/
│   ├── daly-bms.service       ← Service systemd
│   ├── nginx.conf             ← Reverse proxy nginx
│   ├── install-systemd.sh     ← Script d'installation systemd
│   └── uninstall-systemd.sh   ← Script de désinstallation
├── docker/
│   └── mosquitto/config/
│       └── mosquitto.conf     ← Configuration broker MQTT
├── grafana/
│   └── provisioning/
│       ├── dashboards/        ← Dashboard Grafana (JSON + provider.yaml)
│       └── datasources/       ← Datasource InfluxDB (provisionné auto)
├── docs/
│   ├── Plan.md                ← Plan d'implémentation détaillé (v2.2)
│   ├── JSONData.json          ← Structure de données de référence
│   ├── Daly-UART_485-Communications-Protocol-V1.21-1.pdf
│   └── dalyModbusProtocol.xlsx
└── nanoPi/                    ← Config dbus-mqtt-battery (Venus OS)
    ├── config-bms1.ini        ← Instance dbus-mqtt-battery-41 (BMS 0x01)
    ├── config-bms2.ini        ← Instance dbus-mqtt-battery-42 (BMS 0x02)
    └── README.md              ← Guide installation Venus OS
```

### Estimation mémoire

#### Pi5 (master — Docker)

| Service                    | RAM minimale | RAM confortable |
|----------------------------|-------------|-----------------|
| daly-bms-server (Rust)     | ~25 MB      | ~50 MB          |
| Mosquitto                  | ~12 MB      | ~20 MB          |
| InfluxDB 2.x (Go)          | ~200 MB     | ~350 MB         |
| Grafana                    | ~120 MB     | ~200 MB         |
| Node-RED (Node.js)         | ~150 MB     | ~250 MB         |
| OS Raspberry Pi OS Lite    | ~150 MB     | ~200 MB         |
| Docker Engine + overhead   | ~100 MB     | ~150 MB         |
| Marge tampon / cache       | ~200 MB     | ~400 MB         |
| **TOTAL**                  | **~957 MB** | **~1420 MB**    |

#### NanoPi Neo3 (Venus OS — services Rust statiques)

| Service                    | RAM minimale | Notes |
|----------------------------|-------------|-------|
| santuario-venus-bridge     | ~5–8 MB     | Binaire statique musl, zéro dépendance système |
| Venus OS + systemcalc-py   | ~150 MB     | Existant |
| **TOTAL ajouté**           | **~5 MB**   | Impact négligeable |

---

## Prérequis

| Composant | Version | Usage |
|-----------|---------|-------|
| Rust      | 1.80+   | Compilation |
| Docker    | 24+     | Infra (MQTT, InfluxDB, Grafana) |
| Docker Compose v2 | — | `make up` |
| cross     | dernière | Cross-compilation ARM (optionnel) |

> Le dashboard est **SSR (Askama + ECharts)** — Node.js/npm ne sont plus nécessaires.

**Matériel** : Raspberry Pi CM5 (ou Pi 4/5) + adaptateur USB/RS485
**OS** : Debian Bookworm / Ubuntu 24.04 (aarch64 ou x86_64), **Windows 10/11 supporté**
**Permissions Linux** : `sudo usermod -aG dialout $USER`

### Compatibilité multi-plateforme

| Plateforme | Statut | Notes |
|---|---|---|
| Windows 10/11 (x86_64) | ✅ Testé | Port COMx, auto-détection |
| Linux x86_64 | ✅ Compilé | `/dev/ttyUSB0` |
| Raspberry Pi 5 / CM5 (aarch64) | ✅ Validé production | Cross-compile ou natif |
| Cerbo GX / NanoPi Venus OS | N/A | Sert le MQTT, ne fait pas tourner le serveur |

---

## Démarrage rapide

### Mode simulateur (sans matériel BMS — Windows ou Linux)

```bash
# Compiler
cargo build --release

# Lancer avec 2 BMS simulés
cargo run --bin daly-bms-server -- --simulate --sim-bms 0x01,0x02

# Ou avec Make
make run-simulate

# Accéder au dashboard
# http://localhost:8080/dashboard
```

### Infrastructure Docker (5 min)

```bash
cp .env.docker .env            # adapter les tokens et mots de passe
make up                        # Mosquitto:1883 InfluxDB:8086 Grafana:3001 Node-RED:1880
make ps                        # vérifier l'état des containers
```

### Configuration

```bash
sudo mkdir -p /etc/daly-bms
sudo cp Config.toml /etc/daly-bms/config.toml
sudo nano /etc/daly-bms/config.toml   # adapter port série + adresses BMS
```

### Compilation et Lancement (hardware réel)

```bash
# Développement (local)
make run-debug

# Production sur le Pi (cross-compile)
make build-arm
make deploy PI_HOST=pi@192.168.1.100
```

### Service systemd (Linux/RPi5)

```bash
make install        # copie le binaire + installe daly-bms.service
journalctl -u daly-bms -f
```

### Stack Docker complète (serveur + infra)

```bash
# Démarrer toute la stack (y compris daly-bms-server en container)
docker compose up -d

# Ou uniquement l'infra (pour développement local Rust)
make up   # utilise docker-compose.infra.yml
```

---

## Dashboard intégré

Le dashboard est **embarqué dans le binaire** (SSR Askama + ECharts). Aucun npm, aucun serveur web séparé.

| URL | Description |
|-----|-------------|
| `http://localhost:8080/dashboard` | Vue synthèse de tous les BMS |
| `http://localhost:8080/dashboard/bms/1` | Détail BMS (cellules, températures, historique) |

**Fonctionnalités :**
- Cartes par BMS : SOC, tension, courant, température, puissance
- Boxplot tensions cellules (min/max/avg) avec colorisation
- Indicateur équilibrage actif (cellules hautes/basses)
- Profil températures
- Historique temps réel (ring buffer 3600 snapshots)
- Thème clair, badge RS485 multi-BMS
- Noms personnalisés par BMS (`name = "BMS-360Ah"`)

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

# Tensions cellules (16 cellules)
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
make up            # Démarrer Docker (infra seule)
make down          # Arrêter Docker
make restart       # Redémarrer les containers
make ps            # État des containers
make logs          # Logs de tous les containers (follow)
make reset         # Arrêter + supprimer volumes + redémarrer (reset complet)
make reset-influx  # Purger uniquement les données InfluxDB

make build         # Compiler (release, local)
make build-arm     # Cross-compiler pour aarch64 (RPi)
make build-all     # Tous les binaires
make run           # Lancer le serveur
make run-debug     # Debug (RUST_LOG=debug)
make run-simulate  # Mode simulateur (sans matériel)
make test          # Tests unitaires
make test-core     # Tests protocole uniquement
make lint          # Clippy
make fmt           # Format code
make check         # check + fmt + clippy
make deploy        # Cross-compile + deploy SSH sur le Pi
make install       # Installer service systemd
make doc           # Générer et ouvrir la doc Rust
```

---

## Gestion des logs et rétention des données

### Logs Docker (rotation automatique)

Tous les containers utilisent le driver `json-file` avec rotation automatique :

| Service | Max taille | Fichiers |
|---------|-----------|---------|
| dalybms-server | 20 Mo | 5 fichiers |
| Mosquitto | 10 Mo | 3 fichiers |
| InfluxDB | 10 Mo | 3 fichiers |
| Grafana | 10 Mo | 3 fichiers |
| Node-RED | 10 Mo | 3 fichiers |

```bash
# Voir les logs en temps réel
make logs

# Logs d'un service spécifique
docker logs dalybms-server -f --tail 100
docker logs dalybms-influxdb -f --tail 100
docker logs dalybms-grafana -f --tail 100
docker logs dalybms-mosquitto -f --tail 100

# Taille des fichiers log Docker
du -sh /var/lib/docker/containers/*/
```

### Logs systemd (déploiement sans Docker)

```bash
# Logs en temps réel
journalctl -u daly-bms -f

# Logs depuis une date
journalctl -u daly-bms --since "2026-03-17 00:00:00"

# Taille du journal systemd
journalctl --disk-usage

# Limiter la rétention (dans /etc/systemd/journald.conf)
# SystemMaxUse=200M
# MaxRetentionSec=7day
sudo systemctl restart systemd-journald

# Purger manuellement les anciens logs
sudo journalctl --vacuum-time=7d
sudo journalctl --vacuum-size=100M
```

### Rétention des données InfluxDB

Par défaut : **30 jours** (720h), configurable dans `.env` :

```bash
# Dans .env
DOCKER_INFLUXDB_INIT_RETENTION=720h    # 30 jours (défaut)
# DOCKER_INFLUXDB_INIT_RETENTION=2160h  # 90 jours
# DOCKER_INFLUXDB_INIT_RETENTION=0      # Infini (déconseillé sur SD/eMMC)
```

```bash
# Purger uniquement les données InfluxDB (garde la configuration)
make reset-influx

# Vérifier l'espace disque utilisé par InfluxDB
docker exec dalybms-influxdb du -sh /var/lib/influxdb2/

# Interroger InfluxDB directement
docker exec dalybms-influxdb influx query \
  'from(bucket:"daly_bms") |> range(start: -5m) |> limit(n:5)' \
  --org santuario
```

### Nettoyage complet (reset usine)

```bash
# Arrêter tout + supprimer tous les volumes (DONNÉES PERDUES)
make reset

# Reset uniquement InfluxDB (garde Grafana, MQTT…)
make reset-influx

# Nettoyer les images Docker inutilisées
docker system prune -f

# Libérer l'espace disque Docker (images + caches)
docker system prune -a --volumes
```

> **Note RPi/eMMC** : Sur Raspberry Pi avec carte SD ou eMMC, surveiller l'espace disque.
> InfluxDB peut écrire ~50–200 Mo/jour selon la fréquence de polling et le nombre de BMS.
> La rétention 30j = environ 1,5–6 Go max.

---

## Simulateur BMS

Le mode simulateur génère des données **LiFePO4 réalistes** sans matériel :

```bash
# 1 BMS simulé (adresse 0x01 par défaut)
cargo run --bin daly-bms-server -- --simulate

# 2 BMS simulés (adresses 0x01 et 0x02 comme en production)
cargo run --bin daly-bms-server -- --simulate --sim-bms 0x01,0x02
```

**Physique simulée :**
- SOC : courbe de décharge, recharge automatique à 10%, cycle à 95%
- Tension : courbe OCV LiFePO4 (44V vide → 58,4V plein pour 16 cellules)
- Courant : variation sinusoïdale autour de -8,5 A (décharge)
- Température : corrélée au courant + dérive ambiante
- Tensions cellules : déséquilibre réaliste (-15 à +15 mV par cellule)
- Équilibrage : activé automatiquement quand delta > 10 mV
- Alarmes : déclenchées sur seuils SOC/delta

Le simulateur alimente les mêmes bridges que le hardware réel : **MQTT, InfluxDB, AlertEngine, WebSocket, Dashboard**.

---

## Protocole Daly implémenté

### Format trame (13 octets)

```
┌──────┬──────┬──────────┬──────────────────────────────┬──────────┐
│ 0xA5 │ ADDR │ DATA_ID  │ DATA (8 octets, 0x00 lecture)│ CHECKSUM │
└──────┴──────┴──────────┴──────────────────────────────┴──────────┘
  1B     1B     1B          8B                              1B
```
- Baud rate : 9600
- Checksum : somme des octets (modulo 256)

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
sudo usermod -aG dialout $USER  # si permission refusée

# Logs service systemd
journalctl -u daly-bms -f

# Logs Docker
make logs
docker logs dalybms-server -f --tail 100

# Test API
curl http://localhost:8080/api/v1/system/status | jq

# Test WebSocket
wscat -c ws://localhost:8080/ws/bms/stream

# Vérifier les données InfluxDB (5 dernières minutes)
docker exec dalybms-influxdb influx query \
  'from(bucket:"daly_bms") |> range(start: -5m) |> limit(n:3)' \
  --org santuario

# Niveau de logs augmenté
RUST_LOG=debug daly-bms-server

# Diagnostic bas niveau (trame brute RS485)
cargo run --bin daly-bms-probe -- --port /dev/ttyUSB0

# Vérifier état containers
make ps
docker compose -f docker-compose.infra.yml ps

# Redémarrer un container spécifique
docker compose -f docker-compose.infra.yml restart grafana
docker compose -f docker-compose.infra.yml restart dalybms-influxdb
```

---

## Configuration Grafana / InfluxDB

### Accès initial

| Service | URL | Identifiants par défaut |
|---------|-----|------------------------|
| InfluxDB | `http://RPi5:8086` | admin / voir `.env` |
| Grafana | `http://RPi5:3001` | admin / voir `.env` |
| Node-RED | `http://RPi5:1880` | aucun (à sécuriser si exposé) |

> **Après un `make reset`** : utiliser l'URL de base sans chemin (ex. `http://192.168.1.141:8086`).
> L'ancien org ID dans l'URL bookmarkée devient invalide — se reconnecter depuis la page d'accueil.

### Datasource Grafana (provisionné automatiquement)

Le fichier `grafana/provisioning/datasources/influxdb.yaml` configure automatiquement
la connexion InfluxDB au démarrage de Grafana. Aucune configuration manuelle requise.

### Dashboard Grafana

Le dashboard `DalyBMS — Vue d'ensemble` est provisionné depuis :
`grafana/provisioning/dashboards/bms-overview.json`

Il affiche pour chaque BMS :
- SOC (gauge), tension pack, courant, puissance
- Température max cellules, delta cellules (déséquilibre), état MOS
- Séries temporelles : SOC, tension, courant, puissance
- Historique 15 min (auto-refresh 10s)

---

## Roadmap

### Phase 0 — Fondations Rust ✅

- [x] Structure workspace Rust (Cargo.toml, 5 crates)
- [x] Types de données (BmsSnapshot ↔ JSONData.json)
- [x] Protocole UART + checksum + tests unitaires
- [x] API Axum (toutes les routes définies)
- [x] AppState + ring buffer + broadcast WebSocket
- [x] Bridges (MQTT, InfluxDB, AlertEngine)
- [x] CLI (clap, toutes les commandes)
- [x] Outil probe (diagnostic bas niveau)

### Phase 1 — Infrastructure & Intégration ✅

- [x] Infrastructure Docker (Mosquitto, InfluxDB, Grafana, Node-RED)
- [x] Docker complet (Dockerfile + docker-compose.yml stack complète)
- [x] Simulateur BMS avec physique LiFePO4 (validé Windows + Linux)
- [x] Auto-détection port série et adresses BMS
- [x] Dashboard SSR intégré (Askama + ECharts, sans npm)
- [x] MQTT publish_interval_sec réduit à 1s (temps réel)
- [x] Architecture Venus OS confirmée (MQTT → D-Bus)
- [x] Service dbus-canbattery.can0 stoppé sur NanoPi (CAN remplacé par MQTT)
- [x] Compatibilité Windows 10/11 validée

### Phase 2 — Production RPi5 ✅

- [x] RPi5 CM opérationnel — données BMS 0x01 et 0x02 confirmées dans InfluxDB
- [x] Dashboard Grafana fonctionnel en production (17 mars 2026)
- [x] Correction adresses BMS (0x28/0x29 → 0x01/0x02)
- [x] Rotation logs Docker configurée + rétention InfluxDB 30j
- [ ] Validation commandes d'écriture (MOS, SOC, reset) sur hardware réel
- [ ] Tests intégration 24h stabilité

### Phase 3 — Venus OS natif Rust ✅

- [x] Crate `santuario-venus-bridge` : bridge MQTT → D-Bus (zbus pur Rust, sans libdbus)
- [x] Enregistrement `com.victronenergy.battery.*` sur le bus système Venus OS
- [x] Interface `com.victronenergy.BusItem` (GetValue, GetText, SetValue, ItemsChanged)
- [x] Watchdog MQTT (déconnexion propre si source silencieuse > 30s)
- [x] Keepalive D-Bus (republication toutes les 25s)
- [x] Remplacement de `dbus-mqtt-battery` Python par du Rust pur sur le NanoPi
- [x] Décision architecture : binaire unique `santuario-venus-bridge` sur NanoPi pour tous les devices futurs

### Phase 4 — Migration & Consolidation 🚧

- [ ] Renommer le crate `daly-bms-venus` → `santuario-venus-bridge` dans le workspace Rust
- [ ] Migration flows Node-RED du NanoPi vers le Pi5 (docker-compose.infra.yml)
- [ ] Nettoyage NanoPi : services Python retirés, seul `santuario-venus-bridge` reste
- [ ] Validation stabilité 24h post-migration Node-RED

### Phase 5 — Capteur Irradiance & Météo RS485 🔜

> Objectif : corréler la production PV avec l'ensoleillement et les conditions météo

- [ ] Identifier le modèle exact du capteur (protocole Modbus RTU, registres)
- [ ] Créer crate `santuario-solar` (polling RS485, types `SolarSnapshot`, `MeteoSnapshot`)
- [ ] Bridge MQTT : topics `santuario/solar/{n}/venus` et `santuario/meteo/venus`
- [ ] Extension `santuario-venus-bridge` : `solar_service.rs` → `com.victronenergy.meteo.*`
- [ ] Dashboard Grafana : irradiance vs production Victron (corrélation)
- [ ] Alertes : nuages / ombrage détecté (irradiance < seuil)

### Phase 6 — Pompe à Chaleur Chauffe-Eau LG ✅

> Objectif : optimiser le chauffe-eau via surplus PV + monitoring consommation

- [ ] Étudier l'API LG ThinQ / LG SmartThinQ (authentification OAuth2, endpoints)
- [ ] Créer crate `santuario-heatpump` (poller API LG ThinQ toutes les 60s)
- [ ] Types : `HeatPumpSnapshot` (consigne, temp eau, mode, conso instantanée, COP)
- [ ] Bridge MQTT : `santuario/heat/dhw/venus` (Domestic Hot Water)
- [ ] Extension `santuario-venus-bridge` : `heat_service.rs` → `com.victronenergy.temperature.dhw`
- [ ] Commandes : activation / consigne depuis Venus OS (DVCC surplus PV → chauffe)
- [ ] Alertes : température eau hors plage, défaut PAC

### Phase 7 — Pompe à Chaleur Climatisation LG 🔜

> Objectif : monitoring clim + potentiel pilotage depuis surplus PV

- [ ] Évaluation intégration LG Multi-Split (même API ThinQ ou Modbus local ?)
- [ ] Types : `AcSnapshot` (mode, consigne, temp ambiante, conso)
- [ ] Bridge MQTT : `santuario/heat/ac/{zone}/venus`
- [ ] Extension `santuario-venus-bridge` : `com.victronenergy.temperature.ac_{zone}`
- [ ] Définir stratégie : API cloud vs Modbus local (à étudier selon le modèle)

### Phase 8 — ATS (Commutateur de Source Automatique) RS485 🔜

> Objectif : bascule automatique entre réseau EDF / groupe / Victron Multiplus

- [ ] Identifier le modèle ATS et son protocole RS485 (Modbus RTU probable)
- [ ] Créer crate `santuario-ats` (polling état + commandes bascule)
- [ ] Types : `AtsSnapshot` (source active, tensions, fréquence, défauts)
- [ ] Bridge MQTT : `santuario/ats/venus` + commandes `santuario/ats/cmd`
- [ ] Extension `santuario-venus-bridge` : `ats_service.rs` → `com.victronenergy.grid`
- [ ] Intégration Venus OS : systemcalc voit l'ATS comme source grid
- [ ] Logique automatique : surplus PV → bascule Victron, nuit/nuage → grid

### Vision long terme 🔭

- [ ] Crate `santuario-core` : trait `DevicePoller` + `VenusPayload` partagés par tous les services
- [ ] Configuration dynamique : ajout capteur sans recompilation (TOML hot-reload)
- [ ] Dashboard SSR unifié : toutes les sources dans un seul écran
- [ ] Alertes corrélées : ex. "irradiance haute mais production faible → ombrage détecté"
- [ ] Export Home Assistant via MQTT Discovery (alternative Venus OS pour certains capteurs)

---

*Référence protocole : Daly UART/485 Communications Protocol V1.21*
*Runtime : [tokio-serial](https://docs.rs/tokio-serial/latest/tokio_serial/) — [Axum](https://docs.rs/axum/) — [rumqttc](https://docs.rs/rumqttc/)*
