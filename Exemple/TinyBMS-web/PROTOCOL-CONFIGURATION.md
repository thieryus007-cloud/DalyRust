# Configuration du Protocole TinyBMS

## Vue d'ensemble

Ce document explique comment l'interface TinyBMS-web configure automatiquement le protocole de communication avec le TinyBMS.

## Problématique

Le TinyBMS supporte deux protocoles de communication différents, sélectionnables via le **registre 343** :

| Valeur | Protocole | Description |
|--------|-----------|-------------|
| 0 | MODBUS | Protocole par défaut du TinyBMS au démarrage |
| 1 | ASCII | Protocole implémenté dans cette interface web |

### Pourquoi c'est important ?

Si le TinyBMS est configuré sur le protocole MODBUS (valeur par défaut 0) et que notre interface envoie des commandes en protocole ASCII, **la communication échouera silencieusement**. Les commandes envoyées ne seront pas comprises par le BMS.

## Solution implémentée

### Architecture

1. **Classe TinyBMS (`tinybms.js`)** :
   - Nouvelle méthode `setProtocol(protocolValue)` qui écrit dans le registre 343
   - Utilise la fonction d'écriture existante `writeRegister(343, value)`
   - Attend 500ms après l'écriture pour laisser le BMS appliquer le changement

2. **Serveur (`server.js`)** :
   - Modifié pour accepter un paramètre `protocol` dans la requête de connexion
   - Appelle `bms.setProtocol()` automatiquement après la connexion série
   - Gère les erreurs de configuration gracieusement (continue même en cas d'échec)

3. **Interface utilisateur (`index.html` + `app.js`)** :
   - Ajout d'un sélecteur de protocole avec deux options : ASCII (recommandé) et MODBUS
   - Envoie le protocole sélectionné lors de la connexion
   - Valeur par défaut : ASCII (1)

### Flux de connexion

```
1. Utilisateur clique sur "Connect"
   ↓
2. app.js envoie { path, protocol } au serveur
   ↓
3. server.js ouvre le port série
   ↓
4. server.js appelle bms.setProtocol(protocol)
   ↓
5. tinybms.js écrit la valeur dans le registre 343
   ↓
6. Attente de 500ms pour application du changement
   ↓
7. Démarrage du polling des données
```

## Détails techniques

### Méthode `setProtocol()` (tinybms.js:98-119)

```javascript
async setProtocol(protocolValue = 1) {
    if (!this.isConnected) {
        throw new Error("Cannot set protocol: not connected");
    }

    console.log(`Setting TinyBMS protocol to ${protocolValue === 1 ? 'ASCII' : 'MODBUS'}...`);

    try {
        const success = await this.writeRegister(343, protocolValue);
        if (success) {
            console.log(`Protocol successfully set to ${protocolValue === 1 ? 'ASCII' : 'MODBUS'}`);
            // Attendre un peu pour que le BMS applique le changement
            await new Promise(resolve => setTimeout(resolve, 500));
        } else {
            console.warn('Protocol write command sent but no confirmation received');
        }
        return success;
    } catch (error) {
        console.error('Failed to set protocol:', error.message);
        throw error;
    }
}
```

### Configuration dans server.js (server.js:47-58)

```javascript
// Configuration du protocole si spécifié (par défaut ASCII = 1)
const selectedProtocol = protocol !== undefined ? parseInt(protocol) : 1;
console.log(`Configuring TinyBMS protocol to ${selectedProtocol === 1 ? 'ASCII' : 'MODBUS'}...`);

try {
    await bms.setProtocol(selectedProtocol);
    console.log('Protocol configuration successful');
} catch (protocolError) {
    console.warn('Protocol configuration failed:', protocolError.message);
    // Continue même si la configuration du protocole échoue
    // (le BMS pourrait déjà être sur le bon protocole)
}
```

## Gestion des erreurs

La configuration du protocole est conçue pour être **résiliente** :

1. **Timeout** : Si l'écriture du registre timeout (800ms), on continue quand même
2. **Erreur de communication** : Si une erreur se produit, on log un warning mais on continue
3. **BMS déjà configuré** : Si le BMS est déjà sur le bon protocole, l'écriture réussira sans effet

Cette approche permet de garantir que :
- La première connexion configure toujours le protocole correctement
- Les connexions suivantes fonctionnent même si le BMS est déjà configuré
- Les erreurs temporaires ne bloquent pas complètement l'interface

## Utilisation

### Pour l'utilisateur final

1. Sélectionner le port série dans la liste
2. **Choisir le protocole** :
   - **ASCII (recommandé)** : Pour utilisation normale
   - **MODBUS** : Pour test ou débogage uniquement
3. Cliquer sur "Connect"
4. L'interface configure automatiquement le protocole

### Pour le développeur

Si vous voulez forcer un protocole dans le code :

```javascript
// Dans server.js, forcer ASCII
await bms.setProtocol(1);

// Ou forcer MODBUS
await bms.setProtocol(0);
```

## Références

- **Documentation TinyBMS Rev D** : `docs/TinyBMS-UART-Reference.md`
- **Registre 343** : Protocol (0=MODBUS, 1=ASCII)
- **Adresse Modbus** : 0x0157 (Big Endian)
- **Type** : UINT16
- **Catégorie** : Settings / Hardware

## Notes importantes

⚠️ **Attention** : Le changement de protocole est **persistant**. Une fois écrit, le TinyBMS conservera ce réglage même après un redémarrage.

💡 **Conseil** : Toujours utiliser ASCII (1) avec cette interface pour garantir la compatibilité.

🔍 **Débogage** : Les logs de la console serveur indiquent clairement le protocole configuré et les éventuelles erreurs.
