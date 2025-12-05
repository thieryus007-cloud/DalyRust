# Guide de Démarrage Rapide - Interface Web TinyBMS

## 🚀 Lancement Rapide

### Option 1: Script automatique (recommandé)
```bash
cd Exemple/TinyBMS-web
./start_server.sh
```

### Option 2: Commande npm
```bash
cd Exemple/TinyBMS-web
npm start
```

## 📋 Prérequis

- **Node.js** (v14 ou supérieur) ✅ Installé: v22.21.1
- **npm** ✅ Installé: v10.9.4
- **Port USB** disponible pour le TinyBMS (ou mode simulation)

## 🔌 Connexion au TinyBMS

### Mode Matériel Réel

1. **Connecter le TinyBMS**
   - Branchez le câble USB-UART à votre ordinateur
   - Connectez TX, RX, GND au TinyBMS
   - Le port apparaîtra comme `/dev/ttyUSB0` ou `/dev/ttyACM0`

2. **Vérifier le port**
   ```bash
   ls /dev/tty{USB,ACM}*
   ```

3. **Dans l'interface web**
   - Ouvrez http://localhost:3000
   - Sélectionnez le port dans la liste déroulante
   - Cliquez sur "Connect"

### Mode Simulation

Si vous n'avez pas de TinyBMS connecté:

1. Ouvrez http://localhost:3000
2. Sélectionnez **"SIMULATION"** dans la liste
3. Cliquez sur "Connect"
4. Vous verrez des données simulées

## 🎯 Fonctionnalités

### Onglet Dashboard
- **Tensions**: Pack, Min/Max cellules
- **Courant**: Charge/Décharge en temps réel
- **État**: SOC, SOH, Température
- **Statut**: État du BMS, balancing

### Onglet Cells
- Visualisation des 16 cellules individuelles
- Tensions min/max/delta
- Indicateurs visuels

### Onglet Settings
Modifier les paramètres du TinyBMS (regroupés par catégorie):
- **Battery**: Capacité, tensions, SOC
- **Safety**: Protections courant/tension/température
- **Balance**: Seuils de balancing
- **Hardware**: Configuration système

## 🔧 Dépannage

### Le serveur ne démarre pas
```bash
# Réinstaller les dépendances
cd Exemple/TinyBMS-web
rm -rf node_modules
npm install
```

### Aucun port USB détecté
- Vérifiez que le câble USB est branché
- Testez avec: `ls -la /dev/tty*`
- Utilisez le mode SIMULATION pour tester l'interface

### Erreur de connexion au TinyBMS
- Vérifiez que le baudrate est correct (115200)
- Vérifiez les connexions TX/RX (croisées)
- Testez d'abord avec le script Python: `python3 TinyBMS_test.py`

### Le navigateur ne se connecte pas
- Vérifiez que le serveur est démarré (message dans le terminal)
- Essayez http://127.0.0.1:3000
- Vérifiez qu'aucun pare-feu ne bloque le port 3000

## 📊 Protocole Modbus

L'interface utilise le protocole Modbus RTU avec:
- **Adresse slave**: 0xAA
- **Baudrate**: 115200
- **Format adresses**: Big Endian (MSB, LSB)
- **CRC**: Polynomial 0xA001

## 🆘 Support

Pour les problèmes ou questions:
1. Consultez le README principal: `../README.md`
2. Vérifiez les logs du serveur dans le terminal
3. Testez d'abord avec le script Python de test

## 🔄 Arrêt du Serveur

Utilisez `Ctrl+C` dans le terminal pour arrêter proprement le serveur.
