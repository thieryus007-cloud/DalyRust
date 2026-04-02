---

DOCUMENTATION TECHNIQUE - NodeType Batterie pour React Flow

Version : 1.0
Date : Avril 2026
Statut : Template réutilisable

---

1. Objectif

Créer un nœud React Flow personnalisé représentant une batterie avec :

· Direction dynamique du flux (charge/décharge) basée sur le signe du courant
· Animations couleur (vert pour charge, rouge pour décharge)
· Affichage des métriques : SOC, Tension, Courant, Température, Puissance
· Modèle unique réutilisable pour plusieurs instances (BMS-360Ah, BMS-320Ah, etc.)

---

2. Prérequis

```bash
npm install @xyflow/react
```

---

3. Structure des fichiers recommandée

```
src/
├── components/
│   └── nodes/
│       └── BatteryNode.jsx
├── pages/
│   └── Visualisation.jsx
└── styles/
    └── batteryAnimations.css
```

---

4. Code complet du composant BatteryNode

Fichier : src/components/nodes/BatteryNode.jsx

```jsx
import { Handle, Position } from '@xyflow/react';
import './batteryAnimations.css'; // à créer

const BatteryNode = ({ id, data }) => {
  // Données d'entrée
  const {
    label = 'BMS',
    soc = 0,           // State of Charge en %
    voltage = 0,       // Tension en V
    current = 0,       // Courant en A (négatif = décharge, positif = charge)
    temperature = 0,   // Température en °C
    power = 0          // Puissance en kW
  } = data;

  // Déterminer le sens du flux (charge si courant > 0, décharge si < 0)
  const isCharging = current > 0;
  const isDischarging = current < 0;
  const isIdle = current === 0;

  // Couleurs dynamiques
  const currentColor = isCharging ? '#4caf50' : (isDischarging ? '#f44336' : '#ff9800');
  
  // Type de Handle (target si charge, source si décharge)
  const handleType = isCharging ? 'target' : (isDischarging ? 'source' : null);

  // Formatage des valeurs
  const formattedPower = Math.abs(power).toFixed(2);
  const powerUnit = power < 0 ? 'kW' : 'kW';
  const powerSign = power < 0 ? '-' : (power > 0 ? '+' : '');
  const currentAbs = Math.abs(current).toFixed(1);

  return (
    <div 
      className="battery-node"
      style={{
        borderColor: currentColor,
        boxShadow: `0 0 8px ${currentColor}`,
        animation: isCharging ? 'pulseGreen 1.5s infinite' : (isDischarging ? 'pulseRed 1.5s infinite' : 'none')
      }}
    >
      {/* Handle dynamique */}
      {handleType && (
        <Handle 
          type={handleType}
          position={Position.Bottom}
          style={{
            background: currentColor,
            width: 12,
            height: 12,
            animation: 'pulseHandle 1s infinite'
          }}
        />
      )}

      {/* En-tête */}
      <div className="battery-header" style={{ borderBottomColor: currentColor }}>
        <span className="battery-icon">🔋</span>
        <span className="battery-label">{label}</span>
        <span 
          className="battery-status"
          style={{ color: currentColor }}
        >
          {isCharging ? '⚡ CHARGE' : (isDischarging ? '🔻 DÉCHARGE' : '⏸ IDLE')}
        </span>
      </div>

      {/* SOC avec jauge */}
      <div className="battery-soc">
        <span className="soc-value">{soc}%</span>
        <div className="soc-bar-bg">
          <div 
            className="soc-bar-fill" 
            style={{ width: `${soc}%`, backgroundColor: currentColor }}
          />
        </div>
      </div>

      {/* Métriques */}
      <div className="battery-metrics">
        <div className="metric">
          <span className="metric-label">TENSION</span>
          <span className="metric-value">{voltage.toFixed(1)}V</span>
        </div>
        <div className="metric">
          <span className="metric-label">COURANT</span>
          <span className="metric-value" style={{ color: currentColor }}>
            {current >= 0 ? '+' : ''}{currentAbs}A
          </span>
        </div>
        <div className="metric">
          <span className="metric-label">TEMP.</span>
          <span className="metric-value">{temperature.toFixed(1)}°C</span>
        </div>
        <div className="metric">
          <span className="metric-label">PUISSANCE</span>
          <span className="metric-value" style={{ color: currentColor }}>
            {powerSign}{formattedPower} {powerUnit}
          </span>
        </div>
      </div>
    </div>
  );
};

export default BatteryNode;
```

---

5. Fichier CSS des animations

Fichier : src/components/nodes/batteryAnimations.css

```css
.battery-node {
  min-width: 200px;
  background: #1a1a2e;
  border-radius: 12px;
  padding: 12px;
  border: 2px solid;
  transition: all 0.3s ease;
  font-family: 'Segoe UI', monospace;
}

.battery-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 8px;
  margin-bottom: 8px;
  border-bottom: 1px solid;
}

.battery-icon {
  font-size: 20px;
}

.battery-label {
  font-weight: bold;
  color: #fff;
  font-size: 14px;
}

.battery-status {
  font-size: 10px;
  font-weight: bold;
}

.battery-soc {
  margin-bottom: 12px;
}

.soc-value {
  color: #fff;
  font-size: 24px;
  font-weight: bold;
  display: block;
  text-align: center;
  margin-bottom: 6px;
}

.soc-bar-bg {
  background: #2a2a3e;
  border-radius: 10px;
  height: 8px;
  overflow: hidden;
}

.soc-bar-fill {
  height: 100%;
  border-radius: 10px;
  transition: width 0.5s ease;
}

.battery-metrics {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.metric {
  background: #0f0f1a;
  border-radius: 8px;
  padding: 6px 8px;
  text-align: center;
}

.metric-label {
  display: block;
  font-size: 9px;
  color: #888;
  letter-spacing: 0.5px;
}

.metric-value {
  display: block;
  font-size: 13px;
  font-weight: bold;
  color: #ddd;
}

/* Animations */
@keyframes pulseGreen {
  0%, 100% { border-color: #4caf50; box-shadow: 0 0 5px rgba(76, 175, 80, 0.5); }
  50% { border-color: #2e7d32; box-shadow: 0 0 15px rgba(76, 175, 80, 0.8); }
}

@keyframes pulseRed {
  0%, 100% { border-color: #f44336; box-shadow: 0 0 5px rgba(244, 67, 54, 0.5); }
  50% { border-color: #c62828; box-shadow: 0 0 15px rgba(244, 67, 54, 0.8); }
}

@keyframes pulseHandle {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.6; transform: scale(1.2); }
}
```

---

6. Code d'utilisation dans la page

Fichier : src/pages/Visualisation.jsx

```jsx
import { ReactFlow, useNodesState, useEdgesState, Background, Controls } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import BatteryNode from '../components/nodes/BatteryNode';

// Déclaration des types de nœuds personnalisés
const nodeTypes = {
  battery: BatteryNode,
};

// Données initiales (basées sur votre image)
const initialNodes = [
  {
    id: 'bms-360ah',
    type: 'battery',
    position: { x: 100, y: 100 },
    data: {
      label: 'BMS-360Ah',
      soc: 92,
      voltage: 52.8,
      current: -17.3,   // négatif = décharge
      temperature: 14.0,
      power: -0.91
    }
  },
  {
    id: 'bms-320ah',
    type: 'battery',
    position: { x: 100, y: 300 },
    data: {
      label: 'BMS-320Ah',
      soc: 94,
      voltage: 52.9,
      current: -13.1,   // négatif = décharge
      temperature: 16.0,
      power: -0.69
    }
  }
];

const initialEdges = [
  // Exemple de connexion (à adapter à votre besoin)
  { id: 'e1', source: 'bms-360ah', target: 'load', sourceHandle: 'bottom' },
  { id: 'e2', source: 'bms-320ah', target: 'load', sourceHandle: 'bottom' }
];

function Visualisation() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Fonction pour mettre à jour une batterie en temps réel
  const updateBatteryData = (batteryId, newData) => {
    setNodes((nds) =>
      nds.map((node) =>
        node.id === batteryId
          ? { ...node, data: { ...node.data, ...newData } }
          : node
      )
    );
  };

  // Simulation de changement en temps réel (à remplacer par WebSocket ou API)
  setTimeout(() => {
    // Exemple : la batterie 360Ah passe en charge après 5 secondes
    updateBatteryData('bms-360ah', {
      current: 15.2,
      power: 0.80,
      soc: 93
    });
  }, 5000);

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

7. Processus complet de mise en œuvre

Étape 1 : Installation

```bash
npm create vite@latest mon-projet -- --template react
cd mon-projet
npm install @xyflow/react
```

Étape 2 : Créer les fichiers

```bash
mkdir -p src/components/nodes
mkdir -p src/styles
# Copier les codes ci-dessus dans les fichiers correspondants
```

Étape 3 : Configurer le point d'entrée

src/App.jsx

```jsx
import Visualisation from './pages/Visualisation';

function App() {
  return <Visualisation />;
}

export default App;
```

Étape 4 : Lancer l'application

```bash
npm run dev
```

---

8. Comment ajouter un nouveau type de nœud (template futur)

```jsx
// 1. Créer le composant
// src/components/nodes/NewNodeType.jsx
const NewNodeType = ({ data }) => {
  return <div>Mon nouveau nœud</div>;
};

// 2. L'enregistrer dans nodeTypes
const nodeTypes = {
  battery: BatteryNode,
  newType: NewNodeType,  // ← ajout ici
};

// 3. L'utiliser dans un nœud
{ id: 'mon-id', type: 'newType', position: { x: 0, y: 0 }, data: {...} }
```

---

9. Prochaines extensions possibles

Fonctionnalité Description
WebSocket temps réel Mise à jour des données via socket
Tooltips au survol Détails supplémentaires
Drag & drop ajout de nœuds Interface interactive
Export/Import configuration Sauvegarde du graphe
Historique des données Graphique d'évolution

---

10. Notes importantes

· Le signe du courant détermine automatiquement le sens de la flèche
· Une animation pulse indique visuellement l'état actif
· Le composant est complètement réutilisable (plusieurs instances indépendantes)
· Les couleurs suivent la convention : 🟢 charge (courant > 0), 🔴 décharge (courant < 0), 🟠 idle (courant = 0)

---
