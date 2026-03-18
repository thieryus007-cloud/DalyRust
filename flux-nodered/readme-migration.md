# Migration Node-RED : NanoPi Neo3 → Pi5

## Infrastructure

| Machine | IP | Rôle |
|---|---|---|
| Pi5 (cible) | 192.168.1.141 | Node-RED :1880, Mosquitto :1883 (Docker), InfluxDB :8086, Grafana :3001 |
| NanoPi Neo3 (source) | 192.168.1.120 | Venus OS, Node-RED :1881, Mosquitto local |
| VRM Portal ID | `c0619ab9929a` | Victron GX instance |

---

## Architecture MQTT après migration

```
Shelly (192.168.1.136) ──MQTT──► Pi5 Mosquitto (dalybms-mosquitto)
                                        │
                         MQTT bridge ◄──┤──► NanoPi Mosquitto (192.168.1.120)
                                        │        N/c0619ab9929a/#  (Venus data IN)
                                        │        W/c0619ab9929a/#  (Venus writes OUT)
                                        │        R/c0619ab9929a/#  (Venus keepalive OUT)
                                        │
Node-RED Pi5 ────────────────────────────┘
  (tous les flows utilisent dalybms-mosquitto)
```

---

## Ordre de migration (par priorité)

| # | Flow | Fichier | Changements |
|---|---|---|---|
| 1 | meteo | meteo.json | Aucun (HTTP only) |
| 2 | testing | testing.json | Aucun |
| 3 | previsions | previsions.json | Aucun (HTTP OpenWeatherMap) |
| 4 | waterheater | waterheater.json | Aucun (HTTP dynamique) |
| 5 | setwaterheater | setwaterheater.json | Aucun (HTTP dynamique) |
| 6 | deye | deye.json | Broker `192.168.1.120` → `dalybms-mosquitto` + D-Bus → MQTT |
| 7 | shelly | shelly.json | Broker `192.168.1.120` → `dalybms-mosquitto` |
| 8 | shelly2 | shelly2.json | Broker `localhost` → `dalybms-mosquitto`, D-Bus nodes supprimés |
| 9 | feedinpower ⚠️ | feedinpower.json | **Réécriture complète** D-Bus → MQTT Venus bridge |

---

## Étapes de migration

### 1. Déployer le bridge MQTT sur Pi5

Le fichier `docker/mosquitto/config/mosquitto.conf` a été mis à jour avec un bridge vers NanoPi.

Redémarrer Mosquitto sur Pi5 :
```bash
# Sur Pi5 (192.168.1.141)
ssh pi5compute@192.168.1.141
cd ~/Daly-BMS-Rust
docker compose -f docker-compose.infra.yml restart mosquitto
docker compose -f docker-compose.infra.yml logs mosquitto | tail -20
```

Vérifier le bridge :
```bash
# Sur Pi5 — vérifier que les topics Venus arrivent
mosquitto_sub -h 192.168.1.141 -t "N/c0619ab9929a/#" -v
```

### 2. Vérifier Node-RED sur Pi5

Node-RED est déjà dans `docker-compose.infra.yml` → `dalybms-nodered` sur port `:1880`.

```bash
docker compose -f docker-compose.infra.yml ps nodered
curl -s http://192.168.1.141:1880/ | head -5
```

### 3. Importer les flows sur Pi5

Importer chacun des 9 flows dans Node-RED Pi5 (http://192.168.1.141:1880) :
- Menu ≡ → Import → Presse-papiers → Coller le contenu JSON
- Ou via API :

```bash
# Exemple : importer meteo.json via API Node-RED
curl -X POST http://192.168.1.141:1880/flow \
  -H "Content-Type: application/json" \
  -d @flux-nodered/meteo.json
```

**Ordre recommandé** : meteo → testing → previsions → waterheater → setwaterheater → deye → shelly → shelly2 → feedinpower

### 4. Vérifier les flows critiques

#### Flow `deye` (flow 6)
- Broker MQTT → `dalybms-mosquitto`
- Topics Venus (Fréquence, GridConnected) via bridge depuis NanoPi
- Topics Shelly `shellypro2pm-ec62608840a4/#` via bridge ou Shelly reconfiguré

#### Flow `shelly2` (flow 8)
- Broker MQTT → `dalybms-mosquitto`
- Topics `N/c0619ab9929a/switch/50/...` et `51/...` via bridge
- Nœuds D-Bus supprimés (inutilisables sur Pi5)
- Tab était `disabled: true` sur NanoPi → à activer si nécessaire

#### Flow `feedinpower` (flow 9 — ⚠️ CRITIQUE)

**Réécriture complète** — remplacement de tous les nœuds `victron-*` D-Bus par MQTT :

| Nœud original (D-Bus) | Remplacé par (MQTT) |
|---|---|
| `victron-input-vebus` IgnoreAcIn1 | `mqtt in` → `N/c0619ab9929a/vebus/275/Ac/State/IgnoreAcIn1` |
| `victron-input-system` PvPower | `mqtt in` → `N/c0619ab9929a/system/0/Ac/PvOnOutput/L1/Power` |
| `victron-input-system` Consumption | `mqtt in` → `N/c0619ab9929a/system/0/Ac/ConsumptionOnOutput/L1/Power` |
| `victron-output-custom` MaxChargeCurrent | `mqtt out` → `W/c0619ab9929a/vebus/275/Dc/0/MaxChargeCurrent` |
| `victron-output-vebus` PowerAssist | `mqtt out` → `W/c0619ab9929a/vebus/275/Settings/PowerAssistEnabled` |
| `victron-output-settings` MaxFeedInPower | `mqtt out` → `W/c0619ab9929a/settings/0/Settings/CGwacs/MaxFeedInPower` |
| `victron-output-settings` AcExportLimit | `mqtt out` → `W/c0619ab9929a/settings/0/Settings/CGwacs/AcExportLimit` |

Payload Venus MQTT pour les writes : `{"value": X}`

**Logique conservée** :
- Grid déconnecté (IgnoreAcIn1=1) → MaxChargeCurrent=70A + PowerAssist=ON
- Grid connecté + excédent PV>50W → MaxChargeCurrent=4A + PowerAssist=OFF + FeedIn=0W
- Grid connecté + pas d'excédent → MaxChargeCurrent=0A + PowerAssist=OFF + FeedIn=0W

**Vérification** : Activer les nœuds debug et vérifier les topics Venus dans mosquitto_sub.

### 5. Reconfigurer les appareils Shelly

Le Shelly Pro 2PM (`192.168.1.136`, MAC `ec62608840a4`) doit être reconfiguré pour se connecter au Mosquitto Pi5 au lieu du NanoPi :

```
Shelly Web UI → Settings → MQTT → Server: 192.168.1.141:1883
```

Pendant la transition, le bridge MQTT Pi5↔NanoPi permet aux deux de fonctionner.

### 6. Arrêter Node-RED sur NanoPi

Après validation de tous les flows sur Pi5 :
```bash
# Sur NanoPi (192.168.1.120)
ssh root@192.168.1.120  # pw: 12345678
# Node-RED sur Venus OS
/etc/init.d/node-red stop
# ou si systemd
systemctl stop node-red
```

---

## Vérification finale

```bash
# Topics Venus reçus sur Pi5 via bridge
mosquitto_sub -h 192.168.1.141 -t "N/c0619ab9929a/vebus/275/#" -v -C 5

# Test écriture Venus via Pi5 Mosquitto
mosquitto_pub -h 192.168.1.141 \
  -t "W/c0619ab9929a/vebus/275/Settings/PowerAssistEnabled" \
  -m '{"value": 1}'

# Status Node-RED Pi5
curl http://192.168.1.141:1880/flows | python3 -c "import json,sys; flows=json.load(sys.stdin); print(f'Flows: {len([n for n in flows if n[\"type\"]==\"tab\"])} tabs, {len(flows)} nodes')"
```

---

## Changements repo

| Fichier | Modification |
|---|---|
| `docker/mosquitto/config/mosquitto.conf` | + bridge MQTT vers NanoPi (N/, W/, R/ topics Venus + Shelly) |
| `flux-nodered/deye.json` | Broker → `dalybms-mosquitto` + victron D-Bus → MQTT |
| `flux-nodered/shelly.json` | Broker `192.168.1.120` → `dalybms-mosquitto` |
| `flux-nodered/shelly2.json` | Broker `localhost` → `dalybms-mosquitto` + D-Bus nodes supprimés |
| `flux-nodered/feedinpower.json` | **Réécriture complète** victron D-Bus → MQTT Venus bridge |
