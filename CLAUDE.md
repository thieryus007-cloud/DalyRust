# CLAUDE.md — Référence Complète Projet Daly-BMS-Rust

> Ce fichier est la référence principale pour toute session Claude sur ce projet.
> Lire en entier avant toute action.

---

## 1. ARCHITECTURE GLOBALE

```
┌──────────────────────────────────────────────────────────────────┐
│  Batteries (2×)                                                  │
│  BMS-1: Daly 360Ah, adresse RS485 0x01                          │
│  BMS-2: Daly 320Ah, adresse RS485 0x02                          │
└──────────┬───────────────────────────────────────────────────────┘
           │ RS485 USB (/dev/ttyUSB0)
           ▼
┌──────────────────────────────────────────────────────────────────┐
│  Pi5 (Master)  192.168.1.141   user: pi5compute                  │
│  ~/Daly-BMS-Rust/                                                │
│                                                                  │
│  daly-bms-server (systemd)                                       │
│    ├── Poll RS485 → 2 BMS                                        │
│    ├── REST API + WebSocket (port 8080)                          │
│    ├── MQTT publish → 192.168.1.120:1883                         │
│    │     topics: santuario/bms/1/venus, santuario/bms/2/venus    │
│    ├── InfluxDB2 → http://localhost:8086                         │
│    └── SSR Dashboard (Askama + ECharts)                          │
│                                                                  │
│  Docker Stack (make up):                                         │
│    mosquitto   1883 (MQTT), 9001 (WebSocket)                     │
│    influxdb    8086                                              │
│    grafana     3001                                              │
│    nodered     1880  ← MIGRÉ depuis NanoPi ✅ TERMINÉ            │
└──────────┬───────────────────────────────────────────────────────┘
           │ MQTT 192.168.1.120:1883
           ▼
┌──────────────────────────────────────────────────────────────────┐
│  NanoPi (Venus OS / Victron GX)  192.168.1.120  user: root       │
│  /data/daly-bms/                                                 │
│                                                                  │
│  dbus-mqtt-venus (runit service /service/dbus-mqtt-venus)          │
│    ├── Subscribe MQTT → bridge D-Bus                             │
│    ├── com.victronenergy.battery.mqtt_1 (instance 141)           │
│    ├── com.victronenergy.battery.mqtt_2 (instance 142)           │
│    ├── com.victronenergy.temperature.mqtt_1                      │
│    ├── com.victronenergy.heatpump.mqtt_1                         │
│    ├── com.victronenergy.switch.*                                │
│    ├── com.victronenergy.grid.* / acload.*                       │
│    ├── com.victronenergy.pvinverter.mqtt_3 (instance 63) ← ET112 │
│    └── com.victronenergy.meteo (singleton)                       │
│                                                                  │
│  dbus-mqtt-battery (legacy, à terme à supprimer)                 │
│    config-bms1.ini → /data/etc/dbus-mqtt-battery-41/config.ini  │
│    config-bms2.ini → /data/etc/dbus-mqtt-battery-42/config.ini  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 2. RÉSEAU & ACCÈS SSH

| Machine | IP | User | Port SSH |
|---------|-----|------|----------|
| Pi5 (master) | 192.168.1.141 | pi5compute | 22 |
| NanoPi (Venus OS) | 192.168.1.120 | root | 22 |

### Clés SSH configurées sur Pi5

```
~/.ssh/id_nanopi       ← clé dédiée pour NanoPi
~/.ssh/id_nanopi.pub
```

### Config SSH (`~/.ssh/config` sur Pi5)

```
Host nanopi
    HostName 192.168.1.120
    User root
    IdentityFile ~/.ssh/id_nanopi
    StrictHostKeyChecking no

Host 192.168.1.120
    User root
    IdentityFile ~/.ssh/id_nanopi
    StrictHostKeyChecking no
```

> **IMPORTANT** : Les deux entrées (`nanopi` ET `192.168.1.120`) sont nécessaires.
> Le script `install-venus.sh` utilise l'IP directe.

### Setup clé SSH (une seule fois)

```bash
ssh-keygen -t ed25519 -C "pi5-nanopi-deploy" -f ~/.ssh/id_nanopi -N ""
ssh-copy-id -i ~/.ssh/id_nanopi.pub root@192.168.1.120
```

---

## 3. REPOSITORY GIT

- **GitHub** : https://github.com/thieryus007-cloud/Daly-BMS-Rust
- **Branche principale** : `master`
- **Branche de travail courante** : `claude/review-venus-integration-35qN7`
- **Convention branches Claude** : `claude/<description>-<session-id>`

### Workflow standard

```bash
# Sur Pi5 — récupérer les changements avant toute action
git pull origin <branche-courante>

# Après modification — toujours commit + push
git add <fichiers>
git commit -m "type(scope): description"
git push -u origin <branche>
```

> **CRITIQUE** : Toujours faire `git pull` sur le Pi5 avant `make install-venus-v7`
> sinon l'ancien script est exécuté.

---

## 4. STRUCTURE DU PROJET

```
Daly-BMS-Rust/
├── CLAUDE.md                    ← CE FICHIER (référence Claude)
├── Readme.md                    ← Documentation utilisateur (37KB)
├── Cargo.toml                   ← Workspace Rust (edition 2021, rust ≥1.88)
├── Config.toml                  ← Config production (Pi5 + hardware)
├── Config.docker.toml           ← Config Docker (simulateur)
├── Makefile                     ← Toutes les commandes build/deploy
├── Dockerfile                   ← Multi-stage (builder + debian:slim)
├── docker-compose.yml           ← Stack complète
├── docker-compose.infra.yml     ← Infra seule (sans daly-bms-server)
├── .env                         ← Secrets (gitignored)
├── .env.docker                  ← Template secrets
│
├── crates/
│   ├── daly-bms-core/           ← Bibliothèque protocole RS485
│   ├── daly-bms-server/         ← Serveur principal (API, MQTT, InfluxDB)
│   ├── dbus-mqtt-venus/          ← Bridge MQTT→D-Bus (Venus OS)
│   ├── daly-bms-cli/            ← Outil diagnostic CLI
│   └── daly-bms-probe/          ← Sonde protocole bas niveau
│
├── nanoPi/
│   ├── install-venus.sh         ← Script déploiement NanoPi
│   ├── cleanup-dbus-serialbattery.sh
│   ├── config-bms1.ini          ← Config dbus-mqtt-battery instance 41
│   ├── config-bms2.ini          ← Config dbus-mqtt-battery instance 42
│   ├── sv/dbus-mqtt-venus/run    ← Script runit (daemontools)
│   └── README.md
│
├── contrib/
│   ├── daly-bms.service         ← Unité systemd
│   ├── install-systemd.sh       ← Installation service Pi5
│   ├── uninstall-systemd.sh
│   └── nginx.conf               ← Template reverse proxy
│
├── docker/mosquitto/config/
│   └── mosquitto.conf           ← Config broker MQTT
│
├── grafana/provisioning/        ← Dashboards + datasources auto-provisionnés
├── docs/                        ← Documentation technique + PDF protocoles
├── flux-nodered/                ← Flows Node-RED
└── tools/chint-ats/             ← Dashboard CHINT ATS (Node.js)
```

---

## 5. COMMANDES MAKE ESSENTIELLES

```bash
# Infrastructure Docker
make up              # Démarrer Mosquitto, InfluxDB, Grafana, Node-RED
make down            # Arrêter
make reset           # Tout supprimer (volumes inclus) ← DESTRUCTIF
make logs            # Logs Docker

# Build
make build           # x86_64 release
make build-arm       # aarch64 (Pi5)
make build-arm-v7    # armv7 (NanoPi)
make build-venus     # dbus-mqtt-venus aarch64
make build-venus-v7  # dbus-mqtt-venus armv7 ← pour NanoPi
make build-all       # Tous les binaires

# Développement
make run             # Lancer serveur (RUST_LOG=info)
make run-debug       # Mode debug (RUST_LOG=debug)
make test            # Tests unitaires
make lint            # Clippy
make check           # fmt + lint

# Déploiement
make install         # Installer service systemd sur Pi5
make deploy          # SSH vers Pi5 (PI_HOST=pi5compute@192.168.1.141)
make install-venus-v7  # Déployer dbus-mqtt-venus sur NanoPi (armv7)
```

---

## 6. CONFIGURATION PRODUCTION

### Config.toml (Pi5 — matériel réel)

```toml
[serial]
port = "/dev/ttyUSB0"
baud_rate = 9600
poll_interval_ms = 1000
addresses = ["0x01", "0x02"]
ring_buffer_size = 3600     # 1 heure à 1 Hz

[api]
bind = "0.0.0.0:8080"
# api_key = ""              # Désactivé en LAN

[mqtt]
host = "192.168.1.120"
port = 1883
topic = "santuario/bms"
format = "venus"

[influxdb]
url = "http://localhost:8086"
org = "santuario"
bucket = "daly_bms"
# token = depuis .env

[[bms]]
address = "0x01"
name = "BMS-360Ah"
capacity_ah = 360.0

[[bms]]
address = "0x02"
name = "BMS-320Ah"
capacity_ah = 320.0
```

### Fichiers sur NanoPi

```
/data/daly-bms/
├── dbus-mqtt-venus          ← binaire (armv7)
├── config.toml             ← config Venus OS
└── (logs via runit)

/data/etc/sv/dbus-mqtt-venus/
└── run                     ← script runit

/service/dbus-mqtt-venus     ← symlink → /data/etc/sv/dbus-mqtt-venus
                               (création = activation automatique)

/data/etc/dbus-mqtt-battery-41/config.ini   ← legacy BMS-1
/data/etc/dbus-mqtt-battery-42/config.ini   ← legacy BMS-2
```

---

## 7. DÉPLOIEMENT SUR NANOPI — PROCÉDURE COMPLÈTE

```bash
# Sur Pi5

# 1. Récupérer derniers changements
git pull origin claude/migrate-nodered-pi5-91idx

# 2. Compiler pour armv7 (Venus OS NanoPi est armv7)
make build-venus-v7

# 3. Déployer (SSH ControlMaster, arrêt service, copy, redémarrage)
make install-venus-v7
```

### Ce que fait install-venus.sh

1. Établit un ControlMaster SSH (une seule auth pour toutes les commandes)
2. Crée `/data/daly-bms/` et `/data/etc/sv/dbus-mqtt-venus/`
3. Arrête le service avant de copier le binaire (évite "dest open Failure")
4. Supprime daly-bms-server s'il existe (ne doit PAS tourner sur NanoPi)
5. Copie le binaire `dbus-mqtt-venus`
6. Copie `config.toml` si absent
7. Copie le script runit `run`
8. Crée le symlink `/service/dbus-mqtt-venus` (activation automatique)
9. Vérifie le démarrage

### Commandes de diagnostic sur NanoPi

```bash
ssh root@192.168.1.120

# État du service
svstat /service/dbus-mqtt-venus

# Logs en temps réel
tail -f /var/log/dbus-mqtt-venus/current

# Redémarrer
svc -t /service/dbus-mqtt-venus

# Arrêter
svc -d /service/dbus-mqtt-venus

# Démarrer
svc -u /service/dbus-mqtt-venus

# Vérifier D-Bus batteries (nommage réel : mqtt_1 / mqtt_2)
dbus -y com.victronenergy.battery.mqtt_1 /Soc GetValue
dbus -y com.victronenergy.battery.mqtt_2 /Soc GetValue

# Lister tous les services Victron enregistrés
dbus -y | grep victronenergy

# Vérifier MQTT reçu
mosquitto_sub -h 127.0.0.1 -p 1883 -t "santuario/#" -v
```

---

## 8. SERVICE SUR PI5 (systemd)

```bash
# État
systemctl status daly-bms

# Logs
journalctl -u daly-bms -f

# Redémarrer
systemctl restart daly-bms

# Stopper
systemctl stop daly-bms
```

---

## 9. DOCKER STACK (Pi5)

```bash
# Démarrer infra
make up

# Accès services
# Grafana   : http://192.168.1.141:3001  (admin/autre_supersecret)
# InfluxDB  : http://192.168.1.141:8086  (admin/supersecretchangeit)
# Node-RED  : http://192.168.1.141:1880
# MQTT      : 192.168.1.141:1883
# API BMS   : http://192.168.1.141:8080/api/v1/

# Secrets dans .env (gitignored) :
INFLUX_TOKEN=TGEh4wl5TE7SEeJd7GDdyjDebo48xEJaD63MKbgdNhLz54-...
```

---

## 10. PROBLÈMES CONNUS & SOLUTIONS

### Problème : `scp: dest open Failure` lors du déploiement Venus

**Cause** : Le binaire `dbus-mqtt-venus` est en cours d'exécution, on ne peut pas l'écraser.
**Solution** : Le script arrête le service AVANT la copie (étape 2 dans install-venus.sh).
Si ça se reproduit : `ssh root@192.168.1.120 "svc -d /service/dbus-mqtt-venus"` puis redéployer.

### Problème : Multiples demandes de mot de passe SSH

**Cause** : Pas de ControlMaster ET/OU clé SSH non configurée pour l'IP directe.
**Solution** :
1. Vérifier `~/.ssh/config` contient bien `Host 192.168.1.120` (pas seulement `Host nanopi`)
2. Vérifier clé copiée : `ssh-copy-id -i ~/.ssh/id_nanopi.pub root@192.168.1.120`

### Problème : Ancien script exécuté après git push

**Cause** : Le Pi5 n'a pas fait `git pull` avant `make install-venus-v7`.
**Solution** : Toujours `git pull` sur le Pi5 avant de déployer.

### Problème : Cross-compilation armv7 échoue

**Cause** : Toolchain ARM non installée.
**Solution** :
```bash
# Sur Pi5
rustup target add armv7-unknown-linux-gnueabihf
sudo apt install -y gcc-arm-linux-gnueabihf
```

### Problème : `pip3: command not found` lors de install.sh irradiance

**Cause** : `pip3` n'est pas installé par défaut sur Raspberry Pi OS.
**Solution** : `install.sh` utilise désormais `sudo apt-get install -y python3-serial python3-paho-mqtt`.

### Problème : D-Bus Venus non visible dans Victron GUI

**Cause** : Service non démarré ou nom de service incorrect.
**Vérification** :
```bash
# Lister tous les services Victron actifs
dbus -y | grep victronenergy

# Noms corrects des batteries :
# com.victronenergy.battery.mqtt_1   (BMS-1)
# com.victronenergy.battery.mqtt_2   (BMS-2)
# NB : préfixe = "mqtt", index = n° du BMS (pas "mqtt_bms1")

dbus -y com.victronenergy.battery.mqtt_1 / GetItems
dbus-monitor --system "type=signal,sender=com.victronenergy.battery.mqtt_1"
```

### Problème : Victron widget météo — "Température: -" malgré valeur D-Bus correcte

**Constat** (vérifié 2026-03-22) :
```bash
dbus -y com.victronenergy.meteo /ExternalTemperature GetValue
# → 8.4   ← valeur D-Bus correcte
```
**Cause** : Limitation connue Venus OS — le widget "Capteur météo" n'affiche PAS
`/ExternalTemperature` du service `com.victronenergy.meteo`, même si la valeur D-Bus
est correcte. C'est un bug d'affichage Venus OS, PAS un bug du code Rust.

**Statut** : Pas de solution côté code. La valeur existe bien sur D-Bus (utilisable
par d'autres services). L'affichage restera "-" dans le widget météo Victron.

### Problème : Menu "Setup" absent dans Venus OS pour pvinverter MQTT (✅ RÉSOLU 2026-03-23)

**Cause** : Les chemins `/AllowedRoles` et `/Role` étaient absents de `com.victronenergy.pvinverter.mqtt_3`.
Venus OS GUI conditionne le menu "Setup" à la présence de `/AllowedRoles` (liste des rôles possibles).

**Diagnostic** : Comparaison `GetItems` entre cgwacs natif et mqtt_3 :
```bash
# Sur NanoPi :
dbus -y com.victronenergy.pvinverter.cgwacs_ttyUSB0_mb2 / GetItems
# → présent : /AllowedRoles, /Role, /CustomName, /DeviceType, /FirmwareVersion
dbus -y com.victronenergy.pvinverter.mqtt_3 / GetItems
# → absent : tout ce qui précède
```

**Fix** : Commit `7cf720d` — ajout dans `pvinverter_service.rs` :
- `/AllowedRoles` = `["grid", "pvinverter", "genset", "acload", "evcharger", "heatpump"]` (array D-Bus "as")
- `/Role` = `"pvinverter"` (writable — Venus OS peut le modifier via Setup)
- `/CustomName`, `/DeviceType=120`, `/FirmwareVersion="4"`

**Détail technique** : `OwnedValue` ne dérive pas `Clone` dans zvariant 4.2.0 sans default-features.
Solution : `DbusValueKind` enum (Clone-able) qui calcule `OwnedValue` à la demande.

**À vérifier sur NanoPi** après déploiement :
```bash
# Vérifier que le menu Setup apparaît dans Venus OS GUI
dbus -y com.victronenergy.pvinverter.mqtt_3 / GetItems | grep -E "AllowedRoles|Role|CustomName|DeviceType"
```

### Problème : TodaysYield incorrect après reset manuel Node-RED en pleine journée

**Cause** : Le "Reset minuit" pose `pvinv_baseline = cumul_actuel`. Si exécuté en journée,
la production PVInverter antérieure au reset est perdue pour ce jour.

**Procédure de récupération** (📍 NanoPi puis Node-RED) :

```bash
# 1. Sur NanoPi — lire le cumul actuel PVInverter
dbus -y com.victronenergy.pvinverter.cgwacs_ttyUSB0_mb2 /Ac/Energy/Forward GetValue
# ex: 587.2

# 2. Sur Victron GUI — noter la valeur "Solaire" (MPPT + PVInverter total du jour)
# ex: 3.9 kWh

# 3. Sur NanoPi — lire la production MPPT seule du jour
dbus -y | grep solarcharger   # trouver l'instance
dbus -y com.victronenergy.solarcharger.XXX /History/Daily/0/Yield GetValue
# ex: 2.18 kWh
```

```javascript
// 4. Dans Node-RED — Function node à injecter UNE FOIS
const currentCumul = 587.2;   // ← résultat étape 1
const totalVictron  = 3.9;    // ← valeur "Solaire" Victron (étape 2)
const mpptToday     = global.get('mppt_yield_today') || 2.18;

const pvinvToday  = totalVictron - mpptToday;
const newBaseline = currentCumul - pvinvToday;

global.set('pvinv_baseline',    newBaseline);
global.set('pvinv_yield_today', pvinvToday);
global.set('total_yield_today', mpptToday + pvinvToday);

node.status({fill:'green', text:`Total=${(mpptToday+pvinvToday).toFixed(2)} kWh`});
return null;
```

**Vérification** (📍 NanoPi) :
```bash
mosquitto_sub -h 127.0.0.1 -p 1883 -t "santuario/meteo/venus" -C 1
# → TodaysYield doit afficher ~3.9
```

---

## 11. BINAIRES & CIBLES DE COMPILATION

| Binaire | Cible | Usage |
|---------|-------|-------|
| `daly-bms-server` | x86_64 / aarch64 | Pi5 (serveur principal) |
| `dbus-mqtt-venus` | armv7-unknown-linux-gnueabihf | NanoPi Venus OS |
| `dbus-mqtt-venus` | aarch64-unknown-linux-gnu | NanoPi si aarch64 |
| `daly-bms-cli` | x86_64 | Diagnostic local |
| `daly-bms-probe` | x86_64 | Test protocole RS485 |

### Emplacements après build

```
target/release/daly-bms-server
target/armv7-unknown-linux-gnueabihf/release/dbus-mqtt-venus
target/aarch64-unknown-linux-gnu/release/dbus-mqtt-venus
```

---

## 12. API ENDPOINTS PRINCIPAUX

```
GET  /api/v1/system/status           ← état global
GET  /api/v1/system/config           ← config chargée

GET  /api/v1/bms                     ← liste BMS
GET  /api/v1/bms/{id}/snapshot       ← dernière mesure
GET  /api/v1/bms/{id}/history        ← ring buffer 3600 entrées
WS   /api/v1/bms/{id}/stream         ← stream temps réel

POST /api/v1/bms/{id}/charge-mos     ← activer/désactiver MOS charge
POST /api/v1/bms/{id}/discharge-mos  ← activer/désactiver MOS décharge
POST /api/v1/bms/{id}/soc            ← calibrer SOC
POST /api/v1/bms/{id}/reset          ← reset BMS
```

---

## 13. TOPICS MQTT — TABLE COMPLÈTE

### Topics publiés par daly-bms-server (Pi5 → MQTT broker NanoPi)

```
santuario/bms/1/venus        ← payload Venus OS BMS-1 (JSON)
santuario/bms/2/venus        ← payload Venus OS BMS-2 (JSON)
santuario/bms/1/raw          ← données brutes BMS-1
santuario/bms/2/raw          ← données brutes BMS-2
```

### Topics consommés par dbus-mqtt-venus (MQTT → D-Bus NanoPi)

| Topic MQTT | Type device | Service D-Bus résultant |
|---|---|---|
| `santuario/bms/{n}/venus` | Batterie BMS | `com.victronenergy.battery.mqtt_{n}` |
| `santuario/heat/{n}/venus` | Capteur température | `com.victronenergy.temperature.mqtt_{n}` |
| `santuario/heatpump/{n}/venus` | PAC / chauffe-eau | `com.victronenergy.heatpump.mqtt_{n}` |
| `santuario/switch/{n}/venus` | Switch / ATS | `com.victronenergy.switch.mqtt_{n}` |
| `santuario/grid/{n}/venus` | Compteur réseau | `com.victronenergy.grid.mqtt_{n}` |
| `santuario/meteo/venus` | Capteur irradiance | `com.victronenergy.meteo` (singleton) |
| `santuario/platform/venus` | Platform Pi5 | `com.victronenergy.platform` (singleton) |

> **Nommage D-Bus** : `{service_prefix}_{mqtt_index}` — le préfixe par défaut est `"mqtt"`.
> Résultat : `mqtt_1`, `mqtt_2`, etc. (configurable via `venus.service_prefix` dans config.toml)

### Topics publiés par Node-RED (Pi5 → MQTT → NanoPi D-Bus)

```
santuario/heat/{n}/venus     ← capteur température (Shelly, DS18B20, API cloud…)
santuario/heatpump/{n}/venus ← PAC LG ThinQ
santuario/switch/{n}/venus   ← ATS CHINT, relais Shelly
santuario/grid/{n}/venus     ← compteur réseau ET112, Fronius
santuario/meteo/venus        ← irradiance RS485
```

---

## 14. DÉPENDANCES RUST CLÉS

| Crate | Usage |
|-------|-------|
| `tokio` (full) | Runtime async |
| `tokio-serial` | RS485 async |
| `axum 0.7` | HTTP REST + WebSocket |
| `rumqttc 0.25` | Client MQTT |
| `influxdb2 0.5` | Client InfluxDB |
| `zbus 4` (tokio) | D-Bus Venus OS (pur Rust, pas libdbus) |
| `rusqlite 0.31` (bundled) | SQLite alertes |
| `askama 0.12` | Templates SSR |
| `reqwest 0.12` | HTTP client (Telegram, etc.) |
| `lettre 0.11` | Email alertes |
| `clap 4` (derive) | CLI |
| `chrono 0.4` | Dates/heures |
| `serde + serde_json + toml` | Sérialisation |
| `anyhow + thiserror` | Gestion erreurs |
| `tracing + tracing-subscriber` | Logs structurés |

---

## 15. MIGRATION NODE-RED ✅ TERMINÉE

**Objectif** : Déplacer Node-RED du NanoPi vers le Pi5 dans Docker.

**État** : ✅ COMPLET — Branch `claude/migrate-nodered-pi5-91idx`

---

## 15e. PERSISTANCE PRODUCTION SOLAIRE APRÈS REBOOT PI5 ✅ (2026-03-22)

**Problème** : Après reboot Pi5, les globals Node-RED (mémoire) sont perdus → `pvinv_baseline`
réinitialisée au cumul courant → `TodaysYield` repart de 0 pour le reste de la journée.

**Solution** : MQTT retained (Mosquitto `persistence true` + volume Docker)
→ Aucune modification de `docker-compose.yml` ni de `settings.js` nécessaire.

### Fichiers modifiés

| Fichier | Changement |
|---|---|
| `flux-nodered/meteo.json` | Persistance MQTT retained + Open-Meteo 5 min + keepalive 5 min |

### Comment ça marche

```
Chaque fois que pvinv_baseline change (nouveau message PVInverter) :
  pvinv_daily_fn (output 2) → pvinv_persist_out
  → publish retain:true sur santuario/persist/pvinv_baseline
  → Mosquitto stocke sur disque (persistence=true + volume dalybms-mosquitto-data)

Au démarrage Node-RED (ou reboot Pi5) :
  pvinv_persist_in (rap:true) → reçoit immédiatement le retained depuis Mosquitto local
  → restore_baseline_fn → global.set('pvinv_baseline', valeur_restaurée)
  Bridge NanoPi reconnecte ~30s après
  → 1er message PVInverter reçu → baseline déjà présente → delta correct ✓

Reset minuit (00:00) :
  midnight_reset_fn (output 2) → publish payload="" retain:true
  → efface le retained dans Mosquitto → baseline nulle dès le lendemain
  → 1er message PVInverter du matin → nouvelle baseline posée
```

### Fréquences de polling

| Composant | Intervalle | Raison |
|---|---|---|
| Open-Meteo API | **5 min** (300s) | Météo suffisamment fraîche, limite API respectée |
| Keepalive Venus OS | **5 min** (300s) | "Dernière mise à jour" max 5 min dans widget Victron |

### Variables mémoire (recalculées au démarrage)

```
outdoor_temp / outdoor_humidity / outdoor_pressure / outdoor_wind_*
irradiance_wm2
```
→ Open-Meteo `once:true` (fire ~0.1s) + irradiance MQTT retained → disponibles rapidement

### Vérification baseline persistée (📍 Pi5)

```bash
mosquitto_sub -h localhost -p 1883 -t 'santuario/persist/pvinv_baseline' -C 1
# → doit afficher la valeur cumulative kWh (ex: 587.2)
# Si rien n'apparaît : baseline pas encore publiée (premier déploiement)
# → attendre que pvinv_daily_fn reçoive un message PVInverter
```

### Procédure de déploiement (voir §PROC ci-dessous)

> **IMPORTANT** : `make reset` efface le volume Mosquitto (retained perdu).
> Utiliser `make down && make up` — volumes préservés.

---

## 15d. INTÉGRATION VENUS OS — CORRECTIONS (2026-03-22) ✅

**Branche** : `claude/review-venus-integration-35qN7`

### Corrections appliquées

| # | Problème | Fix | Commit |
|---|---|---|---|
| 1 | `TodaysYield` affichait cumul brut (1002 kWh) | Topic pvinverter `32` → wildcard `+` | `1597872` |
| 2 | `ExternalTemperature: -` dans widget météo | Retiré de `meteo_service.rs` | `939595f` |
| 3 | Baseline PVInverter perdue après reset mid-journée | `fix-pvinv-baseline.json` (flow Node-RED) | `3ce46de` |
| 4 | Documentation manquante | CLAUDE.md inventaire D-Bus + procédures | `b8087d2` |
| 5 | Capteur irradiance RS485 (PRALRAN) non intégré | Service systemd + flow Node-RED + fix apt | `bbb5ef8` |

### État final widget météo Victron (Capteur [40]) — vérifié 2026-03-22 ✅

```
Irradiance       : 334 W/m²         ← capteur RS485 PRALRAN actif
Production solaire: ☀ 11 kWh        ← TodaysYield correct
Dernières 24h    : 12.6 kWh         ← hier correct
Température      : -                 ← limitation Venus OS (inévitable)
Dernière màj     : 4 minutes ago    ← keepalive 25s actif
```

**Capteur "Temperature Extérieure"** (widget séparé) : 9.0°C, humidité 66%, pression 1013 hPa
→ via `com.victronenergy.temperature.mqtt_1` (type 4=Outdoor) ✅

### Node-RED — Production solaire (TodaysYield)

- **MPPT** : abonnement `N/c0619ab9929a/solarcharger/+/History/Daily/0/Yield` → `mppt_yield_today`
- **PVInverter** : abonnement `N/c0619ab9929a/pvinverter/+/Ac/Energy/Forward` → delta journalier
  - Service D-Bus réel : `com.victronenergy.pvinverter.cgwacs_ttyUSB0_mb2`
  - Reset automatique à 00:00 via inject cron
- **Total** : `total_yield_today = mppt + pvinv` → publié dans `TodaysYield` toutes les 25s

**Réalisé** :
1. ✅ Flows importés dans Node-RED Docker (Pi5:1880)
2. ✅ Connectivité MQTT vérifiée (broker 192.168.1.120:1883)
3. ✅ Node-RED arrêté sur NanoPi
4. ✅ ~100 MB RAM libérée sur NanoPi

---

## 15b. PERSISTANCE SERVICE VENUS OS (RÉSOLU)

**Problème** : Après reboot NanoPi, le symlink `/service/dbus-mqtt-venus` disparaissait.

**Cause** : Venus OS recrée `/service/` depuis son registre au boot — symlinks manuels non préservés.

**Solution** : `/data/rc.local` (mécanisme officiel Venus OS, survit aux firmware updates) :
```bash
#!/bin/sh
ln -sf /data/etc/sv/dbus-mqtt-venus /service/dbus-mqtt-venus
```

**Script `install-venus.sh`** mis à jour pour créer automatiquement `/data/rc.local`.

---

## 15c. PROBLÈMES CONNUS VENUS OS POST-MIGRATION

### MPPT Solar (SmartSolar MPPT VE.Can 250/100 rev2)
- Connexion : VE.Direct interne sur `ttyS1` ou `ttyS2`
- **Race condition au boot** : parfois absent au 1er reboot, présent au 2ème
- Workaround : `svc -t /service/vedirect-interface.ttyS1`
- Non lié à notre migration

### Shelly (AC Meter [50] et [51])
- Parfois absent après reboot — à investiguer
- Non critique, à traiter ultérieurement

---

## 16. AJOUTER UN APPAREIL DEPUIS NODE-RED (PI5) → VENUS OS

Le principe est simple : Node-RED publie un JSON sur un topic MQTT, et `dbus-mqtt-venus`
sur le NanoPi le reçoit et crée/met à jour le service D-Bus correspondant.

### Étape 1 — Déclarer l'appareil dans config.toml (NanoPi)

Éditer `/data/daly-bms/config.toml` sur le NanoPi :

**Capteur température** (ex: sonde DS18B20, Shelly TRV, API cloud…)
```toml
[[sensors]]
mqtt_index      = 2           # → topic santuario/heat/2/venus
name            = "Eau chaude"
temperature_type = 5          # 0=battery 1=fridge 2=generic 3=room 4=outdoor 5=waterheater 6=freezer
device_instance = 102         # n° unique dans VRM/Victron
```

**PAC / Chauffe-eau**
```toml
[[heatpumps]]
mqtt_index      = 2
name            = "PAC Climatisation"
device_instance = 202
```

**Switch / ATS**
```toml
[[switches]]
mqtt_index      = 2
name            = "ATS Groupe"
device_instance = 302
```

**Compteur réseau / acload**
```toml
[[grids]]
mqtt_index      = 2
name            = "Compteur Fronius"
device_instance = 402
service_type    = "grid"    # "grid" ou "acload"
```

Puis redémarrer le service : `svc -t /service/dbus-mqtt-venus`

### Étape 2 — Publier depuis Node-RED (Pi5)

Dans Node-RED (`http://192.168.1.141:1880`), utiliser un nœud **mqtt out** :

- Serveur MQTT : `192.168.1.120:1883` (broker NanoPi — ou `192.168.1.141:1883` si bridge actif)
- Topic : `santuario/heat/2/venus`
- QoS : 0, Retain : true (pour que Venus OS retrouve la valeur après reboot)

**Payload JSON capteur température** :
```json
{
  "Temperature": 42.5,
  "TemperatureType": 5,
  "Status": 0,
  "ProductName": "Eau chaude sanitaire",
  "CustomName": "Ballon ECS"
}
```

**Payload JSON switch/ATS** :
```json
{
  "State": 1,
  "Position": 2,
  "ProductName": "ATS CHINT",
  "CustomName": "Groupe électrogène"
}
```

**Payload JSON compteur grid** :
```json
{
  "Ac/L1/Power": 1250.0,
  "Ac/L2/Power": 800.0,
  "Ac/L3/Power": 430.0,
  "Ac/L1/Voltage": 230.0,
  "Ac/L1/Current": 5.43
}
```

### Étape 3 — Vérifier l'arrivée sur D-Bus

```bash
ssh root@192.168.1.120
dbus -y | grep victronenergy                          # service doit apparaître
dbus -y com.victronenergy.temperature.mqtt_2 / GetItems
```

### Checklist ajout appareil

- [ ] `[[sensors]]` / `[[heatpumps]]` / `[[switches]]` / `[[grids]]` ajouté dans config.toml
- [ ] `device_instance` unique (pas de conflit avec autres appareils VRM)
- [ ] `svc -t /service/dbus-mqtt-venus` exécuté après modif config
- [ ] Flow Node-RED publiant en **retain:true** sur le bon topic
- [ ] Vérifié avec `dbus -y | grep victronenergy` sur NanoPi

---

## 16b. CAPTEUR IRRADIANCE RS485 ✅ OPÉRATIONNEL (2026-03-22)

**Matériel** : Solar Radiation Sensor PRALRAN, FTDI FT232 USB-RS485
- Branché sur : `Bus 004 Device 002` Pi5 → `/dev/ttyUSB1`
- Adresse Modbus RTU : `0x05` (configurée sur le capteur)
- Registre : `0x0000` → irradiance W/m² (FC=0x04, uint16 big-endian)
- Baud : 9600 8N1 (default usine)

**État** : ✅ Fonctionnel — 334 W/m² visibles dans widget météo Victron

**Architecture** :
```
Capteur RS485 (/dev/ttyUSB1)
  ↓ Modbus RTU (service systemd irradiance-rs485)
irradiance_reader.py  →  MQTT santuario/irradiance/raw  →  localhost:1883
  ↓ Node-RED (onglet "Irradiance RS485")
global.irradiance_wm2
  ↓ keepalive 25s (onglet "Meteo & ET112")
santuario/meteo/venus  →  bridge Mosquitto  →  NanoPi
  ↓ dbus-mqtt-venus
com.victronenergy.meteo /Irradiance  →  VRM widget météo
```

**Fichiers** :
```
contrib/irradiance-rs485/
├── irradiance_reader.py      ← bridge Python (Modbus RTU → MQTT)
├── irradiance-rs485.service  ← unité systemd
└── install.sh                ← script d'installation Pi5

flux-nodered/
└── irradiance-rs485.json     ← flow Node-RED (MQTT subscriber → global)
```

**Installation sur Pi5** (à refaire après un reinstall OS) :
```bash
cd ~/Daly-BMS-Rust
git pull origin claude/review-venus-integration-35qN7
bash contrib/irradiance-rs485/install.sh
# Dépendances installées via apt (python3-serial python3-paho-mqtt)
# Puis importer flux-nodered/irradiance-rs485.json dans Node-RED
# Et déployer le flow meteo.json mis à jour
```

> **NOTE** : `install.sh` utilise `sudo apt-get install python3-serial python3-paho-mqtt`
> (pas `pip3` — non disponible sur Raspberry Pi OS par défaut).

**Diagnostic** :
```bash
# Service systemd
systemctl status irradiance-rs485
journalctl -u irradiance-rs485 -f

# Valeur MQTT brute W/m²
mosquitto_sub -h localhost -p 1883 -t 'santuario/irradiance/raw' -v

# D-Bus NanoPi
ssh root@192.168.1.120 "dbus -y com.victronenergy.meteo /Irradiance GetValue"
```

**Identifier le bon port ttyUSB** :
```bash
# Comparer avec les identifiants USB physiques
ls -la /dev/serial/by-id/
# BMS (Bus 002) → ttyUSB0, Irradiance (Bus 004) → ttyUSB1
# Si inversé : modifier SERIAL_PORT dans contrib/irradiance-rs485/irradiance_reader.py
```

**Note baud rate** : 9600 baud est le défaut usine (§4 PDF). Si le capteur ne répond pas, essayer 4800 baud.

---

## 16c. INTÉGRATION ET112 CARLO GAVAZZI (2026-03-22) ✅

**Objectif** : Remplacer le capteur irradiance RS485 (déplacé) par un compteur ET112
connecté sur le même port `/dev/ttyUSB1`, et l'exposer comme `com.victronenergy.pvinverter`
sur le D-Bus Venus OS (micro-inverseurs, instance 63).

### Matériel

| Composant | Détail |
|---|---|
| Compteur | Carlo Gavazzi ET112 |
| Interface | FTDI FT232 USB-RS485 — Pi5 Bus 004 Device 003 |
| Port | `/dev/ttyUSB1` |
| Adresse Modbus | `0x03` (slave address configurée sur le compteur) |
| Baud | 9600 8N1 |
| Protocole | Modbus RTU FC=04, FLOAT32 Big-Endian |

### Registres Modbus ET112 (FLOAT32 = 2 words par registre)

| Adresse | Grandeur | Unité |
|---|---|---|
| 0x0000 | Tension V | V |
| 0x0002 | Courant I | A |
| 0x0004 | Puissance active | W |
| 0x0006 | Puissance apparente | VA |
| 0x0008 | Puissance réactive | VAR |
| 0x000A | Facteur de puissance | — |
| 0x000C | Angle de phase | ° |
| 0x000E | Fréquence | Hz |
| 0x0010 | Énergie importée | Wh |
| 0x0012 | Énergie exportée | Wh |

### Architecture

```
ET112 (/dev/ttyUSB1, addr 0x03)
  ↓ Modbus RTU (tokio-modbus 0.14, FLOAT32 Big-Endian)
daly-bms-server — et112::run_et112_poll_loop (5s)
  ├── AppState::on_et112_snapshot() → ring buffer 720 entrées
  ├── API REST : GET /api/v1/et112, /api/v1/et112/3/status, /api/v1/et112/3/history
  ├── Dashboard SSR : /dashboard/et112/3 (ECharts temps réel)
  ├── MQTT : santuario/pvinverter/3/venus (retain=true)
  └── InfluxDB : measurement et112_status (tags: address, name)

santuario/pvinverter/3/venus → (NanoPi broker)
  ↓ dbus-mqtt-venus — PvinverterManager → PvinverterServiceHandle
com.victronenergy.pvinverter.mqtt_3 (device instance 63)
  ├── /Ac/Power
  ├── /Ac/Energy/Forward
  ├── /Ac/L1/{Voltage, Current, Power, Energy/Forward}
  ├── /StatusCode=7 (Running)
  ├── /ErrorCode=0
  ├── /Position=1 (AC Output — micro-inverseurs sur AC output)
  └── /IsGenericEnergyMeter=1 (ET112 masquerade)
```

### Fichiers créés/modifiés

| Fichier | Modification |
|---|---|
| `crates/daly-bms-server/src/et112/types.rs` | Struct `Et112Snapshot` |
| `crates/daly-bms-server/src/et112/poll.rs` | Polling Modbus RTU async |
| `crates/daly-bms-server/src/et112/mod.rs` | Module public |
| `crates/daly-bms-server/src/api/et112.rs` | Endpoints REST |
| `crates/daly-bms-server/src/dashboard/mod.rs` | Handler dashboard ET112 |
| `crates/daly-bms-server/templates/et112.html` | Template Askama |
| `crates/daly-bms-server/templates/base.html` | Lien nav ET112 |
| `crates/daly-bms-server/src/bridges/mqtt.rs` | `publish_et112_snapshot()` |
| `crates/daly-bms-server/src/bridges/influx.rs` | `et112_snapshot_to_point()` |
| `crates/dbus-mqtt-venus/src/types.rs` | `PvinverterPayload` |
| `crates/dbus-mqtt-venus/src/config.rs` | `PvinverterConfig`, `PvinverterRef` |
| `crates/dbus-mqtt-venus/src/mqtt_source.rs` | `start_pvinverter_mqtt_source()` |
| `crates/dbus-mqtt-venus/src/pvinverter_service.rs` | Service D-Bus pvinverter |
| `crates/dbus-mqtt-venus/src/pvinverter_manager.rs` | Manager pvinverter |
| `crates/dbus-mqtt-venus/src/main.rs` | Wire-up PvinverterManager |
| `Config.toml` | Section `[et112]` + `[[et112.devices]]` |
| `nanoPi/config-nanopi.toml` | Section `[pvinverter]` + `[[pvinverters]]` |

### Architecture cible : 3 ET112 sur Pi5 (migration depuis Victron)

**Rôle de chaque ET112 (décision 2026-03-22)** :

| ET112 | Usage | Type D-Bus | Topic MQTT | Device instance |
|---|---|---|---|---|
| addr 0x03 (actuel Pi5) | **Micro-inverseurs** | `pvinverter` | `santuario/pvinverter/3/venus` | 63 |
| addr à définir | **PAC Climatisation** (AC Out 1) | `acload` | `santuario/grid/4/venus` | 501 |
| addr à définir | **Chauffe-eau** (AC Out 1) | `acload` | `santuario/grid/5/venus` | 502 |

**État actuel** :
- `com.victronenergy.pvinverter.cgwacs_ttyUSB0_mb2` = ET112 micro-inverseurs connecté directement sur le Victron (Modbus addr 0x02)
- Les 2 ET112 acload (PAC + chauffe-eau) ne sont **pas encore installés**
- Le ET112 Pi5 (addr 0x03, mqtt_index=3) est **déjà actif** mais pvinverter config pas encore dans config.toml NanoPi

**Procédure de migration (quand prêt)** :
1. Brancher les 3 ET112 sur Pi5 (`/dev/ttyUSB1`, `/dev/ttyUSB2`, `/dev/ttyUSB3`)
2. Configurer adresses RS485 sur les ET112 : 0x03=micro-inv, 0x04=PAC, 0x05=chauffe-eau (ou selon dispo)
3. Dans `Config.toml` Pi5 : ajouter `[[et112.devices]]` pour chaque ET112, avec un champ `service_type` ("pvinverter" ou "acload")
4. Adapter `et112/poll.rs` pour publier sur `santuario/pvinverter/{n}/venus` ou `santuario/grid/{n}/venus` selon `service_type`
5. Dans `nanoPi/config-nanopi.toml` : décommenter les entrées `[[grids]]` mqtt_index=4 et 5
6. Débrancher l'ancien câble USB Victron → `cgwacs_ttyUSB0_mb2` disparaît du D-Bus
7. Déployer `daly-bms-server` (Pi5) + `dbus-mqtt-venus` (NanoPi) + redémarrer

**Format JSON pour acload ET112** (compatible `GridPayload`) :
```json
{
  "Ac": {
    "L1": { "Voltage": 230.1, "Current": 8.2, "Power": 1886.0,
            "Energy": { "Forward": 142.5, "Reverse": 0.0 } }
  },
  "DeviceType": 340,
  "IsGenericEnergyMeter": 1
}
```
Publié sur `santuario/grid/4/venus` (PAC) et `santuario/grid/5/venus` (chauffe-eau).

### Topics MQTT pvinverter

```
santuario/pvinverter/3/venus  ← ET112 addr=0x03 (Pi5 → NanoPi)
```

Format JSON (compatible `dbus-mqtt-venus` PvinverterPayload) :
```json
{
  "Ac": {
    "L1": { "Voltage": 230.1, "Current": 5.43, "Power": 1250.0,
            "Energy": { "Forward": 587.23, "Reverse": 0.0 } },
    "Power": 1250.0,
    "Energy": { "Forward": 587.23, "Reverse": 0.0 }
  },
  "StatusCode": 7,
  "ErrorCode": 0,
  "Position": 1,
  "IsGenericEnergyMeter": 1,
  "ProductName": "ET112 addr=0x03",
  "CustomName": "Micro-inverseurs"
}
```

### Vérification D-Bus (📍 NanoPi)

```bash
# Vérifier que le service apparaît
dbus -y | grep pvinverter

# Lire la puissance instantanée
dbus -y com.victronenergy.pvinverter.mqtt_3 /Ac/Power GetValue

# Lire l'énergie totale
dbus -y com.victronenergy.pvinverter.mqtt_3 /Ac/Energy/Forward GetValue

# Tous les chemins
dbus -y com.victronenergy.pvinverter.mqtt_3 / GetItems
```

### Diagnostic Pi5

```bash
# Dashboard web
http://192.168.1.141:8080/dashboard/et112/3

# API REST
curl http://192.168.1.141:8080/api/v1/et112/3/status
curl http://192.168.1.141:8080/api/v1/et112/3/history?limit=60

# MQTT publié
mosquitto_sub -h 192.168.1.120 -p 1883 -t 'santuario/pvinverter/3/venus' -v

# InfluxDB
# measurement: et112_status, tags: address=0x03, name=Micro-inverseurs
```

---

## 17. MAINTENANCE OPÉRATIONNELLE

### Checklist quotidienne / hebdomadaire

```bash
# Sur Pi5 : état global
systemctl status daly-bms
journalctl -u daly-bms --since "1 hour ago" | grep -E "ERROR|WARN"

# Docker
docker compose ps
docker compose logs --since 1h | grep -i error

# Sur NanoPi : état Venus bridge
ssh root@192.168.1.120 "svstat /service/dbus-mqtt-venus"
ssh root@192.168.1.120 "tail -20 /var/log/dbus-mqtt-venus/current"
```

### Mise à jour dbus-mqtt-venus (flux complet)

```bash
# Sur Pi5 (dans ~/Daly-BMS-Rust)
git pull origin <branche>          # récupérer les changements
make build-venus-v7                # compiler armv7
make install-venus-v7              # déployer (arrêt auto, copie, redémarrage)

# Vérifier
ssh root@192.168.1.120 "svstat /service/dbus-mqtt-venus"
ssh root@192.168.1.120 "dbus -y | grep victronenergy"
```

### Mise à jour daly-bms-server (Pi5)

```bash
git pull origin <branche>
make build-arm                     # aarch64 Pi5
sudo systemctl stop daly-bms
sudo cp target/aarch64-unknown-linux-gnu/release/daly-bms-server /usr/local/bin/
sudo systemctl start daly-bms
journalctl -u daly-bms -f
```

### Redémarrage propre de l'infrastructure

```bash
# Tout redémarrer sans perte de données
make down && make up               # Docker (Mosquitto, InfluxDB, Grafana, Node-RED)
sudo systemctl restart daly-bms   # BMS server
ssh root@192.168.1.120 "svc -t /service/dbus-mqtt-venus"  # Venus bridge
```

### Services D-Bus présents en production (état nominal)

```
com.victronenergy.battery.mqtt_1                    ← BMS-360Ah (instance 141)
com.victronenergy.battery.mqtt_2                    ← BMS-320Ah (instance 142)
com.victronenergy.temperature.mqtt_1                ← capteur température extérieure (type 4=Outdoor)
com.victronenergy.heatpump.mqtt_1                   ← PAC / chauffe-eau
com.victronenergy.switch.*                          ← ATS / relais (si configuré)
com.victronenergy.grid.*                            ← compteur réseau (si configuré)
com.victronenergy.meteo                             ← capteur irradiance + TodaysYield
com.victronenergy.pvinverter.cgwacs_ttyUSB0_mb2     ← onduleur PV (cgwacs Modbus ttyUSB0 addr 2)
```

Si un service manque : vérifier logs `dbus-mqtt-venus` ET que le topic MQTT est bien publié.

> **IMPORTANT** : Le nom exact du PV inverter est `cgwacs_ttyUSB0_mb2` — NE PAS utiliser `rs485`.
> Toujours vérifier avec `dbus -y | grep pvinverter` avant toute commande.

### Sauvegarde config NanoPi

La config `/data/daly-bms/config.toml` sur le NanoPi est **persistante** (volume `/data`
survivre aux mises à jour Venus OS). Mais la copier dans le repo pour traçabilité :

```bash
scp root@192.168.1.120:/data/daly-bms/config.toml nanoPi/config-nanopi.toml
git add nanoPi/config-nanopi.toml
git commit -m "chore(nanopi): backup config.toml"
```

---

## 18. RÈGLES DE TRAVAIL AVEC CLAUDE

1. **Toujours lire ce fichier en début de session** avant toute action.
2. **Git pull sur Pi5 avant déploiement** — ne jamais assumer que le Pi5 est à jour.
3. **Ne jamais déployer daly-bms-server sur NanoPi** — uniquement `dbus-mqtt-venus`.
4. **Tester la compilation avant de déployer** : `make build-venus-v7` d'abord.
5. **Commit + push systématique** après chaque changement validé.
6. **Branche courante** : vérifier `git branch` avant tout push.
7. **Secrets** : ne jamais committer `.env`, `Config.toml` contient des valeurs mais pas les tokens (dans `.env`).
8. **Architecture armv7** : NanoPi = armv7, Pi5 = aarch64. Ne pas confondre les binaires.
9. **SSH** : utiliser `ssh root@192.168.1.120` (pas `nanopi`) pour éviter les problèmes de config.
10. **Service Venus** : arrêter avant toute copie de binaire (`svc -d /service/dbus-mqtt-venus`).
11. **CLAUDE.md = mémoire projet** : toute information découverte (nom de service réel, limitation, procédure) doit être ajoutée ICI immédiatement, puis committée. Ne jamais redemander la même information à l'utilisateur.
12. **Avant toute commande D-Bus** : toujours vérifier le nom exact du service avec `dbus -y | grep <type>` — les noms peuvent être inattendus (ex: `cgwacs_ttyUSB0_mb2` et non `rs485`).

---

## 19. INVENTAIRE MATÉRIEL D-BUS PRODUCTION (vérifié 2026-03-22)

| Service D-Bus | Description | Commande vérification |
|---|---|---|
| `com.victronenergy.battery.mqtt_1` | BMS-360Ah | `dbus -y ... /Soc GetValue` |
| `com.victronenergy.battery.mqtt_2` | BMS-320Ah | `dbus -y ... /Soc GetValue` |
| `com.victronenergy.temperature.mqtt_1` | Capteur ext. (type 4) | `dbus -y ... /Temperature GetValue` |
| `com.victronenergy.heatpump.mqtt_1` | PAC / chauffe-eau | `dbus -y ... /State GetValue` |
| `com.victronenergy.meteo` | Irradiance + TodaysYield | `dbus -y ... /TodaysYield GetValue` |
| `com.victronenergy.pvinverter.mqtt_3` | ET112 micro-inverseurs Pi5 (instance 63) | `dbus -y ... /Ac/Power GetValue` |
| `com.victronenergy.pvinverter.cgwacs_ttyUSB0_mb2` | Onduleur PV AC (Victron direct) | `dbus -y ... /Ac/Energy/Forward GetValue` |

### Commandes de diagnostic rapide (📍 NanoPi)

```bash
# Lister tout ce qui tourne
dbus -y | grep victronenergy

# Vérifier les valeurs clés
dbus -y com.victronenergy.meteo /TodaysYield GetValue
dbus -y com.victronenergy.temperature.mqtt_1 /Temperature GetValue
dbus -y com.victronenergy.temperature.mqtt_1 /Connected GetValue
dbus -y com.victronenergy.pvinverter.cgwacs_ttyUSB0_mb2 /Ac/Energy/Forward GetValue
dbus -y com.victronenergy.battery.mqtt_1 /Soc GetValue
dbus -y com.victronenergy.battery.mqtt_2 /Soc GetValue
```

### Architecture température (décision définitive 2026-03-22)

La température extérieure est publiée **uniquement** via `com.victronenergy.temperature.mqtt_1` (type 4=Outdoor).

- `/ExternalTemperature` est **absent** de `com.victronenergy.meteo` et du struct `MeteoValues`.
- **"Température: -"** dans le widget météo Venus OS est **inévitable** : le champ est codé en dur dans le QML Venus OS et s'affiche toujours, que le chemin D-Bus existe ou non. Aucun contournement possible depuis notre code.
- La température réelle est accessible via `com.victronenergy.temperature.mqtt_1` (Connected=1, ~8.8°C).
- **NE PAS réajouter** `external_temperature` dans `MeteoValues` ni `meteo_service.rs`.

### Limitations connues Venus OS

- **"Dernières 24h" dans widget météo** : valeur correcte depuis la correction baseline PVInverter (6.6 kWh). Était erronée (671.1 kWh) car `TodaysYield` retenait l'ancienne valeur cumulative brute.
- **MPPT SmartSolar VE.CAN** : race condition au boot (cf. §15c).
