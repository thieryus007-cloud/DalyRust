# Phase 3 — Intégration Venus OS via D-Bus natif en Rust

> **Contexte** : Victron EasySolar II GX — Venus OS v3.7 — NanoPi intégré
> **Objectif** : Présenter les BMS Daly dans le VRM Portal via D-Bus natif
> **Approche retenue** : MQTT → Service Rust D-Bus natif (`daly-bms-venus`)

---

## 1. Évaluation de l'existant

### Ce qui est déjà en place (Phase 1 & 2)

| Composant | État | Fichier |
|-----------|------|---------|
| Polling RS485 Daly BMS | ✅ Opérationnel | `crates/daly-bms-core/` |
| `BmsSnapshot` riche (toutes métriques) | ✅ Complet | `types.rs` |
| Bridge MQTT actif | ✅ Opérationnel | `bridges/mqtt.rs` |
| Payload format Venus OS (`/venus` topic) | ✅ Implémenté | `build_venus_payload()` |
| Broker Mosquitto sur NanoPi | ✅ En place | `192.168.1.120:1883` |
| Topics publiés : `santuario/bms/{n}/venus` | ✅ Actif | Config.toml `mqtt_index` |
| Dashboard web + WebSocket | ✅ Opérationnel | `crates/daly-bms-server/` |

### Gap à combler pour Venus OS

Le payload Venus est **déjà produit** en MQTT. Il manque uniquement le pont
MQTT → D-Bus qui permettrait à Venus OS de voir les batteries dans systemcalc/VRM.

---

## 2. Architecture retenue

```
┌─────────────────────────────────────────────────────────────────────┐
│                    NanoPi (Venus OS v3.7)                           │
│                                                                     │
│  [RS485]──►[daly-bms-server]──►[Mosquitto]──►[daly-bms-venus]     │
│                                  :1883          │                   │
│                                             D-Bus system bus        │
│                                                 │                   │
│                                    com.victronenergy.battery.1      │
│                                    com.victronenergy.battery.2      │
│                                                 │                   │
│                                         [systemcalc-py]             │
│                                                 │                   │
│                                         [VRM Portal]                │
└─────────────────────────────────────────────────────────────────────┘
```

### Justification du choix MQTT → D-Bus

| Critère | Direct RS485→D-Bus | MQTT→D-Bus (retenu) |
|---------|-------------------|---------------------|
| Couplage avec le serveur principal | Fort | Aucun |
| Redémarrage indépendant | Non | ✅ Oui |
| Test hors Venus OS | Difficile | ✅ Via MQTT seul |
| Impact sur le polling BMS | Risqué | ✅ Zéro |
| Architecture | Monolithique | ✅ Microservice |

---

## 3. Analyse de compatibilité Venus OS v3.7

### Interface D-Bus attendue par Venus OS

Chaque batterie doit s'enregistrer comme :
- **Service name** : `com.victronenergy.battery.{n}` (ex: `.battery.mqtt_1`)
- **Object paths** : chaque métrique est un objet D-Bus séparé
- **Interface** : `com.victronenergy.BusItem` sur chaque path

```
Méthodes requises par com.victronenergy.BusItem :
  GetValue()  → Variant(value)
  GetText()   → String
  SetValue(v) → Int32(0 = ok)

Signal :
  ItemsChanged(dict<string, dict<string, variant>>)
```

### Paths D-Bus requis pour apparaître dans VRM

| Path D-Bus | Source BmsSnapshot | Obligatoire |
|------------|--------------------|-------------|
| `/Connected` | `1` (constant) | ✅ |
| `/Dc/0/Voltage` | `dc.voltage` | ✅ |
| `/Dc/0/Current` | `dc.current` | ✅ |
| `/Dc/0/Power` | `dc.power` | ✅ |
| `/Dc/0/Temperature` | `dc.temperature` | ✅ |
| `/Soc` | `soc` | ✅ |
| `/Capacity` | `bms_reported_capacity_ah` | Recommandé |
| `/InstalledCapacity` | `installed_capacity` | Recommandé |
| `/ConsumedAmphours` | `consumed_amphours` | Recommandé |
| `/TimeToGo` | `time_to_go` | Recommandé |
| `/Info/MaxChargeVoltage` | `info.max_charge_voltage` | DVCC |
| `/Info/MaxChargeCurrent` | `info.max_charge_current` | DVCC |
| `/Info/MaxDischargeCurrent` | `info.max_discharge_current` | DVCC |
| `/Io/AllowToCharge` | `io.allow_to_charge` | DVCC |
| `/Io/AllowToDischarge` | `io.allow_to_discharge` | DVCC |
| `/Balancing` | `balancing` | Info |
| `/System/MinCellVoltage` | `system.min_cell_voltage` | Info |
| `/System/MaxCellVoltage` | `system.max_cell_voltage` | Info |
| `/System/MinCellTemperature` | `system.min_cell_temperature` | Info |
| `/System/MaxCellTemperature` | `system.max_cell_temperature` | Info |
| `/Alarms/LowVoltage` | `alarms.low_voltage` | Alertes |
| `/Alarms/HighVoltage` | `alarms.high_voltage` | Alertes |
| `/Alarms/LowSoc` | `alarms.low_soc` | Alertes |
| `/ProductName` | `"Daly BMS"` | Identification |
| `/ProductId` | `0x0000` | Identification |
| `/FirmwareVersion` | `firmware_sw` | Identification |
| `/DeviceInstance` | `mqtt_index` (1, 2, …) | ✅ |

### Keepalive Venus OS

Venus OS surveille la présence du service via le signal `ItemsChanged`.
- Émettre `ItemsChanged` au minimum toutes **60 secondes**
- En pratique : à chaque mise à jour MQTT (≈1 Hz) → amplement suffisant

---

## 4. Choix technologique : `zbus` vs `dbus` crate

| Critère | `dbus` crate | `zbus` (retenu) |
|---------|-------------|-----------------|
| Dépendance C (libdbus) | Oui — complexe cross-compile | ✅ Pure Rust |
| Cross-compilation ARM | Difficile (linking libdbus.so) | ✅ Simple |
| Async/await natif | Non (bloquant) | ✅ Tokio natif |
| Qualité API | Bas niveau | ✅ Ergonomique |
| Stabilité | Mature | ✅ Stable (v4.x) |
| Taille binaire | Petit | Légèrement plus grand |

**Choix : `zbus 4.x`** — pure Rust, async Tokio, cross-compilation sans douleur.

---

## 5. Structure du nouveau crate `daly-bms-venus`

```
crates/daly-bms-venus/
├── Cargo.toml
└── src/
    ├── main.rs           # Point d'entrée, config, orchestration
    ├── config.rs         # VenusConfig (MQTT host, D-Bus bus, instances)
    ├── mqtt_source.rs    # Abonnement MQTT + parsing payload Venus
    ├── battery_service.rs # Service D-Bus pour un BMS (paths + interface)
    └── manager.rs        # Gestion dynamique des N services D-Bus
```

### Flux de données

```
MQTT subscriber
  └── subscribe("santuario/bms/+/venus")
      └── parse VenusPayload (serde_json)
          └── BatteryService::update(payload)
              └── émet ItemsChanged sur D-Bus
                  └── systemcalc-py lit les valeurs
                      └── VRM Portal affiche les batteries
```

---

## 6. Plan d'implémentation — 4 étapes

### Étape 1 — Crate `daly-bms-venus` (structure + MQTT)
**Durée estimée** : Phase de développement
**Fichiers** : nouveau crate dans `crates/daly-bms-venus/`

- [ ] `Cargo.toml` avec dépendances `zbus`, `rumqttc`, `serde_json`, `tokio`
- [ ] `config.rs` : `VenusConfig` chargée depuis TOML existant
- [ ] `mqtt_source.rs` : subscribe `{prefix}/+/venus`, parse `VenusPayload`
- [ ] `types.rs` : `VenusPayload` miroir du payload `build_venus_payload()`

### Étape 2 — Service D-Bus `BatteryService`
**Fichiers** : `battery_service.rs`, `manager.rs`

- [ ] Implémenter `com.victronenergy.BusItem` via `zbus` pour chaque path
- [ ] Gérer le `/DeviceInstance` unique par BMS
- [ ] Émettre le signal `ItemsChanged` à chaque mise à jour MQTT
- [ ] Watchdog : republier `/Connected=1` toutes les 30s si pas de données

### Étape 3 — Intégration workspace + config
**Fichiers** : `Cargo.toml` workspace, `Config.toml`

- [ ] Ajouter `daly-bms-venus` au workspace
- [ ] Ajouter section `[venus]` dans `Config.toml` :
  ```toml
  [venus]
  enabled = true
  dbus_bus = "system"        # "system" sur Venus OS, "session" pour test
  service_prefix = "mqtt"    # → com.victronenergy.battery.mqtt_1
  ```
- [ ] Ajouter la section `[venus]` dans `config.rs` du server

### Étape 4 — Cross-compilation et déploiement Venus OS
**Fichiers** : `Makefile`, `nanoPi/` (scripts de déploiement)

- [ ] Target ARM64 : `aarch64-unknown-linux-gnu` (NanoPi Neo3/R2S)
  ```makefile
  cross build --target aarch64-unknown-linux-gnu --release -p daly-bms-venus
  ```
- [ ] Script de déploiement `nanoPi/install-venus.sh` :
  ```sh
  # Copie le binaire dans /data/daly-bms/ (persistent post-firmware-update)
  # Crée le service runit dans /data/etc/sv/daly-bms-venus/
  # Symlink dans /service/ pour activation automatique
  ```
- [ ] Service runit `nanoPi/sv/daly-bms-venus/run` :
  ```sh
  #!/bin/sh
  exec /data/daly-bms/daly-bms-venus --config /data/daly-bms/config.toml 2>&1
  ```

---

## 7. Risques et mitigations

| Risque | Probabilité | Mitigation |
|--------|------------|------------|
| Interface D-Bus Venus v3.7 légèrement différente | Moyen | Tester `dbus-send` manuellement depuis le GX d'abord |
| `zbus` incompatible avec D-Bus version Venus | Faible | zbus suit la spec D-Bus standard |
| Conflit `DeviceInstance` avec autre service BMS | Faible | Utiliser des instances hautes (ex: 10, 11) |
| MPPT réagit à `AllowToCharge=0` (DVCC actif) | Attention | Les `Io` sont figés à `1` dans le payload actuel (intentionnel) |
| binaire ARM trop grand pour /data/ | Très faible | `strip=true` + `panic=abort` déjà dans profile release |

---

## 8. Configuration finale dans Config.toml

```toml
[venus]
enabled       = true
# "system" sur Venus OS réel — "session" pour test local sans sudo
dbus_bus      = "system"
# Suffixe du service D-Bus : com.victronenergy.battery.{prefix}_{mqtt_index}
service_prefix = "mqtt"
# Watchdog : republier Connected=1 si pas de données MQTT pendant N sec
watchdog_sec  = 30
```

La section `[[bms]]` existante fournit déjà `mqtt_index` (1, 2, …) qui sert
de `DeviceInstance` D-Bus.

---

## 9. Validation

### Test hors Venus OS (développement)

```bash
# Démarrer un D-Bus session bus de test
dbus-daemon --session --print-address

# Lancer le service en mode session
DBUS_SESSION_BUS_ADDRESS=... DALY_CONFIG=Config.toml \
  cargo run -p daly-bms-venus

# Vérifier les services enregistrés
dbus-send --session --dest=com.victronenergy.battery.mqtt_1 \
  --print-reply / com.victronenergy.BusItem.GetValue

# Vérifier le SOC
dbus-send --session --dest=com.victronenergy.battery.mqtt_1 \
  --print-reply /Soc com.victronenergy.BusItem.GetValue
```

### Test sur Venus OS

```sh
# Sur le GX via SSH
dbus -y com.victronenergy.battery.mqtt_1 /Soc GetValue
dbus -y com.victronenergy.battery.mqtt_1 /Dc/0/Voltage GetValue
```

---

## 10. Fichiers à créer / modifier

| Fichier | Action |
|---------|--------|
| `crates/daly-bms-venus/Cargo.toml` | Créer |
| `crates/daly-bms-venus/src/main.rs` | Créer |
| `crates/daly-bms-venus/src/config.rs` | Créer |
| `crates/daly-bms-venus/src/mqtt_source.rs` | Créer |
| `crates/daly-bms-venus/src/battery_service.rs` | Créer |
| `crates/daly-bms-venus/src/manager.rs` | Créer |
| `Cargo.toml` (workspace) | Modifier — ajouter membre |
| `Config.toml` | Modifier — ajouter section `[venus]` |
| `nanoPi/install-venus.sh` | Créer |
| `nanoPi/sv/daly-bms-venus/run` | Créer |
| `Makefile` | Modifier — ajouter cible `build-venus` |

---

*Document généré le 2026-03-17 — Phase 3 Venus OS D-Bus integration*
