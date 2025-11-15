# Guide Technique - Interface Mac Locale TinyBMS

**Version:** 0.1.0
**Date:** 2025-11-15
**Objectif:** Outil de dépannage et configuration initiale du TinyBMS via USB-UART

---

## Table des matières

1. [Vue d'ensemble](#vue-densemble)
2. [Architecture du système](#architecture-du-système)
3. [Protocole TinyBMS](#protocole-tinybms)
4. [Flux de communication](#flux-de-communication)
5. [Considérations de fiabilité](#considérations-de-fiabilité)
6. [Guide de dépannage](#guide-de-dépannage)
7. [Limitations connues](#limitations-connues)

---

## Vue d'ensemble

### Objectif

Cette application Node.js permet de **configurer et diagnostiquer** le TinyBMS directement depuis un Mac mini via une connexion USB-UART. Elle est conçue pour :

- **Dépannage occasionnel** : lecture rapide des registres pour diagnostic
- **Configuration initiale** : écriture des paramètres de base avant déploiement
- **Validation terrain** : vérification des valeurs en temps réel

### Architecture simplifiée

```
┌─────────────────────────────────────────────┐
│         Navigateur (localhost:5173)         │
│  ┌──────────────────────────────────────┐   │
│  │   Interface web (public/index.html)  │   │
│  │   - Table des registres              │   │
│  │   - Édition inline                   │   │
│  │   - Filtrage par groupe              │   │
│  └──────────────────────────────────────┘   │
└──────────────────┬──────────────────────────┘
                   │ HTTP REST API
                   ▼
┌─────────────────────────────────────────────┐
│       Serveur Express (src/server.js)       │
│  ┌──────────────────────────────────────┐   │
│  │  Endpoints:                          │   │
│  │  GET  /api/ports                     │   │
│  │  POST /api/connection/open           │   │
│  │  GET  /api/registers?group=...       │   │
│  │  POST /api/registers                 │   │
│  │  POST /api/system/restart            │   │
│  └──────────────────────────────────────┘   │
└──────────────────┬──────────────────────────┘
                   │
        ┌──────────┴──────────┐
        │                     │
        ▼                     ▼
┌──────────────────┐  ┌──────────────────┐
│ registers.js     │  │   serial.js      │
│ - Parse catalog  │  │ - TinyBMS proto  │
│ - Scale/enum     │  │ - CRC16          │
│ - Validation     │  │ - Frame building │
└──────────────────┘  └─────────┬────────┘
                                │
                                ▼
                      ┌──────────────────┐
                      │  SerialPort API  │
                      │  (Node.js lib)   │
                      └─────────┬────────┘
                                │
                                ▼
                      ┌──────────────────┐
                      │  /dev/ttyUSB0    │
                      │  115200 baud     │
                      │  8N1             │
                      └─────────┬────────┘
                                │
                                ▼
                      ┌──────────────────┐
                      │     TinyBMS      │
                      │  (ESP32 firmware)│
                      └──────────────────┘
```

### Composants principaux

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/server.js` | 187 | Serveur HTTP Express + API REST |
| `src/serial.js` | 405 | Communication série + protocole TinyBMS |
| `src/registers.js` | 323 | Parsing du catalogue + conversion scale/raw |
| `public/index.html` | - | Interface web interactive |
| `data/registers.json` | - | Catalogue précompilé (34 registres) |

---

## Architecture du système

### 1. Serveur Express (`server.js`)

#### Initialisation

```javascript
const app = express();
const serial = new TinyBmsSerial();
const catalogue = getRegisterCatalogue();
const PORT = 5173;

app.use(express.json({ limit: '1mb' }));
app.use(express.static(publicDir));
```

#### Endpoints REST

##### **GET /api/ports**
Liste les ports série USB disponibles.

**Réponse :**
```json
{
  "ports": [
    {
      "path": "/dev/tty.usbserial-A50285BI",
      "manufacturer": "FTDI",
      "serialNumber": "A50285BI",
      "vendorId": "0403",
      "productId": "6001"
    }
  ]
}
```

##### **POST /api/connection/open**
Ouvre la connexion série.

**Requête :**
```json
{
  "path": "/dev/tty.usbserial-A50285BI",
  "baudRate": 115200
}
```

**Réponse :**
```json
{
  "connected": true,
  "port": {
    "path": "/dev/tty.usbserial-A50285BI",
    "baudRate": 115200
  }
}
```

##### **GET /api/registers?group=battery**
Lit tous les registres (optionnellement filtrés par groupe).

**Flux d'exécution :**
1. Vérification de la connexion série (`ensureConnected()`)
2. Filtrage du catalogue par groupe si `?group=...`
3. Lecture séquentielle via `serial.readCatalogue(descriptors)`
4. Conversion raw → user pour chaque registre
5. Retour JSON avec tous les métadonnées

**Réponse :**
```json
{
  "total": 34,
  "registers": [
    {
      "key": "battery_voltage",
      "label": "Tension batterie",
      "unit": "V",
      "group": "battery",
      "type": "uint16",
      "access": "ro",
      "address": 1,
      "address_hex": "0x0001",
      "scale": 0.01,
      "precision": 2,
      "raw": 1234,
      "value": 12.34,
      "current_user_value": 12.34,
      "default": 0,
      "min": 0,
      "max": 60,
      "step": 0.01,
      "enum": []
    }
  ]
}
```

##### **POST /api/registers**
Écrit un registre et lit la valeur confirmée.

**Requête :**
```json
{
  "key": "cell_overvoltage_protection",
  "value": 4.2
}
```

**Flux d'exécution :**
1. Validation du payload (`key` et `value` requis)
2. Recherche du descripteur via `findRegisterDescriptorByKey(key)`
3. Conversion user → raw via `userToRawValue(descriptor, value)`
4. Écriture série + readback automatique (`serial.writeRegister()`)
5. Retour de la valeur confirmée

**Réponse :**
```json
{
  "status": "updated",
  "key": "cell_overvoltage_protection",
  "address": 16,
  "raw": 4200,
  "value": 4.2,
  "current_user_value": 4.2
}
```

##### **POST /api/system/restart**
Envoie la commande de redémarrage au TinyBMS.

**Réponse :**
```json
{
  "status": "restarting"
}
```

**Note :** Le TinyBMS redémarre immédiatement. La connexion série sera perdue et doit être rouverte après ~5 secondes.

#### Gestion d'erreurs

```javascript
app.use('/api', (err, req, res, next) => {
  const status = err.statusCode || 500;
  res.status(status).json({
    error: err.message || 'Erreur interne'
  });
});
```

**Codes HTTP :**
- `400` : Payload invalide, validation échouée
- `404` : Registre inconnu
- `500` : Erreur série, timeout, CRC invalide
- `503` : Port série non connecté

---

### 2. Communication série (`serial.js`)

#### Classe TinyBmsSerial

##### État interne

```javascript
class TinyBmsSerial {
  constructor() {
    this.port = null;              // Instance SerialPort
    this._readBuffer = Buffer.alloc(0);  // Buffer de réception cumulatif
    this._pending = [];            // Files d'attente des promesses
    this._mutex = new Mutex();     // Exclusion mutuelle pour transactions
    this._portInfo = null;         // { path, baudRate }
  }
}
```

##### Mutex basé sur Promise

```javascript
class Mutex {
  constructor() {
    this._current = Promise.resolve();
  }

  runExclusive(task) {
    const run = this._current.then(() => task());
    this._current = run.then(
      () => undefined,
      () => undefined
    );
    return run;
  }
}
```

**Garanties :**
- Une seule transaction série active à la fois
- Évite les collisions de trames entre lectures/écritures concurrentes
- Simplifie la gestion des timeouts

##### Ouverture/fermeture

```javascript
async open(path, { baudRate = 115200 } = {}) {
  if (this.isOpen()) {
    await this.close();
  }

  const port = new SerialPort({
    path,
    baudRate,
    autoOpen: false
  });

  await new Promise((resolve, reject) => {
    port.open((err) => {
      if (err) reject(err);
      else resolve();
    });
  });

  port.on('data', this._onData);
  port.on('error', this._onError);

  this.port = port;
  this._readBuffer = Buffer.alloc(0);
  this._pending = [];
  this._portInfo = { path, baudRate };
  return this._portInfo;
}
```

**Bonne pratique :** Toujours fermer (`close()`) avant de rouvrir pour éviter les handles en double.

##### Lecture de registre

```javascript
async readRegister(address, timeoutMs = 750) {
  return this._mutex.runExclusive(() =>
    this._readRegisterLocked(address, timeoutMs)
  );
}

async _readRegisterLocked(address, timeoutMs) {
  await this._prepareTransaction();        // Flush buffers
  const request = buildReadFrame(address); // Construit trame READ
  const responsePromise = this._waitForFrame(
    (frame) => frame[1] === 0x07,          // Attend réponse READ (0x07)
    timeoutMs
  );

  try {
    await this._writeFrame(request);       // Envoi série
  } catch (error) {
    responsePromise.cancel();              // Annule timeout si envoi échoue
    throw error;
  }

  const frame = await responsePromise;     // Attend réponse
  const raw = frame[3] | (frame[4] << 8);  // Extrait uint16 little-endian
  return raw;
}
```

**Étapes clés :**
1. `_prepareTransaction()` : Vide le buffer série et réinitialise `_readBuffer`
2. `buildReadFrame(address)` : Construit trame binaire avec CRC16
3. `_writeFrame(request)` : Envoi série avec `port.write()` + `port.drain()`
4. `_waitForFrame(matcher, timeout)` : Attend trame correspondante ou timeout
5. Extraction du `raw` value (uint16 little-endian)

##### Écriture de registre

```javascript
async writeRegister(address, rawValue, timeoutMs = 750) {
  return this._mutex.runExclusive(() =>
    this._writeRegisterLocked(address, rawValue, timeoutMs)
  );
}

async _writeRegisterLocked(address, rawValue, timeoutMs) {
  await this._prepareTransaction();
  const frame = buildWriteFrame(address, rawValue);
  const ackPromise = this._waitForFrame(
    (received) => received[1] === 0x01 || received[1] === 0x81,
    timeoutMs
  );

  try {
    await this._writeFrame(frame);
  } catch (error) {
    ackPromise.cancel();
    throw error;
  }

  const ack = await ackPromise;
  if (ack[1] === 0x81) {
    const errorCode = ack.length > 3 ? ack[3] : 0;
    throw new Error(`TinyBMS NACK (code 0x${errorCode.toString(16).padStart(2, '0')})`);
  }

  const readback = await this._readRegisterLocked(address, timeoutMs);
  return readback;
}
```

**Différences avec lecture :**
- Attend ACK (0x01) ou NACK (0x81)
- Si NACK, extrait le code erreur du payload
- **Readback automatique** : relit le registre pour confirmer l'écriture

**Codes NACK possibles :**
- `0x01` : Registre en lecture seule
- `0x02` : Valeur hors limites
- `0x03` : Adresse invalide

##### Gestion des données entrantes

```javascript
_handleData(data) {
  this._readBuffer = Buffer.concat([this._readBuffer, data]);
  let parsing = true;
  while (parsing) {
    const { frame, buffer } = extractFrame(this._readBuffer);
    this._readBuffer = buffer;
    if (!frame) {
      parsing = false;
      break;
    }
    this._dispatchFrame(frame);
  }
}
```

**Flux :**
1. Accumule les données dans `_readBuffer`
2. Boucle `extractFrame()` jusqu'à ce qu'aucune trame complète ne soit trouvée
3. Dispatch chaque trame vers la promesse en attente via `_dispatchFrame()`

**⚠️ Point de fiabilité :** Le buffer peut croître indéfiniment si des données corrompues arrivent sans jamais former de trame valide.

**Amélioration suggérée :**
```javascript
_handleData(data) {
  this._readBuffer = Buffer.concat([this._readBuffer, data]);

  // Limite à 4KB pour éviter débordement mémoire
  if (this._readBuffer.length > 4096) {
    this._readBuffer = this._readBuffer.slice(-4096);
  }

  let parsing = true;
  while (parsing) {
    const { frame, buffer } = extractFrame(this._readBuffer);
    this._readBuffer = buffer;
    if (!frame) {
      parsing = false;
      break;
    }
    this._dispatchFrame(frame);
  }
}
```

---

## Protocole TinyBMS

### Format des trames

Toutes les trames suivent le format :

```
┌─────────┬────────┬─────────┬─────────────────┬──────────┐
│ Préamb. │ Cmd ID │ Payload │     Payload     │   CRC16  │
│  (0xAA) │ (1 B)  │ Len (1B)│   (N bytes)     │  (2 B)   │
└─────────┴────────┴─────────┴─────────────────┴──────────┘
```

**Offset :**
- `[0]` : Préambule (toujours `0xAA`)
- `[1]` : Command ID
- `[2]` : Longueur du payload (N)
- `[3..2+N]` : Payload
- `[3+N..4+N]` : CRC16 little-endian

### Commandes

#### READ (0x07) - Lecture d'un registre

**Requête (7 bytes) :**
```
AA 07 01 [Addr_L] [Addr_H] [CRC_L] [CRC_H]
```

**Exemple - Lire registre 0x0001 :**
```
AA 07 01 01 00 [CRC_L] [CRC_H]
```

**Réponse (7 bytes) :**
```
AA 07 02 [Value_L] [Value_H] [CRC_L] [CRC_H]
```

#### WRITE (0x0D) - Écriture d'un registre

**Requête (9 bytes) :**
```
AA 0D 04 [Addr_L] [Addr_H] [Value_L] [Value_H] [CRC_L] [CRC_H]
```

**Exemple - Écrire 4200 (0x1068) à registre 0x0010 :**
```
AA 0D 04 10 00 68 10 [CRC_L] [CRC_H]
```

**Réponse ACK (5 bytes) :**
```
AA 01 00 [CRC_L] [CRC_H]
```

**Réponse NACK (6 bytes) :**
```
AA 81 01 [Error_Code] [CRC_L] [CRC_H]
```

#### RESTART (0x0D) - Redémarrage du TinyBMS

**Requête spéciale (9 bytes) :**
```
AA 0D 04 86 00 5A A5 [CRC_L] [CRC_H]
```

- Adresse : `0x0086` (registre de contrôle système)
- Valeur magique : `0xA55A`

**Réponse :** ACK ou NACK (comme WRITE)

### Calcul CRC16

Algorithme Modbus (polynôme 0xA001, init 0xFFFF) :

```javascript
function crc16(buffer) {
  let crc = 0xffff;
  for (let i = 0; i < buffer.length; i += 1) {
    crc ^= buffer[i];
    for (let bit = 0; bit < 8; bit += 1) {
      if (crc & 0x0001) {
        crc = ((crc >> 1) ^ 0xa001) & 0xffff;
      } else {
        crc = (crc >> 1) & 0xffff;
      }
    }
  }
  return crc & 0xffff;
}
```

**Important :** Le CRC couvre **tout** sauf lui-même (bytes 0 à N+2).

### Extraction de trame

```javascript
function extractFrame(buffer) {
  if (!buffer || buffer.length === 0) {
    return { frame: null, buffer: Buffer.alloc(0) };
  }

  let working = buffer;
  const preambleIndex = working.indexOf(0xaa);
  if (preambleIndex === -1) {
    return { frame: null, buffer: Buffer.alloc(0) };
  }

  if (preambleIndex > 0) {
    working = working.slice(preambleIndex);  // Skip garbage
  }

  if (working.length < 5) {
    return { frame: null, buffer: working };  // Attendre plus de données
  }

  const payloadLength = working[2];
  const frameLength = 3 + payloadLength + 2;
  if (working.length < frameLength) {
    return { frame: null, buffer: working };
  }

  const frame = working.slice(0, frameLength);
  const expected = frame.readUInt16LE(frameLength - 2);
  const computed = crc16(frame.subarray(0, frameLength - 2));
  if (expected !== computed) {
    return extractFrame(working.slice(1));  // CRC invalide, skip 1 byte
  }

  return { frame, buffer: working.slice(frameLength) };
}
```

**Stratégie de récupération :**
- Si CRC invalide, skip 1 byte et re-scan pour `0xAA`
- Continue jusqu'à trouver une trame valide ou buffer vide

---

## Flux de communication

### Scénario 1 : Lecture de tous les registres

```
Client                 Server (Express)        Serial (TinyBmsSerial)      TinyBMS
  │                           │                         │                      │
  ├─ GET /api/registers ─────►│                         │                      │
  │                           ├─ ensureConnected() ──►  │                      │
  │                           ├─ readCatalogue(34) ───► │                      │
  │                           │                         ├─ mutex.runExclusive() │
  │                           │                         ├─ for descriptor #1    │
  │                           │                         ├─ _prepareTransaction()│
  │                           │                         ├─ flush buffers ───────►│
  │                           │                         ├─ buildReadFrame(0x01) │
  │                           │                         ├─ write ───────────────►│
  │                           │                         │                      ┌─┤
  │                           │                         │                      │ Process
  │                           │                         │                      └─┤
  │                           │                         │◄───── response ───────┤
  │                           │                         ├─ extractFrame()       │
  │                           │                         ├─ raw = 1234           │
  │                           │                         ├─ for descriptor #2    │
  │                           │                         ├─ ... (repeat 34x)     │
  │                           │◄─ results[] ────────────┤                      │
  │                           ├─ map to JSON            │                      │
  │◄─ 200 OK { registers } ──┤                         │                      │
```

**Temps typique :** ~2-3 secondes pour 34 registres (750ms timeout × échecs potentiels).

### Scénario 2 : Écriture d'un registre

```
Client                 Server                  Serial                     TinyBMS
  │                           │                         │                      │
  ├─ POST /api/registers ────►│                         │                      │
  │   { key: "cell_ovp",      │                         │                      │
  │     value: 4.2 }          │                         │                      │
  │                           ├─ findDescriptor()       │                      │
  │                           ├─ userToRaw(4.2)         │                      │
  │                           │   → raw = 4200          │                      │
  │                           ├─ writeRegister(addr, 4200) ───►                │
  │                           │                         ├─ mutex.runExclusive() │
  │                           │                         ├─ _prepareTransaction()│
  │                           │                         ├─ buildWriteFrame()    │
  │                           │                         ├─ write ───────────────►│
  │                           │                         │                      ┌─┤
  │                           │                         │                      │ Validate
  │                           │                         │                      │ Write NVS
  │                           │                         │                      └─┤
  │                           │                         │◄───── ACK (0x01) ─────┤
  │                           │                         ├─ readRegisterLocked() │
  │                           │                         ├─ buildReadFrame()     │
  │                           │                         ├─ write ───────────────►│
  │                           │                         │◄───── readback ───────┤
  │                           │                         ├─ raw = 4200           │
  │                           │◄─ readback ─────────────┤                      │
  │                           ├─ rawToUser(4200) → 4.2  │                      │
  │◄─ 200 OK { value: 4.2 } ─┤                         │                      │
```

**Temps typique :** ~100-150ms (write + readback).

### Scénario 3 : Erreur NACK

```
Client                 Server                  Serial                     TinyBMS
  │                           │                         │                      │
  ├─ POST /api/registers ────►│                         │                      │
  │   { key: "battery_sn",    │                         │                      │
  │     value: 12345 }        │                         │                      │
  │                           ├─ descriptor.access = "ro"                     │
  │                           ├─ userToRaw(12345) → 12345                     │
  │                           ├─ writeRegister(addr, 12345) ──►               │
  │                           │                         ├─ write ──────────────►│
  │                           │                         │                      ┌─┤
  │                           │                         │                      │ Reject
  │                           │                         │                      │ (read-only)
  │                           │                         │                      └─┤
  │                           │                         │◄─ NACK 0x81 code=0x01─┤
  │                           │                         ├─ throw Error("NACK 0x01")
  │                           │◄─ throw ────────────────┤                      │
  │◄─ 500 { error: "NACK" } ─┤                         │                      │
```

**Note :** Le client ne distingue pas entre erreur réseau, timeout, et NACK applicatif.

---

## Considérations de fiabilité

### 1. Buffer de réception non borné

**Problème identifié :**

```javascript
_handleData(data) {
  this._readBuffer = Buffer.concat([this._readBuffer, data]);
  // ...
}
```

Si le TinyBMS envoie des données corrompues en continu sans jamais former de trame valide avec préambule `0xAA`, le buffer peut croître indéfiniment jusqu'à épuisement de la RAM Node.js.

**Scénario d'échec :**
1. Câble UART mal blindé génère du bruit
2. Données corrompues arrivent en rafale (ex: `0xFF 0xFF 0xFF ...`)
3. `extractFrame()` ne trouve jamais `0xAA`, retourne `buffer` intact
4. `_readBuffer` grossit à chaque appel de `_handleData()`
5. Après ~1GB, Node.js crashe avec `FATAL ERROR: CALL_AND_RETRY_LAST Allocation failed`

**Solution suggérée :**

```javascript
_handleData(data) {
  this._readBuffer = Buffer.concat([this._readBuffer, data]);

  // Limite de sécurité : 4KB max (10x la taille max d'une trame)
  const MAX_BUFFER_SIZE = 4096;
  if (this._readBuffer.length > MAX_BUFFER_SIZE) {
    // Garde seulement les 4KB les plus récents
    this._readBuffer = this._readBuffer.slice(-MAX_BUFFER_SIZE);
  }

  let parsing = true;
  while (parsing) {
    const { frame, buffer } = extractFrame(this._readBuffer);
    this._readBuffer = buffer;
    if (!frame) {
      parsing = false;
      break;
    }
    this._dispatchFrame(frame);
  }
}
```

**Impact :** Aucun en usage normal (trames ~10 bytes). Protection contre cas pathologiques.

### 2. Timeouts sans cleanup

**Problème identifié :**

```javascript
_waitForFrame(matcher, timeoutMs) {
  let pendingRef = null;
  const promise = new Promise((resolve, reject) => {
    const pending = { matcher, resolve, reject, timeoutHandle: null };

    pending.timeoutHandle = setTimeout(() => {
      this._removePending(pending);
      reject(new Error('Timeout de réponse TinyBMS'));
    }, timeoutMs);

    pendingRef = pending;
    this._pending.push(pending);
  });

  promise.cancel = () => {
    if (pendingRef) {
      clearTimeout(pendingRef.timeoutHandle);
      this._removePending(pendingRef);
      pendingRef = null;
    }
  };

  return promise;
}
```

Si une promesse timeout puis qu'une trame arrive tardivement, `_dispatchFrame()` peut essayer de résoudre une promesse déjà rejetée.

**Scénario d'échec :**
1. `readRegister()` envoie requête à t=0
2. Timeout à t=750ms, promesse rejetée
3. TinyBMS répond à t=800ms (lent)
4. `_dispatchFrame()` trouve `pending.matcher()` mais la promesse est déjà settled
5. `pending.resolve(frame)` n'a aucun effet (promesse déjà rejetée)
6. La trame est "perdue" et pourrait corrompre une transaction suivante

**Solution suggérée :**

```javascript
_dispatchFrame(frame) {
  if (this._pending.length === 0) {
    return;
  }
  for (let i = 0; i < this._pending.length; i += 1) {
    const entry = this._pending[i];
    let matches = false;
    try {
      matches = Boolean(entry.matcher(frame));
    } catch (error) {
      entry.reject(error);
      this._pending.splice(i, 1);
      return;
    }
    if (matches) {
      clearTimeout(entry.timeoutHandle);  // ✅ Cleanup explicite
      this._pending.splice(i, 1);
      entry.resolve(frame);
      return;
    }
  }
}
```

**Impact :** Évite les fuites de timers actifs.

### 3. Validation des valeurs utilisateur

**Problème identifié :**

```javascript
export function userToRawValue(descriptor, userValue) {
  if (descriptor.valueClass === 'enum') {
    const candidate = Number.parseInt(userValue, 10);
    if (!descriptor.enum.some((entry) => entry.value === candidate)) {
      throw new Error(`Valeur ${userValue} non valide pour ${descriptor.key}`);
    }
    return candidate;
  }

  if (descriptor.scale === 0) {
    throw new Error(`Registre ${descriptor.key} possède une échelle invalide.`);
  }

  const requestedRaw = userValue / descriptor.scale;
  let alignedRaw = requestedRaw;
  const step = descriptor.stepRaw || 0;
  if (step > 0) {
    const base = descriptor.hasMin && typeof descriptor.minRaw === 'number' ? descriptor.minRaw : 0;
    const steps = Math.round((alignedRaw - base) / step);
    alignedRaw = base + steps * step;
  }

  if (descriptor.hasMin && typeof descriptor.minRaw === 'number' && alignedRaw < descriptor.minRaw) {
    throw new Error(`Valeur trop basse pour ${descriptor.key}`);
  }
  if (descriptor.hasMax && typeof descriptor.maxRaw === 'number' && alignedRaw > descriptor.maxRaw) {
    throw new Error(`Valeur trop élevée pour ${descriptor.key}`);
  }

  if (alignedRaw < 0 || alignedRaw > 0xffff) {
    throw new Error(`Valeur hors limites pour ${descriptor.key}`);
  }

  return Math.round(alignedRaw);
}
```

**Cas non géré :** `userValue` peut être `NaN`, `Infinity`, ou une chaîne non numérique.

**Scénario d'échec :**
```javascript
userToRawValue(descriptor, "abc")  // NaN / descriptor.scale = NaN
// Math.round(NaN) = NaN
// 0 <= NaN <= 0xffff → false
// Throw "Valeur hors limites"
```

**Solution suggérée :**

```javascript
export function userToRawValue(descriptor, userValue) {
  if (!descriptor) {
    throw new Error('Descripteur de registre introuvable.');
  }

  // ✅ Validation stricte du type
  if (typeof userValue !== 'number' || !Number.isFinite(userValue)) {
    throw new Error(`Valeur numérique invalide pour ${descriptor.key}: ${userValue}`);
  }

  if (descriptor.valueClass === 'enum') {
    const candidate = Math.round(userValue);
    if (!descriptor.enum.some((entry) => entry.value === candidate)) {
      throw new Error(`Valeur enum ${userValue} non valide pour ${descriptor.key}`);
    }
    return candidate;
  }

  if (descriptor.scale === 0) {
    throw new Error(`Registre ${descriptor.key} possède une échelle invalide.`);
  }

  // ... reste du code
}
```

### 4. Gestion de la déconnexion intempestive

**Problème identifié :**

Si le câble USB est débranché pendant une transaction, `serialport` émet un événement `error`, mais `_handleError()` ne fait que rejeter les promesses en attente. Le port reste dans un état incohérent.

**Solution suggérée :**

```javascript
_handleError(error) {
  // Rejeter toutes les promesses en attente
  this._rejectAll(error);

  // Marquer le port comme fermé
  if (this.port) {
    this.port.removeListener('data', this._onData);
    this.port.removeListener('error', this._onError);
    // Ne pas appeler close() ici (déjà fermé par l'OS)
    this.port = null;
    this._portInfo = null;
    this._readBuffer = Buffer.alloc(0);
  }
}
```

**Impact :** Évite les tentatives d'écriture sur un port fermé.

### 5. Retry automatique pour trames corrompues

**Amélioration suggérée :**

Actuellement, si une trame est corrompue (CRC invalide), `extractFrame()` skip 1 byte et continue. Mais si toute la trame est corrompue, cela peut prendre du temps.

**Solution :**

```javascript
async _readRegisterLocked(address, timeoutMs, retries = 2) {
  let lastError = null;

  for (let attempt = 0; attempt <= retries; attempt++) {
    try {
      await this._prepareTransaction();
      const request = buildReadFrame(address);
      const responsePromise = this._waitForFrame((frame) => frame[1] === 0x07, timeoutMs);

      try {
        await this._writeFrame(request);
      } catch (error) {
        if (typeof responsePromise.cancel === 'function') {
          responsePromise.cancel();
        }
        throw error;
      }

      const frame = await responsePromise;
      if (frame.length < 5 || frame[2] < 2) {
        throw new Error('Réponse TinyBMS invalide');
      }
      const raw = frame[3] | (frame[4] << 8);
      return raw;
    } catch (error) {
      lastError = error;
      if (attempt < retries) {
        // Attendre 100ms avant retry
        await new Promise(resolve => setTimeout(resolve, 100));
      }
    }
  }

  throw lastError;
}
```

**Impact :** Réduit les échecs sporadiques dus au bruit électrique.

---

## Guide de dépannage

### Problème : "Port série non connecté" (503)

**Causes possibles :**
1. `POST /api/connection/open` n'a pas été appelé
2. Le câble USB a été débranché
3. Le port série est utilisé par un autre processus

**Diagnostic :**
```bash
# Lister les ports disponibles
GET /api/ports

# Vérifier si macOS retient le port
lsof | grep tty.usbserial

# Tuer le processus si bloqué
kill -9 <PID>
```

**Solution :**
1. Vérifier que le câble est branché
2. Appeler `POST /api/connection/open` avec le bon `path`
3. Si persistant, redémarrer le serveur (`Ctrl+C`, `npm start`)

### Problème : "Timeout de réponse TinyBMS"

**Causes possibles :**
1. TinyBMS éteint ou en bootloop
2. Mauvais baudrate (doit être 115200)
3. Câble UART RX/TX inversés
4. TinyBMS occupe à traiter une commande longue

**Diagnostic :**
```bash
# Vérifier les logs série (mode debug)
screen /dev/tty.usbserial-A50285BI 115200

# Envoyer une trame manuelle (hex)
echo -ne '\xaa\x07\x01\x01\x00\x...\x...' > /dev/tty.usbserial-A50285BI
```

**Solution :**
1. Vérifier l'alimentation du TinyBMS (LED allumée)
2. Vérifier le branchement UART :
   - Mac RX → TinyBMS TX
   - Mac TX → TinyBMS RX
   - GND commun
3. Tester avec un baudrate différent (rare)
4. Redémarrer le TinyBMS (bouton reset)

### Problème : "TinyBMS NACK (code 0x01)"

**Code erreur :** Registre en lecture seule.

**Solution :**
1. Vérifier `descriptor.access` dans le catalogue
2. Ne pas tenter d'écrire les registres `"ro"` (read-only)

**Exemple :** `battery_voltage`, `cell_voltage_1`, etc.

### Problème : "TinyBMS NACK (code 0x02)"

**Code erreur :** Valeur hors limites.

**Solution :**
1. Vérifier `descriptor.minUser` et `descriptor.maxUser`
2. Respecter les contraintes métier (ex: OVP > UVP)

**Exemple :** `cell_overvoltage_protection` doit être > `cell_undervoltage_protection`.

### Problème : "Valeur trop basse/élevée pour X"

**Cause :** Validation côté client avant envoi série.

**Solution :**
1. Vérifier les limites dans le tableau UI
2. Ajuster la valeur dans l'intervalle `[min, max]`

### Problème : Interface web ne charge pas

**Causes possibles :**
1. Serveur pas démarré (`npm start`)
2. Port 5173 déjà utilisé
3. Fichiers `public/` manquants

**Diagnostic :**
```bash
# Vérifier si le serveur écoute
lsof -i :5173

# Tester l'API directement
curl http://localhost:5173/api/ports
```

**Solution :**
1. Démarrer le serveur : `npm start`
2. Changer le port : `MAC_LOCAL_PORT=8080 npm start`
3. Vérifier que `public/index.html` existe

### Problème : Registre lu mais valeur incorrecte

**Causes possibles :**
1. Mauvaise interprétation du scale
2. TinyBMS en cours de mise à jour

**Diagnostic :**
```javascript
// Vérifier le descripteur
const descriptor = findRegisterDescriptorByKey('battery_voltage');
console.log(descriptor.scale, descriptor.precision);

// Lire la valeur raw
const raw = await serial.readRegister(descriptor.address);
console.log('Raw:', raw, 'User:', rawToUserValue(descriptor, raw));
```

**Solution :**
1. Vérifier que le firmware TinyBMS est à jour
2. Re-générer le catalogue : `npm run refresh-registers`
3. Redémarrer le TinyBMS pour forcer NVS reload

### Problème : Lecture de catalogue très lente (>10 secondes)

**Cause :** Timeouts en cascade sur registres inaccessibles.

**Diagnostic :**
```javascript
// Activer les logs verbose
const results = await serial.readCatalogue(descriptors);
results.forEach(({ descriptor, raw }) => {
  console.log(`${descriptor.key}: ${raw} (${Date.now()})`);
});
```

**Solution :**
1. Filtrer par groupe : `GET /api/registers?group=battery`
2. Augmenter le timeout : Modifier `DEFAULT_TIMEOUT_MS` dans `serial.js`

---

## Limitations connues

### 1. Pas de retry automatique

Actuellement, si une transaction échoue (timeout, CRC), elle est immédiatement rejetée. L'utilisateur doit réessayer manuellement.

**Impact :** Peut nécessiter plusieurs tentatives en environnement bruité.

**Amélioration future :** Implémenter retry dans `_readRegisterLocked()` et `_writeRegisterLocked()`.

### 2. Pas de logging persistant

Les erreurs série sont loggées dans `stdout` mais pas sauvegardées.

**Impact :** Difficile de diagnostiquer les problèmes intermittents.

**Amélioration future :** Ajouter Winston ou Pino avec rotation de logs.

### 3. Pas de détection de version firmware

Le catalogue est statique (34 registres). Si le firmware TinyBMS change, le catalogue peut être obsolète.

**Impact :** Lectures/écritures sur mauvaises adresses possibles.

**Amélioration future :** Ajouter endpoint `/api/firmware/version` avec validation catalogue.

### 4. Pas de support multi-device

L'application ne peut gérer qu'un seul TinyBMS à la fois.

**Impact :** Pour tester plusieurs devices, il faut fermer/rouvrir le port.

**Amélioration future :** Ajouter gestion de pool de connexions série.

### 5. Interface web non responsive

Le tableau des registres n'est pas optimisé pour mobile/tablette.

**Impact :** Difficile d'utiliser sur iPhone/iPad.

**Amélioration future :** CSS responsive avec Bootstrap ou Tailwind.

### 6. Pas de graphiques temps réel

Les valeurs sont affichées sous forme de tableau statique.

**Impact :** Pas de visualisation des tendances (voltage, current).

**Amélioration future :** Ajouter Chart.js pour graphs live via polling.

### 7. Pas de validation CSRF

L'API REST n'a pas de protection contre requêtes forgées.

**Impact :** Non pertinent pour usage local (localhost).

**Note :** Cet outil est prévu pour dépannage occasionnel, pas pour production exposée.

### 8. Pas de gestion des droits utilisateur

Tous les registres en `"rw"` sont modifiables par n'importe qui.

**Impact :** Risque de modification accidentelle de paramètres critiques.

**Amélioration future :** Ajouter confirmation modale pour registres critiques (OVP, UVP, balance threshold).

---

## Annexes

### A. Structure du catalogue

Exemple de descripteur complet :

```json
{
  "address": 16,
  "addressHex": "0x0010",
  "key": "cell_overvoltage_protection",
  "label": "Seuil de surcharge cellule",
  "unit": "V",
  "group": "protection",
  "comment": "Déclenche alarme si une cellule dépasse cette tension",
  "type": "uint16",
  "access": "rw",
  "scale": 0.001,
  "precision": 3,
  "hasMin": true,
  "hasMax": true,
  "minRaw": 3000,
  "maxRaw": 4500,
  "stepRaw": 1,
  "defaultRaw": 4200,
  "valueClass": "numeric",
  "enum": [],
  "minUser": 3.0,
  "maxUser": 4.5,
  "defaultUser": 4.2,
  "stepUser": 0.001
}
```

### B. Commandes NPM utiles

```bash
# Démarrer le serveur
npm start

# Lister le catalogue sans connexion série
npm run list-registers

# Régénérer le catalogue JSON depuis firmware
npm run refresh-registers

# Lancer les tests (si implémentés)
npm test
```

### C. Variables d'environnement

```bash
# Changer le port du serveur (défaut: 5173)
MAC_LOCAL_PORT=8080 npm start

# Activer les logs détaillés (Node.js)
DEBUG=* npm start
```

### D. Dépendances

- **express** (^4.19.2) : Serveur HTTP + middleware JSON
- **serialport** (^12.0.0) : Communication USB-UART

**Compatibilité :** Node.js ≥ 18 (pour `import`/`export` natif).

---

## Conclusion

Cette interface Mac locale est un **outil de dépannage robuste** pour la configuration initiale du TinyBMS. Elle implémente correctement le protocole binaire avec CRC16, mutex pour transactions, et conversion scale/raw/user.

**Points forts :**
- ✅ Parsing automatique du catalogue firmware
- ✅ Validation des valeurs utilisateur (min/max/step/enum)
- ✅ Readback automatique après écriture
- ✅ Gestion des erreurs NACK avec codes explicites
- ✅ Interface web interactive avec filtrage par groupe

**Points d'amélioration suggérés (non critiques) :**
- 🔄 Limite sur buffer de réception (4KB)
- 🔄 Cleanup explicite des timeouts
- 🔄 Retry automatique (2x) pour robustesse
- 🔄 Logging persistant avec Winston
- 🔄 Tests unitaires pour `registers.js` et `serial.js`

**Usage recommandé :**
1. Brancher TinyBMS via USB-UART
2. Ouvrir `http://localhost:5173`
3. Sélectionner port série → **Se connecter**
4. Filtrer par groupe (ex: "protection") → **Charger les valeurs**
5. Modifier les registres `"rw"` → **Sauvegarder**
6. Optionnel : **Redémarrer** pour appliquer

**Durée de vie typique d'une session :** 5-10 minutes (configuration initiale ou diagnostic rapide).

---

**Document généré le 2025-11-15**
**Basé sur l'analyse statique du code mac-local v0.1.0**
