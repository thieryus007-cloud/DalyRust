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
│    nodered     1880  ← migré depuis NanoPi (en cours)            │
└──────────┬───────────────────────────────────────────────────────┘
           │ MQTT 192.168.1.120:1883
           ▼
┌──────────────────────────────────────────────────────────────────┐
│  NanoPi (Venus OS / Victron GX)  192.168.1.120  user: root       │
│  /data/daly-bms/                                                 │
│                                                                  │
│  daly-bms-venus (runit service /service/daly-bms-venus)          │
│    ├── Subscribe MQTT → bridge D-Bus                             │
│    ├── com.victronenergy.battery.mqtt_bms1 (instance 141)        │
│    ├── com.victronenergy.battery.mqtt_bms2 (instance 142)        │
│    ├── com.victronenergy.temperature.*                           │
│    ├── com.victronenergy.heatpump.*                              │
│    └── com.victronenergy.meteo                                   │
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
- **Branche de travail courante** : `claude/migrate-nodered-pi5-91idx`
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
│   ├── daly-bms-venus/          ← Bridge MQTT→D-Bus (Venus OS)
│   ├── daly-bms-cli/            ← Outil diagnostic CLI
│   └── daly-bms-probe/          ← Sonde protocole bas niveau
│
├── nanoPi/
│   ├── install-venus.sh         ← Script déploiement NanoPi
│   ├── cleanup-dbus-serialbattery.sh
│   ├── config-bms1.ini          ← Config dbus-mqtt-battery instance 41
│   ├── config-bms2.ini          ← Config dbus-mqtt-battery instance 42
│   ├── sv/daly-bms-venus/run    ← Script runit (daemontools)
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
make build-venus     # daly-bms-venus aarch64
make build-venus-v7  # daly-bms-venus armv7 ← pour NanoPi
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
make install-venus-v7  # Déployer daly-bms-venus sur NanoPi (armv7)
make install-venus     # Déployer daly-bms-venus sur NanoPi (aarch64)
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
├── daly-bms-venus          ← binaire (armv7)
├── config.toml             ← config Venus OS
└── (logs via runit)

/data/etc/sv/daly-bms-venus/
└── run                     ← script runit

/service/daly-bms-venus     ← symlink → /data/etc/sv/daly-bms-venus
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
2. Crée `/data/daly-bms/` et `/data/etc/sv/daly-bms-venus/`
3. Arrête le service avant de copier le binaire (évite "dest open Failure")
4. Supprime daly-bms-server s'il existe (ne doit PAS tourner sur NanoPi)
5. Copie le binaire `daly-bms-venus`
6. Copie `config.toml` si absent
7. Copie le script runit `run`
8. Crée le symlink `/service/daly-bms-venus` (activation automatique)
9. Vérifie le démarrage

### Commandes de diagnostic sur NanoPi

```bash
ssh root@192.168.1.120

# État du service
svstat /service/daly-bms-venus

# Logs en temps réel
tail -f /var/log/daly-bms-venus/current

# Redémarrer
svc -t /service/daly-bms-venus

# Arrêter
svc -d /service/daly-bms-venus

# Démarrer
svc -u /service/daly-bms-venus

# Vérifier D-Bus
dbus-send --system --print-reply --dest=com.victronenergy.battery.mqtt_bms1 \
    / com.victronenergy.BusItem.GetValue

# Vérifier MQTT reçu
mosquitto_sub -h 127.0.0.1 -p 1883 -t "santuario/bms/#" -v
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

**Cause** : Le binaire `daly-bms-venus` est en cours d'exécution, on ne peut pas l'écraser.
**Solution** : Le script arrête le service AVANT la copie (étape 2 dans install-venus.sh).
Si ça se reproduit : `ssh root@192.168.1.120 "svc -d /service/daly-bms-venus"` puis redéployer.

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

### Problème : D-Bus Venus non visible dans Victron GUI

**Cause** : Service non démarré ou nom de service incorrect.
**Vérification** :
```bash
dbus-spy  # ou
dbus-monitor --system "type=signal,sender=com.victronenergy.battery.mqtt_bms1"
```

---

## 11. BINAIRES & CIBLES DE COMPILATION

| Binaire | Cible | Usage |
|---------|-------|-------|
| `daly-bms-server` | x86_64 / aarch64 | Pi5 (serveur principal) |
| `daly-bms-venus` | armv7-unknown-linux-gnueabihf | NanoPi Venus OS |
| `daly-bms-venus` | aarch64-unknown-linux-gnu | NanoPi si aarch64 |
| `daly-bms-cli` | x86_64 | Diagnostic local |
| `daly-bms-probe` | x86_64 | Test protocole RS485 |

### Emplacements après build

```
target/release/daly-bms-server
target/armv7-unknown-linux-gnueabihf/release/daly-bms-venus
target/aarch64-unknown-linux-gnu/release/daly-bms-venus
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

## 13. TOPICS MQTT

```
santuario/bms/1/venus    ← payload Venus OS BMS-1 (JSON)
santuario/bms/2/venus    ← payload Venus OS BMS-2 (JSON)
santuario/bms/1/raw      ← données brutes BMS-1
santuario/bms/2/raw      ← données brutes BMS-2
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

## 15. MIGRATION NODE-RED (EN COURS)

**Objectif** : Déplacer Node-RED du NanoPi vers le Pi5 dans Docker.

**État** : Branch `claude/migrate-nodered-pi5-91idx`

**Flows** : `flux-nodered/` — flows exportés depuis NanoPi.

**Étapes restantes** :
1. Importer les flows dans Node-RED Docker (Pi5:1880)
2. Vérifier connectivité MQTT (broker reste sur 192.168.1.120:1883 pour Venus)
3. Arrêter Node-RED sur NanoPi
4. Libérer ~100 MB RAM sur NanoPi

---

## 16. RÈGLES DE TRAVAIL AVEC CLAUDE

1. **Toujours lire ce fichier en début de session** avant toute action.
2. **Git pull sur Pi5 avant déploiement** — ne jamais assumer que le Pi5 est à jour.
3. **Ne jamais déployer daly-bms-server sur NanoPi** — uniquement `daly-bms-venus`.
4. **Tester la compilation avant de déployer** : `make build-venus-v7` d'abord.
5. **Commit + push systématique** après chaque changement validé.
6. **Branche courante** : vérifier `git branch` avant tout push.
7. **Secrets** : ne jamais committer `.env`, `Config.toml` contient des valeurs mais pas les tokens (dans `.env`).
8. **Architecture armv7** : NanoPi = armv7, Pi5 = aarch64. Ne pas confondre les binaires.
9. **SSH** : utiliser `ssh root@192.168.1.120` (pas `nanopi`) pour éviter les problèmes de config.
10. **Service Venus** : arrêter avant toute copie de binaire (`svc -d /service/daly-bms-venus`).
