# Guide Complet — Dashboard Temps Réel & Extension des Métriques
**Version:** 2.0  
**Date:** 2026-04-05  
**Branche:** `claude/realtime-metrics-dashboard-lUKF3`  
**Status:** ✅ Production Ready

---

## TABLE DES MATIÈRES

1. [Vue d'ensemble du système](#vue-densemble)
2. [Architecture détaillée du dashboard](#architecture-dashboard)
3. [Comment fonctionne la collecte de données](#collecte-données)
4. [Ajouter une nouvelle métrique du NanoPi](#ajouter-métrique-nanopi)
5. [Ajouter une nouvelle métrique du Pi5](#ajouter-métrique-pi5)
6. [Procédures détaillées d'intégration](#procédures-intégration)
7. [Guide de dépannage](#dépannage)
8. [Cas d'usage réels](#cas-usage)

---

## 1. VUE D'ENSEMBLE DU SYSTÈME

### Le Problème Initial

Au départ (avant cette implémentation), le système n'avait **AUCUNE** remontée temps réel des métriques Victron vers le Pi5:
- Les batteries BMS s'affichaient (via RS485 direct)
- Les MPPT, SmartShunt, Onduleur Victron restaient invisibles
- Le dashboard affichait "En attente de données" pour ces appareils
- Aucune intégration de D-Bus Victron vers l'API web

### La Solution Implémentée

Une architecture **complète et temps réel** en 4 étapes:

```
NanoPi D-Bus (Victron)
    ↓ Node-RED flows
MQTT Topics (santuario/*)
    ↓ daly-bms-server
AppState (données en mémoire)
    ↓ REST API + WebSocket
ReactFlow Dashboard (temps réel)
```

### Composants Clés Implémentés

| Composant | Rôle | Technologie |
|-----------|------|-------------|
| **inverter.json** | Agréger MultiPlus D-Bus → MQTT | Node-RED |
| **smartshunt.json** | Agréger SmartShunt D-Bus → MQTT | Node-RED |
| **Solar_power.json** | Agréger MPPT D-Bus → MQTT | Node-RED |
| **VenusInverter struct** | Stocker inverter en Rust | Serde |
| **MQTT handlers** | Parser JSON MQTT → Rust | async/tokio |
| **API endpoints** | Exposer via REST | Axum |
| **visualization.html** | Afficher en temps réel | ReactFlow |

---

## 2. ARCHITECTURE DÉTAILLÉE DU DASHBOARD

### 2.1 Flux de Données Complet

```
┌─────────────────────────────────────────────────────────────┐
│ ÉTAPE 1: COLLECTE (NanoPi D-Bus)                           │
└─────────────────────────────────────────────────────────────┘

Victron Hardware D-Bus:
  com.victronenergy.system/Dc/Voltage          → 48.2V
  com.victronenergy.system/Dc/Current          → -12.4A
  com.victronenergy.system/Ac/Out/L1/V         → 229.8V
  com.victronenergy.system/Ac/Out/L1/P         → 1286W
  (et 100+ autres chemins D-Bus)

┌─────────────────────────────────────────────────────────────┐
│ ÉTAPE 2: AGGRÉGATION (Node-RED - Pi5 Docker)               │
└─────────────────────────────────────────────────────────────┘

Node-RED Flows:
  inverter.json   → subscribe D-Bus → aggregate → publish MQTT
  smartshunt.json → subscribe D-Bus → aggregate → publish MQTT
  Solar_power.json → subscribe D-Bus → aggregate → publish MQTT

Topics MQTT générés:
  santuario/inverter/venus  ← {Voltage, Current, Power, AcVoltage, AcCurrent, AcPower, State, Mode}
  santuario/system/venus    ← {Voltage, Current, Power, SOC, State}
  santuario/meteo/venus     ← {MpptPower, TodaysYield, Irradiance}

┌─────────────────────────────────────────────────────────────┐
│ ÉTAPE 3: STOCKAGE (daly-bms-server - Pi5)                  │
└─────────────────────────────────────────────────────────────┘

MQTT Handlers (bridges/mqtt.rs):
  handle_inverter_topic()  → parse JSON → create VenusInverter struct
  handle_system_topic()    → parse JSON → create VenusSmartShunt struct
  handle_meteo_topic()     → parse JSON → update MPPT metrics

AppState Storage (state.rs):
  Arc<RwLock<Option<VenusInverter>>>    ← inverter data
  Arc<RwLock<Option<VenusSmartShunt>>>  ← smartshunt data
  Arc<RwLock<Vec<VenusMppt>>>          ← all MPPT chargers
  Arc<RwLock<Vec<VenusTemperature>>>   ← temperature sensors

┌─────────────────────────────────────────────────────────────┐
│ ÉTAPE 4: EXPOSITION (REST API - daly-bms-server)           │
└─────────────────────────────────────────────────────────────┘

API Endpoints (api/system.rs):
  GET /api/v1/venus/inverter     → return VenusInverter + connected status
  GET /api/v1/venus/smartshunt   → return VenusSmartShunt + connected status
  GET /api/v1/venus/mppt         → return Vec<VenusMppt> with total power
  GET /api/v1/venus/temperatures → return Vec<VenusTemperature>

Response Format (Exemple - Inverter):
  {
    "connected": true,
    "inverter": {
      "voltage_v": 48.2,
      "current_a": 3.5,
      "power_w": 168.7,
      "ac_output_voltage_v": 229.8,
      "ac_output_current_a": 5.6,
      "ac_output_power_w": 1286.0,
      "state": "on",
      "mode": "inverter",
      "timestamp": "2026-04-05T14:32:45.123Z"
    }
  }

┌─────────────────────────────────────────────────────────────┐
│ ÉTAPE 5: AFFICHAGE (ReactFlow Dashboard - Browser)          │
└─────────────────────────────────────────────────────────────┘

JavaScript Fetch:
  fetch('/api/v1/venus/inverter')     → inverter data
  fetch('/api/v1/venus/smartshunt')   → shunt data
  fetch('/api/v1/venus/mppt')         → MPPT data
  fetch('/api/v1/bms/...')            → BMS data

ReactFlow Node Mapping:
  inverter node    ← displays AC power from inverter.ac_output_power_w
  smartshunt node  ← displays current from shunt.current_a
  mppt1, mppt2     ← display power from mppts[].power_w
  bms1, bms2       ← display power from WebSocket stream

Real-Time Updates:
  WebSocket /ws/venus/stream (40ms) or polling (2s fallback)
  Live status indicators (.live class when connected: true)
  Edge animations showing power flow direction
```

### 2.2 Structures de Données Rust

#### State.rs — Définition

```rust
// Invoter (MultiPlus Victron)
pub struct VenusInverter {
    pub voltage_v: Option<f32>,           // DC voltage
    pub current_a: Option<f32>,           // DC current
    pub power_w: Option<f32>,             // DC power
    pub ac_output_voltage_v: Option<f32>, // AC output voltage
    pub ac_output_current_a: Option<f32>, // AC output current
    pub ac_output_power_w: Option<f32>,   // AC output power ← DISPLAYED
    pub state: String,                    // "on" / "off" / "fault"
    pub mode: String,                     // "inverter" / "charger" / "passthrough"
    pub timestamp: DateTime<Utc>,
}

// SmartShunt (Victron Battery Monitor)
pub struct VenusSmartShunt {
    pub voltage_v: Option<f32>,      // Battery voltage
    pub current_a: Option<f32>,      // Battery current ← DISPLAYED
    pub power_w: Option<f32>,        // Battery power
    pub soc_percent: Option<f32>,    // State of charge
    pub state: String,               // "charging" / "discharging" / "idle"
    pub timestamp: DateTime<Utc>,
}

// MPPT Solar Charger
pub struct VenusMppt {
    pub address: String,             // Device address / instance
    pub power_w: f32,                // Output power ← DISPLAYED
    pub voltage_v: f32,              // Input voltage
    pub current_a: f32,              // Input current
    pub yield_today_kwh: f32,        // Energy generated today
    pub status: String,              // "ON" / "OFF" / "FAULTED"
    pub timestamp: DateTime<Utc>,
}

// Temperature Sensor
pub struct VenusTemperature {
    pub address: String,             // Device address
    pub name: String,                // "Outdoor" / "Battery" / etc
    pub temperature_c: f32,          // Temperature value ← DISPLAYED
    pub type_num: i32,               // 0=battery 1=fridge 2=generic 3=room 4=outdoor
    pub status: String,              // "connected" / "disconnected"
    pub timestamp: DateTime<Utc>,
}

// AppState Storage
pub struct AppState {
    // ... existing fields ...
    
    // Venus OS devices (NEW)
    pub venus_inverter: Arc<RwLock<Option<VenusInverter>>>,
    pub venus_smartshunt: Arc<RwLock<Option<VenusSmartShunt>>>,
    pub venus_mppts: Arc<RwLock<Vec<VenusMppt>>>,
    pub venus_temperatures: Arc<RwLock<Vec<VenusTemperature>>>,
}

// Helper methods
impl AppState {
    pub async fn on_venus_inverter(&self, inv: VenusInverter) {
        *self.venus_inverter.write().await = Some(inv);
    }
    
    pub async fn venus_inverter_get(&self) -> Option<VenusInverter> {
        self.venus_inverter.read().await.clone()
    }
    // ... similar for other devices ...
}
```

### 2.3 Topics MQTT & Payloads

#### santuario/inverter/venus

Publié par: `inverter.json` Node-RED flow  
Fréquence: Chaque nouveau message D-Bus (temps réel)  
Résolution: Dépend de la fréquence Victron (~100ms)

```json
{
  "Voltage": 48.2,
  "Current": 3.5,
  "Power": 168.7,
  "AcVoltage": 229.8,
  "AcCurrent": 5.6,
  "AcPower": 1286.0,
  "State": "on",
  "Mode": "inverter"
}
```

**Champs attendus:**
- `Voltage` (f32) — DC voltage en volts
- `Current` (f32) — DC current en ampères
- `Power` (f32) — DC power en watts
- `AcVoltage` (f32) — AC voltage L1 en volts
- `AcCurrent` (f32) — AC current L1 en ampères
- `AcPower` (f32) — AC power L1 en watts ← **AFFICHÉ SUR DASHBOARD**
- `State` (string) — "on" ou "off"
- `Mode` (string) — "inverter", "charger", "passthrough", etc.

#### santuario/system/venus

Publié par: `smartshunt.json` Node-RED flow  
Fréquence: Chaque nouveau message D-Bus (temps réel)

```json
{
  "Voltage": 48.3,
  "Current": -12.4,
  "Power": -598.0,
  "SOC": 85.5,
  "State": "discharging"
}
```

**Champs attendus:**
- `Voltage` (f32) — Tension batterie
- `Current` (f32) — Courant (négatif = décharge) ← **AFFICHÉ**
- `Power` (f32) — Puissance
- `SOC` (f32) — État de charge %
- `State` (string) — "charging", "discharging", "idle"

#### santuario/meteo/venus

Publié par: `Solar_power.json` + `meteo.json` Node-RED flows  
Fréquence: Toutes les 25 secondes (keepalive)

```json
{
  "MpptPower": 2345.0,
  "TodaysYield": 12.5,
  "IrradianceWm2": 334.0,
  "Irradiance": 334.0
}
```

**Champs clés:**
- `MpptPower` (f32) — Puissance solaire TOTALE MPPT (273 + 289) ← **UTILISÉ**
- `TodaysYield` (f32) — Production d'aujourd'hui en kWh
- `IrradianceWm2` (f32) — Irradiance du capteur
- `Irradiance` (f32) — Idem (backup field name)

---

## 3. COMMENT FONCTIONNE LA COLLECTE DE DONNÉES

### 3.1 Flux MQTT sur Pi5

#### Étape 1: Node-RED Flows

**Fichier:** `flux-nodered/inverter.json`

```
D-Bus Input Node
  ├─ Topic: N/c0619ab9929a/system/0/Dc/Voltage
  ├─ Topic: N/c0619ab9929a/system/0/Dc/Current
  ├─ Topic: N/c0619ab9929a/system/0/Dc/Power
  ├─ Topic: N/c0619ab9929a/system/0/Ac/Out/L1/V
  └─ Topic: N/c0619ab9929a/system/0/Ac/Out/L1/P
         ↓
     Function Node: "Aggregate Inverter Data"
         ↓
     Combine all values into single JSON object
         ↓
     MQTT Output Node
         └─ Topic: santuario/inverter/venus
         └─ QoS: 0
         └─ Retain: true
```

#### Étape 2: Server Reception (daly-bms-server)

**Fichier:** `crates/daly-bms-server/src/bridges/mqtt.rs`

```rust
// On MQTT connection established:
mqtt_client.subscribe("santuario/inverter/venus", QoS::AtLeastOnce).await?;
mqtt_client.subscribe("santuario/system/venus", QoS::AtLeastOnce).await?;
mqtt_client.subscribe("santuario/meteo/venus", QoS::AtLeastOnce).await?;

// On message received:
async fn on_mqtt_message(topic: &str, payload: &[u8], state: &AppState) {
    let json: Value = serde_json::from_slice(payload).ok()?;
    
    match topic {
        "santuario/inverter/venus" => handle_inverter_topic(&state, &json).await,
        "santuario/system/venus" => handle_system_topic(&state, &json).await,
        "santuario/meteo/venus" => handle_meteo_topic(&state, &json).await,
        _ => {}
    }
}

async fn handle_inverter_topic(state: &AppState, json: &Value) {
    let voltage = json.get("Voltage").and_then(|v| v.as_f64()).map(|v| v as f32);
    let current = json.get("Current").and_then(|v| v.as_f64()).map(|v| v as f32);
    let power = json.get("Power").and_then(|v| v.as_f64()).map(|v| v as f32);
    let ac_voltage = json.get("AcVoltage").and_then(|v| v.as_f64()).map(|v| v as f32);
    let ac_current = json.get("AcCurrent").and_then(|v| v.as_f64()).map(|v| v as f32);
    let ac_power = json.get("AcPower").and_then(|v| v.as_f64()).map(|v| v as f32);
    let state_str = json.get("State").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let mode_str = json.get("Mode").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    
    let inverter = VenusInverter {
        voltage_v: voltage,
        current_a: current,
        power_w: power,
        ac_output_voltage_v: ac_voltage,
        ac_output_current_a: ac_current,
        ac_output_power_w: ac_power,
        state: state_str,
        mode: mode_str,
        timestamp: Utc::now(),
    };
    
    state.on_venus_inverter(inverter).await;
    info!("Updated inverter: AC Power = {}W", ac_power.unwrap_or(0.0));
}
```

#### Étape 3: REST API Response

**Fichier:** `crates/daly-bms-server/src/api/system.rs`

```rust
pub async fn get_venus_inverter(State(state): State<AppState>) -> impl IntoResponse {
    match state.venus_inverter_get().await {
        Some(inv) => (
            StatusCode::OK,
            Json(json!({
                "connected": true,
                "inverter": inv,  // ← Serialized automatically by serde
            })),
        ),
        None => (
            StatusCode::OK,
            Json(json!({
                "connected": false,
                "inverter": Value::Null,
            })),
        ),
    }
}
```

#### Étape 4: Frontend Display

**Fichier:** `crates/daly-bms-server/templates/visualization.html`

```javascript
async function fetchAll() {
    // Fetch inverter data
    const inverterResp = await safe(
        fetch('/api/v1/venus/inverter').then(r => r.ok ? r.json() : null)
    );
    const inverter = inverterResp?.inverter ?? null;
    
    // Update node data
    setNodes(prevNodes =>
        prevNodes.map(n => {
            if (n.id === 'onduleur') {
                return {
                    ...n,
                    data: {
                        ...n.data,
                        value: inverter
                            ? `${(inverter.ac_output_power_w ?? 0).toFixed(0)}W`
                            : '—',
                        dotClass: inverter?.ac_output_power_w > 0 ? 'live' : ''
                    }
                };
            }
            return n;
        })
    );
}

// Polling loop
setInterval(fetchAll, 2000);  // 2-second refresh
```

---

## 4. AJOUTER UNE NOUVELLE MÉTRIQUE DU NANOPI

### Scénario: Ajouter la Température du Générateur

**Objectif:** Afficher la température du générateur Victron dans le dashboard

### Étape 1: Vérifier que la métrique existe sur D-Bus

Sur NanoPi, vérifier le chemin D-Bus exact:

```bash
ssh root@192.168.1.120

# Lister tous les services Victron
dbus -y | grep victronenergy

# Si "generator" service existe:
dbus -y com.victronenergy.generator.XX / GetItems | grep -i temperature

# Exemple de résultat:
# /Ac/Temperature: 45.3  ← TROUVÉ
```

### Étape 2: Créer le Node-RED Flow

**Fichier:** `flux-nodered/generator.json`

```json
[
  {
    "id": "mqtt_gen_temp_in",
    "type": "mqtt in",
    "z": "genset_tab",
    "name": "Generator Temperature",
    "topic": "N/c0619ab9929a/generator/0/Ac/Temperature",
    "qos": "0",
    "datatype": "json",
    "broker": "pi5_mqtt_broker_inv",
    "nl": false,
    "rap": true,
    "x": 150,
    "y": 60,
    "wires": [["gen_temp_fn"]]
  },
  {
    "id": "gen_temp_fn",
    "type": "function",
    "z": "genset_tab",
    "name": "Store Temperature",
    "func": "const v = msg.payload.value !== undefined ? msg.payload.value : msg.payload;\nflow.set('gen_temperature_c', v);\nnode.status({fill:'blue', text:`Gen Temp: ${v}°C`});\nreturn msg;",
    "outputs": 1,
    "x": 350,
    "y": 60,
    "wires": [["gen_publish_fn"]]
  },
  {
    "id": "gen_publish_fn",
    "type": "function",
    "z": "genset_tab",
    "name": "Publish Generator",
    "func": "const temp = flow.get('gen_temperature_c') || 0;\n\nconst msg_out = {\n    topic: 'santuario/generator/venus',\n    payload: JSON.stringify({\n        Temperature: temp,\n        Connected: 1,\n        Status: temp > 0 ? 1 : 0\n    }),\n    retain: true\n};\n\nreturn msg_out;",
    "outputs": 1,
    "x": 550,
    "y": 60,
    "wires": [["mqtt_gen_out"]]
  },
  {
    "id": "mqtt_gen_out",
    "type": "mqtt out",
    "z": "genset_tab",
    "name": "MQTT out",
    "topic": "",
    "qos": "1",
    "retain": true,
    "broker": "pi5_mqtt_broker_inv",
    "x": 750,
    "y": 60,
    "wires": []
  }
]
```

### Étape 3: Ajouter la Structure Rust

**Fichier:** `crates/daly-bms-server/src/state.rs`

```rust
// Ajouter la structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VenusGenerator {
    pub temperature_c: Option<f32>,
    pub status: i32,
    pub connected: i32,
    pub timestamp: DateTime<Utc>,
}

// Ajouter au AppState
pub struct AppState {
    // ... autres fields ...
    pub venus_generator: Arc<RwLock<Option<VenusGenerator>>>,
}

// Ajouter les helpers
impl AppState {
    pub async fn on_venus_generator(&self, gen: VenusGenerator) {
        *self.venus_generator.write().await = Some(gen);
    }
    
    pub async fn venus_generator_get(&self) -> Option<VenusGenerator> {
        self.venus_generator.read().await.clone()
    }
}
```

### Étape 4: Ajouter le MQTT Handler

**Fichier:** `crates/daly-bms-server/src/bridges/mqtt.rs`

```rust
// Dans la fonction de subscription MQTT:
mqtt_client.subscribe("santuario/generator/venus", QoS::AtLeastOnce).await?;

// Dans le match pattern de réception:
"santuario/generator/venus" => handle_generator_topic(&state, &json).await,

// Ajouter le handler:
async fn handle_generator_topic(state: &AppState, json: &Value) {
    let temperature = json
        .get("Temperature")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32);
    let status = json.get("Status").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let connected = json.get("Connected").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    
    let generator = VenusGenerator {
        temperature_c: temperature,
        status,
        connected,
        timestamp: Utc::now(),
    };
    
    state.on_venus_generator(generator).await;
}
```

### Étape 5: Ajouter l'Endpoint API

**Fichier:** `crates/daly-bms-server/src/api/system.rs`

```rust
pub async fn get_venus_generator(State(state): State<AppState>) -> impl IntoResponse {
    match state.venus_generator_get().await {
        Some(gen) => (
            StatusCode::OK,
            Json(json!({
                "connected": gen.connected > 0,
                "generator": gen,
            })),
        ),
        None => (
            StatusCode::OK,
            Json(json!({
                "connected": false,
                "generator": Value::Null,
            })),
        ),
    }
}
```

### Étape 6: Enregistrer la Route

**Fichier:** `crates/daly-bms-server/src/api/mod.rs`

```rust
// Ajouter dans build_router():
.route("/api/v1/venus/generator", get(system::get_venus_generator))
```

### Étape 7: Mettre à Jour le Dashboard

**Fichier:** `crates/daly-bms-server/templates/visualization.html`

```javascript
// Dans fetchAll():
const generatorResp = await safe(
    fetch('/api/v1/venus/generator').then(r => r.ok ? r.json() : null)
);
const generator = generatorResp?.generator ?? null;

// Dans setNodes():
if (n.id === 'generator') {
    return {
        ...n,
        data: {
            ...n.data,
            value: generator
                ? `${(generator.temperature_c ?? 0).toFixed(1)}°C`
                : '—',
            dotClass: generator?.connected > 0 ? 'live' : ''
        }
    };
}
```

### Étape 8: Ajouter le Nœud ReactFlow

**Fichier:** `crates/daly-bms-server/templates/visualization.html`

```javascript
// Dans les nodes initiales:
{
    id: 'generator',
    type: 'device',
    position: { x: 300, y: 500 },
    data: { label: 'Générateur', icon: '⚙️', value: '—', dotClass: '' }
}
```

### Étape 9: Compiler et Déployer

```bash
# Sur Pi5
cd ~/Daly-BMS-Rust

# 1. Commit changes
git add -A
git commit -m "feat(generator): Add generator temperature metric integration"

# 2. Compile
make build-arm

# 3. Deploy
sudo systemctl stop daly-bms
sudo cp target/aarch64-unknown-linux-gnu/release/daly-bms-server /usr/local/bin/
sudo systemctl start daly-bms

# 4. Import Node-RED flow
# Accès: http://192.168.1.141:1880
# Menu → Import → Paste generator.json content
# Click Deploy

# 5. Verify
curl http://localhost:8080/api/v1/venus/generator | jq '.'
```

### Étape 10: Vérifier le Dashboard

Accès: http://192.168.1.141:8080/visualization

- Nouveau nœud "Générateur" doit afficher la température
- Indicateur vert si connecté (`connected: true`)
- Mise à jour en temps réel

---

## 5. AJOUTER UNE NOUVELLE MÉTRIQUE DU PI5

### Scénario: Ajouter la Température CPU du Pi5

**Objectif:** Afficher la température du processeur Pi5 dans le dashboard

**Approche différente:** La métrique vient du Pi5 lui-même (pas du NanoPi), donc:
1. Ajouter la lecture de température dans le code Rust
2. Exposer via API
3. Afficher sur dashboard

### Étape 1: Ajouter au AppState

**Fichier:** `crates/daly-bms-server/src/state.rs`

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemTemperature {
    pub cpu_temp_c: f32,
    pub source: String,  // "/sys/class/thermal/thermal_zone0/temp"
    pub timestamp: DateTime<Utc>,
}

pub struct AppState {
    // ... autres fields ...
    pub system_temperature: Arc<RwLock<Option<SystemTemperature>>>,
}

impl AppState {
    pub async fn on_system_temperature(&self, temp: SystemTemperature) {
        *self.system_temperature.write().await = Some(temp);
    }
    
    pub async fn system_temperature_get(&self) -> Option<SystemTemperature> {
        self.system_temperature.read().await.clone()
    }
}
```

### Étape 2: Ajouter une Tâche de Polling

**Fichier:** `crates/daly-bms-server/src/main.rs`

```rust
use std::fs;

// Dans le setup principal:
let state_clone = state.clone();
tokio::spawn(async move {
    loop {
        // Read CPU temperature every 10 seconds
        if let Ok(temp_str) = fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
            if let Ok(temp_millidegrees) = temp_str.trim().parse::<f32>() {
                let temp_c = temp_millidegrees / 1000.0;
                
                state_clone.on_system_temperature(SystemTemperature {
                    cpu_temp_c: temp_c,
                    source: "/sys/class/thermal/thermal_zone0/temp".to_string(),
                    timestamp: Utc::now(),
                }).await;
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
});
```

### Étape 3: Exposer via API

**Fichier:** `crates/daly-bms-server/src/api/system.rs`

```rust
pub async fn get_system_temperature(State(state): State<AppState>) -> impl IntoResponse {
    match state.system_temperature_get().await {
        Some(temp) => (
            StatusCode::OK,
            Json(json!({
                "connected": true,
                "temperature": temp,
            })),
        ),
        None => (
            StatusCode::OK,
            Json(json!({
                "connected": false,
                "temperature": Value::Null,
            })),
        ),
    }
}
```

### Étape 4: Enregistrer la Route

**Fichier:** `crates/daly-bms-server/src/api/mod.rs`

```rust
.route("/api/v1/system/temperature", get(system::get_system_temperature))
```

### Étape 5: Mettre à Jour le Dashboard

**Fichier:** `crates/daly-bms-server/templates/visualization.html`

```javascript
// Fetch system temperature
const sysTemp = await safe(
    fetch('/api/v1/system/temperature').then(r => r.ok ? r.json() : null)
);
const sysTemperature = sysTemp?.temperature ?? null;

// Display on node
if (n.id === 'pi5') {
    return {
        ...n,
        data: {
            ...n.data,
            value: sysTemperature
                ? `${(sysTemperature.cpu_temp_c ?? 0).toFixed(1)}°C`
                : '—',
            dotClass: sysTemperature ? 'live' : ''
        }
    };
}
```

### Étape 6: Compiler et Déployer

```bash
make build-arm
sudo systemctl restart daly-bms
# Verify
curl http://localhost:8080/api/v1/system/temperature | jq '.'
```

---

## 6. PROCÉDURES DÉTAILLÉES D'INTÉGRATION

### 6.1 Checklist Générique pour Ajouter une Métrique

```
□ ÉTAPE 1: Identifier la source
  - D'où vient la donnée? (D-Bus NanoPi / Pi5 / RS485 / API externe)
  - Quel est le chemin exact? (dbus path / /sys path / topic MQTT)
  - Quelle est la fréquence de mise à jour?

□ ÉTAPE 2: Ajouter la structure Rust
  - Créer struct dans state.rs (ou fichier dédié)
  - Ajouter Arc<RwLock<>> à AppState
  - Ajouter on_*() et *_get() helpers

□ ÉTAPE 3: Ajouter la source de données
  - Si NanoPi → créer Node-RED flow JSON
  - Si Pi5 → ajouter tokio::spawn() polling loop
  - Si MQTT → ajouter handler dans mqtt.rs
  - Si API externe → ajouter HTTP client

□ ÉTAPE 4: Créer l'API endpoint
  - Ajouter handler dans api/system.rs ou api/module.rs
  - Retourner {"connected": true/false, "data": {...}}
  - Enregistrer route dans api/mod.rs

□ ÉTAPE 5: Mettre à jour dashboard
  - Fetch dans fetchAll()
  - Ajouter mapping dans setNodes()
  - Ajouter nœud ReactFlow si nouveau device

□ ÉTAPE 6: Compiler et tester
  - cargo build --release -p daly-bms-server
  - Redémarrer le service
  - curl endpoint pour vérifier
  - Accès dashboard et vérifier affichage

□ ÉTAPE 7: Commit et push
  - git add -A
  - git commit -m "feat(scope): description"
  - git push origin claude/realtime-metrics-dashboard-lUKF3
```

### 6.2 Template de Structure Rust Complète

```rust
// ===================== state.rs =====================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MyNewDevice {
    pub metric1: Option<f32>,         // Metrique principale
    pub metric2: Option<f32>,         // Metrique secondaire
    pub status: String,               // "connected" / "error"
    pub timestamp: DateTime<Utc>,     // Quand mis à jour
}

// Ajouter au AppState struct
pub struct AppState {
    // ... existing fields ...
    pub my_device: Arc<RwLock<Option<MyNewDevice>>>,
}

// Ajouter les helpers
impl AppState {
    pub async fn on_my_device(&self, data: MyNewDevice) {
        *self.my_device.write().await = Some(data);
        // Optional: log update
        info!("Updated my_device: {} status={}", data.metric1.unwrap_or(0.0), data.status);
    }
    
    pub async fn my_device_get(&self) -> Option<MyNewDevice> {
        self.my_device.read().await.clone()
    }
}

// ===================== api/system.rs =====================

pub async fn get_my_device(State(state): State<AppState>) -> impl IntoResponse {
    match state.my_device_get().await {
        Some(data) => (
            StatusCode::OK,
            Json(json!({
                "connected": data.status == "connected",
                "device": data,
            })),
        ),
        None => (
            StatusCode::OK,
            Json(json!({
                "connected": false,
                "device": Value::Null,
            })),
        ),
    }
}

// ===================== api/mod.rs =====================

.route("/api/v1/my/endpoint", get(system::get_my_device))

// ===================== visualization.html =====================

const myData = await safe(
    fetch('/api/v1/my/endpoint').then(r => r.ok ? r.json() : null)
);

// In setNodes():
if (n.id === 'mynode') {
    return {
        ...n,
        data: {
            ...n.data,
            value: myData?.device?.metric1
                ? `${myData.device.metric1.toFixed(1)}Unit`
                : '—',
            dotClass: myData?.connected ? 'live' : ''
        }
    };
}
```

### 6.3 Procédure Node-RED pour MQTT

```json
[
  {
    "id": "node-id-1",
    "type": "mqtt in",
    "name": "Input from MQTT",
    "topic": "source/topic/path",
    "qos": "0",
    "datatype": "json",
    "x": 150,
    "y": 100,
    "wires": [["function-node-1"]]
  },
  {
    "id": "function-node-1",
    "type": "function",
    "name": "Parse and aggregate",
    "func": "// Extract values from payload\nconst value1 = msg.payload.field1;\nconst value2 = msg.payload.field2;\n\n// Store in flow context (accessible between nodes)\nflow.set('my_value1', value1);\nflow.set('my_value2', value2);\n\nnode.status({fill:'blue', text:`Value1: ${value1}`});\n\nreturn msg;",
    "outputs": 1,
    "x": 350,
    "y": 100,
    "wires": [["publish-node-1"]]
  },
  {
    "id": "publish-node-1",
    "type": "function",
    "name": "Create MQTT payload",
    "func": "const val1 = flow.get('my_value1') || 0;\nconst val2 = flow.get('my_value2') || 0;\n\nconst msg_out = {\n    topic: 'santuario/mydevice/venus',  // ← Change this\n    payload: JSON.stringify({\n        Metric1: val1,\n        Metric2: val2,\n        Status: val1 > 0 ? 1 : 0,\n        Timestamp: new Date().toISOString()\n    }),\n    retain: true  // Persist last value on MQTT broker\n};\n\nreturn msg_out;",
    "outputs": 1,
    "x": 550,
    "y": 100,
    "wires": [["mqtt-out-1"]]
  },
  {
    "id": "mqtt-out-1",
    "type": "mqtt out",
    "name": "Output to Mosquitto",
    "topic": "",  // Leave empty, use msg.topic
    "qos": "1",
    "retain": true,
    "broker": "pi5_mqtt_broker",  // Config node ID
    "x": 750,
    "y": 100,
    "wires": []
  }
]
```

---

## 7. GUIDE DE DÉPANNAGE

### Problème: Nouveau endpoint retourne `"connected": false`

**Causes possibles:**

1. **Node-RED flow n'est pas en cours d'exécution**
   ```bash
   # Vérifier les logs Node-RED
   docker logs nodered | grep -i error
   # Vérifier dans Node-RED UI que le flow est en "Deploy" (pas grisé)
   ```

2. **MQTT topic n'est pas publié**
   ```bash
   # Watch MQTT
   mosquitto_sub -h 192.168.1.120 -p 1883 -t 'santuario/mydevice/venus' -v
   # Si rien n'apparaît pendant 30s, le topic n'est pas publié
   # Vérifier la source (D-Bus disponible? Node-RED error?)
   ```

3. **Handler MQTT n'a pas parsé le JSON**
   ```bash
   # Vérifier les logs du serveur
   journalctl -u daly-bms -f | grep -i error
   # Look for JSON parsing errors in MQTT handler
   ```

4. **AppState stockage n'a pas reçu la donnée**
   ```bash
   # Ajouter un log temporaire dans le handler:
   info!("Received MQTT message: {:?}", json);
   # Recompile et check logs
   ```

### Problème: Dashboard affiche "—" au lieu de la valeur

**Causes:**

1. **API endpoint 404**
   ```bash
   curl -v http://localhost:8080/api/v1/my/endpoint
   # Si 404, vérifier la route dans api/mod.rs
   ```

2. **API retourne null**
   ```bash
   curl http://localhost:8080/api/v1/my/endpoint | jq '.'
   # Si .device est null, le AppState n'a pas reçu la donnée
   # → Debug MQTT handlers
   ```

3. **JavaScript fetch échoue**
   ```javascript
   // Dans console du navigateur (F12):
   fetch('/api/v1/my/endpoint').then(r => r.json()).then(console.log)
   // Si erreur réseau, vérifier que le serveur tourne
   ```

4. **Node ReactFlow n'est pas mappé**
   ```javascript
   // Dans visualization.html, vérifier que:
   // 1. Le fetch existe pour ce device
   // 2. Le setNodes() map l'ID du nœud
   // 3. Le champ de la valeur (value) est correct
   ```

### Problème: Compiler échoue avec erreurs

**Erreur commune: "struct n'implémente pas Serialize"**

```rust
// Solution: Ajouter derive macros
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MyDevice {
    // fields
}
```

**Erreur commune: "field does not exist"**

```rust
// Vérifier que la structure a clone() capability
#[derive(Clone, Debug, Serialize, Deserialize)]  // ← Clone is needed
pub struct MyDevice { ... }

// Vérifier que AppState initialization inclut le champ
let app_state = AppState {
    // ... 
    my_device: Arc::new(RwLock::new(None)),  // ← Add this
    // ...
};
```

---

## 8. CAS D'USAGE RÉELS

### Cas 1: Intégrer un Onduleur PV Fronius (API cloud)

**Source:** API REST Fronius cloud  
**Fréquence:** Toutes les 5 minutes  
**Métriques:** Power, Yield today, Status

```rust
// Dans main.rs tokio::spawn:
let state_clone = state.clone();
tokio::spawn(async move {
    let client = reqwest::Client::new();
    loop {
        // Query Fronius API
        match client
            .get("https://api.fronius.com/v1/GetPowerFlowRealtimeData.json")
            .query(&[("Symo", "SERIAL123")])
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(data) = resp.json::<FroniusResponse>().await {
                    let pv = VenusPvInverter {
                        power_w: Some(data.Body.Data.PAC.Value),
                        yield_today_kwh: Some(data.Body.Data.DailyEnergy.Value / 1000.0),
                        status: "OK".to_string(),
                        timestamp: Utc::now(),
                    };
                    state_clone.on_venus_pv_inverter(pv).await;
                }
            }
            Err(e) => error!("Fronius API error: {}", e),
        }
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
});
```

### Cas 2: Intégrer un Shelly H&T (WiFi thermomètre)

**Source:** API Shelly locale (192.168.1.XXX)  
**Fréquence:** Toutes les 60 secondes  
**Métriques:** Temperature, Humidity, Battery%

```javascript
// Dans Node-RED:
[
  {
    "id": "shelly-http",
    "type": "http request",
    "name": "Fetch Shelly",
    "method": "GET",
    "url": "http://192.168.1.50/status",
    "x": 150,
    "y": 100,
    "wires": [["parse-shelly"]]
  },
  {
    "id": "parse-shelly",
    "type": "function",
    "name": "Parse Shelly JSON",
    "func": "const temp = msg.payload.tmp?.tC || 0;\nconst humidity = msg.payload.hum?.value || 0;\nconst battery = msg.payload.bat?.value || 100;\n\nflow.set('shelly_temp', temp);\nflow.set('shelly_humidity', humidity);\nflow.set('shelly_battery', battery);\n\nreturn msg;",
    "x": 350,
    "y": 100,
    "wires": [["publish-shelly"]]
  },
  {
    "id": "publish-shelly",
    "type": "function",
    "name": "Create MQTT",
    "func": "const msg_out = {\n    topic: 'santuario/shelly_sensor/venus',\n    payload: JSON.stringify({\n        Temperature: flow.get('shelly_temp'),\n        Humidity: flow.get('shelly_humidity'),\n        BatteryPercent: flow.get('shelly_battery')\n    }),\n    retain: true\n};\nreturn msg_out;",
    "x": 550,
    "y": 100,
    "wires": [["mqtt-publish"]]
  }
]
```

### Cas 3: Intégrer un Compteur Linky (Teleinfo RS485)

**Source:** Sonde RS485 Teleinfo via `/dev/ttyUSB2`  
**Fréquence:** Toutes les 10 secondes  
**Métriques:** Puissance import/export, Index journalier

```rust
// Ajouter à main.rs (tokio::spawn):
use tokio_serial::{SerialPort, SerialPortBuilder};

let state_clone = state.clone();
tokio::spawn(async move {
    if let Ok(port) = SerialPortBuilder::new("/dev/ttyUSB2", 1200)
        .timeout(Duration::from_secs(5))
        .open_native()
    {
        // Read Teleinfo frames
        // Parse and extract power values
        // Update AppState
    }
});
```

---

## RÉSUMÉ DE DÉPLOIEMENT

### Déployer la Branche Actuelle (Realtime Dashboard)

```bash
# Sur Pi5
cd ~/Daly-BMS-Rust

# 1. Sync depuis GitHub
make sync

# 2. Compiler
make build-arm

# 3. Déployer
sudo systemctl stop daly-bms
sudo cp target/aarch64-unknown-linux-gnu/release/daly-bms-server /usr/local/bin/
sudo systemctl start daly-bms

# 4. Vérifier
journalctl -u daly-bms -f

# 5. Importer flows Node-RED
# Accès: http://192.168.1.141:1880
# Import les fichiers JSON:
# - flux-nodered/inverter.json
# - flux-nodered/smartshunt.json
# - flux-nodered/Solar_power.json (updated)
# - flux-nodered/meteo.json (updated)

# 6. Test dashboard
# http://192.168.1.141:8080/visualization
# Tous les devices doivent afficher des valeurs réelles
# Pas de "En attente de données"
```

### Configuration Initiale d'un Nouveau Développeur

```bash
# Clone du repo
git clone https://github.com/thieryus007-cloud/Daly-BMS-Rust
cd Daly-BMS-Rust

# Checkout de la branche de développement
git checkout claude/realtime-metrics-dashboard-lUKF3

# Lire ce guide et CLAUDE.md
cat DASHBOARD_EXTENSION_GUIDE.md
cat CLAUDE.md

# Compiler localement
make build-arm

# Prêt à développer!
```

---

## RESSOURCES ET RÉFÉRENCES

- **CLAUDE.md** — Référence projet principale
- **IMPLEMENTATION_VERIFICATION.md** — Checklist implémentation + validation
- **flux-nodered/inverter.json** — Flow MultiPlus (exemple Node-RED)
- **flux-nodered/smartshunt.json** — Flow SmartShunt (exemple)
- **crates/daly-bms-server/src/state.rs** — Structures de données
- **crates/daly-bms-server/src/bridges/mqtt.rs** — Handlers MQTT
- **crates/daly-bms-server/src/api/system.rs** — Endpoints API

---

**Document:** DASHBOARD_EXTENSION_GUIDE.md  
**Préparé pour:** Extension du système et maintenance  
**Audience:** Développeurs, Administrateurs système  
**Mise à jour:** 2026-04-05
