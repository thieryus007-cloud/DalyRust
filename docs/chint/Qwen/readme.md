# Documentation Technique Complète – Commutateur de Transfert Automatique (ATS) CHINT
## Séries NXZ(H)MN, NZ5(H)M, NXZ(H)BN, NZ5(H)B – Protocole Modbus-RTU

**Version :** V1.1 (Février 2025)  
**Adresse Modbus configurée :** `0x06` (décimal : 6)  
**Langue :** Français  
**Format :** Markdown (exportable vers Word/PDF)

---

## 📋 Table des Matières

1. [Introduction et Domaine d'Application](#1-introduction-et-domaine-dapplication)
2. [Aperçu du Protocole Modbus-RTU](#2-apercu-du-protocole-modbus-rtu)
3. [Couche Physique](#3-couche-physique)
4. [Couche Liaison de Données](#4-couche-liaison-de-données)
5. [Couche Application – Format des Trames](#5-couche-application--format-des-trames)
6. [Codes Fonction (Function Codes)](#6-codes-fonction-function-codes)
7. [Registres de Communication – Jeu de Données Complet](#7-registres-de-communication--jeu-de-données-complet)
8. [Commandes de Contrôle et Transfert Forcé](#8-commandes-de-contrôle-et-transfert-forcé)
9. [Méthodes de Connexion et Configuration](#9-méthodes-de-connexion-et-configuration)
10. [Dépannage des Communications](#10-dépannage-des-communications)
11. [Annexe A – Principe de Génération CRC-16](#annexe-a--principe-de-génération-crc-16)
12. [Annexe B – Bibliothèque Complète de Trames de Test (Adresse 0x06)](#annexe-b--bibliothèque-complète-de-trames-de-test-adresse-0x06)

---

## 1. Introduction et Domaine d'Application

Ce manuel définit les spécifications techniques pour la connexion physique, la liaison de communication et les normes applicatives entre les commutateurs de transfert automatique (ATS) CHINT des séries suivantes et un système maître (PLC, SCADA, PC) :

| Série | Modèles concernés |
|-------|-----------------|
| NXZMN / NXZHMN | Type T avec communication |
| NZ5M / NZ5HM | Type T avec communication |
| NXZBN / NXZHBN | Type T avec communication |
| NZ5B / NZ5HB | Type T avec communication |

> ⚠️ **Remarque** : Ce document suppose que l'adresse Modbus de l'ATS a été configurée sur **`0x06`** (au lieu de la valeur par défaut `0x03`). Toutes les trames d'exemple ci-dessous utilisent cette adresse.

---

## 2. Aperçu du Protocole Modbus-RTU

Le protocole Modbus est un protocole de bus industriel basé sur le modèle OSI à 7 couches, mais n'utilisant que 3 couches simplifiées :

| Couche | Rôle |
|--------|------|
| **Physique** | Fournit le lien physique pour la transmission transparente (RS485) |
| **Liaison de données** | Assure une transmission fiable entre nœuds adjacents (Modbus-RTU) |
| **Application** | Réalise l'échange de données et les fonctions métier |

### 2.1 Définitions et Termes

| Terme | Définition |
|-------|-----------|
| **Modèle OSI** | Standard ISO (1984) pour l'interconnexion de systèmes hétérogènes |
| **Trame (Frame)** | Structure d'information prédéfinie composée de bits/champs pour le transport des données |
| **Maître/Esclave** | Architecture où le maître initie les requêtes et l'esclave répond |

---

## 3. Couche Physique

| Paramètre | Valeur / Plage | Remarques |
|-----------|---------------|-----------|
| **Mode de communication** | RS485 | Half-duplex (semi-duplex) |
| **Adresse de communication** | 1 ~ 247 | **Défaut : 3** → **Configuré : 6 (0x06)** |
| **Débit (Baud Rate)** | 4.8 / 9.6 / 19.2 / 38.4 kbps | **Défaut : 9.6 kbps** |
| **Distance max.** | ≤ 1000 m | À bas débit |
| **Média** | Paire torsadée blindée (Cat. A) | Recommandé |

### Câblage RS485

```
[Maître/PC] <===> [Convertisseur USB-RS485] <===> [ATS]
                          |
                    A+ ───┼─── A+ (ATS)
                    B- ───┼─── B- (ATS)
                    GND ──┴─── GND (ATS)
```

> ✅ **Bonnes pratiques** :
> - Utiliser un câble blindé avec mise à la terre unique
> - Terminaison 120Ω aux extrémités du bus si distance > 300 m
> - Éviter les boucles de masse

---

## 4. Couche Liaison de Données

### 4.1 Format de Transmission Série (1 trame)

| Start | D0 | D1 | D2 | D3 | D4 | D5 | D6 | D7 | Parité | Stop |
|-------|----|----|----|----|----|----|----|----|--------|------|
| 1 bit | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 1 bit | 1 bit |

→ **Format : 8N1 ou 8E1 ou 8O1** (selon configuration parité)

### 4.2 Format de Paquet Modbus-RTU

| Début | Adresse | Fonction | Données | CRC | Fin |
|-------|---------|----------|---------|-----|-----|
| T3.5 | 8 bits | 8 bits | N × 8 bits | 16 bits | T3.5 |

- **T3.5** : Silence minimum de 3.5 temps de caractère avant/après trame
- **Intervalle entre paquets** : ≥ 200 ms recommandé
- **Période d'envoi** : > 100 ms pour éviter les pertes de paquets

---

## 5. Couche Application – Format des Trames

### 5.1 Structure Générale d'une Trame Modbus-RTU

| Champ | Taille | Description |
|-------|--------|-------------|
| **Adresse** | 1 octet | Adresse de l'esclave (1-247) → **0x06** dans ce document |
| **Fonction** | 1 octet | Code opération (03=lecture, 06=écriture, 83/86=erreur) |
| **Adresse registre** | 2 octets | Adresse de départ (MSB puis LSB) |
| **Nombre/Donnée** | 2 octets | Nombre de registres (lecture) ou valeur à écrire |
| **CRC** | 2 octets | Contrôle d'erreur (LSB puis MSB) |

### 5.2 Exemple de Trame de Lecture (Fonction 0x03)

**Requête Maître → Esclave (adresse 0x06)** : Lire 3 registres à partir de 0x0006

```
[06] [03] [00] [06] [00] [03] [CRC_L] [CRC_H]
```

**Réponse Esclave → Maître** :

```
[06] [03] [06] [D1_H] [D1_L] [D2_H] [D2_L] [D3_H] [D3_L] [CRC_L] [CRC_H]
```

> 📌 **Ordre des octets** : 
> - Adresses et données : **Big-Endian** (MSB en premier)
> - CRC : **Little-Endian** (LSB en premier)

---

## 6. Codes Fonction (Function Codes)

| Code Hex | Nom | Comportement | Réponse Erreur |
|----------|-----|-------------|----------------|
| **0x03** | Lire registres | Lecture d'un ou plusieurs registres | **0x83** |
| **0x06** | Écrire registre | Écriture d'un seul registre | **0x86** |
| **0x83** | Erreur lecture | Retourne si requête 0x03 invalide | – |
| **0x86** | Erreur écriture | Retourne si requête 0x06 invalide | – |

### 6.1 Lecture de Registres (0x03) – Exemple avec Adresse 0x06

**Objectif** : Lire les tensions phase A, B, C de la Source I (registres 0x0006 à 0x0008)

**Requête** :
```
06 03 00 06 00 03 E4 28
```
| Octet | Valeur | Signification |
|-------|--------|--------------|
| 1 | 0x06 | Adresse esclave |
| 2 | 0x03 | Fonction : lecture |
| 3-4 | 0x0006 | Adresse registre de départ |
| 5-6 | 0x0003 | Nombre de registres à lire |
| 7-8 | 0xE428 | CRC-16 |

**Réponse attendue** (si Uan=220V, Ubn=230V, Ucn=240V) :
```
06 03 06 00 DC 00 E6 00 F0 08 75
```
| Octets | Valeur | Interprétation |
|--------|--------|----------------|
| 1-2 | 06 03 | Adresse + fonction |
| 3 | 06 | Nombre d'octets de données suivants |
| 4-5 | 00 DC | 220 décimal → Uan = 220 V |
| 6-7 | 00 E6 | 230 décimal → Ubn = 230 V |
| 8-9 | 00 F0 | 240 décimal → Ucn = 240 V |
| 10-11 | 08 75 | CRC-16 |

### 6.2 Écriture d'un Registre (0x06) – Exemple avec Adresse 0x06

**Objectif** : Modifier le seuil de sous-tension Source I (registre 0x2065) à 160 V (0x00A0)

**Requête** :
```
06 06 20 65 00 A0 93 8F
```

**Réponse (écho)** :
```
06 06 20 65 00 A0 93 8F
```

### 6.3 Gestion des Erreurs

Si la requête est invalide, l'esclave répond avec le bit 7 du code fonction positionné à 1 :

| Requête | Réponse Erreur | Code Exception | Signification |
|---------|----------------|----------------|--------------|
| 06 **03** ... | 06 **83** 02 ... | 0x02 | Adresse registre invalide |
| 06 **06** ... | 06 **86** 01 ... | 0x01 | Donnée illégale |
| 06 **03** ... | 06 **83** 03 ... | 0x03 | Nombre de registres invalide |

**Exemple de réponse d'erreur** :
```
06 83 02 C1 31
```
→ Fonction 0x83, code erreur 0x02 (adresse registre incorrecte)

---

## 7. Registres de Communication – Jeu de Données Complet

> 🔹 **Légende** :  
> - **R** = Lecture seule | **R/W** = Lecture/Écriture | **W** = Écriture seule  
> - **UINT** = Entier non signé 16 bits (2 octets)  
> - Adresses en hexadécimal

### 7.1 Registres de Mesure et Statuts (Lecture)

| # | Paramètre | Type | Unité | Accès | Adresse | Description | Produits |
|---|-----------|------|-------|-------|---------|-------------|----------|
| 1 | Tension NL1 (Phase A Source I) | UINT | V | R | `0x0006` | Tension phase A, Source I | Tous |
| 2 | Tension NL2 (Phase B Source I) | UINT | V | R | `0x0007` | Tension phase B, Source I | Tous |
| 3 | Tension NL3 (Phase C Source I) | UINT | V | R | `0x0008` | Tension phase C, Source I | Tous |
| 4 | Tension RL1 (Phase A Source II) | UINT | V | R | `0x0009` | Tension phase A, Source II | Tous |
| 5 | Tension RL2 (Phase B Source II) | UINT | V | R | `0x000A` | Tension phase B, Source II | Tous |
| 6 | Tension RL3 (Phase C Source II) | UINT | V | R | `0x000B` | Tension phase C, Source II | Tous |
| 7 | Version logicielle | UINT | – | R | `0x000C` | Version firmware | Tous |
| 8 | Fréquence réseau | UINT | Hz | R | `0x000D` | Voir §7.2 | NXZ(H)MN / NZ5(H)M |
| 9 | Parité Modbus | UINT | – | R/W | `0x000E` | 0=None, 1=Odd, 2=Even (défaut) | Tous |
| 10 | MAX-N-A (Tension max A Source I) | UINT | V | R | `0x000F` | Historique tension max | Tous |
| 11 | MAX-N-B | UINT | V | R | `0x0010` | Idem phase B Source I | Tous |
| 12 | MAX-N-C | UINT | V | R | `0x0011` | Idem phase C Source I | Tous |
| 13 | MAX-R-A | UINT | V | R | `0x0012` | Tension max Source II, phase A | Tous |
| 14 | MAX-R-B | UINT | V | R | `0x0013` | Idem phase B Source II | Tous |
| 15 | MAX-R-C | UINT | V | R | `0x0014` | Idem phase C Source II | Tous |
| 16 | Compteur commutations Source I | UINT | – | R | `0x0015` | Nombre de transferts | Tous |
| 17 | Compteur commutations Source II | UINT | – | R | `0x0016` | Nombre de transferts | Tous |
| 18 | Temps de fonctionnement total | UINT | h | R | `0x0017` | Cumul heures (RAZ à l'extinction) | Tous |
| 19 | **Adresse Modbus** | UINT | – | **R/W** | `0x0100` | Plage 1-247, défaut 3 | Tous |
| 20 | **Débit Modbus** | UINT | kbps | **R/W** | `0x0101` | 0=4.8, 1=9.6, 2=19.2, 3=38.4 | Tous |
| 21 | Statut Sources I/II | UINT | – | R | `0x004F` | Voir §7.3 | Tous |
| 22 | Statut Commutateur | UINT | – | R | `0x0050` | Voir §7.4 | Tous |

### 7.2 Registre de Fréquence (0x000D) – Format Bit

```
Adresse 0x000D (Lecture seule) – 16 bits
┌─────────────────┬─────────────────┐
│ Bits 15-8       │ Bits 7-0        │
├─────────────────┼─────────────────┤
│ Bits 14-8 :     │ Bits 7-0 :      │
│ Fréquence Src I │ Fréquence Src II│
│ (Hz, UINT)      │ (Hz, UINT)      │
└─────────────────┴─────────────────┘
```

**Exemple** : Src I = 50 Hz (0x32), Src II = 0 Hz (0x00)  
→ Valeur registre = `0x3200`  
→ Trame réponse : `06 03 02 32 00 D4 E4`

### 7.3 Registre de Statut des Sources (0x004F) – Format Bit

| Bit | Champ | Valeurs | Signification |
|-----|-------|---------|--------------|
| 0-1 | Src II – Phase A | 00=Normal, 01=Sous-tension, 10=Surtension | Statut tension |
| 2-3 | Src II – Phase B | Idem | Statut tension |
| 4-5 | Src II – Phase C | Idem | Statut tension |
| 6-7 | Src I – Phase A | Idem | Statut tension |
| 8-9 | Src I – Phase B | Idem | Statut tension |
| 10-11| Src I – Phase C | Idem | Statut tension |
| 12-15| Réservé | – | – |

**Exemple** : Src I normale (00), Src II en sous-tension sur les 3 phases (01)  
→ Bits 0-5 = `01 01 01` = 0x15, Bits 6-11 = `00 00 00` = 0x00  
→ Valeur = `0x0015`  
→ Trame réponse : `06 03 02 00 15 0F 17`

### 7.4 Registre de Statut du Commutateur (0x0050) – Format Bit

| Bit | Champ | Valeurs | Signification |
|-----|-------|---------|--------------|
| 0 | Mode | 0=Manuel, 1=Automatique | État de fonctionnement |
| 1 | Générateur | 0=Arrêt, 1=Marche | Statut groupe électrogène |
| 2 | Position intermédiaire | 0=Non, 1=Oui | Double position ouverte |
| 3 | Position Source II | 0=Ouvert, 1=Fermé | Contacteur Source II |
| 4 | Position Source I | 0=Ouvert, 1=Fermé | Contacteur Source I |
| 5-7 | Type de défaut | 000=Aucun, 001=Feu, 010=Moteur, 011=Saut Src I, 100=Saut Src II, 101=Signal fermeture, 110=Phase Src I, 111=Phase Src II | Code défaut |
| 8 | Contrôle à distance | 0=Non, 1=Oui | Mode télécommande actif |
| 9-15 | Réservé | – | – |

**Exemple** : Source I ouverte, Source II fermée, générateur arrêté, mode auto  
→ Bits 0=1, 1=0, 2=0, 3=1, 4=0 → `0001 0001` = 0x11  
→ Trame réponse : `06 03 02 00 11 01 88`

### 7.5 Registres de Configuration (Lecture/Écriture) – Séries MN/M

| # | Paramètre | Adresse | Plage | Défaut | Unité | Description |
|---|-----------|---------|-------|--------|-------|-------------|
| 23 | Seuil sous-tension Src I (U1) | `0x2065` | 150-200 | – | V | Déclenchement si < seuil |
| 24 | Seuil sous-tension Src II (U2) | `0x2066` | 150-200 | – | V | Idem Source II |
| 25 | Seuil surtension Src I (U3) | `0x2067` | 240-290 | – | V | Déclenchement si > seuil |
| 26 | Seuil surtension Src II (U4) | `0x2068` | 240-290 | – | V | Idem Source II |
| 27 | Délai de transfert T1 | `0x2069` | 0-180 | 5 | s | Temporisation avant transfert |
| 28 | Délai de retour T2 | `0x206A` | 0-180 | 5 | s | Temporisation avant retour |
| 29 | Démarrage générateur T3 | `0x206B` | 0-180 | 5 | s | Délai avant start générateur |
| 30 | Arrêt générateur T4 | `0x206C` | 0-180 | 5 | s | Délai avant stop générateur |
| 31 | Mode de fonctionnement | `0x206D` | 0-5 | 0 | – | Voir tableau ci-dessous |

#### Modes de Fonctionnement (Registre 0x206D)

| Valeur | Mode | Description |
|--------|------|-------------|
| 0 | Auto-transfert / Auto-retour | Retour automatique à la source principale quand normale |
| 1 | Auto-transfert / Non-auto-retour | Reste sur source secondaire même si principale revient |
| 2 | Secours mutuel | Les deux sources sont équivalentes |
| 3 | Auto-transfert / Auto-retour (Générateur) | Avec gestion groupe électrogène |
| 4 | Auto-transfert / Non-auto-retour (Générateur) | Idem + pas de retour auto |
| 5 | Secours mutuel (Générateur) | Avec gestion générateur en secours |

> ⚠️ **Séries BN/B** : Registres 0x2069 et 0x206A en **lecture seule**, plage 0-30 s.  
> Modes disponibles : 1=Auto/Auto, 2=Auto/Non-Auto, 3=Secours mutuel, 4=Test.

---

## 8. Commandes de Contrôle et Transfert Forcé

### 8.1 Registre de Commande (0x2800) – Écriture Seule

| Bit | Fonction | Valeur 0 | Valeur 1 | Action |
|-----|----------|----------|----------|--------|
| 0 | Effacer historique | Non | Oui | RAZ compteurs et tensions max |
| 1 | Restaurer paramètres | Non | Oui | Retour aux valeurs d'usine (adresse, baudrate, etc.) |
| 2 | Contrôle à distance | Non | Oui | Active le mode télécommande (prérequis pour transfert forcé) |
| 3 | Réservé | – | – | – |
| 4 | Effacer défaut Feu | Non | Oui | Acquitter l'alarme incendie |
| 5 | Effacer défaut Moteur | Non | Oui | Acquitter le timeout moteur |
| 6-15 | Réservés | – | – | – |

**Exemples de trames (adresse 0x06)** :

| Action | Trame Hexadécimale | Description |
|--------|-------------------|-------------|
| Activer contrôle à distance | `06 06 28 00 00 04 80 4B` | Bit 2 = 1 |
| Restaurer paramètres usine | `06 06 28 00 00 02 00 49` | Bit 1 = 1 |
| Effacer historique | `06 06 28 00 00 01 40 48` | Bit 0 = 1 |
| Désactiver contrôle à distance | `06 06 28 00 00 00 81 88` | Tous bits = 0 |

### 8.2 Registre de Transfert Forcé (0x2700) – Écriture Seule

> ⚠️ **Prérequis** : Le mode **contrôle à distance** doit être actif (bit 2 de 0x2800 = 1)  
> ⚠️ La source cible doit être **normale** (tension dans les seuils) pour que le transfert s'exécute

| Valeur | Action | Condition |
|--------|--------|-----------|
| `0x0000` | Transférer vers **Source I** | Source I normale |
| `0x00AA` | Transférer vers **Source II** | Source II normale |
| `0x00FF` | Transférer vers **Double Ouvert** (position médiane) | Aucune condition de tension |

**Exemples de trames (adresse 0x06)** :

| Action | Trame Hexadécimale | CRC |
|--------|-------------------|-----|
| Transfert vers Source I | `06 06 27 00 00 00 82 9C` | 0x9C82 |
| Transfert vers Source II | `06 06 27 00 00 AA 02 E3` | 0xE302 |
| Transfert vers Double Ouvert | `06 06 27 00 00 FF C2 DC` | 0xDCC2 |

> 🔁 **Séquence typique de télécommande** :
> 1. Activer mode distant : `06 06 28 00 00 04 80 4B`
> 2. Envoyer commande de transfert : `06 06 27 00 00 AA 02 E3`
> 3. Vérifier statut via registre 0x0050
> 4. Désactiver mode distant : `06 06 28 00 00 00 81 88`

---

## 9. Méthodes de Connexion et Configuration

### 9.1 Paramètres de Communication à Vérifier

| Paramètre | Valeur par défaut | Valeur configurée (exemple) | Registre | Méthode de modification |
|-----------|------------------|----------------------------|----------|------------------------|
| Adresse Modbus | 3 | **6** | `0x0100` | Menu A-A (MN/M) ou Modbus écriture (BN/B) |
| Débit (Baud Rate) | 9.6 kbps | 9.6 kbps | `0x0101` | Menu A-C (MN/M) ou Modbus |
| Parité | Even (2) | Even (2) | `0x000E` | Menu A-E (MN/M) ou Modbus |
| Bits de données | 8 | 8 (fixe) | – | Non modifiable |
| Bits d'arrêt | 1 | 1 (fixe) | – | Non modifiable |
| Contrôle de flux | Aucun | Aucun (fixe) | – | Non modifiable |

### 9.2 Configuration selon la Série

| Série | Écran intégré | Modification paramètres |
|-------|--------------|------------------------|
| **NXZ(H)MN / NZ5(H)M** | ✅ Oui | Via menu local (touches +/-) **ou** Modbus |
| **NXZ(H)BN / NZ5(H)B** | ❌ Non | Uniquement via Modbus **ou** signal feu >60 s pour réinitialisation |

> 🔧 **Réinitialisation usine (BN/B)** : Appliquer un signal feu (DC24V) pendant ≥ 60 secondes → tous les paramètres reviennent aux valeurs par défaut (adresse=3, baud=9.6k, parité=Even).

---

## 10. Dépannage des Communications

### Checklist de Diagnostic

| Étape | Vérification | Action corrective |
|-------|-------------|------------------|
| 1 | Câblage RS485 (A+/B-/GND) | Resserrer, inverser A/B si nécessaire, vérifier continuité |
| 2 | Alimentation contrôleur | Vérifier tension d'alimentation (24VDC ou 230VAC selon modèle) |
| 3 | Paramètres Modbus (maître/esclave) | Uniformiser : adresse, baudrate, parité, format 8N1/8E1 |
| 4 | Convertisseur USB-RS485 | Tester avec un autre adaptateur, vérifier drivers |
| 5 | Adresse Modbus | Confirmer que l'ATS répond à `0x06` (et non 0x03 par défaut) |
| 6 | Trame de test basique | Envoyer `06 03 00 06 00 01 [CRC]` pour lire tension phase A |
| 7 | Logs d'erreur | Analyser codes 0x83/0x86 pour identifier type d'erreur |

### Outils Recommandés

- **Modbus Poll** (Windows) ou **QModMaster** (multi-plateforme) pour tests manuels
- **Wireshark** avec plugin Modbus pour analyse avancée
- **Oscilloscope** pour vérifier l'intégrité du signal RS485

---

## Annexe A – Principe de Génération CRC-16

### Algorithme Modbus CRC-16 (Polynôme 0xA001)

```python
def calculate_crc16(data: bytes) -> int:
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 0x0001:
                crc = (crc >> 1) ^ 0xA001
            else:
                crc >>= 1
    return crc  # Retourne CRC en format Little-Endian (LSB first)
```

### Étapes de Calcul

1. Initialiser registre CRC à `0xFFFF`
2. Pour chaque octet de la trame (adresse → données) :
   - XOR avec l'octet courant
   - Décaler à droite de 1 bit, 8 fois
   - Si bit de poids faible = 1 : XOR avec `0xA001`
3. Le résultat final est le CRC (envoyer LSB puis MSB)

### Vérification en Ligne

- [Modbus CRC Calculator](https://www.lammertbies.nl/comm/info/crc-calculation.html)
- Entrer la trame sans CRC → comparer avec valeur calculée

---

## Annexe B – Bibliothèque Complète de Trames de Test (Adresse 0x06)

> 📦 **Format** : Hexadécimal, espace entre octets  
> 🔁 **CRC** : Calculé avec polynôme 0xA001, LSB en premier  
> ✅ **Réponse attendue** : Indiquée entre crochets

### 🔹 Trames de Lecture (Fonction 0x03)

| Objectif | Adresse Registre | Nb Registres | Trame Requête | CRC | Réponse Attendue (exemple) |
|----------|-----------------|--------------|---------------|-----|---------------------------|
| Lire tension Phase A Source I | `0x0006` | 1 | `06 03 00 06 00 01` | `A5 88` | `[06 03 02 00 DC A1 7F]` (220 V) |
| Lire tensions 3 phases Source I | `0x0006` | 3 | `06 03 00 06 00 03` | `E4 28` | `[06 03 06 00 DC 00 E6 00 F0 08 75]` |
| Lire version logiciel | `0x000C` | 1 | `06 03 00 0C 00 01` | `F4 38` | `[06 03 02 01 02 XX XX]` (v1.2) |
| Lire fréquence | `0x000D` | 1 | `06 03 00 0D 00 01` | `14 2B` | `[06 03 02 32 00 D4 E4]` (50 Hz / 0 Hz) |
| Lire statut sources | `0x004F` | 1 | `06 03 00 4F 00 01` | `B4 3F` | `[06 03 02 00 15 0F 17]` |
| Lire statut commutateur | `0x0050` | 1 | `06 03 00 50 00 01` | `85 F9` | `[06 03 02 00 11 01 88]` |
| Lire adresse Modbus | `0x0100` | 1 | `06 03 01 00 00 01` | `C5 38` | `[06 03 02 00 06 XX XX]` (confirmant adresse=6) |
| Lire débit Modbus | `0x0101` | 1 | `06 03 01 01 00 01` | `94 39` | `[06 03 02 00 01 XX XX]` (9.6 kbps) |

### 🔹 Trames d'Écriture (Fonction 0x06)

| Objectif | Adresse Registre | Valeur à Écrire | Trame Requête | CRC | Réponse (Écho) |
|----------|-----------------|-----------------|---------------|-----|----------------|
| Modifier adresse Modbus → 6 | `0x0100` | `0x0006` | `06 06 01 00 00 06` | `49 00` | `06 06 01 00 00 06 49 00` |
| Modifier débit → 19.2 kbps | `0x0101` | `0x0002` | `06 06 01 01 00 02` | `B8 00` | `06 06 01 01 00 02 B8 00` |
| Modifier parité → None | `0x000E` | `0x0000` | `06 06 00 0E 00 00` | `89 04` | `06 06 00 0E 00 00 89 04` |
| Seuil sous-tension Src I = 170 V | `0x2065` | `0x00AA` | `06 06 20 65 00 AA` | `D3 8F` | `06 06 20 65 00 AA D3 8F` |
| Seuil surtension Src II = 260 V | `0x2068` | `0x0104` | `06 06 20 68 01 04` | `42 1C` | `06 06 20 68 01 04 42 1C` |
| Délai T1 = 10 s | `0x2069` | `0x000A` | `06 06 20 69 00 0A` | `72 5D` | `06 06 20 69 00 0A 72 5D` |
| Mode = Secours mutuel (2) | `0x206D` | `0x0002` | `06 06 20 6D 00 02` | `63 4C` | `06 06 20 6D 00 02 63 4C` |

### 🔹 Commandes de Contrôle (Registre 0x2800)

| Action | Valeur | Trame | CRC | Réponse |
|--------|--------|-------|-----|---------|
| Activer contrôle à distance | `0x0004` | `06 06 28 00 00 04` | `80 4B` | `06 06 28 00 00 04 80 4B` |
| Restaurer paramètres usine | `0x0002` | `06 06 28 00 00 02` | `00 49` | `06 06 28 00 00 02 00 49` |
| Effacer historique | `0x0001` | `06 06 28 00 00 01` | `40 48` | `06 06 28 00 00 01 40 48` |
| Acquitter défaut Feu | `0x0010` | `06 06 28 00 00 10` | `C1 45` | `06 06 28 00 00 10 C1 45` |
| Acquitter défaut Moteur | `0x0020` | `06 06 28 00 00 20` | `02 44` | `06 06 28 00 00 20 02 44` |
| Désactiver contrôle distant | `0x0000` | `06 06 28 00 00 00` | `81 88` | `06 06 28 00 00 00 81 88` |

### 🔹 Transfert Forcé (Registre 0x2700 – Mode Distant Requis)

| Action | Valeur | Trame | CRC | Condition |
|--------|--------|-------|-----|-----------|
| Transfert vers Source I | `0x0000` | `06 06 27 00 00 00` | `82 9C` | Source I normale |
| Transfert vers Source II | `0x00AA` | `06 06 27 00 00 AA` | `02 E3` | Source II normale |
| Transfert vers Double Ouvert | `0x00FF` | `06 06 27 00 00 FF` | `C2 DC` | Aucune |

### 🔹 Trames de Test d'Erreurs (Pour Validation Robustesse)

| Scénario | Trame Invalide | Réponse Attendue | Code Erreur |
|----------|----------------|------------------|-------------|
| Adresse registre inexistante | `06 03 FF FF 00 01 [CRC]` | `06 83 02 [CRC]` | 0x02 = Adresse invalide |
| Nombre de registres > limite | `06 03 00 06 00 64 [CRC]` | `06 83 03 [CRC]` | 0x03 = Longueur invalide |
| Valeur hors plage (ex: U1=100V) | `06 06 20 65 00 64 [CRC]` | `06 86 01 [CRC]` | 0x01 = Donnée illégale |
| Adresse esclave incorrecte | `07 03 00 06 00 01 [CRC]` | *Aucune réponse* | – |

---

## 📥 Export et Utilisation

✅ **Pour exporter en Word/PDF** :
1. Copier ce document Markdown
2. Coller dans [Typora](https://typora.io/) ou [VS Code](https://code.visualstudio.com/) avec extension Markdown
3. Exporter via `File → Export → PDF` ou `Word (.docx)`

✅ **Pour tester les trames** :
- Utiliser **Modbus Poll** :  
  `Connection → Connect` → Port COM, 9600-8-E-1 → Adresse slave = 6  
  `Setup → Read Holding Registers` → Adresse = 6, Quantity = 1

✅ **Script Python de test rapide** :

```python
import serial
import struct

def crc16(data: bytes) -> bytes:
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 0x0001:
                crc = (crc >> 1) ^ 0xA001
            else:
                crc >>= 1
    return struct.pack("<H", crc)  # Little-Endian

def send_modbus(port, frame_hex):
    frame = bytes.fromhex(frame_hex)
    crc = crc16(frame)
    ser = serial.Serial(port, 9600, timeout=1)
    ser.write(frame + crc)
    response = ser.read(256)
    ser.close()
    return response.hex()

# Exemple : Lire tension Phase A Source I (adresse 0x06)
print(send_modbus("COM3", "06 03 00 06 00 01"))
```

---

> ℹ️ **Mentions légales** :  
> Ce document est une traduction technique basée sur les manuels officiels CHINT (V1.1, Février 2025).  
> En cas de divergence, le document original en chinois fait foi.  
> CHINT se réserve le droit de modifier les spécifications sans préavis.

🔚 **Fin du document** – Version 1.0 – Adresse Modbus : `0x06`
