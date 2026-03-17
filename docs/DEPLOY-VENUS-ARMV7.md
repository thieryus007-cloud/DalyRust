# Déploiement daly-bms-venus sur Venus OS (NanoPi ARMv7)

> **Date** : 2026-03-17
> **Système cible** : NanoPi intégré dans EasySolar II GX — Venus OS
> **Processeur** : **ARMv7l 32-bit** (`armv7-unknown-linux-gnueabihf`)
> **Init system** : daemontools (`svscan` / `svc` / `svstat`)
> **Shell** : BusyBox

---

## Architecture finale

```
PC Windows (x86_64)
  └─ daly-bms-server  ← polling RS485 Daly BMS (2 batteries)
       │ MQTT publish toutes les 1s (retain=true)
       ▼
FlashMQ (NanoPi 192.168.1.120:1883)
  ├─ santuario/bms/1/venus  ──▶  dbus-mqtt-battery-41  →  com.victronenergy.battery.mqtt_battery_141  (MQTT Battery 360Ah [141])
  ├─ santuario/bms/2/venus  ──▶  dbus-mqtt-battery-42  →  com.victronenergy.battery.mqtt_battery_142  (MQTT Battery 320Ah [142])
  └─ santuario/bms/+/venus  ──▶  daly-bms-venus (Rust natif)
                                  ├─ com.victronenergy.battery.mqtt_1  →  BMS-360Ah [151]
                                  └─ com.victronenergy.battery.mqtt_2  →  BMS-320Ah [152]

D-Bus Venus OS → GUI / VRM Portal / systemcalc / hub4control
```

---

## Matériel et logiciel

| Élément | Valeur |
|---------|--------|
| Appareil | EasySolar II GX (NanoPi intégré) |
| Processeur | **ARMv7l 32-bit** (`uname -m` → `armv7l`) |
| OS | Venus OS |
| Init system | daemontools (`svscan` PID 918 surveille `/service`) |
| Shell | BusyBox sh |
| MQTT broker | FlashMQ sur `127.0.0.1:1883` |
| Chemin persistant | `/data/` (survit aux mises à jour firmware) |

---

## Problèmes rencontrés et solutions

### 1. `sv` introuvable

**Symptôme** : `-sh: sv: command not found`

**Cause** : Venus OS utilise **daemontools**, pas runit.

**Solution** : Utiliser `svc` et `svstat` à la place de `sv`.

```bash
svstat /service/daly-bms-venus    # état
svc -t /service/daly-bms-venus    # restart (SIGTERM)
svc -d /service/daly-bms-venus    # stop
svc -u /service/daly-bms-venus    # start
```

---

### 2. Exec format error — mauvaise architecture

**Symptôme** :
```
/data/daly-bms/daly-bms-venus: cannot execute binary file: Exec format error
```

**Cause** : Binaires compilés pour `aarch64` (64-bit), NanoPi est **ARMv7l (32-bit)**.

**Solution** : Cross-compiler pour `armv7-unknown-linux-gnueabihf`.

```bash
# Sur la machine de développement
sudo apt install -y gcc-arm-linux-gnueabihf
rustup target add armv7-unknown-linux-gnueabihf
make build-venus-armv7
make install-venus-v7 GX_IP=192.168.1.120
```

---

### 3. Run script incorrect

**Symptôme** : Crash loop — services redémarrent toutes les secondes.

**Cause** : `/service/daly-bms-venus/run` contenait `exec /data/daly-bms-venus`
(mauvais chemin, 36 bytes, script hérité).

**Solution** : Corriger le script directement sur le NanoPi.

```bash
cat > /service/daly-bms-venus/run << 'EOF'
#!/bin/sh
exec /data/daly-bms/daly-bms-venus \
    --config /data/daly-bms/config.toml \
    2>&1
EOF
chmod +x /service/daly-bms-venus/run
svc -t /service/daly-bms-venus
```

---

### 4. `name already taken on the bus`

**Symptôme** :
```
ERROR daly_bms_venus::manager: Erreur traitement événement MQTT : name already taken on the bus
```

**Cause** : Le binaire a été lancé **manuellement** alors que le **daemon** tournait
déjà et avait enregistré les noms D-Bus.

**Solution** : Ne jamais lancer le binaire manuellement si le service daemon est actif.
Vérifier avec `svstat` avant tout test manuel.

---

### 5. `logread` non fonctionnel

**Symptôme** : `logread: can't find syslogd buffer: No such file or directory`

**Solution** : Lancer le binaire manuellement pour voir les logs :
```bash
/data/daly-bms/daly-bms-venus --config /data/daly-bms/config.toml 2>&1 | head -30
```

---

### 6. `ps aux` non supporté (BusyBox)

**Symptôme** : `ps: invalid option -- 'a'`

**Solution** : Utiliser `ps` sans options (BusyBox).
```bash
ps | grep daly
```

---

## Procédure de déploiement complète

### Étape 1 — Prérequis sur la machine de développement

```bash
sudo apt install -y gcc-arm-linux-gnueabihf
rustup target add armv7-unknown-linux-gnueabihf
```

### Étape 2 — Cross-compiler pour ARMv7

```bash
make build-venus-armv7
# Produit :
#   target/armv7-unknown-linux-gnueabihf/release/daly-bms-server
#   target/armv7-unknown-linux-gnueabihf/release/daly-bms-venus
```

### Étape 3 — Déployer sur le NanoPi

```bash
make install-venus-v7 GX_IP=192.168.1.120
# Copie les binaires, config et scripts de service via SSH
```

### Étape 4 — Corriger le run script venus (première fois uniquement)

```bash
ssh root@192.168.1.120
cat > /service/daly-bms-venus/run << 'EOF'
#!/bin/sh
exec /data/daly-bms/daly-bms-venus \
    --config /data/daly-bms/config.toml \
    2>&1
EOF
chmod +x /service/daly-bms-venus/run
```

### Étape 5 — Redémarrer les services

```bash
svc -t /service/daly-bms-server
svc -t /service/daly-bms-venus
sleep 5
svstat /service/daly-bms-server
svstat /service/daly-bms-venus
# Résultat attendu : "up (pid XXXXX) N seconds"
```

### Étape 6 — Vérifier D-Bus

```bash
dbus -y com.victronenergy.battery.mqtt_1 /Soc GetValue
dbus -y com.victronenergy.battery.mqtt_2 /Soc GetValue
# Résultat attendu : valeur numérique (ex: 99.0)
```

### Étape 7 — Persistence au reboot

```bash
cat >> /data/rc.local << 'EOF'
ln -sf /data/etc/sv/daly-bms-server /service/daly-bms-server 2>/dev/null || true
ln -sf /data/etc/sv/daly-bms-venus  /service/daly-bms-venus  2>/dev/null || true
EOF
```

---

## Structure fichiers sur le NanoPi

```
/data/
  daly-bms/
    daly-bms-server      ← binaire ARMv7 (polling RS485 + MQTT)
    daly-bms-venus       ← binaire ARMv7 (bridge MQTT → D-Bus)
    config.toml          ← configuration (conservée entre déploiements)

  etc/
    sv/
      daly-bms-server/
        run              ← exec /data/daly-bms/daly-bms-server --config ...
      daly-bms-venus/
        run              ← exec /data/daly-bms/daly-bms-venus --config ...

  rc.local               ← symlinks recréés au boot

/service/
  daly-bms-server  →  /data/etc/sv/daly-bms-server   (symlink)
  daly-bms-venus/                                      (répertoire + run script)
    run
    supervise/
```

---

## Commandes de diagnostic

```bash
# État des services
svstat /service/daly-bms-server
svstat /service/daly-bms-venus

# Processus actifs (BusyBox)
ps | grep daly

# D-Bus — liste services battery (syntaxe correcte Venus OS)
dbus -y | grep battery

# D-Bus — lire valeurs
dbus -y com.victronenergy.battery.mqtt_1 /Soc GetValue
dbus -y com.victronenergy.battery.mqtt_1 /Dc/0/Voltage GetValue
dbus -y com.victronenergy.battery.mqtt_2 /Soc GetValue
dbus -y com.victronenergy.battery.mqtt_2 /Dc/0/Voltage GetValue

# Tester le binaire manuellement (service STOPPÉ au préalable)
svc -d /service/daly-bms-venus
/data/daly-bms/daly-bms-venus --config /data/daly-bms/config.toml
```

---

## Services D-Bus enregistrés

| Service D-Bus | Device Instance | Batterie | SOC observé |
|---------------|----------------|----------|-------------|
| `com.victronenergy.battery.mqtt_1` | 151 | BMS-360Ah | 99.0 % |
| `com.victronenergy.battery.mqtt_2` | 152 | BMS-320Ah | 99.4 % |

Les instances 141/142 sont réservées à `dbus-mqtt-battery` (service Python existant).

---

## Makefile — cibles ARMv7

```bash
make build-venus-armv7          # compile pour armv7l
make install-venus-v7 GX_IP=X  # compile + déploie via SSH
make build-venus-arm            # compile pour aarch64 (64-bit, non compatible NanoPi)
make install-venus GX_IP=X      # compile aarch64 + déploie
```
