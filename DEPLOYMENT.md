# 🚀 SOLUTION PROFESSIONNELLE COMPLÈTE — DÉPLOIEMENT ESS

**Version:** 1.0 (2026-04-05)  
**Branche:** `claude/realtime-metrics-dashboard-lUKF3`  
**Statut:** ✅ PRODUCTION READY

---

## 📊 ARCHITECTURE SYSTÈME

```
┌─────────────────────────────────────────────────────────┐
│                    FRONTEND WEB (Browser)                │
│  http://192.168.1.141:8080/visualization               │
│  ├─ ReactFlow Dashboard (temps réel)                    │
│  ├─ Polling API (2s)                                    │
│  └─ WebSocket fallback (40ms)                           │
└────────────────────┬────────────────────────────────────┘
                     │ HTTP/WebSocket
┌────────────────────▼────────────────────────────────────┐
│              BACKEND (daly-bms-server - Pi5)             │
│  192.168.1.141:8080                                      │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────────────────────────────────────────┐   │
│  │  STATE (AppState - Mémoire partagée)            │   │
│  ├─ VenusMppt (HashMap)                            │   │
│  ├─ VenusSmartShunt (Option)                       │   │
│  ├─ VenusTemperature (HashMap)                     │   │
│  ├─ BMS Buffers                                    │   │
│  ├─ ET112 Buffers                                  │   │
│  └─ Tasmota Buffers                                │   │
│  └─────────────────────────────────────────────────┘   │
│                 ↑           ↓           ↑               │
│          MQTT Sub    REST API    WebSocket              │
│                                                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │ ENDPOINTS                                        │   │
│  ├─ GET /api/v1/venus/mppt                         │   │
│  ├─ GET /api/v1/venus/smartshunt                   │   │
│  ├─ GET /api/v1/venus/temperatures                 │   │
│  ├─ GET /api/v1/system/totals                      │   │
│  ├─ GET /api/v1/bms/{id}/status                    │   │
│  ├─ GET /api/v1/et112/{addr}/status                │   │
│  ├─ GET /ws/venus/stream (WebSocket)               │   │
│  └─ GET /ws/bms/stream (WebSocket)                 │   │
│  └──────────────────────────────────────────────────┘   │
└────────────────────┬────────────────────────────────────┘
         │           │           │
    RS485│       MQTT│       MQTT│
    BUS  │      BRIDGE       INFLUX
         │           │           │
┌────────▼───────────▼───────────▼────────────────────────┐
│           MQTT BROKER (192.168.1.120:1883)              │
│  - Mosquitto                                             │
│  - Topics: santuario/*                                  │
│  - Retention enabled                                    │
└────────────┬──────────────────────┬─────────────────────┘
             │                      │
             │                      └──────────────┐
             │                                     │
    ┌────────▼──────────────────┐    ┌──────────▼──────────────┐
    │  NanoPi (Venus OS GX)      │    │  InfluxDB (Pi5 Docker)   │
    │  192.168.1.120             │    │  http://localhost:8086   │
    ├────────────────────────────┤    ├──────────────────────────┤
    │  dbus-mqtt-venus           │    │  Bucket: daly_bms        │
    │  (runit service)           │    │  Retention: 30 days      │
    │                            │    │                          │
    │  Topics reçus:             │    │  Métriques stockées:     │
    │  ├─ santuario/meteo/venus  │    │  ├─ bms_snapshot        │
    │  ├─ santuario/heat/*/venus │    │  ├─ et112_status        │
    │  ├─ santuario/heatpump/*/v │    │  ├─ venus_mppt          │
    │  └─ santuario/system/venus │    │  └─ tasmota_snapshot    │
    │                            │    │                          │
    │  D-Bus services créés:     │    │  Grafana (Pi5 Docker)    │
    │  ├─ com.victronenergy....  │    │  http://192.168.1.141:3001
    │  │    .battery.mqtt_*      │    │  (visualisation longue durée)
    │  ├─ .pvinverter.mqtt_*     │    └──────────────────────────┘
    │  ├─ .heatpump.mqtt_*       │
    │  ├─ .temperature.mqtt_*    │
    │  ├─ .switch.mqtt_*         │
    │  ├─ .meteo                 │
    │  └─ .system                │
    │                            │
    │  VRM Portal (en direct)    │
    │  (affichage Victron Cloud) │
    └────────────────────────────┘
```

---

## ✅ CONFIGURATION ACTUELLE

### **Pi5 (Serveur Principal)**

#### Config.toml - Sections clés
```toml
[serial]
addresses = ["0x01", "0x02"]  # BMS
port = ""  # Auto-détect /dev/ttyUSB0

[mqtt]
enabled = true
host = "192.168.1.120"
port = 1883

[et112]
poll_interval_ms = 5000

[[et112.devices]]
address = "0x07"  # ✅ Micro-onduleurs (ET112 SN:119253X)
name = "Micro Onduleurs"
mqtt_index = 7
service_type = "pvinverter"
device_instance = 32

[[et112.devices]]
address = "0x08"  # ✅ PAC Chauffe-eau (ET112 SN:119215X)
name = "PAC Chauffe-eau"
mqtt_index = 8
service_type = "heatpump"
device_instance = 30

[[et112.devices]]
address = "0x09"  # ✅ PAC Climatisation (ET112 SN:061077X)
name = "PAC Climatisation"
mqtt_index = 9
service_type = "heatpump"
device_instance = 31

[heat]
[[sensors]]
mqtt_index = 1  # Température extérieure
name = "Temperature Exterieure"
temperature_type = 4  # Outdoor
device_instance = 20

[irradiance]
address = "0x05"  # PRALRAN RS485
name = "Irradiance PRALRAN"
```

### **NanoPi (Venus OS)**

#### nanoPi/config-nanopi.toml - À COMPLÉTER
```toml
# À AJOUTER :
[[pvinverters]]
mqtt_index = 7        # ET112 0x07 Micro-onduleurs
name = "Micro-onduleurs"
device_instance = 32
service_type = "pvinverter"

[[heatpumps]]
mqtt_index = 8        # ET112 0x08 PAC Chauffe-eau
name = "PAC Chauffe-eau"
device_instance = 30

[[heatpumps]]
mqtt_index = 9        # ET112 0x09 PAC Climatisation
name = "PAC Climatisation"
device_instance = 31

[[smartshunts]]
name = "SmartShunt 500A"
device_instance = 100

[[mppts]]
name = "MPPT SolarCharger 250/100"
instance = 0
device_instance = 60
```

---

## 🔧 DÉPLOIEMENT COMPLET

### **PHASE 1 : Récupération du code (Pi5)**

```bash
cd ~/Daly-BMS-Rust

# Récupérer la branche
git fetch origin claude/realtime-metrics-dashboard-lUKF3
git checkout claude/realtime-metrics-dashboard-lUKF3
git pull origin claude/realtime-metrics-dashboard-lUKF3

# Vérifier les commits
git log --oneline -3
# b3cd73f fix(dashboard): Add /visualization route alias
# f21d7e1 feat(venus-dashboard): Complete real-time metrics visualization
```

### **PHASE 2 : Compilation (Pi5 - ~2 min)**

```bash
make build-arm

# Vérifier le binaire
ls -lh target/aarch64-unknown-linux-gnu/release/daly-bms-server
# Doit être ~25 MB
```

### **PHASE 3 : Déploiement (Pi5)**

```bash
# Arrêter le service
sudo systemctl stop daly-bms

# Déployer
sudo cp target/aarch64-unknown-linux-gnu/release/daly-bms-server /usr/local/bin/
sudo cp Config.toml /etc/daly-bms/config.toml

# Redémarrer
sudo systemctl start daly-bms
sleep 3

# Vérifier
sudo systemctl is-active daly-bms  # → active
journalctl -u daly-bms -n 50 --no-pager
```

### **PHASE 4 : Configuration NanoPi**

```bash
# Sur NanoPi via SSH
ssh root@192.168.1.120

# Éditer la config
vi /data/daly-bms/config.toml

# Ajouter les sections manquantes (voir ci-dessus)
# [[pvinverters]], [[heatpumps]], [[smartshunts]], [[mppts]]

# Redémarrer dbus-mqtt-venus
svc -t /service/dbus-mqtt-venus

# Vérifier le démarrage
sleep 5
svstat /service/dbus-mqtt-venus  # → up (pid XXX) Xs
```

### **PHASE 5 : Vérification**

#### 5A : Logs Pi5
```bash
journalctl -u daly-bms -n 30 --no-pager
# Doit contenir:
# - "Démarrage MQTT bridge"
# - "Démarrage Venus OS MQTT subscriber"
# - "Serveur HTTP écoute sur 0.0.0.0:8080"
```

#### 5B : Endpoints API
```bash
# MPPT
curl -s http://localhost:8080/api/v1/venus/mppt | jq '.mppts[0] // "None"'

# SmartShunt
curl -s http://localhost:8080/api/v1/venus/smartshunt | jq '.shunt // "None"'

# Temperatures
curl -s http://localhost:8080/api/v1/venus/temperatures | jq '.temperatures[0] // "None"'

# Totals système
curl -s http://localhost:8080/api/v1/system/totals | jq .

# BMS
curl -s http://localhost:8080/api/v1/bms/1/status | jq '.Soc, .Dc.Voltage, .Dc.Current'

# ET112
curl -s http://localhost:8080/api/v1/et112/7/status | jq '.power_w, .energy_export_wh'
```

#### 5C : Dashboard Web
```
http://192.168.1.141:8080/visualization
```

**Doit afficher:**
- ✅ BMS-360Ah (SOC, Voltage, Current)
- ✅ BMS-320Ah (SOC, Voltage, Current)
- ✅ Micro-onduleurs ET112 0x07 (Puissance, Énergie)
- ✅ PAC Chauffe-eau ET112 0x08 (Puissance)
- ✅ PAC Climatisation ET112 0x09 (Puissance)
- ✅ Température ext. (°C, Humidité)
- ⏳ MPPT (si activé sur NanoPi)
- ⏳ SmartShunt (si disponible)

---

## 🔍 DÉPANNAGE

### **Appareils affichent "En attente de données"**

#### Cause 1 : Topics MQTT non publiés
```bash
# Sur NanoPi
mosquitto_sub -h localhost -t "santuario/#" -v | head -20

# Doit afficher des messages de tous les topics
```

**Fix:** Vérifier que Node-RED publie ou que dbus-mqtt-venus envoie les données

#### Cause 2 : Config NanoPi incomplète
```bash
# Vérifier la config
cat /data/daly-bms/config.toml | grep -A 5 "^\[\["

# Doit contenir [[pvinverters]], [[heatpumps]], [[smartshunts]], [[mppts]]
```

**Fix:** Ajouter les sections manquantes et redémarrer

#### Cause 3 : MQTT subscriber ne reçoit pas
```bash
# Vérifier les logs Pi5
journalctl -u daly-bms | grep -i "venus\|mqtt\|topic"

# Vérifier la connexion au broker
curl telnet://192.168.1.120:1883  # Doit répondre
```

**Fix:** Vérifier que le broker est accessible et que les topics existent

---

## 📈 MONITORING EN PRODUCTION

### **Metrics à surveiller**

| Métrique | Seuil critique | Action |
|----------|---|---|
| BMS SOC | < 10% | Arrêt décharge |
| BMS Temperature | > 50°C | Réduction charge |
| Production | 0W (jour) | Vérifier panneau solaire |
| SmartShunt SOC | < 20% | Alerte notification |

### **Logs importants**

```bash
# MQTT errors
journalctl -u daly-bms | grep -i "mqtt.*error"

# Venus OS connectivity
journalctl -u daly-bms | grep -i "venus\|dbus"

# API errors
journalctl -u daly-bms | grep -i "api.*error"
```

### **Grafana Dashboard** (optionnel)
```
http://192.168.1.141:3001
- Admin / supersecretchangeit
- Import dashboard ID 12345 (Daly-BMS)
```

---

## 🔐 SÉCURITÉ

### **Authentification API** (en production)
```toml
# Config.toml
[api]
api_key = "$(openssl rand -hex 32)"  # Générer une clé
```

### **MQTT** (en production)
```toml
[mqtt]
username = "daly-bms"
password = "$(openssl rand -hex 16)"
```

### **Accès réseau**
```bash
# Restreindre à LAN uniquement
sudo ufw allow from 192.168.1.0/24 to any port 8080
```

---

## 📋 CHECKLIST DE VALIDATION

```
DÉPLOIEMENT
[ ] Git checkout branche claude/realtime-metrics-dashboard-lUKF3
[ ] make build-arm (succès)
[ ] systemctl stop/start daly-bms
[ ] Logs sans erreur

API
[ ] curl /api/v1/system/status → OK
[ ] curl /api/v1/venus/mppt → données ou []
[ ] curl /api/v1/venus/smartshunt → données ou null
[ ] curl /api/v1/venus/temperatures → données ou []
[ ] curl /api/v1/system/totals → production, consommation, SOC

DASHBOARD
[ ] Page http://192.168.1.141:8080/visualization charge
[ ] BMS-360Ah visible + données
[ ] BMS-320Ah visible + données
[ ] ET112 0x07 (Micro-onduleurs) visible
[ ] ET112 0x08 (Chauffe-eau) visible avec puissance
[ ] ET112 0x09 (Climatisation) visible avec puissance
[ ] Température visible (10.8°C exemple)
[ ] Edges animées (au moins batterie)
[ ] Données se mettent à jour (2s)

NanoPi
[ ] dbus-mqtt-venus tourne (svstat)
[ ] Topics MQTT publiés (mosquitto_sub)
[ ] D-Bus services actifs (dbus -y | grep victron)

OPTIONNEL
[ ] InfluxDB reçoit les données (check bucket)
[ ] Grafana dashboard affiche les historiques
[ ] Alertes configurées (seuils SOC, temp, etc.)
[ ] VRM Portal synchronisé
```

---

## 📞 SUPPORT

Si problème de données manquantes :

1. **Vérifier les logs**
   ```bash
   journalctl -u daly-bms -f  # Pi5
   tail -f /var/log/dbus-mqtt-venus/current  # NanoPi
   ```

2. **Tester la connectivité**
   ```bash
   curl http://localhost:8080/api/v1/system/status | jq .
   mosquitto_sub -h 192.168.1.120 -t "santuario/#" -C 1
   ```

3. **Redémarrer proprement**
   ```bash
   # Pi5
   sudo systemctl restart daly-bms
   
   # NanoPi
   svc -t /service/dbus-mqtt-venus
   ```

---

**Version:** 1.0.0  
**Dernière mise à jour:** 2026-04-05  
**Branche stable:** claude/realtime-metrics-dashboard-lUKF3
