# 
---

```markdown
# CHINT ATS - Supervision Modbus RTU CRC-16/MODBUS

Application web de supervision pour les automatismes de transfert de source (ATS) CHINT séries NXZ(H)MN, NZ5(H)M, NXZ(H)BN, NZ5(H)B.

# Model : NXZBN-63 S/2 D T C 32A

## Table des matières

1. [Présentation](#présentation)
2. [Prérequis](#prérequis)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [Utilisation](#utilisation)
6. [Protocole Modbus RTU](#protocole-modbus-rtu)
7. [Registres Modbus](#registres-modbus)
8. [Commandes disponibles](#commandes-disponibles)
9. [Dépannage](#dépannage)
10. [Fichiers générés](#fichiers-générés)

---

## Présentation

Cette application permet de superviser et contrôler à distance un ATS CHINT via une interface web moderne. Elle communique en Modbus RTU sur liaison RS485.

### Fonctionnalités

- **Supervision temps réel** : tensions, fréquences, état des sources, état du commutateur
- **Commandes à distance** : activation/désactivation de la télécommande, forçage des positions
- **Console Modbus** : envoi de trames hexadécimales personnalisées
- **Journalisation** : logs détaillés des communications
- **Détection automatique du modèle** : adaptation de l'interface (MN ou BN)

### Modèles supportés

| Série | Réglages | Afficheur |
|-------|----------|-----------|
| NXZ(H)MN / NZ5(H)M | ✅ Complets | ✅ |
| NXZ(H)BN / NZ5(H)B | ❌ Limités | ❌ |

---

## Prérequis

### Matériel

- ATS CHINT série compatible (adresse Modbus configurée)
- Convertisseur USB-RS485 (chipset FTDI, CH340, ou équivalent)
- Câble RS485 (blindé recommandé pour longue distance)

### Logiciel

- Windows / Linux / macOS
- [Rust](https://www.rust-lang.org/) (pour la compilation)
- Navigateur web (Chrome, Edge, Brave, Firefox)

### Paramètres de communication

| Paramètre | Valeur |
|-----------|--------|
| Protocole | Modbus RTU |
| Baud rate | 9600 |
| Data bits | 8 |
| Parité | Even |
| Stop bits | 1 |
| Adresse | 6 (par défaut après configuration) |

---

## Installation

### 1. Installation de Rust

```bash
# Windows (via winget)
winget install Rustlang.Rust

# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Création du projet

```bash
cargo new chint_ats
cd chint_ats
```

### 3. Configuration des dépendances

Ajoutez ceci dans `Cargo.toml` :

```toml
[package]
name = "chint_ats"
version = "0.1.0"
edition = "2021"

[dependencies]
actix-web = "4"
actix-files = "0.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serialport = "4"
chrono = "0.4"
```

### 4. Copie des fichiers

- `src/main.rs` → contenu du code Rust fourni
- `index.html` → interface web (à la racine du projet)

### 5. Compilation et exécution

```bash
cargo build --release
cargo run
```

L'application est accessible sur : http://localhost:5000

---

## Configuration

### Modification de l'adresse Modbus

Pour changer l'adresse de 3 à 6 (trame avec CRC) :

```
06 06 01 00 00 06 89 FE
```

### Configuration des paramètres série

Si nécessaire, modifiez ces valeurs dans `main.rs` :

```rust
let port_name = "COM5";     // Port série
let addr = 6;               // Adresse Modbus
```

---

## Utilisation

### Interface web

| Section | Description |
|---------|-------------|
| **Bandeau supérieur** | Source active, mode, télécommande, défauts, commutations |
| **Onduleur / Réseau** | Tensions, fréquences, maxima, état des phases |
| **Temporisations** | T1, T2, T3, T4 |
| **Configuration** | Mode opératoire, paramètres Modbus |
| **Statistiques** | Compteurs de commutations, runtime, version |
| **Commandes** | Activation télécommande, forçages |
| **Console Modbus** | Envoi de trames hexadécimales personnalisées |

### Commandes disponibles

| Bouton | Action | Trame Modbus |
|--------|--------|--------------|
| 📡 Activer télécommande | Active le mode distant | `06 06 28 00 00 04 49 14` |
| 🔒 Désactiver télécommande | Désactive le mode distant | `06 06 28 00 00 00 48 D4` |
| ⏹️ Forcer double déclenché | Ouvre les deux sources | `06 06 27 00 00 FF 83 91` |
| 🔋 Forcer Onduleur | Ferme la source I | `06 06 27 00 00 00 43 D1` |
| ⚡ Forcer Réseau | Ferme la source II | `06 06 27 00 00 AA C3 98` |

### Console Modbus

Permet d'envoyer n'importe quelle trame Modbus RTU en hexadécimal.

Exemple de trame pour lire l'état des sources (0x004F) :

```
06 03 00 4F 00 01 B4 6A
```

Format : `[Adresse] [Fonction] [Registre haut] [Registre bas] [Nb registres haut] [Nb registres bas] [CRC]`

---

## Protocole Modbus RTU

### Format de trame

| Champ | Taille | Description |
|-------|--------|-------------|
| Adresse | 1 octet | 1-247 (6 par défaut) |
| Fonction | 1 octet | 03 (lecture), 06 (écriture) |
| Données | N octets | Registre et valeurs |
| CRC | 2 octets | Contrôle d'erreur (little-endian) |

### Temps inter-trame

Le respect du silence de 3.5 caractères (≈ 3.6 ms à 9600 bauds) est requis. L'application gère automatiquement ce délai.

### Calcul du CRC16

Algorithme CRC-16 Modbus (polynôme 0xA001) :

```rust
fn calculate_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}
```

---

## Registres Modbus

### Lectures principales (adresse 6)

| Registre | Description | Format | Exemple |
|----------|-------------|--------|---------|
| 0x0006 | Tension phase A Source I | UINT (V) | `06 03 00 06 00 01 25 F4` |
| 0x0007 | Tension phase B Source I | UINT (V) | `06 03 00 07 00 01 74 34` |
| 0x0008 | Tension phase C Source I | UINT (V) | `06 03 00 08 00 01 35 F4` |
| 0x0009 | Tension phase A Source II | UINT (V) | `06 03 00 09 00 01 64 34` |
| 0x000A | Tension phase B Source II | UINT (V) | `06 03 00 0A 00 01 25 F5` |
| 0x000B | Tension phase C Source II | UINT (V) | `06 03 00 0B 00 01 74 35` |
| 0x000C | Version logicielle | UINT (x/100) | `06 03 00 0C 00 01 45 F5` |
| 0x000D | Fréquence | UINT (Hz) | `06 03 00 0D 00 01 14 35` |
| 0x000E | Parité Modbus | 0=None,1=Odd,2=Even | `06 03 00 0E 00 01 C5 F4` |
| 0x004F | État des sources | Bitmap | `06 03 00 4F 00 01 75 F4` |
| 0x0050 | État du commutateur | Bitmap | `06 03 00 50 00 01 44 BE` |
| 0x0015 | Compteur commutations I | UINT | `06 03 00 15 00 01 94 79` |
| 0x0016 | Compteur commutations II | UINT | `06 03 00 16 00 01 64 79` |
| 0x0017 | Temps de fonctionnement | UINT (h) | `06 03 00 17 00 01 35 B9` |
| 0x0100 | Adresse Modbus | UINT | `06 03 01 00 00 01 85 F5` |
| 0x0101 | Baud rate | 0=4800,1=9600,2=19200,3=38400 | `06 03 01 01 00 01 D4 35` |

### Écritures

| Registre | Description | Valeur | Trame |
|----------|-------------|--------|-------|
| 0x2700 | Forçage position | 0x0000 (Source I) | `06 06 27 00 00 00 43 D1` |
| 0x2700 | Forçage position | 0x00AA (Source II) | `06 06 27 00 00 AA C3 98` |
| 0x2700 | Forçage position | 0x00FF (Double) | `06 06 27 00 00 FF 83 91` |
| 0x2800 | Télécommande | 0x0004 (Activer) | `06 06 28 00 00 04 49 14` |
| 0x2800 | Télécommande | 0x0000 (Désactiver) | `06 06 28 00 00 00 48 D4` |

### Réglages (modèles MN uniquement)

| Registre | Description | Plage |
|----------|-------------|-------|
| 0x2065 | Sous-tension Source I | 150-200 V |
| 0x2066 | Sous-tension Source II | 150-200 V |
| 0x2067 | Surtension Source I | 240-290 V |
| 0x2068 | Surtension Source II | 240-290 V |
| 0x2069 | T1 (transfert) | 0-180 s |
| 0x206A | T2 (retour) | 0-180 s |
| 0x206B | T3 (démarrage générateur) | 0-180 s |
| 0x206C | T4 (arrêt générateur) | 0-180 s |
| 0x206D | Mode | 0-5 |

### Décodage du registre 0x004F (état des sources)

| Bits | Source | Valeur |
|------|--------|--------|
| 0-1 | Source II phase A | 00=Normal, 01=Sous-tension, 10=Surtension |
| 2-3 | Source II phase B | idem |
| 4-5 | Source II phase C | idem |
| 8-9 | Source I phase A | idem |
| 10-11 | Source I phase B | idem |
| 12-13 | Source I phase C | idem |

### Décodage du registre 0x0050 (état du commutateur)

| Bit | Description |
|-----|-------------|
| 0 | 1=Mode Auto, 0=Manuel |
| 1 | 1=Source I fermée |
| 2 | 1=Source II fermée |
| 3 | 1=Position double |
| 4-6 | Code défaut |
| 8 | 1=Télécommande activée |
| 12 | 1=Générateur démarré |

---

## Dépannage

### Erreur "Timeout" ou "Pas de réponse"

1. Vérifier les connexions RS485 (A, B, GND)
2. Vérifier l'alimentation de l'ATS
3. Confirmer les paramètres série : 9600, 8, Even, 1
4. Vérifier l'adresse Modbus (6 par défaut)
5. Activer le mode debug dans l'interface

### Erreur "Adresse registre invalide"

Ce message apparaît sur les modèles BN qui ne supportent pas les registres de réglage. L'application détecte automatiquement le modèle et adapte l'interface.

### Problèmes de connexion série

- Vérifier que le convertisseur USB-RS485 est bien sur le port COM5
- Aucun autre programme ne doit utiliser le port
- Sous Windows, vérifier le gestionnaire de périphériques

### Activation du mode debug

Cliquer sur le bouton "Debug ON" dans l'interface. Les logs sont écrits dans `modbus_debug.log`.

---

## Fichiers générés

| Fichier | Description |
|---------|-------------|
| `modbus_debug.log` | Logs détaillés des échanges (debug activé) |
| `modbus_commands.log` | Historique des commandes envoyées via la console |
| `chint_ats.exe` | Exécutable (après compilation release) |

---

## Architecture technique

### Backend (Rust)

- **Framework** : Actix-web
- **Série** : serialport-rs
- **Logs** : chrono
- **API REST** : endpoints pour lecture/écriture Modbus

### Frontend (HTML/CSS/JS)

- **Pure HTML/CSS** sans frameworks
- **Fetch API** pour les appels REST
- **Auto-refresh** toutes les 5 secondes
- **Console Modbus** interactive

### Points d'API

| Endpoint | Méthode | Description |
|----------|---------|-------------|
| `/` | GET | Interface web |
| `/api/read_all` | GET | Lecture tous les registres |
| `/api/remote_on` | GET | Activer télécommande |
| `/api/remote_off` | GET | Désactiver télécommande |
| `/api/force_double` | GET | Forçage double |
| `/api/force_source1` | GET | Forçage Source I |
| `/api/force_source2` | GET | Forçage Source II |
| `/api/send_raw` | POST | Envoi trame brute |
| `/api/debug_on` | GET | Activer logs debug |
| `/api/debug_off` | GET | Désactiver logs debug |

---

## Sécurité

- L'application est locale (`localhost:5000`) et n'est pas exposée au réseau
- Aucune authentification requise (utilisation en local uniquement)
- Les logs contiennent les trames Modbus (sensibles)

---

## Limitations

- Les modèles BN ne supportent pas la modification des seuils
- La fréquence n'est disponible que sur les modèles MN
- T3/T4 uniquement sur modèles MN
- La télécommande doit être activée avant les forçages

---

## Support

Pour toute question ou problème, consulter la documentation CHINT :
- Manuel utilisateur : NXZ(H)MN、NZ5(H)M、NXZ(H)BN、NZ5(H)B 系列自动转换开关电器通讯协议

---


---

## Commande :

1. ** 06 06 28 00 00 04 80 1E		# Activation mode télécommande (0x2800) >> Screen Blinking**
2. ** 06 06 27 00 00 00 37 4B			Left Position 	# Forçage position source I (0x2700 = 0x0000)**
3. ** 06 06 27 00 00 FF C2 89			Middle Position # Forçage position double déclenché (0x2700 = 0x00FF)**
4. ** 06 06 27 00 00 AA 02 B6			Right position	# Forçage position source II (0x2700 = 0x00AA)**
5. ** 06 06 28 00 00 00 81 DD		# Sortie mode télécommande**

---

## Version

**v1.0** - Mars 2026
```

---

Ce README.md contient :

1. **Toutes les informations d'installation et configuration**
2. **La liste complète des registres Modbus avec leurs trames**
3. **Les commandes disponibles et leurs trames**
4. **Les codes de décodage pour les registres d'état**
5. **Un guide de dépannage**
6. **L'architecture technique**
