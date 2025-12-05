réaliser interface web moderne et autonome (serveur Node.js sur Mac Mini connecté au TinyBMS via USB-UART), avec trois parties principales :

Backend (Node.js) : Pour gérer la connexion série (USB-UART), implémenter le protocole Modbus/UART du TinyBMS et servir l'application web.

Frontend (HTML/JS/CSS) : Une interface "Dark Mode" responsive avec des jauges et des formulaires, inspirée de "Battery Insider".

Logique de Communication : Le pont entre le navigateur (WebSocket) et le port série.

Voici la structure complète et les fichiers prêts à l'emploi.

Structure du projet

Créez un dossier tinybms-web et organisez-le ainsi :

tinybms-web/
├── package.json
├── server.js
├── tinybms.js
└── public/
    ├── index.html
    ├── styles.css
    └── app.js

1. Installation des dépendances

Ouvrez votre terminal dans le dossier tinybms-web et lancez :

npm init -y
npm install express socket.io serialport

2. Le Backend (Node.js)

tinybms.js

Ce fichier contient la logique spécifique au protocole binaire du TinyBMS (CRC, lecture/écriture des registres) basée sur les spécifications techniques.

server.js

Le serveur Express qui gère les WebSockets et polling du BMS.

3. Le Frontend (Interface Graphique)

public/index.html

La structure de la page avec les onglets et le layout.

public/styles.css

Le design "Dark Mode" moderne et épuré.

public/app.js

La logique client pour l'affichage dynamique et la gestion des WebSockets.

Comment lancer le projet sur le Mac Mini

Connecter le matériel :

Branchez l'adaptateur USB-UART au port USB du Mac Mini.

Connectez les fils (TX, RX, GND) au connecteur du TinyBMS (Attention : TX mac -> RX bms et RX mac -> TX bms).

Identifier le port série : Dans le terminal, tapez ls /dev/tty.usb*. Notez le chemin (ex: /dev/tty.usbserial-1420).

Lancer le serveur :

cd tinybms-web
node server.js

4. Accéder à l'interface : Ouvrez votre navigateur sur http://localhost:3000. Sélectionnez le port USB dans la liste déroulante, choisissez le protocole (ASCII recommandé) et cliquez sur "Connect".

### ⚠️ Configuration du Protocole (IMPORTANT)

Le TinyBMS supporte deux protocoles de communication via le registre 343 :
- **MODBUS** (valeur 0) : Protocole par défaut du TinyBMS
- **ASCII** (valeur 1) : Protocole implémenté dans cette interface

**Problème :** Si le TinyBMS utilise le protocole MODBUS par défaut et que notre interface utilise le protocole ASCII, la communication échouera.

**Solution :** L'interface configure **automatiquement** le protocole lors de la connexion. Vous pouvez choisir le protocole dans le sélecteur :
- **ASCII (recommandé)** : Protocole utilisé par cette interface
- **MODBUS** : Pour tester ou déboguer

**Fonctionnement :**
1. Lors de la connexion, l'interface écrit automatiquement la valeur du protocole sélectionné dans le registre 343
2. Le TinyBMS applique le changement et communique ensuite avec le protocole choisi
3. Si la configuration échoue (ex: timeout), l'interface continue en supposant que le BMS est déjà sur le bon protocole

Fonctionnalités clés incluses :

Design "Battery Insider" like : Onglets Dashboard, Cells, Settings.

Monitoring temps réel : Tensions globales, courant, SOC, températures.

Détail des cellules : Visualisation des 16 cellules avec min/max/delta.

Dark Mode : CSS optimisé pour une utilisation agréable.

Modulaire : La logique de communication tinybms.js est séparée pour être facilement étendue (ex: ajouter l'écriture des paramètres).


### Instructions rapides pour l'installation

1.  Copiez ce contenu dans un fichier nommé `package.json` à la racine de votre dossier `tinybms-web`.
2.  Ouvrez votre terminal dans ce dossier.
3.  Lancez la commande pour tout installer automatiquement :
    ```bash
    npm install
    ```
4.  Pour lancer le serveur :
    ```bash
    npm start
    ```

**Note :** J'ai ajouté `nodemon` dans les `devDependencies` et un script `npm run dev`. C'est très utile pendant le développement car cela redémarre automatiquement le serveur Node.js à chaque fois que vous modifiez un fichier (comme `server.js` ou `tinybms.js`), vous évitant de devoir couper et relancer le serveur manuellement.    
