MANUEL_ATSE_RS485.md

# CHINT ATSE model NXZBN-63S/2DT 32A 

**Protocole de communication des interrupteurs automatiques de transfert série (ATS)**

**Éditeur :** Zhejiang CHINT Electric Co., Ltd.  
**Version :** V1.1

---

## Table des matières

- [Préface](#préface)
- [1. Domaine d'application](#1-domaine-dapplication)
- [2. Aperçu du protocole](#2-aperçu-du-protocole)
  - [2.1 Définitions et terminologie](#21-définitions-et-terminologie)
  - [2.2 Couche physique](#22-couche-physique)
  - [2.3 Couche liaison de données](#23-couche-liaison-de-données)
  - [2.4 Couche application](#24-couche-application)
- [3. Codes de fonction](#3-codes-de-fonction)
  - [3.1 Lecture des registres de données (03H)](#31-lecture-des-registres-de-données-03h)
  - [3.2 Écriture d'un registre de données (06H)](#32-écriture-dun-registre-de-données-06h)
  - [3.3 Gestion des erreurs](#33-gestion-des-erreurs)
- [4. Registres de données de communication](#4-registres-de-données-de-communication)
  - [4.1 Ensemble de données](#41-ensemble-de-données)
  - [4.2 Registre de fréquence d'alimentation](#42-registre-de-fréquence-dalimentation)
  - [4.3 Registre d'état de l'alimentation](#43-registre-détat-de-lalimentation)
  - [4.4 Registre d'état du commutateur](#44-registre-détat-du-commutateur)
  - [4.5 Registre de commande](#45-registre-de-commande)
  - [4.6 Registre de transfert forcé](#46-registre-de-transfert-forcé)
- [5. Méthodes de connexion de communication](#5-méthodes-de-connexion-de-communication)
  - [5.1 Connexion matérielle](#51-connexion-matérielle)
  - [5.2 Configuration des paramètres de communication](#52-configuration-des-paramètres-de-communication)
  - [5.3 Dépannage des anomalies de communication](#53-dépannage-des-anomalies-de-communication)
- [Annexe A – Principe de génération CRC-16](#annexe-a--principe-de-génération-crc-16)
- [Annexe B – Exemple d'application de communication NXZ(H)MN](#annexe-b--exemple-dapplication-de-communication-nxzhmn)

---

## Préface

Ce manuel d'utilisation est proposé par **Zhejiang CHINT Electric Co., Ltd.**  
**Version actuelle : V1.1**  
Ce manuel représente uniquement le contenu de cette version. En cas de mise à jour, aucune notification ne sera effectuée. Veuillez consulter la **dernière version** sur le site officiel de la société.

---

## 1. Domaine d'application

Ce manuel spécifie la connexion physique, la liaison de communication et les spécifications techniques entre l'interrupteur automatique de transfert et la station maître.

**Produits concernés :** interrupteurs automatiques de transfert de type T (avec communication) des séries suivantes :

- NXZMN, NXZHMN
- NZ5M, NZ5HM
- NXZBN, NXZHBN
- NZ5B, NZ5HB

---

## 2. Aperçu du protocole

Le protocole utilisé est **Modbus-RTU** (simplification du modèle OSI – seules les couches 1, 2 et 7 sont utilisées).

### 2.1 Définitions et terminologie

- **OSI** : Open Systems Interconnection
- **Couche physique** : transmission physique des bits
- **Couche liaison de données** : transmission fiable entre nœuds adjacents
- **Couche application** : fonctions métier
- **Trame** : unité de transmission de données

### 2.2 Couche physique

| Paramètre                  | Valeur                                      | Remarques                     |
|----------------------------|---------------------------------------------|-------------------------------|
| Mode de communication      | RS485 semi-duplex                           |                               |
| Adresse de communication   | 1 ~ 247                                     | **Par défaut : 3**            |
| Débit en bauds             | 4.8 / 9.6 / 19.2 / 38.4 kbps                | **Par défaut : 9.6 kbps**     |
| Distance max               | ≤ 1000 m                                    | À faible débit                |
| Câble recommandé           | Paire torsadée blindée Classe A             |                               |

### 2.3 Couche liaison de données

- **Mode** : maître-esclave (maître interroge, esclave répond)
- **Protocole** : **Modbus-RTU**
- Format série : 1 bit start – 8 bits données – 1 bit parité – 1 bit stop

**Format trame RTU :**

T3.5 Adresse Fonction Données CRC (16 bits) T3.5 (8 bits) (8 bits) (n×8 bits)
- **T3.5** = silence ≥ 3,5 caractères (dépend du débit)
- Intervalle minimum entre trames : **≥ 200 ms** recommandé

### 2.4 Couche application

Structure générale d’une requête 03H (exemple) :

Adresse | Fonction | Adr. début (2 octets) | Nb registres (2 octets) | CRC
---

## 3. Codes de fonction

| Code     | Définition                        | Action                              |
|----------|-----------------------------------|-------------------------------------|
| **03H**  | Lecture de registres              | Lit 1 ou plusieurs registres        |
| **06H**  | Écriture d’un seul registre       | Écrit dans un registre              |
| **83H**  | Erreur lecture                    | Réponse esclave en cas d’erreur 03H |
| **86H**  | Erreur écriture                   | Réponse esclave en cas d’erreur 06H |

### 3.1 Lecture des registres de données (03H)

**Exemple – Lecture tensions A,B,C alimentation I**

Maître → 03 03 00 06 00 03 E4 28 Esclave → 03 03 06 00 DC 00 E6 00 F0 07 85 → Ua=220 V, Ub=230 V, Uc=240 V
### 3.2 Écriture d’un registre de données (06H)

**Exemple – Seuil sous-tension U1 à 160 V (registre 0x2065)**

Maître → 03 06 20 65 00 A0 93 8F Esclave → 03 06 20 65 00 A0 93 8F (écho)
### 3.3 Gestion des erreurs

Codes d’exception :

- 01 : Données illégales
- 02 : Adresse de registre incorrecte
- 03 : Erreur de longueur de données

---

## 4. Registres de données de communication

### 4.1 Ensemble de données

| N° | Paramètre                              | Type   | Unité | Accès | Adresse   | Description / Plage                              | Produits concernés     |
|----|----------------------------------------|--------|-------|-------|-----------|--------------------------------------------------|------------------------|
| 1  | Tension phase A NL1 (alim I)           | UINT   | V     | R     | 0x0006    | Tension phase A alimentation I                   | Tous                   |
| 2  | Tension phase B NL2                    | UINT   | V     | R     | 0x0007    |                                                  | Tous                   |
| …  | …                                      | …      | …     | …     | …         | …                                                | …                      |
| 19 | Adresse Modbus                         | UINT   | —     | R/W   | 0x0100    | 1 ~ 247, défaut 3                                | Tous                   |
| 20 | Débit en bauds                         | UINT   | kbps  | R/W   | 0x0101    | 0=4.8, 1=9.6, 2=19.2, 3=38.4 kbps, défaut 1     | Tous                   |
| 21 | État alimentation I & II               | UINT   | —     | R     | 0x004F    | Voir détail §4.3                                 | Tous                   |
| 22 | État du commutateur                    | UINT   | —     | R     | 0x0050    | Voir détail §4.4                                 | Tous                   |
| 23 | Seuil sous-tension U1 alim I           | UINT   | V     | R/W   | 0x2065    | 150 ~ 200 V                                      | NXZ(H)MN / NZ5(H)M     |
| 31 | Sélection du mode                      | UINT   | —     | R/W   | 0x206D    | 0=Auto-retour, 1=Auto sans retour… (voir doc)   | NXZ(H)MN / NZ5(H)M     |
| 32 | Commande transfert forcé               | UINT   | —     | W     | 0x2700    | Voir §4.6                                        | Tous                   |
| 33 | Commande de contrôle                   | UINT   | —     | W     | 0x2800    | Voir §4.5                                        | Tous                   |

*(tableau complet tronqué ici – à compléter selon besoin)*

### 4.5 Registre de commande (0x2800) – Écriture seule

Exemples utiles :

- Restaurer paramètres par défaut : `03 06 28 00 00 02 00 49`
- Effacer historique : `03 06 28 00 00 01 40 48`
- Entrer contrôle à distance : `03 06 28 00 00 04 80 4B`
- Quitter contrôle à distance : `03 06 28 00 00 00 81 88`

### 4.6 Registre de transfert forcé (0x2700)

| Valeur   | Action                              | Condition                                 |
|----------|-------------------------------------|-------------------------------------------|
| 0x0000   | Forcer vers alimentation I          | Tension normale + contrôle à distance     |
| 0x00AA   | Forcer vers alimentation II         | Tension normale + contrôle à distance     |
| 0x00FF   | Forcer double ouverture (position 0)| Contrôle à distance actif                 |

---

## 5. Méthodes de connexion de communication

### 5.2 Configuration des paramètres de communication

| N° | Paramètre              | Valeur par défaut       | Plage / Choix                              | Registre   |
|----|------------------------|--------------------------|--------------------------------------------|------------|
| 1  | Adresse                | 3                        | 1 ~ 247                                    | 0x0100     |
| 2  | Débit en bauds         | 9.6 kbps                 | 4.8 / 9.6 / 19.2 / 38.4 kbps               | 0x0101     |
| 3  | Parité                 | Even (paire)             | None / Odd / Even                          | 0x000E     |
| 4  | Bits de données        | 8                        | fixe                                       | —          |
| 5  | Bits d’arrêt           | 1                        | fixe                                       | —          |
| 6  | Contrôle de flux       | Aucun                    | fixe                                       | —          |

---

## Annexe A – Principe de génération CRC-16

Algorithme standard Modbus CRC-16 :

1. Initialiser registre CRC à **0xFFFF**
2. Pour chaque octet de la trame (sauf start/stop/parité) :
   - XOR avec octet bas du registre
   - Pour 8 fois :
     - Décaler droite (>> 1), MSB ← 0
     - Si LSB était 1 → XOR avec **0xA001**
3. À la fin : **octet bas** envoyé en premier, puis octet haut

---

## Annexe B – Exemple d'application NXZ(H)MN  
Contrôle à distance → double ouverture

1. Câblage RS485 (A+, B-, GND)
2. Paramètres par défaut : 9600,E,8,1 – Adresse 3
3. Entrer contrôle distant :  
   `03 06 28 00 00 04 80 4B`
4. Forcer double ouverture :  
   `03 06 27 00 00 FF C2 DC`

---

Bonne publication sur GitHub !

Vous pouvez copier-coller ce contenu dans un fichier `.md`.  
Si vous voulez une version encore plus détaillée (tous les registres, tous les bits des états, etc.), dites-le-moi et je complète les tableaux manquants.
Bon courage pour votre dépôt !
