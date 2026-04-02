DOCUMENTATION TECHNIQUE - NodeType Switch pour React Flow

Version : 1.0
Date : Avril 2026
Statut : Template réutilisable

---

1. Objectif

Créer un nœud React Flow personnalisé représentant un interrupteur commandable (type Tongou) avec :

· Toggle switch ON/OFF pour envoyer des commandes
· 1 ou 2 handles configurables (entrée/sortie ou passage)
· Affichage des métriques : Puissance, Tension, Courant, Cos φ
· Affichage des consommations : Aujourd'hui, Hier, Total
· Modèle unique réutilisable pour plusieurs instances

---

2. Prérequis

```bash
npm install @xyflow/react
```

---

3. Structure des fichiers

```
src/
├── components/
│   └── nodes/
│       ├── BatteryNode.jsx      (déjà créé)
│       ├── ET112Node.jsx         (déjà créé)
│       └── SwitchNode.jsx        (à créer)
├── pages/
│   └── Visualisation.jsx
└── styles/
    └── switchAnimations.css
```

---

4. Code complet du composant SwitchNode

Fichier : src/components/nodes/SwitchNode.jsx

```jsx
import { useState } from 'react';
import { Handle, Position } from '@xyflow/react';
import './switchAnimations.css';

const SwitchNode = ({ id, data, onToggle }) => {
  // État local du switch (ON/OFF)
  const [isOn, setIsOn] = useState(data.isOn !== undefined ? data.isOn : true);
  
  // Données d'entrée avec valeurs par défaut (basées sur l'image)
  const {
    label = 'Tongou Switch',
    deviceId = 'tongou_3BC764',
    time = '20:17:48',
    power = 2.0,        // Puissance en W
    voltage = 231.0,    // Tension en V
    current = 0.04,     // Courant en A
    cosPhi = 0.26,      // Facteur de puissance
    today = 4.26,       // kWh aujourd'hui
    yesterday = 2.62,   // kWh hier
    total = 42.3,       // kWh total
    // Configuration des handles
    handles = {
      left: false,      // handle gauche (entrée)
      right: false      // handle droit (sortie)
    }
  } = data;

  // Couleur dynamique selon état
  const switchColor = isOn ? '#4caf50' : '#f44336';
  const statusText = isOn ? 'ON' : 'OFF';

  // Gestion du toggle
  const handleToggle = () => {
    const newState = !isOn;
    setIsOn(newState);
    // Appel du callback externe si fourni
    if (onToggle) {
      onToggle(id, newState);
    }
    // Mise à jour des données du nœud
    if (data.onToggle) {
      data.onToggle(id, newState);
    }
  };

  return (
    <div 
      className="switch-node"
      style={{
        borderColor: switchColor,
        boxShadow: isOn ? `0 0 8px ${switchColor}` : 'none',
        opacity: isOn ? 1 : 0.7
      }}
    >
      {/* Handles configurables */}
      {handles.left && (
        <Handle 
          type="target"
          position={Position.Left}
          id="left-input"
          style={{ background: switchColor }}
        />
      )}
      
      {handles.right && (
        <Handle 
          type="source"
          position={Position.Right}
          id="right-output"
          style={{ background: switchColor }}
        />
      )}

      {/* En-tête avec état ON/OFF */}
      <div className="switch-header">
        <div className="switch-status" style={{ backgroundColor: switchColor }}>
          {statusText}
        </div>
        <div className="switch-title">
          <span className="switch-icon">🔌</span>
          <span className="switch-label">{label}</span>
        </div>
      </div>

      {/* ID et Time */}
      <div className="switch-id-time">
        <span className="device-id">{deviceId}</span>
        <span className="device-time">{time}</span>
      </div>

      {/* Toggle Switch */}
      <div className="switch-toggle-container">
        <button 
          className={`toggle-switch ${isOn ? 'on' : 'off'}`}
          onClick={handleToggle}
        >
          <span className="toggle-slider"></span>
        </button>
        <span className="toggle-label" style={{ color: switchColor }}>
          {isOn ? 'COMMANDÉ ON' : 'COMMANDÉ OFF'}
        </span>
      </div>

      {/* Grille des métriques (4 colonnes) */}
      <div className="switch-metrics">
        <div className="metric">
          <span className="metric-label">PUISSANCE</span>
          <span className="metric-value" style={{ color: switchColor }}>
            {power.toFixed(1)} W
          </span>
        </div>
        <div className="metric">
          <span className="metric-label">TENSION</span>
          <span className="metric-value">{voltage.toFixed(1)} V</span>
        </div>
        <div className="metric">
          <span className="metric-label">COURANT</span>
          <span className="metric-value">{current.toFixed(2)} A</span>
        </div>
        <div className="metric">
          <span className="metric-label">COS Φ</span>
          <span className="metric-value">{cosPhi.toFixed(2)}</span>
        </div>
      </div>

      {/* Consommations journalières */}
      <div className="switch-consumption">
        <div className="consumption-item">
          <span className="consumption-label">AUJOURD'HUI</span>
          <span className="consumption-value">{today.toFixed(2)} kWh</span>
        </div>
        <div className="consumption-item">
          <span className="consumption-label">HIER</span>
          <span className="consumption-value">{yesterday.toFixed(2)} kWh</span>
        </div>
        <div className="consumption-item total">
          <span className="consumption-label">TOTAL</span>
          <span className="consumption-value">{total.toFixed(1)} kWh</span>
        </div>
      </div>

      {/* Détails (cliquable) */}
      <div className="switch-details">
        Details →
      </div>
    </div>
  );
};

export default SwitchNode;
```

---

5. Fichier CSS des animations

Fichier : src/components/nodes/switchAnimations.css

```css
.switch-node {
  min-width: 280px;
  background: #0d1117;
  border-radius: 16px;
  padding: 14px;
  border: 2px solid;
  transition: all 0.3s ease;
  font-family: 'Segoe UI', monospace;
}

.switch-node:hover {
  transform: translateY(-2px);
}

/* En-tête */
.switch-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.switch-status {
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
  font-weight: bold;
  color: white;
  letter-spacing: 1px;
}

.switch-title {
  display: flex;
  align-items: center;
  gap: 6px;
}

.switch-icon {
  font-size: 16px;
}

.switch-label {
  font-weight: bold;
  color: #fff;
  font-size: 13px;
}

/* ID et Time */
.switch-id-time {
  display: flex;
  justify-content: space-between;
  margin-bottom: 12px;
  font-size: 10px;
  color: #888;
  font-family: monospace;
}

/* Toggle Switch */
.switch-toggle-container {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
  padding: 8px 0;
  border-top: 1px solid #2a2f3e;
  border-bottom: 1px solid #2a2f3e;
}

.toggle-switch {
  width: 52px;
  height: 28px;
  background: #333;
  border-radius: 30px;
  border: none;
  cursor: pointer;
  position: relative;
  transition: background 0.3s ease;
  padding: 0;
}

.toggle-switch.on {
  background: #4caf50;
}

.toggle-switch.off {
  background: #f44336;
}

.toggle-slider {
  position: absolute;
  width: 22px;
  height: 22px;
  background: white;
  border-radius: 50%;
  top: 3px;
  left: 4px;
  transition: transform 0.3s ease;
}

.toggle-switch.on .toggle-slider {
  transform: translateX(22px);
}

.toggle-label {
  font-size: 11px;
  font-weight: bold;
  letter-spacing: 0.5px;
}

/* Grille métriques 4 colonnes */
.switch-metrics {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin-bottom: 16px;
}

.switch-metrics .metric {
  background: #1a1f2e;
  border-radius: 10px;
  padding: 6px 4px;
  text-align: center;
}

.switch-metrics .metric-label {
  display: block;
  font-size: 8px;
  color: #888;
}

.switch-metrics .metric-value {
  display: block;
  font-size: 11px;
  font-weight: bold;
  color: #ddd;
}

/* Consommations */
.switch-consumption {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  padding: 8px 0;
  background: #1a1f2e;
  border-radius: 12px;
}

.consumption-item {
  flex: 1;
  text-align: center;
}

.consumption-item.total {
  border-left: 1px solid #2a2f3e;
}

.consumption-label {
  display: block;
  font-size: 8px;
  color: #666;
}

.consumption-value {
  display: block;
  font-size: 11px;
  font-weight: bold;
  color: #ddd;
}

/* Détails */
.switch-details {
  text-align: right;
  font-size: 11px;
  color: #58a6ff;
  cursor: pointer;
  padding-top: 8px;
  border-top: 1px solid #2a2f3e;
  transition: opacity 0.2s;
}

.switch-details:hover {
  opacity: 0.7;
}

/* Animation pour le switch quand il change d'état */
@keyframes switchPulse {
  0% { transform: scale(1); }
  50% { transform: scale(1.05); }
  100% { transform: scale(1); }
}

.switch-node.state-changing {
  animation: switchPulse 0.3s ease;
}
```

---

6. Exemples de configuration des handles

Cas 1 : Interrupteur simple (1 handle, sortie uniquement)

```jsx
{
  id: 'switch-simple',
  type: 'switch',
  position: { x: 300, y: 200 },
  data: {
    label: 'Tongou Switch',
    deviceId: 'tongou_3BC764',
    isOn: true,
    power: 2.0,
    handles: { left: false, right: true }  // commande une charge
  }
}
```

Cas 2 : Interrupteur avec entrée et sortie (2 handles)

```jsx
{
  id: 'switch-between',
  type: 'switch',
  position: { x: 300, y: 200 },
  data: {
    label: 'Tongou Switch',
    isOn: false,
    handles: { left: true, right: true }  // entre source et charge
  }
}
```

Cas 3 : Interrupteur sans handles (commande seule)

```jsx
{
  id: 'switch-standalone',
  type: 'switch',
  position: { x: 300, y: 200 },
  data: {
    label: 'Tongou Switch',
    isOn: true,
    handles: { left: false, right: false }  // juste pour la commande
  }
}
```

---

7. Code d'utilisation dans la page

Fichier : src/pages/Visualisation.jsx (mise à jour)

```jsx
import { ReactFlow, useNodesState, useEdgesState, Background, Controls } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import BatteryNode from '../components/nodes/BatteryNode';
import ET112Node from '../components/nodes/ET112Node';
import SwitchNode from '../components/nodes/SwitchNode';

// Déclaration des types de nœuds
const nodeTypes = {
  battery: BatteryNode,
  et112: ET112Node,
  switch: SwitchNode,
};

const initialNodes = [
  // Source (ex: batterie)
  {
    id: 'bms-360ah',
    type: 'battery',
    position: { x: 50, y: 200 },
    data: {
      label: 'BMS-360Ah',
      soc: 92,
      current: -17.3,
      power: -0.91
    }
  },
  
  // Interrupteur Tongou
  {
    id: 'tongou-switch',
    type: 'switch',
    position: { x: 300, y: 200 },
    data: {
      label: 'Tongou Switch',
      deviceId: 'tongou_3BC764',
      time: '20:17:48',
      isOn: true,
      power: 2.0,
      voltage: 231.0,
      current: 0.04,
      cosPhi: 0.26,
      today: 4.26,
      yesterday: 2.62,
      total: 42.3,
      handles: { left: true, right: true }
    }
  },
  
  // Charge
  {
    id: 'load',
    type: 'default',
    position: { x: 550, y: 200 },
    data: { label: 'Charge' }
  }
];

const initialEdges = [
  { id: 'e1', source: 'bms-360ah', target: 'tongou-switch', targetHandle: 'left-input' },
  { id: 'e2', source: 'tongou-switch', target: 'load', sourceHandle: 'right-output' }
];

// Callback pour les changements d'état du switch
const handleSwitchToggle = (switchId, isOn) => {
  console.log(`Switch ${switchId} est maintenant ${isOn ? 'ON' : 'OFF'}`);
  // Ici vous pouvez :
  // - Envoyer une commande MQTT/API
  // - Mettre à jour d'autres nœuds
  // - Logger l'événement
};

function Visualisation() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Mise à jour d'un switch
  const updateSwitchState = (switchId, isOn) => {
    setNodes((nds) =>
      nds.map((node) =>
        node.id === switchId
          ? { ...node, data: { ...node.data, isOn } }
          : node
      )
    );
  };

  return (
    <div style={{ width: '100vw', height: '100vh' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
      >
        <Background />
        <Controls />
      </ReactFlow>
    </div>
  );
}

export default Visualisation;
```

---

8. Intégration avec une API (commande réelle)

```jsx
// Exemple d'envoi de commande MQTT ou HTTP
const sendSwitchCommand = async (deviceId, isOn) => {
  try {
    const response = await fetch('/api/switch/command', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        deviceId: deviceId,
        state: isOn ? 'ON' : 'OFF'
      })
    });
    
    if (!response.ok) {
      throw new Error('Commande échouée');
    }
    
    console.log(`Commande ${isOn ? 'ON' : 'OFF'} envoyée à ${deviceId}`);
  } catch (error) {
    console.error('Erreur:', error);
    // Option : remettre l'état précédent
  }
};

// Dans le composant SwitchNode, remplacer handleToggle par :
const handleToggle = () => {
  const newState = !isOn;
  setIsOn(newState);
  sendSwitchCommand(data.deviceId, newState);
  if (onToggle) onToggle(id, newState);
};
```

---

9. Résumé des configurations possibles

Configuration Handles Cas d'usage
{ left: false, right: true } 1 (sortie) Commande d'une charge
{ left: true, right: false } 1 (entrée) Sectionneur amont
{ left: true, right: true } 2 Interrupteur passant
{ left: false, right: false } 0 Télégestion seule

---

10. Processus complet de mise en œuvre

Étape 1 : Créer les fichiers

```bash
# Créer le composant Switch
touch src/components/nodes/SwitchNode.jsx

# Créer les styles
touch src/components/nodes/switchAnimations.css
```

Étape 2 : Copier les codes

· Copier le code de la section 4 dans SwitchNode.jsx
· Copier le code de la section 5 dans switchAnimations.css

Étape 3 : Mettre à jour Visualisation.jsx

· Ajouter l'import de SwitchNode
· Ajouter switch: SwitchNode dans nodeTypes

Étape 4 : Lancer l'application

```bash
npm run dev
```

---

11. Évolution possible : Commande groupée

Pour commander plusieurs switches simultanément :

```jsx
// Bouton de commande groupée
const groupCommand = (state) => {
  setNodes((nds) =>
    nds.map((node) =>
      node.type === 'switch'
        ? { ...node, data: { ...node.data, isOn: state } }
        : node
    )
  );
};

// Utilisation
<button onClick={() => groupCommand(true)}>Tout ON</button>
<button onClick={() => groupCommand(false)}>Tout OFF</button>
```

---

Fin du document - Le NodeType Switch est prêt à être utilisé avec toggle ON/OFF et 1 ou 2 handles selon votre besoin.
