# Plan d'Implémentation — DalyBMS Rust Edition

**Version** : 2.2
**Date** : 17 Mars 2026
**Référence Python** : [thieryus007-cloud/Daly-BMS](https://github.com/thieryus007-cloud/Daly-BMS)
**Dépôt Rust** : [thieryus007-cloud/Daly-BMS-Rust](https://github.com/thieryus007-cloud/Daly-BMS-Rust)

---

## Table des matières

1. [Objectifs et périmètre](#1-objectifs-et-périmètre)
2. [Architecture technique](#2-architecture-technique)
3. [Structure du workspace](#3-structure-du-workspace)
4. [Protocole Daly UART](#4-protocole-daly-uart)
5. [Structures de données](#5-structures-de-données)
6. [Plan d'implémentation par phases](#6-plan-dimplémentation-par-phases)
7. [API REST et WebSocket](#7-api-rest-et-websocket)
8. [Bridges (MQTT, InfluxDB, Alertes)](#8-bridges-mqtt-influxdb-alertes)
9. [Tests et validation](#9-tests-et-validation)
10. [Déploiement et opérations](#10-déploiement-et-opérations)

---

## 1. Objectifs et périmètre

### 1.1 Objectif principal

Réécrire **entièrement** le projet Python Daly-BMS en Rust, en conservant :
- **100% des fonctionnalités** du projet Python (protocole, API, bridges, alertes)
- **La même infrastructure Docker** (Mosquitto, InfluxDB, Grafana, Node-RED)
- **Le même dashboard React** (WebSocket compatible)
- **La même intégration Venus OS** (dbus-mqtt-battery via MQTT)

### 1.2 Gains attendus

| Métrique | Python | Rust | Facteur |
|----------|--------|------|---------|
| RAM | 150–300 Mo | 10–35 Mo | ÷5–10 |
| CPU polling | ref | ref/3–5 | ÷3–5 |
| Latence WS/API | ref | ref/5–10 | ÷5–10 |
| Taille déploiement | ~150 Mo | ~15 Mo | ÷10 |
| Démarrage | ~3 s | < 150 ms | ÷20 |
| Sécurité mémoire | GC | Ownership | ∞ |

### 1.3 Contraintes

- Même matériel : Raspberry Pi CM5 + convertisseur USB/RS485
- Même bus RS485 partagé entre 2–32 BMS (séquentiel)
- Même format JSON des snapshots (compatible `JSONData.json`)
- API REST et WebSocket identiques (dashboard React inchangé)
- Cross-compilation `aarch64-unknown-linux-gnu`

---

## 2. Architecture technique

### 2.1 Stack Rust

```
[daly-bms-server]           Binaire principal
    ├── Axum 0.7            HTTP server (API REST + WebSocket)
    ├── Tower               Middleware (CORS, rate-limit, auth)
    ├── rumqttc 0.24        Client MQTT asynchrone
    ├── influxdb2 0.5       Client InfluxDB v2
    ├── rusqlite 0.31       SQLite (journal alertes) — bundled
    ├── reqwest 0.12        HTTP client (Telegram notifications)
    └── lettre 0.11         Email SMTP

[daly-bms-core]             Bibliothèque protocole
    ├── tokio-serial 5.5    Port série asynchrone
    ├── tokio 1             Runtime async
    └── serde 1             Sérialisation JSON/TOML

[daly-bms-cli]              Outil diagnostic
    └── clap 4              Parsing arguments CLI
```

### 2.2 Modèle de concurrence

```
Thread principal (Tokio runtime)
├── Tâche poll_loop()       → tokio::spawn (polling RS485)
├── Tâche MQTT bridge       → tokio::spawn (interval 5s)
├── Tâche InfluxDB bridge   → tokio::spawn (batch flush)
├── Tâche AlertEngine       → tokio::spawn (rx broadcast)
└── Axum serve()            → listeners HTTP/WS

Communication interne :
├── AppState.buffers        → Arc<RwLock<BTreeMap<u8, RingBuffer>>>
└── AppState.ws_tx          → broadcast::Sender<Arc<Vec<BmsSnapshot>>>
```

---

## 3. Structure du workspace

```
Daly-BMS-Rust/
├── Cargo.toml              ← [workspace] resolver=2, [workspace.dependencies]
├── Config.toml             ← Configuration exemple (copier → /etc/daly-bms/)
├── Makefile                ← up/build/build-arm/test/deploy/install
├── .gitignore
├── .env                    ← Secrets Docker (INFLUX_TOKEN, GRAFANA_PASSWORD)
├── docker-compose.infra.yml
│
├── crates/
│   ├── daly-bms-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs      DalyError (thiserror), Result<T>
│   │       ├── types.rs      BmsSnapshot + sous-structs (JSONData.json)
│   │       ├── protocol.rs   DataId, RequestFrame, ResponseFrame, checksum
│   │       ├── bus.rs        DalyPort (Arc<Mutex>), DalyBusManager
│   │       ├── commands.rs   get_pack_status, get_cell_voltages…
│   │       ├── write.rs      set_charge_mos, set_soc, reset_bms
│   │       └── poll.rs       poll_loop, retry, PollConfig
│   │
│   ├── daly-bms-server/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs       Init, config, port, spawn bridges, Axum serve
│   │       ├── config.rs     AppConfig (TOML serde)
│   │       ├── state.rs      AppState, BmsRingBuffer, broadcast channel
│   │       ├── api/
│   │       │   ├── mod.rs    build_router() — toutes les routes
│   │       │   ├── system.rs GET /system/status, /config, /discover
│   │       │   └── bms.rs    GET/POST /bms/{id}/*, WebSocket /ws/*
│   │       └── bridges/
│   │           ├── mod.rs
│   │           ├── mqtt.rs   run_mqtt_bridge() — Venus OS payload
│   │           ├── influx.rs run_influx_bridge() — batch DataPoint
│   │           └── alerts.rs AlertEngine, Telegram, SMTP, SQLite
│   │
│   └── daly-bms-cli/
│       ├── Cargo.toml
│       └── src/main.rs     clap CLI (status, cells, discover, set-*, poll…)
│
├── dashboard/              React SPA (inchangée)
├── contrib/                systemd, nginx, scripts install
├── docker/                 Configs Docker (Mosquitto, Grafana)
├── docs/
│   ├── Plan.md             Ce fichier
│   ├── JSONData.json       Structure JSON de référence
│   └── *.pdf / *.xlsx      Documentation protocole Daly
└── nanoPi/                 Config dbus-mqtt-battery (Venus OS)
```

---

## 4. Protocole Daly UART

### 4.1 Paramètres série

| Paramètre | Valeur |
|-----------|--------|
| Baud rate | 9600 bps |
| Data bits | 8 |
| Stop bits | 1 |
| Parity | None |
| Flow control | None |

### 4.2 Format trame (13 octets)

```
[0]  0xA5        Start flag (fixe)
[1]  Adresse     0x40 (requête PC) ou 0x01-0xFF (réponse BMS)
[2]  Data ID     Commande (voir tableau)
[3]  0x08        Longueur data (fixe = 8)
[4-11] Data      8 octets (requête : 0x00; réponse : valeurs)
[12] Checksum    Σ octets [0–11] mod 256
```

### 4.3 Data IDs — Lecture

| Data ID | Module Rust | Description |
|---------|-------------|-------------|
| 0x90 | `commands::get_pack_status` | Tension, courant, SOC |
| 0x91 | `commands::get_cell_voltage_minmax` | Min/max cellule + index |
| 0x92 | `commands::get_temperature_minmax` | Min/max temp + capteur |
| 0x93 | `commands::get_mos_status` | MOS, cycles, capacité |
| 0x94 | `commands::get_status_info` | Cellules, capteurs, état |
| 0x95 | `commands::get_cell_voltages` | Tensions (3/trame, multi) |
| 0x96 | `commands::get_temperatures` | Temps (7/trame, multi) |
| 0x97 | `commands::get_balance_flags` | Flags équilibrage (bits LE) |
| 0x98 | `commands::get_alarm_flags` | Alarmes protection (7 bytes) |

### 4.4 Data IDs — Écriture

| Data ID | Module Rust | Description | Vérification |
|---------|-------------|-------------|--------------|
| 0xD9 | `write::set_discharge_mos` | MOS décharge ON/OFF | Relecture 0x93 |
| 0xDA | `write::set_charge_mos` | MOS charge ON/OFF | Relecture 0x93 |
| 0x21 | `write::set_soc` | Calibration SOC | — |
| 0x00 | `write::reset_bms` | Reset BMS | — |

### 4.5 Encodages clés

```rust
voltage  = u16::from_be_bytes([b0, b1]) as f32 / 10.0          // Volts
current  = (u16::from_be_bytes([b0, b1]) as i32 - 30000) / 10  // Ampères
soc      = u16::from_be_bytes([b0, b1]) as f32 / 10.0          // %
cell_v   = u16::from_be_bytes([b0, b1]) as f32 / 1000.0        // Volts
temp     = raw_byte as f32 - 40.0                               // °C
checksum = bytes[0..12].iter().sum::<u32>() as u8
```

---

## 5. Structures de données

### 5.1 BmsSnapshot (types.rs)

Correspond exactement à `JSONData.json` :

```rust
pub struct BmsSnapshot {
    address:            BmsAddress,
    timestamp:          DateTime<Utc>,
    dc:                 DcData,           // power, voltage, current, temperature
    installed_capacity: f32,
    consumed_amphours:  f32,
    capacity:           f32,
    soc:                f32,              // (0x90)
    soh:                f32,
    time_to_go:         u32,
    balancing:          u8,
    system_switch:      u8,
    alarms:             Alarms,           // 13 flags (0x98)
    info:               InfoData,
    history:            HistoryData,
    system:             SystemData,       // min/max cellule + temp + MOS
    voltages:           BTreeMap<String, f32>,  // "Cell1" → 3.405
    balances:           BTreeMap<String, u8>,
    io:                 IoData,
    heating:            u8,
    time_to_soc:        BTreeMap<u8, u32>,
}
```

### 5.2 Correspondance JSON → Data IDs

| Champ | Data ID | Calcul |
|-------|---------|--------|
| `Dc.Voltage` | 0x90 | bytes[0-1] / 10 |
| `Dc.Current` | 0x90 | (bytes[2-3] - 30000) / 10 |
| `Soc` | 0x90 | bytes[4-5] / 10 |
| `System.MaxCellVoltage` | 0x91 | bytes[0-1] / 1000 |
| `System.MaxVoltageCellId` | 0x91 | "C" + bytes[2] |
| `System.MinCellVoltage` | 0x91 | bytes[3-4] / 1000 |
| `System.MinVoltageCellId` | 0x91 | "C" + bytes[5] |
| `System.MaxCellTemperature` | 0x92 | bytes[0] - 40 |
| `System.MinCellTemperature` | 0x92 | bytes[2] - 40 |
| `Io.AllowToCharge` | 0x93 | bit 1 |
| `Io.AllowToDischarge` | 0x93 | bit 0 |
| `History.ChargeCycles` | 0x93 | bytes[2-3] uint16 |
| `System.NrOfCellsPerBattery` | 0x94 | bytes[0] |
| `Voltages.Cell1..N` | 0x95 | uint16/1000, multi-trames |
| `Balances.Cell1..N` | 0x97 | bits little-endian |
| `Alarms.*` | 0x98 | 7 bytes flags |

---

## 6. Plan d'implémentation par phases

### Phase 0 — Squelette et types ✅ COMPLÉTÉ

**Livrable** : Structure compilable, tests protocole

- [x] Workspace Cargo.toml avec toutes les dépendances (4 crates)
- [x] `daly-bms-core` : error, types, protocol, bus, commands, write, poll
- [x] `daly-bms-server` : config, state, api/, bridges/, dashboard/, simulator, autodetect
- [x] `daly-bms-cli` : toutes les commandes (status, cells, temps, mos, alarms, discover, poll, set-*)
- [x] `daly-bms-probe` : outil diagnostic bas niveau, trames brutes, 3 variantes adressage
- [x] Makefile, .gitignore, docs mis à jour

---

### Phase 1 — Infrastructure Docker ✅ COMPLÉTÉ

**Durée** : 30 min

```bash
make up     # Mosquitto:1883 InfluxDB:8086 Grafana:3001 Node-RED:1880
make ps     # vérifier
```

---

### Phase 1.5 — Simulateur + Dashboard + Intégration Venus OS ✅ COMPLÉTÉ

#### Simulateur BMS (sans matériel)

- **Fichier** : `crates/daly-bms-server/src/simulator.rs` (308 LOC)
- **Activation** : `--simulate` ou `--simulate --sim-bms 0x28,0x29`
- **Physique LiFePO4** : SOC, tension OCV, courant, température, déséquilibre cellules, équilibrage
- **Validé sur Windows 10/11 et Linux x86_64**
- Alimente tous les bridges (MQTT, InfluxDB, AlertEngine, WebSocket, Dashboard)

#### Dashboard SSR intégré

- **Technologie** : Askama (templates compilés) + Apache ECharts — **aucun npm**
- Routes : `/dashboard` (vue synthèse) + `/dashboard/bms/:id` (détail)
- Boxplot tensions cellules, colorisation min/max, indicateur équilibrage
- Noms personnalisés par BMS, badge RS485, thème clair
- **Embarqué dans le binaire** — zéro dépendance externe au runtime

#### Auto-détection port série

- **Fichier** : `crates/daly-bms-server/src/autodetect.rs`
- Scan des ports COM/tty, détection signature Daly
- `port = ""` dans Config.toml → auto-détection au démarrage
- Fix double ouverture port sur Windows

#### Intégration Venus OS / NanoPi (15 mars 2026)

- **Flux confirmé** : MQTT → `dbus-mqtt-battery` → D-Bus Venus OS
- **Service CAN stoppé** : `svc -d /service/dbus-canbattery.can0` (récupère ~60 MB RAM + 6% CPU)
- **publish_interval_sec** réduit de 5s → **1s** pour rafraîchissement temps réel

#### État ressources NanoPi Cerbo GX (512 MB RAM)

| Process | RAM | CPU |
|---|---|---|
| node-red | ~229 MB | ~4% |
| gui (VNC) | ~164 MB | ~6% |
| dbus-modbus-client | ~91 MB | 0% |
| dbus-canbattery.can0 | stoppé | — |
| dbus-mqtt-battery x2 | ~84 MB | ~2% |

RAM disponible : ~77 MB. Le binary Rust (~10 MB) remplacera plusieurs scripts Python.

#### Compatibilité multi-plateforme confirmée

| Plateforme | Statut |
|---|---|
| Windows 10/11 (x86_64) | ✅ Testé avec simulateur |
| Linux x86_64 | ✅ Compilé et fonctionnel |
| Raspberry Pi 5 / CM5 (aarch64) | ✅ `make build-arm` (cross-compile) |

- Service manager différent : `systemd` sur RPi5 vs `runit/s6` sur Cerbo GX
- Compilation native RPi5 : `cargo build --release`

---

### Phase 1.6 — Corrections protocole et configuration ✅ COMPLÉTÉ (17 mars 2026)

#### Correction protocole Daly (commit `e8082fd`)

**Bug** : Les octets data des trames de lecture étaient remplis avec des valeurs aléatoires.
**Fix** : Forcer `[0u8; 8]` pour toutes les commandes de lecture — le BMS exige 8 zéros.

```
Trame correcte : A5 40 90 08 00 00 00 00 00 00 00 00 7D
```

#### Port API 8000 → 8080 (commit `14086f4`)

**Raison** : Le port 8000 entre en conflit avec Docker Desktop sur Windows.
**Impact** : Tous les appels API passent désormais sur `http://localhost:8080`.

```toml
# Config.toml
[api]
bind = "0.0.0.0:8080"
```

#### Correction Grafana — UID datasource (17 mars 2026)

**Bug** : Le fichier `grafana/provisioning/datasources/influxdb.yaml` déclarait
`uid: influxdb-dalybms-flux` mais les **33 panels** du dashboard utilisaient
`uid: influxdb-dalybms` → tous les panels affichaient "Datasource not found".

**Fix** : Aligner le UID dans le YAML sur celui utilisé par le dashboard :
```yaml
# Avant (cassé)
uid: influxdb-dalybms-flux

# Après (correct)
uid: influxdb-dalybms
```

Après ce fix, redémarrer Grafana pour recharger le provisioning :
```bash
docker compose -f docker-compose.infra.yml restart grafana
# ou
make restart-grafana
```

---

### Phase 1.7 — Configuration hardware Raspberry Pi 5 ✅ COMPLÉTÉ

#### Matériel identifié

| Composant | Détail |
|---|---|
| Calculateur | Raspberry Pi 5 (ou CM5) |
| Adaptateur USB/RS485 | FTDI — Numéro de série : `BG03CWA2` |
| Port assigné Linux | `/dev/ttyUSB0` (stable — même n° de série FTDI) |
| Bus RS485 | 2 BMS en parallèle (adresses 0x01 et 0x02) |

#### Configuration actuelle (Config.toml)

| BMS | Adresse RS485 | Nom | Capacité | Index MQTT |
|-----|--------------|-----|----------|-----------|
| BMS 1 | `0x01` | BMS-360Ah | 360 Ah | 1 |
| BMS 2 | `0x02` | BMS-320Ah | 320 Ah | 2 |

#### Comportement auto-détection

**Port USB** :
- `port = "/dev/ttyUSB0"` → port fixe, pas d'auto-détection
- `port = ""` → scan automatique de tous les ports tty/COM au démarrage
- L'adaptateur FTDI (n° série `BG03CWA2`) revient toujours sur `ttyUSB0` → configuration fixe recommandée

**Adresses BMS** :
- Tant qu'un `Config.toml` existe, c'est la liste `addresses` qui est utilisée
- L'auto-découverte (`auto_discover_addrs`) ne s'active que **sans fichier config** (`!config_from_file`)
- Pour un 3ème BMS : ajouter manuellement dans Config.toml :

```toml
addresses = ["0x01", "0x02", "0x03"]   # ← ajouter 0x03

[[bms]]
address         = "0x03"
name            = "BMS-280Ah"
capacity_ah     = 280.0
max_charge_a    = 200.0
max_discharge_a = 120.0
mqtt_index      = 3
```

#### Infrastructure Docker sur RPi5

Services démarrés avec `docker compose -f docker-compose.infra.yml up -d` :

| Service | Port | URL |
|---------|------|-----|
| Mosquitto MQTT | 1883 | mqtt://localhost:1883 |
| InfluxDB 2.7 | 8086 | http://localhost:8086 |
| Grafana 11.6 | 3001 | http://localhost:3001 |
| Node-RED | 1880 | http://localhost:1880 |

Serveur Rust (natif, hors Docker) :

| Service | Port | URL |
|---------|------|-----|
| API REST + WebSocket | 8080 | http://localhost:8080/api/v1/ |
| Dashboard SSR | 8080 | http://localhost:8080/dashboard |

---

### Phase 2 — Validation hardware réel (PROCHAINE ÉTAPE — RPi5 + BMS physiques)

**Durée estimée** : 3–5 jours | **Prérequis** : Matériel BMS physique

#### 2.1 Préparation matérielle
```bash
ls -l /dev/ttyUSB*
sudo usermod -aG dialout $USER
newgrp dialout
```

#### 2.2 Test CLI de base
```bash
cargo build --release

# Découverte
./target/release/daly-bms-cli --port /dev/ttyUSB0 discover --start 1 --end 4

# Status BMS 1
./target/release/daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 status

# Cellules
./target/release/daly-bms-cli --port /dev/ttyUSB0 --addr 0x01 cells --count 16
```

#### 2.3 Points de vigilance
- **Timing inter-trames** : ajuster `INTER_FRAME_DELAY_MS` si timeout (50ms → 100ms)
- **Multi-trames 0x95** : vérifier le numéro de bloc dans data[0]
- **Offset courant** : certains firmware utilisent un offset différent de 30000
- **Réponse discovery** : le BMS peut répondre lentement, augmenter le timeout

#### 2.4 Validation snapshot
```bash
cargo run --bin daly-bms-server
curl http://localhost:8000/api/v1/bms/1/status | python3 -m json.tool
# Comparer avec docs/JSONData.json
```

---

### Phase 3 — Commandes d'écriture

**Durée estimée** : 2–3 jours

**Modifications à faire dans `api/bms.rs`** :

Remplacer les stubs `NOT_IMPLEMENTED` par les vrais appels :

```rust
// Dans AppState, ajouter :
pub bus_port: Option<Arc<DalyPort>>

// Dans set_mos handler :
if let Some(port) = &state.bus_port {
    write::set_charge_mos(port, addr, body.charge, state.config.read_only.enabled).await?;
    write::set_discharge_mos(port, addr, body.discharge, ...).await?;
}
```

Test :
```bash
# Via CLI (safe — dry_run)
daly-bms-cli --addr 0x01 set-charge-mos --enable --dry-run

# Via API
curl -X POST http://localhost:8000/api/v1/bms/1/mos \
  -H "Content-Type: application/json" \
  -d '{"charge": true, "discharge": true}'
```

---

### Phase 4 — Bridges et intégrations

**Durée estimée** : 3–4 jours

#### MQTT
```bash
mosquitto_sub -h localhost -t "santuario/bms/#" -v
# Attendre : santuario/bms/1/soc → 56.4
```

#### InfluxDB
```bash
# http://localhost:8086
# Query : from(bucket:"daly_bms") |> range(start:-1h) |> filter(fn:(r) => r._measurement == "bms_status")
```

#### Alertes
```bash
# Réduire seuil pour tester
# config.toml : cell_ovp_v = 3.40
sqlite3 /var/lib/daly-bms/alerts.db "SELECT * FROM alert_events;"
```

---

### Phase 5 — Dashboard React

**Durée estimée** : 1–2 jours

```bash
cd dashboard
npm install && npm run dev    # proxy vers :8000
# Vérifier WebSocket et données temps réel

npm run build
sudo cp -r dist /opt/dalybms/frontend/
```

---

### Phase 6 — Cross-compilation Pi

**Durée estimée** : 1 jour

```bash
cargo install cross
make build-arm
make deploy PI_HOST=pi@192.168.1.100
```

---

### Phase 7 — Tests d'intégration et 24h

**Durée estimée** : 2–3 jours

```bash
make test                     # tests unitaires
# + test stabilité 24h
watch -n 60 'curl -s http://localhost:8000/api/v1/system/status | jq .polling_active'
```

---

### Phase 8 — Containerisation complète (optionnelle)

```dockerfile
FROM rust:1.80-alpine AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --bin daly-bms-server

FROM alpine:3.19
COPY --from=builder /build/target/release/daly-bms-server /usr/local/bin/
CMD ["daly-bms-server"]
```

---

## 7. API REST et WebSocket

### Routes complètes

```
GET  /api/v1/system/status
GET  /api/v1/config
GET  /api/v1/discover

GET  /api/v1/bms/{id}/status
GET  /api/v1/bms/{id}/cells
GET  /api/v1/bms/{id}/temperatures
GET  /api/v1/bms/{id}/alarms
GET  /api/v1/bms/{id}/mos
GET  /api/v1/bms/{id}/history
GET  /api/v1/bms/{id}/history/summary
GET  /api/v1/bms/{id}/export/csv
GET  /api/v1/bms/compare

POST /api/v1/bms/{id}/mos          { charge: bool, discharge: bool }
POST /api/v1/bms/{id}/soc          { soc: f32 }
POST /api/v1/bms/{id}/soc/full
POST /api/v1/bms/{id}/soc/empty
POST /api/v1/bms/{id}/reset        { confirm: true }

WS   /ws/bms/stream
WS   /ws/bms/{id}/stream
```

Format `{id}` accepté : `"0x01"`, `"1"`, `"01"`.

---

## 8. Bridges (MQTT, InfluxDB, Alertes)

### MQTT — Topics publiés

```
{prefix}/{addr}/soc           → "56.4"
{prefix}/{addr}/voltage       → "52.53"
{prefix}/{addr}/current       → "-1.60"
{prefix}/{addr}/power         → "-84.0"
{prefix}/{addr}/status        → JSON complet (retain=true)
{prefix}/{addr}/cells         → JSON tensions
{prefix}/{addr}/alarms        → JSON alarmes
{prefix}/{addr}/venus         → JSON dbus-mqtt-battery (retain=true)
```

### InfluxDB — Measurements

| Measurement | Tags | Champs principaux |
|-------------|------|------------------|
| `bms_status` | `address` | soc, voltage, current, power, temp_max, cell_delta_mv, any_alarm |
| `bms_cell_voltage` | `address`, `cell` | voltage |

### AlertEngine — Règles avec hysteresis

| ID | Trigger | Clear | Cooldown |
|----|---------|-------|---------|
| `cell_ovp` | > 3.60V | < 3.55V | 5 min |
| `cell_uvp` | < 2.90V | > 2.95V | 5 min |
| `cell_imbalance` | > 100mV | < 90mV | 10 min |
| `soc_low` | < 20% | > 25% | 15 min |
| `soc_critical` | < 10% | > 12% | 5 min |
| `temp_high` | > 45°C | < 43°C | 5 min |
| `high_current` | > 80A | < 75A | 1 min |

---

## 9. Tests et validation

### 9.1 Tests unitaires (Phase 0)

```bash
cargo test -p daly-bms-core
# test_checksum_pack_status
# test_decode_voltage
# test_decode_current_discharge
# test_decode_cell_voltage
# test_decode_temperature
# test_request_frame_checksum
```

### 9.2 Checklist production

- [ ] Adresses BMS uniques configurées
- [ ] Câblage A, B, GND commun
- [ ] Port série `ls -l /dev/ttyUSB*` OK
- [ ] Droits `groups | grep dialout` OK
- [ ] `cargo build --release` sans erreur
- [ ] CLI : données cohérentes avec JSONData.json
- [ ] API : `curl http://localhost:8000/api/v1/system/status`
- [ ] WebSocket : `wscat -c ws://localhost:8000/ws/bms/stream`
- [ ] MQTT : `mosquitto_sub -h localhost -t 'santuario/bms/#' -v`
- [ ] InfluxDB : données visibles dans le dashboard
- [ ] Alertes : SQLite créé et journal fonctionnel
- [ ] Service systemd actif et démarrage auto
- [ ] Stabilité 24h validée

---

## 10. Déploiement et opérations

### Installation

```bash
make build && sudo make install
sudo nano /etc/daly-bms/config.toml
sudo systemctl restart daly-bms
journalctl -u daly-bms -f
```

### Déploiement SSH Pi

```bash
make build-arm
make deploy PI_HOST=pi@192.168.1.100
```

### Surveillance

```bash
systemctl status daly-bms
journalctl -u daly-bms -f
curl http://localhost:8000/api/v1/system/status | jq
```

### Variables d'environnement

| Variable | Défaut | Description |
|----------|--------|-------------|
| `DALY_CONFIG` | `/etc/daly-bms/config.toml` | Chemin config |
| `RUST_LOG` | `info` | Niveau logs |
| `DALY_API_KEY` | — | Surcharge `api.api_key` |

---

## Annexe — Correspondance Python → Rust

| Module Python | Module Rust | Statut |
|--------------|-------------|--------|
| `daly_protocol.py` | `core/commands.rs` + `protocol.rs` | ✅ Complet + 7 tests unitaires |
| `daly_write.py` | `core/write.rs` | ✅ Implémenté (à valider sur hardware) |
| `daly_api.py` | `server/api/` | ✅ Toutes les routes opérationnelles |
| `daly_mqtt.py` | `server/bridges/mqtt.rs` | ✅ Validé en production (Venus OS) |
| `daly_influx.py` | `server/bridges/influx.rs` | ✅ Complet |
| `daly_alerts.py` | `server/bridges/alerts.rs` | ✅ Moteur complet |
| `config.py` | `server/config.rs` | ✅ Complet, per-BMS config |
| `dashboard/` (React) | `server/dashboard/` (Askama+ECharts) | ✅ SSR sans npm |
| N/A | `server/simulator.rs` | ✅ Nouveau — physique LiFePO4 |
| N/A | `server/autodetect.rs` | ✅ Nouveau — auto-détection port/BMS |
| N/A | `daly-bms-probe` | ✅ Nouveau — diagnostic bas niveau |

---

*Document mis à jour le 15 mars 2026 — Version 2.1*
