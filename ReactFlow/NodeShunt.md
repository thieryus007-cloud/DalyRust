---

DOCUMENTATION TECHNIQUE - NodeType Shunt pour React Flow

Version : 1.0
Date : Avril 2026
Statut : Template réutilisable

---

1. Objectif

Créer un nœud React Flow personnalisé représentant un shunt (capteur de courant / moniteur de batterie) avec :

· 2 handles obligatoires (entrée et sortie pour mesurer le flux)
· Direction dynamique du flux (charge/décharge) basée sur le signe du courant
· Affichage des métriques : Puissance, SOC, Tension, Courant, Temps restant
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
│       ├── SwitchNode.jsx        (déjà créé)
│       └── ShuntNode.jsx         (à créer)
├── pages/
│   └── Visualisation.jsx
└── styles/
    └── shuntAnimations.css
```

---

4. Code complet du composant ShuntNode

Fichier : src/components/nodes/ShuntNode.jsx

```jsx
import { Handle, Position } from '@xyflow/react';
import './shuntAnimations.css';

const ShuntNode = ({ id, data }) => {
  // Données d'entrée avec valeurs par défaut (basées sur l'image)
  const {
    label = 'Shunt',
    status = 'Décharge en cours',  // ou 'Charge en cours'
    power = -1664,                 // Puissance en W (négatif = décharge)
    soc = 90.2,                    // State of Charge en %
    voltage = 52.81,               // Tension en V
    current = -31.5,               // Courant en A (négatif = décharge)
    timeRemaining = 13.10,         // Temps restant en heures
    // Configuration des handles (2 handles obligatoires)
    handles = {
      left: true,   // handle gauche (entrée depuis source)
      right: true   // handle droit (sortie vers charge)
    }
  } = data;

  // Déterminer l'état (charge/décharge)
  const isCharging = current > 0;
  const isDischarging = current < 0;
  const isIdle = current === 0;
  
  // Couleur dynamique
  const flowColor = isCharging ? '#4caf50' : (isDischarging ? '#f44336' : '#ff9800');
  
  // Texte du statut
  const statusText = isCharging ? 'Charge en cours' : (isDischarging ? 'Décharge en cours' : 'Repos');
  
  // Valeurs absolues pour l'affichage
  const absPower = Math.abs(power).toFixed(0);
  const absCurrent = Math.abs(current).toFixed(1);
  const powerUnit = 'W';
  
  // Format du temps restant
  const hours = Math.floor(timeRemaining);
  const minutes = Math.round((timeRemaining - hours) * 60);
  const formattedTime = `${hours}.${minutes.toString().padStart(2, '0')} h`;

  return (
    <div 
      className="shunt-node"
      style={{
        borderColor: flowColor,
        boxShadow: `0 0 8px ${flowColor}`,
        animation: isCharging ? 'pulseGreen 1.5s infinite' : (isDischarging ? 'pulseRed 1.5s infinite' : 'none')
      }}
    >
      {/* Handles - 2 obligatoires (entrée à gauche, sortie à droite) */}
      {handles.left && (
        <Handle 
          type="target"
          position={Position.Left}
          id="left-input"
          style={{ background: flowColor }}
        />
      )}
      
      {handles.right && (
        <Handle 
          type="source"
          position={Position.Right}
          id="right-output"
          style={{ background: flowColor }}
        />
      )}

      {/* Statut principal avec animation */}
      <div className="shunt-status" style={{ color: flowColor }}>
        <span className="status-dot" style={{ backgroundColor: flowColor }}></span>
        {statusText}
      </div>

      {/* Puissance principale */}
      <div className="shunt-power" style={{ color: flowColor }}>
        <span className="power-sign">{power < 0 ? '-' : '+'}</span>
        <span className="power-value">{absPower}</span>
        <span className="power-unit">{powerUnit}</span>
      </div>

      {/* SOC avec jauge circulaire */}
      <div className="shunt-soc">
        <div className="soc-circle">
          <svg viewBox="0 0 100 100" className="soc-svg">
            <circle 
              cx="50" cy="50" r="45" 
              fill="none" 
              stroke="#2a2f3e" 
              strokeWidth="8"
            />
            <circle 
              cx="50" cy="50" r="45" 
              fill="none" 
              stroke={flowColor}
              strokeWidth="8"
              strokeDasharray={`${(soc / 100) * 283} 283`}
              strokeLinecap="round"
              transform="rotate(-90 50 50)"
              className="soc-progress"
            />
          </svg>
          <span className="soc-percent" style={{ color: flowColor }}>
            {soc.toFixed(1)}%
          </span>
        </div>
      </div>

      {/* Grille des métriques */}
      <div className="shunt-metrics">
        <div className="metric">
          <span className="metric-label">Tension</span>
          <span className="metric-value">{voltage.toFixed(2)} V</span>
        </div>
        <div className="metric">
          <span className="metric-label">Courant</span>
          <span className="metric-value" style={{ color: flowColor }}>
            {current >= 0 ? '+' : ''}{absCurrent} A
          </span>
        </div>
        <div className="metric-full">
          <span className="metric-label">Temps restant</span>
          <span className="metric-value time-value" style={{ color: flowColor }}>
            {formattedTime}
          </span>
        </div>
      </div>
    </div>
  );
};

export default ShuntNode;
```

---

5. Fichier CSS des animations

Fichier : src/components/nodes/shuntAnimations.css

```css
.shunt-node {
  min-width: 260px;
  background: #0d1117;
  border-radius: 20px;
  padding: 16px;
  border: 2px solid;
  transition: all 0.3s ease;
  font-family: 'Segoe UI', monospace;
  position: relative;
}

.shunt-node:hover {
  transform: translateY(-2px);
}

/* Statut */
.shunt-status {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 12px;
  font-weight: bold;
  letter-spacing: 1px;
  margin-bottom: 16px;
  text-transform: uppercase;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  animation: pulse 1s infinite;
}

/* Puissance principale */
.shunt-power {
  display: flex;
  align-items: baseline;
  justify-content: center;
  gap: 4px;
  margin-bottom: 20px;
  padding: 8px;
  background: #1a1f2e;
  border-radius: 16px;
}

.power-sign {
  font-size: 20px;
  font-weight: bold;
}

.power-value {
  font-size: 36px;
  font-weight: bold;
}

.power-unit {
  font-size: 14px;
  font-weight: normal;
}

/* SOC - Cercle de progression */
.shunt-soc {
  display: flex;
  justify-content: center;
  margin-bottom: 20px;
}

.soc-circle {
  position: relative;
  width: 100px;
  height: 100px;
}

.soc-svg {
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}

.soc-progress {
  transition: stroke-dasharray 0.5s ease;
}

.soc-percent {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-size: 18px;
  font-weight: bold;
}

/* Grille métriques */
.shunt-metrics {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.metric {
  background: #1a1f2e;
  border-radius: 12px;
  padding: 8px;
  text-align: center;
}

.metric-full {
  grid-column: span 2;
  background: #1a1f2e;
  border-radius: 12px;
  padding: 8px;
  text-align: center;
}

.metric-label {
  display: block;
  font-size: 9px;
  color: #888;
  letter-spacing: 0.5px;
  margin-bottom: 4px;
}

.metric-value {
  display: block;
  font-size: 14px;
  font-weight: bold;
  color: #ddd;
}

.time-value {
  font-size: 16px;
  font-weight: bold;
}

/* Animations */
@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.4; transform: scale(0.8); }
}

@keyframes pulseGreen {
  0%, 100% { border-color: #4caf50; box-shadow: 0 0 5px rgba(76, 175, 80, 0.5); }
  50% { border-color: #2e7d32; box-shadow: 0 0 15px rgba(76, 175, 80, 0.8); }
}

@keyframes pulseRed {
  0%, 100% { border-color: #f44336; box-shadow: 0 0 5px rgba(244, 67, 54, 0.5); }
  50% { border-color: #c62828; box-shadow: 0 0 15px rgba(244, 67, 54, 0.8); }
}

/* Animation pour le courant */
@keyframes currentFlow {
  0% { opacity: 0.6; }
  100% { opacity: 1; }
}

.current-animation {
  animation: currentFlow 0.5s ease-in-out infinite alternate;
}
```

---

6. Configuration des handles

Le Shunt a 2 handles obligatoires pour mesurer le flux entre deux éléments :

```jsx
// Configuration standard (inchangée)
handles: {
  left: true,   // Entrée (ex: batterie)
  right: true   // Sortie (ex: onduleur)
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
import ShuntNode from '../components/nodes/ShuntNode';

// Déclaration des types de nœuds
const nodeTypes = {
  battery: BatteryNode,
  et112: ET112Node,
  switch: SwitchNode,
  shunt: ShuntNode,
};

const initialNodes = [
  // Batterie
  {
    id: 'bms-360ah',
    type: 'battery',
    position: { x: 50, y: 200 },
    data: {
      label: 'BMS-360Ah',
      soc: 90.2,
      voltage: 52.81,
      current: -31.5,
      power: -1664
    }
  },
  
  // Shunt (mesure entre batterie et onduleur)
  {
    id: 'shunt-main',
    type: 'shunt',
    position: { x: 300, y: 200 },
    data: {
      label: 'Shunt',
      status: 'Décharge en cours',
      power: -1664,
      soc: 90.2,
      voltage: 52.81,
      current: -31.5,
      timeRemaining: 13.10,
      handles: { left: true, right: true }
    }
  },
  
  // Onduleur / Charge
  {
    id: 'inverter',
    type: 'et112',
    position: { x: 550, y: 200 },
    data: {
      label: 'ET112',
      power: 1664,
      handles: { left: true, right: false }
    }
  }
];

const initialEdges = [
  { id: 'e1', source: 'bms-360ah', target: 'shunt-main', targetHandle: 'left-input' },
  { id: 'e2', source: 'shunt-main', target: 'inverter', sourceHandle: 'right-output' }
];

// Mise à jour temps réel du shunt
const updateShuntData = (shuntId, newData) => {
  setNodes((nds) =>
    nds.map((node) =>
      node.id === shuntId
        ? { ...node, data: { ...node.data, ...newData } }
        : node
    )
  );
};

// Simulation de données temps réel
setInterval(() => {
  // Exemple : mise à jour des valeurs
  updateShuntData('shunt-main', {
    current: -31.5 + (Math.random() - 0.5) * 2,
    power: -1664 + (Math.random() - 0.5) * 50,
    soc: 90.2 - 0.01,
    timeRemaining: 13.10 - 0.01
  });
}, 5000);

function Visualisation() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

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

8. Intégration avec WebSocket (temps réel)

```jsx
// Connexion WebSocket pour les données du shunt
useEffect(() => {
  const ws = new WebSocket('ws://your-backend/shunt-data');
  
  ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    updateShuntData('shunt-main', {
      current: data.current,
      power: data.power,
      soc: data.soc,
      voltage: data.voltage,
      timeRemaining: data.timeRemaining
    });
  };
  
  return () => ws.close();
}, []);
```

---

9. Résumé des spécificités du Shunt

Propriété Valeur Description
Handles 2 (left + right) Entrée et sortie obligatoires
Direction Dynamique Charge (vert) / Décharge (rouge)
Affichage principal Puissance Grand format avec signe
SOC Jauge circulaire Animation de progression
Temps restant Format hh.hh Calcul automatique

---

10. Processus complet de mise en œuvre

Étape 1 : Créer les fichiers

```bash
# Créer le composant Shunt
touch src/components/nodes/ShuntNode.jsx

# Créer les styles
touch src/components/nodes/shuntAnimations.css
```

Étape 2 : Copier les codes

· Copier le code de la section 4 dans ShuntNode.jsx
· Copier le code de la section 5 dans shuntAnimations.css

Étape 3 : Mettre à jour Visualisation.jsx

· Ajouter l'import de ShuntNode
· Ajouter shunt: ShuntNode dans nodeTypes

Étape 4 : Lancer l'application

```bash
npm run dev
```

---

11. Différence entre Shunt, ET112 et Switch

Type Handles Fonction principale Commande
Shunt 2 fixes Mesure courant/tension Non
ET112 1 ou 2 Mesure énergie Non
Switch 1 ou 2 Commande ON/OFF Oui (toggle)
Battery 1 dynamique Stockage Non

---

Fin du document - Le NodeType Shunt est prêt à être utilisé avec 2 handles pour mesurer le flux entre deux éléments du système électrique.
