DOCUMENTATION TECHNIQUE - NodeType MPPT (Chargeur Solaire)

Version : 1.0
Date : Avril 2026
Statut : Template réutilisable et extensible

---

1. Objectif

Créer un nœud React Flow personnalisé représentant un chargeur MPPT (Maximum Power Point Tracker) avec :

· Affichage de la puissance totale consolidée
· Affichage par MPPT : tension, courant, puissance
· Architecture extensible pour ajouter facilement d'autres MPPT
· Intégration possible dans un flux électrique (batterie, shunt, etc.)

---

2. Structure des données - Modèle extensible

Le composant accepte un tableau mppts qui peut contenir 1 à N MPPT.

```javascript
// Structure de données standard
{
  label: "Chargeur PV",
  totalPower: 1169,           // Puissance totale en W
  mppts: [
    { id: "MPPT-273", voltage: 98.70, current: 1.9, power: 777 },
    { id: "MPPT-289", voltage: 98.71, current: 4.3, power: 423 }
    // Ajouter autant de MPPT que nécessaire
  ]
}
```

---

3. Code complet - NodeTypeMPPT

Fichier : src/components/nodes/MPPTNode.jsx

```jsx
import { Handle, Position } from '@xyflow/react';
import './mpptAnimations.css';

const MPPTNode = ({ id, data }) => {
  // Données d'entrée avec valeurs par défaut
  const {
    label = 'Chargeur PV',
    totalPower = 0,              // Puissance totale en W
    mppts = [],                  // Tableau des MPPT
    // Configuration des handles
    handles = {
      bottom: false,             // Handle bas (sortie vers batterie)
      top: false,                // Handle haut (entrée panneaux)
      left: false,
      right: false
    },
    // Énergie (optionnel)
    energyToday = 0,             // kWh produits aujourd'hui
    energyTotal = 0,             // kWh produits au total
    efficiency = 0               // Rendement en % (optionnel)
  } = data;

  // Calcul automatique de la puissance totale si non fournie
  const computedTotalPower = totalPower > 0 
    ? totalPower 
    : mppts.reduce((sum, mppt) => sum + (mppt.power || 0), 0);

  // Couleur dynamique basée sur la puissance
  const getPowerColor = (power) => {
    if (power === 0) return '#888';
    if (power < 500) return '#ff9800';
    if (power < 1000) return '#4caf50';
    return '#2196f3';
  };

  const totalColor = getPowerColor(computedTotalPower);

  return (
    <div 
      className="mppt-node"
      style={{
        borderColor: totalColor,
        boxShadow: computedTotalPower > 0 ? `0 0 8px ${totalColor}` : 'none'
      }}
    >
      {/* Handles configurables */}
      {handles.top && <Handle type="target" position={Position.Top} id="pv-input" />}
      {handles.bottom && <Handle type="source" position={Position.Bottom} id="battery-output" />}
      {handles.left && <Handle type="target" position={Position.Left} />}
      {handles.right && <Handle type="source" position={Position.Right} />}

      {/* En-tête */}
      <div className="mppt-header">
        <span className="mppt-icon">☀️</span>
        <span className="mppt-label">{label}</span>
        {computedTotalPower > 0 && (
          <span className="mppt-badge" style={{ backgroundColor: totalColor }}>
            ACTIF
          </span>
        )}
      </div>

      {/* Puissance totale */}
      <div className="mppt-total-power" style={{ color: totalColor }}>
        <span className="total-value">{computedTotalPower}</span>
        <span className="total-unit">W</span>
      </div>

      {/* Liste des MPPT */}
      <div className="mppt-list">
        {mppts.map((mppt, index) => {
          const mpptColor = getPowerColor(mppt.power);
          return (
            <div key={mppt.id || index} className="mppt-item">
              <div className="mppt-header-item">
                <span className="mppt-id">{mppt.id}</span>
                <span className="mppt-power" style={{ color: mpptColor }}>
                  {mppt.power} W
                </span>
              </div>
              <div className="mppt-metrics">
                <div className="metric">
                  <span className="metric-label">Tension</span>
                  <span className="metric-value">{mppt.voltage.toFixed(2)} V</span>
                </div>
                <div className="metric">
                  <span className="metric-label">Courant</span>
                  <span className="metric-value" style={{ color: mpptColor }}>
                    {mppt.current.toFixed(1)} A
                  </span>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Section Énergie (optionnelle) */}
      {(energyToday > 0 || energyTotal > 0) && (
        <div className="mppt-energy">
          {energyToday > 0 && (
            <div className="energy-item">
              <span>📅 Aujourd'hui</span>
              <span>{energyToday.toFixed(1)} kWh</span>
            </div>
          )}
          {energyTotal > 0 && (
            <div className="energy-item total">
              <span>📊 Total</span>
              <span>{energyTotal.toFixed(1)} kWh</span>
            </div>
          )}
        </div>
      )}

      {/* Rendement (optionnel) */}
      {efficiency > 0 && (
        <div className="mppt-efficiency">
          <div className="efficiency-bar-bg">
            <div 
              className="efficiency-bar-fill" 
              style={{ width: `${efficiency}%`, backgroundColor: totalColor }}
            />
          </div>
          <span className="efficiency-text">Rendement {efficiency}%</span>
        </div>
      )}
    </div>
  );
};

export default MPPTNode;
```

---

4. Fichier CSS - Animations et styles

Fichier : src/components/nodes/mpptAnimations.css

```css
.mppt-node {
  min-width: 260px;
  background: linear-gradient(135deg, #1a2a1a 0%, #0d1a0d 100%);
  border-radius: 20px;
  padding: 16px;
  border: 2px solid;
  font-family: 'Segoe UI', monospace;
  transition: all 0.3s ease;
}

.mppt-node:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0,0,0,0.3);
}

/* En-tête */
.mppt-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid #2a3a2a;
}

.mppt-icon {
  font-size: 20px;
}

.mppt-label {
  font-weight: bold;
  color: #fff;
  font-size: 13px;
  flex: 1;
}

.mppt-badge {
  font-size: 9px;
  padding: 2px 8px;
  border-radius: 20px;
  color: white;
  font-weight: bold;
}

/* Puissance totale */
.mppt-total-power {
  text-align: center;
  margin-bottom: 16px;
  padding: 8px;
  background: #1a2a1a;
  border-radius: 16px;
}

.total-value {
  font-size: 36px;
  font-weight: bold;
}

.total-unit {
  font-size: 14px;
  margin-left: 4px;
}

/* Liste des MPPT */
.mppt-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 12px;
}

.mppt-item {
  background: #1a2a1a;
  border-radius: 12px;
  padding: 10px;
}

.mppt-header-item {
  display: flex;
  justify-content: space-between;
  margin-bottom: 8px;
}

.mppt-id {
  font-size: 11px;
  font-weight: bold;
  color: #88ff88;
  font-family: monospace;
}

.mppt-power {
  font-size: 13px;
  font-weight: bold;
}

.mppt-metrics {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.mppt-metrics .metric {
  background: #0d1a0d;
  border-radius: 8px;
  padding: 6px;
  text-align: center;
}

.mppt-metrics .metric-label {
  display: block;
  font-size: 8px;
  color: #888;
}

.mppt-metrics .metric-value {
  display: block;
  font-size: 12px;
  font-weight: bold;
  color: #ddd;
}

/* Section Énergie */
.mppt-energy {
  display: flex;
  gap: 12px;
  margin-top: 12px;
  padding-top: 10px;
  border-top: 1px solid #2a3a2a;
}

.mppt-energy .energy-item {
  flex: 1;
  background: #1a2a1a;
  border-radius: 8px;
  padding: 6px;
  text-align: center;
}

.mppt-energy .energy-item span:first-child {
  display: block;
  font-size: 8px;
  color: #888;
}

.mppt-energy .energy-item span:last-child {
  display: block;
  font-size: 11px;
  font-weight: bold;
  color: #ddd;
}

.mppt-energy .energy-item.total span:last-child {
  color: #ff9800;
}

/* Rendement */
.mppt-efficiency {
  margin-top: 12px;
}

.efficiency-bar-bg {
  background: #2a3a2a;
  border-radius: 10px;
  height: 6px;
  overflow: hidden;
  margin-bottom: 6px;
}

.efficiency-bar-fill {
  height: 100%;
  border-radius: 10px;
  transition: width 0.5s ease;
}

.efficiency-text {
  display: block;
  font-size: 9px;
  color: #888;
  text-align: center;
}

/* Animation de pulsation quand production > 0 */
@keyframes solarPulse {
  0%, 100% { opacity: 0.8; }
  50% { opacity: 1; }
}

.mppt-node.producing {
  animation: solarPulse 2s infinite;
}
```

---

5. Exemples d'utilisation - Extension à N MPPT

Exemple 1 : 2 MPPT (basé sur l'image)

```jsx
{
  id: 'mppt-chargeur',
  type: 'mppt',
  position: { x: 100, y: 200 },
  data: {
    label: 'Chargeur PV',
    totalPower: 1169,
    mppts: [
      { id: 'MPPT-273', voltage: 98.70, current: 1.9, power: 777 },
      { id: 'MPPT-289', voltage: 98.71, current: 4.3, power: 423 }
    ]
  }
}
```

Exemple 2 : 3 MPPT (extension)

```jsx
{
  id: 'mppt-3x',
  type: 'mppt',
  position: { x: 100, y: 200 },
  data: {
    label: 'Chargeur PV Tri',
    totalPower: 2450,
    mppts: [
      { id: 'MPPT-01', voltage: 120.5, current: 5.2, power: 626 },
      { id: 'MPPT-02', voltage: 118.3, current: 8.1, power: 958 },
      { id: 'MPPT-03', voltage: 121.0, current: 7.2, power: 871 }
    ],
    energyToday: 18.5,
    energyTotal: 12500,
    efficiency: 96.5
  }
}
```

Exemple 3 : 1 MPPT (simple)

```jsx
{
  id: 'mppt-simple',
  type: 'mppt',
  position: { x: 100, y: 200 },
  data: {
    label: 'Chargeur PV',
    totalPower: 1200,
    mppts: [
      { id: 'MPPT-001', voltage: 110.0, current: 10.9, power: 1200 }
    ]
  }
}
```

Exemple 4 : 4 MPPT (grande installation)

```jsx
{
  id: 'mppt-4x',
  type: 'mppt',
  position: { x: 100, y: 200 },
  data: {
    label: 'Solar Array',
    totalPower: 5200,
    mppts: [
      { id: 'MPPT-A', voltage: 150.2, current: 8.5, power: 1277 },
      { id: 'MPPT-B', voltage: 149.8, current: 9.2, power: 1378 },
      { id: 'MPPT-C', voltage: 151.1, current: 7.8, power: 1179 },
      { id: 'MPPT-D', voltage: 150.5, current: 9.5, power: 1430 }
    ],
    energyToday: 42.8,
    energyTotal: 38500,
    efficiency: 97.2
  }
}
```

---

6. Intégration dans la page de visualisation globale

Fichier : src/pages/VisualisationComplete.jsx (mise à jour)

```jsx
import { ReactFlow, useNodesState, useEdgesState, Background, Controls, MiniMap } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import BatteryNode from '../components/nodes/BatteryNode';
import ET112Node from '../components/nodes/ET112Node';
import SwitchNode from '../components/nodes/SwitchNode';
import ShuntNode from '../components/nodes/ShuntNode';
import MeteoNode from '../components/nodes/MeteoNode';
import TemperatureNode from '../components/nodes/TemperatureNode';
import MPPTNode from '../components/nodes/MPPTNode';  // NOUVEAU

// Déclaration des types de nœuds
const nodeTypes = {
  battery: BatteryNode,
  et112: ET112Node,
  switch: SwitchNode,
  shunt: ShuntNode,
  meteo: MeteoNode,
  temperature: TemperatureNode,
  mppt: MPPTNode,  // NOUVEAU
};

// Configuration complète des nœuds
const initialNodes = [
  // 1. MPPT - Chargeur solaire
  {
    id: 'mppt-chargeur',
    type: 'mppt',
    position: { x: 100, y: 100 },
    data: {
      label: 'Chargeur PV',
      totalPower: 1169,
      mppts: [
        { id: 'MPPT-273', voltage: 98.70, current: 1.9, power: 777 },
        { id: 'MPPT-289', voltage: 98.71, current: 4.3, power: 423 }
      ],
      energyToday: 12.5,
      energyTotal: 3450,
      efficiency: 94.5,
      handles: { bottom: true }  // Sortie vers batterie
    }
  },

  // 2. Shunt - Mesure du courant
  {
    id: 'shunt-main',
    type: 'shunt',
    position: { x: 400, y: 100 },
    data: {
      label: 'Shunt',
      power: 1169,
      voltage: 52.8,
      current: 22.1,
      soc: 90.2,
      handles: { left: true, right: true }
    }
  },

  // 3. Batterie - Stockage
  {
    id: 'battery-360ah',
    type: 'battery',
    position: { x: 400, y: 320 },
    data: {
      label: 'BMS-360Ah',
      soc: 92,
      voltage: 52.8,
      current: 22.1,
      power: 1169,
      energyImported: 1250,
      energyExported: 890
    }
  },

  // 4. Switch - Interrupteur
  {
    id: 'tongou-switch',
    type: 'switch',
    position: { x: 700, y: 100 },
    data: {
      label: 'Tongou Switch',
      isOn: true,
      power: 1160,
      today: 12.4,
      total: 3420,
      handles: { left: true, right: true }
    }
  },

  // 5. ET112 - Compteur final
  {
    id: 'et112-final',
    type: 'et112',
    position: { x: 950, y: 100 },
    data: {
      label: 'ET112',
      power: 1160,
      voltage: 230.4,
      current: 5.04,
      imported: 760.30,
      handles: { left: true }
    }
  },

  // 6. Météo - Irradiance
  {
    id: 'meteo-station',
    type: 'meteo',
    position: { x: 100, y: 500 },
    data: {
      irradiance: 850,
      productionTotal: 31,
      productionDay: 12.5
    }
  },

  // 7. Température
  {
    id: 'temp-station',
    type: 'temperature',
    position: { x: 400, y: 500 },
    data: {
      temperature: 22.5,
      humidity: 53,
      pressure: 1012
    }
  }
];

// Connexions entre les nœuds
const initialEdges = [
  // MPPT → Shunt
  { id: 'e1', source: 'mppt-chargeur', target: 'shunt-main', sourceHandle: 'battery-output', targetHandle: 'left-input' },
  // Shunt → Switch
  { id: 'e2', source: 'shunt-main', target: 'tongou-switch', sourceHandle: 'right-output', targetHandle: 'left-input' },
  // Switch → ET112
  { id: 'e3', source: 'tongou-switch', target: 'et112-final', sourceHandle: 'right-output', targetHandle: 'left-input' },
  // Batterie ↔ Shunt
  { id: 'e4', source: 'battery-360ah', target: 'shunt-main', targetHandle: 'left-input' }
];

function VisualisationComplete() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Simulation de production temps réel
  const updateProduction = () => {
    setNodes((nds) =>
      nds.map((node) => {
        if (node.type === 'mppt') {
          const variation = 0.95 + Math.random() * 0.1;
          const newTotal = Math.round(1169 * variation);
          return {
            ...node,
            data: {
              ...node.data,
              totalPower: newTotal,
              mppts: node.data.mppts.map((mppt, idx) => ({
                ...mppt,
                power: idx === 0 ? Math.round(777 * variation) : Math.round(423 * variation),
                current: idx === 0 ? +(1.9 * variation).toFixed(1) : +(4.3 * variation).toFixed(1)
              }))
            }
          };
        }
        if (node.type === 'shunt' && node.id === 'shunt-main') {
          const mpptNode = nds.find(n => n.id === 'mppt-chargeur');
          const currentPower = mpptNode?.data?.totalPower || 1169;
          return {
            ...node,
            data: {
              ...node.data,
              power: currentPower,
              current: currentPower / 52.8
            }
          };
        }
        return node;
      })
    );
  };

  // Mise à jour toutes les 3 secondes
  setInterval(updateProduction, 3000);

  return (
    <div style={{ width: '100vw', height: '100vh' }}>
      <div style={{
        position: 'absolute',
        top: 10,
        left: 10,
        zIndex: 10,
        background: '#0d1117',
        padding: '8px 16px',
        borderRadius: 8,
        color: '#fff',
        fontSize: 12,
        fontFamily: 'monospace'
      }}>
        🔋 Visualisation Énergétique Complète | MPPT + Shunt + Batterie + Switch + ET112 + Météo
      </div>

      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
        minZoom={0.3}
        maxZoom={1.5}
        defaultViewport={{ x: 0, y: 0, zoom: 0.7 }}
      >
        <Background />
        <Controls />
        <MiniMap 
          nodeColor={(node) => {
            switch(node.type) {
              case 'mppt': return '#4caf50';
              case 'battery': return '#ff9800';
              case 'shunt': return '#2196f3';
              case 'switch': return '#f44336';
              case 'et112': return '#9c27b0';
              case 'meteo': return '#ffeb3b';
              case 'temperature': return '#00bcd4';
              default: return '#888';
            }
          }}
        />
      </ReactFlow>
    </div>
  );
}

export default VisualisationComplete;
```

---

7. Processus complet d'ajout d'un nouveau MPPT

Pour ajouter un MPPT supplémentaire, il suffit de :

1. Ajouter l'objet dans le tableau mppts :

```jsx
mppts: [
  { id: 'MPPT-273', voltage: 98.70, current: 1.9, power: 777 },
  { id: 'MPPT-289', voltage: 98.71, current: 4.3, power: 423 },
  { id: 'MPPT-290', voltage: 98.65, current: 3.2, power: 316 }  // NOUVEAU
]
```

1. Mettre à jour totalPower (optionnel, calcul auto) :

```jsx
totalPower: 1516  // 777 + 423 + 316
```

1. Aucune modification du composant nécessaire - il s'adapte automatiquement.

---

8. Récapitulatif des propriétés

Propriété Type Requis Description
label string Non Nom du chargeur
totalPower number Non Puissance totale (calcul auto si absent)
mppts array Oui Liste des MPPT
mppts[].id string Oui Identifiant du MPPT
mppts[].voltage number Oui Tension en V
mppts[].current number Oui Courant en A
mppts[].power number Oui Puissance en W
energyToday number Non Production du jour (kWh)
energyTotal number Non Production totale (kWh)
efficiency number Non Rendement (%)
handles object Non Configuration des connexions

---

9. Structure finale des fichiers

```
src/
├── components/
│   └── nodes/
│       ├── BatteryNode.jsx
│       ├── ET112Node.jsx
│       ├── SwitchNode.jsx
│       ├── ShuntNode.jsx
│       ├── MeteoNode.jsx
│       ├── TemperatureNode.jsx
│       └── MPPTNode.jsx           (NOUVEAU)
├── pages/
│   └── VisualisationComplete.jsx
└── styles/
    ├── batteryAnimations.css
    ├── et112Animations.css
    ├── switchAnimations.css
    ├── shuntAnimations.css
    ├── meteoAnimations.css
    ├── temperatureAnimations.css
    └── mpptAnimations.css         (NOUVEAU)
```

---

10. Commandes d'installation

```bash
# Créer les fichiers
touch src/components/nodes/MPPTNode.jsx
touch src/styles/mpptAnimations.css

# Mettre à jour VisualisationComplete.jsx avec les imports

# Lancer l'application
npm run dev
```

---

Fin du document - Le NodeTypeMPPT est prêt, extensible à N MPPT, et intégré dans la visualisation globale avec tous les autres types de nœuds.
