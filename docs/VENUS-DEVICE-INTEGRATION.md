# Guide d'intégration d'un device Venus OS via MQTT → D-Bus (Rust)

Ce document décrit exactement ce qui a été mis en place pour intégrer un nouveau type
de device sur le bus D-Bus de Venus OS, en utilisant le bridge MQTT → Rust → D-Bus.
Il sert de référence pour toute future intégration.

---

## Architecture générale

```
[Source de données]
        │
        │ (HTTP, RS485, Shelly, LG ThinQ API...)
        ▼
[Node-RED sur Pi5]
        │
        │ MQTT publish  topic: santuario/{type}/{index}/venus
        │               payload: JSON {"Champ": valeur, ...}
        ▼
[Mosquitto Pi5 - dalybms-mosquitto:1883]
        │
        │ Bridge MQTT  direction: out  (Pi5 → NanoPi)
        ▼
[Mosquitto NanoPi - 192.168.1.120:1883]
        │
        │ MQTT subscribe
        ▼
[daly-bms-venus (Rust) sur NanoPi]
        │
        │ zbus / D-Bus system bus
        ▼
[Venus OS D-Bus]
        │
        ├─ com.victronenergy.{type}.{prefix}_{index}
        │     /Connected, /ProductName, /DeviceInstance
        │     /Mgmt/ProcessName, /Mgmt/ProcessVersion, /Mgmt/Connection
        │     + chemins spécifiques au type de device
        ▼
[VRM Portal / GX Device local UI]
```

---

## Infrastructure réseau

| Machine | IP | Rôle |
|---|---|---|
| Pi5 (Raspberry Pi 5) | 192.168.1.141 | Docker : Mosquitto, Node-RED, InfluxDB, Grafana |
| NanoPi Neo3 | 192.168.1.120 | Venus OS, service Rust daly-bms-venus, D-Bus |

---

## Exemple complet : Capteur de température extérieure

### 1. Type D-Bus Victron utilisé

`com.victronenergy.temperature` — wiki Victron :
<https://github.com/victronenergy/venus/wiki/dbus#temperatures>

Chemins exposés :
- `/Temperature` — °C (float)
- `/TemperatureType` — 0=battery 1=fridge 2=generic 3=Room 4=Outdoor 5=WaterHeater 6=Freezer
- `/CustomName` — chaîne libre
- `/Humidity` — % humidité (float, 0.0 si absent)
- `/Pressure` — kPa (float, 0.0 si absent)
- `/Status` — 0=OK, 1=Disconnected
- `/Connected` — 0 ou 1
- `/ProductName`, `/ProductId`, `/DeviceInstance`
- `/Mgmt/ProcessName`, `/Mgmt/ProcessVersion`, `/Mgmt/Connection`

### 2. Configuration Config.toml (NanoPi)

```toml
[heat]
topic_prefix = "santuario/heat"

[[sensors]]
mqtt_index       = 1
name             = "Temperature Exterieure"
temperature_type = 4        # 4 = Outdoor
device_instance  = 20       # doit être unique sur le bus D-Bus
```

### 3. Topic MQTT

```
santuario/heat/1/venus
```

Payload JSON (publié par Node-RED) :
```json
{"Temperature": 11.5, "Humidity": 42.0}
```

### 4. Nom du service D-Bus résultant

```
com.victronenergy.temperature.mqtt_1
```

### 5. Flux Node-RED (meteo.json)

**Inject → HTTP Open-Meteo → Extraire température → mqtt out**

Fréquence de fetch : toutes les 15 minutes (900s)
Keepalive MQTT : toutes les **25 secondes** (< watchdog Rust de 30s)

Fonction "Extraire température" :
```javascript
const temp     = msg.payload.current.temperature_2m;
const humidity = msg.payload.current.relative_humidity_2m;

global.set('outdoor_temp', temp);
global.set('outdoor_humidity', humidity);

node.status({fill: 'green', shape: 'dot', text: `${temp}°C — ${humidity}%`});

return {
    topic:   'santuario/heat/1/venus',
    payload: JSON.stringify({ Temperature: temp, Humidity: humidity })
};
```

Fonction "Republier depuis contexte" (keepalive 25s) :
```javascript
const temp     = global.get('outdoor_temp');
const humidity = global.get('outdoor_humidity');

if (temp === undefined || temp === null) { return null; }

return {
    topic:   'santuario/heat/1/venus',
    payload: JSON.stringify({ Temperature: temp, Humidity: humidity })
};
```

**Point critique :** le keepalive doit être < `watchdog_sec` (30s par défaut).
Si le keepalive est trop long (ex: 60s), le service Rust met `/Connected = 0`
entre les publications et le device disparaît du VRM.

---

## Configuration Mosquitto bridge (Pi5)

Fichier : `docker/mosquitto/config/mosquitto.conf`

### Direction NanoPi → Pi5 (données publiées par le Rust)
```
topic santuario/# in 0
```
Sert à InfluxDB/Grafana pour lire les données BMS.

### Direction Pi5 → NanoPi (commandes Node-RED → service Rust)
```
topic santuario/heat/#     out 0
topic santuario/heatpump/# out 0
topic santuario/meteo/#    out 0
```

**Règle :** chaque nouveau type de device nécessite une règle `out` spécifique.
Ne pas utiliser `santuario/# both` pour éviter les boucles de messages.

---

## Watchdog et keepalive

Le service Rust gère deux intervalles (configurables dans Config.toml section `[venus]`) :

| Paramètre | Défaut | Rôle |
|---|---|---|
| `republish_sec` | 25s | Réémet `ItemsChanged` vers D-Bus même sans nouveau MQTT |
| `watchdog_sec` | 30s | Après ce délai sans MQTT, met `/Connected = 0` |

Node-RED doit publier le topic au moins une fois par `watchdog_sec`.
Pour les sources lentes (Open-Meteo = 15 min), un nœud keepalive est obligatoire.

---

## Fichiers Rust impactés pour un nouveau device

| Fichier | Rôle |
|---|---|
| `crates/daly-bms-venus/src/types.rs` | Struct payload MQTT (serde Deserialize) |
| `crates/daly-bms-venus/src/config.rs` | Config TOML : `[heat]`, `[[sensors]]`, etc. |
| `crates/daly-bms-venus/src/{type}_service.rs` | Enregistrement D-Bus zbus |
| `crates/daly-bms-venus/src/{type}_manager.rs` | Boucle MQTT → D-Bus, watchdog |
| `crates/daly-bms-venus/src/mqtt_source.rs` | Abonnement MQTT, événements |
| `crates/daly-bms-venus/src/main.rs` | Lancement du manager en tâche Tokio |

### Point important sur l'enregistrement des chemins D-Bus

Les objets feuilles D-Bus sont enregistrés **une seule fois** à la création du service,
depuis l'état initial `disconnected()`. Il faut donc que **tous les chemins** soient
présents dans `to_items()` même à l'état déconnecté, avec une valeur par défaut.

```rust
// CORRECT : toujours inclus, 0.0 si absent
m.insert("/Humidity".into(), DbusItem::f64(self.humidity.unwrap_or(0.0), "%"));

// INCORRECT : chemin non enregistré si None au démarrage
if let Some(h) = self.humidity {
    m.insert("/Humidity".into(), DbusItem::f64(h, "%"));
}
```

La méthode `GetItems()` sur la racine `/` (utilisée par VRM) fonctionne dans les deux
cas car elle appelle `to_items()` au moment de la requête. Mais `GetValue()` sur un
chemin individuel échoue avec "Unknown object" si l'objet feuille n'est pas enregistré.

---

## Procédure de déploiement (compilation ARMv7 → NanoPi)

Le NanoPi est en architecture **ARMv7 32-bit** (`armv7-unknown-linux-gnueabihf`).
La compilation cross-platform se fait sur la machine de développement.

### Prérequis (une seule fois)

```bash
# Installer le cross-compilateur ARM
apt-get install -y gcc-arm-linux-gnueabihf

# Ajouter la target Rust
rustup target add armv7-unknown-linux-gnueabihf
```

### Compilation

```bash
CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc \
  cargo build --release \
  --target armv7-unknown-linux-gnueabihf \
  -p daly-bms-venus
```

Binaire produit : `target/armv7-unknown-linux-gnueabihf/release/daly-bms-venus`

### Déploiement sur NanoPi

**Ordre obligatoire : arrêter avant de copier.**

```bash
# 1. Sur NanoPi — arrêter le service
svc -d /data/etc/sv/daly-bms-venus

# 2. Sur Pi5 — copier le binaire (depuis ~/Daly-BMS-Rust)
scp target/armv7-unknown-linux-gnueabihf/release/daly-bms-venus \
    root@192.168.1.120:/data/daly-bms/daly-bms-venus

# 3. Sur NanoPi — redémarrer le service
svc -u /data/etc/sv/daly-bms-venus
```

Si le service est actif pendant le `scp`, l'erreur suivante apparaît :
```
scp: dest open "/data/daly-bms/daly-bms-venus": Failure
```

### Déploiement de la configuration

Le fichier `Config.toml` est partagé par `daly-bms-server` et `daly-bms-venus`.

```bash
# Depuis Pi5
scp Config.toml root@192.168.1.120:/data/daly-bms/config.toml
```

Redémarrer les deux services après changement de config :
```bash
svc -d /data/etc/sv/daly-bms-venus
svc -d /data/etc/sv/daly-bms-server
svc -u /data/etc/sv/daly-bms-server
svc -u /data/etc/sv/daly-bms-venus
```

### Déploiement de mosquitto.conf (Pi5)

```bash
# Sur Pi5
cd ~/Daly-BMS-Rust
git pull origin claude/migrate-nodered-pi5-91idx
docker compose restart mosquitto
```

---

## Procédure d'import d'un flux Node-RED

1. Ouvrir Node-RED : http://192.168.1.141:1880
2. Double-clic sur l'onglet existant → clic **Delete** → confirmer
3. Menu ≡ → **Import** → coller le JSON → **Import**
4. Vérifier les nœuds (broker connecté, topics corrects)
5. Cliquer **Deploy**

---

## Commandes de vérification (NanoPi)

### Vérifier que le service tourne

```bash
ps | grep daly
# Doit afficher : /data/daly-bms/daly-bms-venus --config /data/daly-bms/config.toml
```

### Lister tous les services D-Bus Victron actifs

```bash
dbus -y | grep victronenergy
```

### Lire toutes les valeurs d'un service (méthode principale)

```bash
dbus -y com.victronenergy.temperature.mqtt_1 / GetItems
```

Retourne un dictionnaire de tous les chemins avec valeur et texte.

### Lire une valeur individuelle

```bash
dbus -y com.victronenergy.temperature.mqtt_1 /Temperature GetValue
dbus -y com.victronenergy.temperature.mqtt_1 /Humidity    GetValue
dbus -y com.victronenergy.temperature.mqtt_1 /Connected   GetValue
```

**Note :** `GetValue` sur un chemin individuel nécessite que l'objet feuille
soit enregistré dans zbus. Si non, erreur "Unknown object". `GetItems` sur `/`
fonctionne toujours. VRM utilise `GetItems`.

### Vérifier la réception MQTT sur NanoPi

```bash
mosquitto_sub -h localhost -t "santuario/heat/1/venus" -v
```

### Vérifier les logs du service Rust

Le service utilise `supervise` (runit) sans fichier log dédié.
Les traces apparaissent dans `readproctitle` :

```bash
ps | grep readproctitle
```

---

## Devices implémentés

| Device | Service D-Bus | Topic MQTT | Index config |
|---|---|---|---|
| Batterie Daly | `com.victronenergy.battery.mqtt_{n}` | `santuario/bms/{n}/venus` | `[[bms]]` |
| Température extérieure | `com.victronenergy.temperature.mqtt_{n}` | `santuario/heat/{n}/venus` | `[[sensors]]` |
| Chauffe-eau (HeatPump) | `com.victronenergy.heatpump.mqtt_{n}` | `santuario/heatpump/{n}/venus` | `[[heatpumps]]` |
| Irradiance (Meteo) | `com.victronenergy.meteo` | `santuario/meteo/venus` | `[meteo]` |

---

## Résolution des problèmes courants

### Service D-Bus non visible

1. Vérifier que le service Rust tourne : `ps | grep daly-bms-venus`
2. Vérifier qu'un message MQTT a été reçu (le service D-Bus est créé au 1er message)
3. Vérifier le bridge Mosquitto : règle `out` présente pour le topic concerné
4. Vérifier que Node-RED est déployé et le nœud connecté (vert "Connecté")

### /Connected = 0 (device déconnecté dans VRM)

Le keepalive Node-RED est trop long (> `watchdog_sec` = 30s).
Réduire le repeat de l'inject keepalive à 25s maximum.

### scp échoue avec "Failure"

Le service cible est actif et verrouille le binaire. Faire `svc -d` avant le `scp`.

### git pull échoue (local changes)

```bash
# Si fichier appartient à un autre utilisateur (ex: mosquitto Docker)
sudo chown $(whoami):$(whoami) docker/mosquitto/config/
sudo chown $(whoami):$(whoami) docker/mosquitto/config/mosquitto.conf
```

### Architecture mismatch (binaire invalide)

NanoPi = ARMv7 32-bit. Le binaire Pi5 (aarch64) ne fonctionne pas.
Toujours compiler avec `--target armv7-unknown-linux-gnueabihf`.

### Onglet Node-RED vide après docker compose down/up

Les volumes Node-RED sont persistants. Si les flux disparaissent :
1. Vérifier `docker volume ls | grep nodered`
2. Réimporter depuis `flux-nodered/*.json`
