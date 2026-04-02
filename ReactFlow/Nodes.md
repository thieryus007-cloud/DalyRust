---

DOCUMENTATION - Visualisation Énergétique Complète avec React Flow

Version : 1.0
Date : Avril 2026
Statut : Template intégré

---

1. Objectif

Créer une page de visualisation unique intégrant tous les types de nœuds développés :

· BatteryNode (stockage)
· ET112Node (compteur / mesure)
· SwitchNode (interrupteur commandable)
· ShuntNode (capteur de courant)

Chaque nœud est enrichi avec des mesures d'énergie (importée/exportée ou consommée) pour une supervision complète du flux électrique.

---

2. Architecture du système visualisé

```
[Source] → [Shunt] → [Switch] → [ET112] → [Charge]
              ↓
         [Battery]
```

Flux d'énergie :

· La Source (ex: panneaux solaires) alimente le système
· Le Shunt mesure le courant entrant/sortant de la batterie
· La Batterie stocke ou restitue l'énergie
· Le Switch commande l'alimentation de la charge
· L'ET112 mesure la consommation finale

---

3. Mise à jour de chaque composant avec mesure d'énergie

3.1 BatteryNode - Ajout des compteurs

Fichier : src/components/nodes/BatteryNode.jsx (version enrichie)

```jsx
import { Handle, Position } from '@xyflow/react';
import './batteryAnimations.css';

const BatteryNode = ({ id, data }) => {
  const {
    label = 'BMS',
    soc = 0,
    voltage = 0,
    current = 0,
    temperature = 0,
    power = 0,
    // NOUVEAUX CHAMPS ÉNERGIE
    energyImported = 0,    // kWh importés (charge)
    energyExported = 0,    // kWh exportés (décharge)
    energyTotal = 0        // kWh totaux traités
  } = data;

  const isCharging = current > 0;
  const isDischarging = current < 0;
  const currentColor = isCharging ? '#4caf50' : (isDischarging ? '#f44336' : '#ff9800');
  const handleType = isCharging ? 'target' : (isDischarging ? 'source' : null);

  return (
    <div className="battery-node" style={{ borderColor: currentColor, boxShadow: `0 0 8px ${currentColor}` }}>
      {handleType && <Handle type={handleType} position={Position.Bottom} style={{ background: currentColor }} />}

      <div className="battery-header">
        <span>🔋 {label}</span>
        <span style={{ color: currentColor }}>{isCharging ? 'CHARGE' : 'DÉCHARGE'}</span>
      </div>

      <div className="battery-soc">
        <span className="soc-value">{soc}%</span>
        <div className="soc-bar-bg"><div className="soc-bar-fill" style={{ width: `${soc}%`, backgroundColor: currentColor }} /></div>
      </div>

      <div className="battery-metrics">
        <div className="metric"><span>TENSION</span><span>{voltage.toFixed(1)}V</span></div>
        <div className="metric"><span>COURANT</span><span style={{ color: currentColor }}>{Math.abs(current).toFixed(1)}A</span></div>
        <div className="metric"><span>TEMP.</span><span>{temperature.toFixed(1)}°C</span></div>
        <div className="metric"><span>PUISSANCE</span><span style={{ color: currentColor }}>{Math.abs(power).toFixed(0)}W</span></div>
      </div>

      {/* SECTION ÉNERGIE */}
      <div className="battery-energy">
        <div className="energy-item">
          <span>📥 IMPORTÉ</span>
          <span>{energyImported.toFixed(1)} kWh</span>
        </div>
        <div className="energy-item">
          <span>📤 EXPORTÉ</span>
          <span>{energyExported.toFixed(1)} kWh</span>
        </div>
        <div className="energy-item total">
          <span>🔁 TOTAL</span>
          <span>{energyTotal.toFixed(1)} kWh</span>
        </div>
      </div>
    </div>
  );
};

export default BatteryNode;
```

---

3.2 ShuntNode - Ajout des compteurs

Fichier : src/components/nodes/ShuntNode.jsx (version enrichie)

```jsx
// Dans la section des métriques, AJOUTER :
const {
  // ... existants
  energyDay = 0,       // kWh aujourd'hui
  energyWeek = 0,      // kWh cette semaine
  energyMonth = 0,     // kWh ce mois
  energyTotal = 0      // kWh total
} = data;

// AJOUTER DANS LE JSX :
<div className="shunt-energy">
  <div className="energy-header">📊 ÉNERGIE MESURÉE</div>
  <div className="energy-grid">
    <div className="energy-item"><span>AUJOURD'HUI</span><span>{energyDay.toFixed(1)} kWh</span></div>
    <div className="energy-item"><span>CETTE SEMAINE</span><span>{energyWeek.toFixed(1)} kWh</span></div>
    <div className="energy-item"><span>CE MOIS</span><span>{energyMonth.toFixed(1)} kWh</span></div>
    <div className="energy-item total"><span>TOTAL</span><span>{energyTotal.toFixed(1)} kWh</span></div>
  </div>
</div>
```

---

3.3 SwitchNode - Ajout des compteurs (déjà présent)

Le SwitchNode a déjà les champs today, yesterday, total dans sa configuration.

---

3.4 ET112Node - Ajout des compteurs (déjà présent)

L'ET112Node a déjà les champs imported et exported.

---

4. Styles CSS communs pour les sections énergie

Fichier : src/styles/energyCommon.css

```css
/* Styles communs pour tous les nœuds - Section ÉNERGIE */

.energy-section {
  margin-top: 12px;
  padding-top: 10px;
  border-top: 1px solid #2a2f3e;
}

.energy-header {
  font-size: 9px;
  color: #888;
  letter-spacing: 1px;
  margin-bottom: 8px;
  text-align: center;
}

.energy-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}

.energy-item {
  background: #1a1f2e;
  border-radius: 8px;
  padding: 6px 8px;
  text-align: center;
}

.energy-item span:first-child {
  display: block;
  font-size: 8px;
  color: #666;
}

.energy-item span:last-child {
  display: block;
  font-size: 11px;
  font-weight: bold;
  color: #ddd;
}

.energy-item.total {
  grid-column: span 2;
  background: #252a3e;
}

.energy-item.total span:last-child {
  color: #58a6ff;
  font-size: 13px;
}

/* Version batterie */
.battery-energy {
  margin-top: 12px;
  display: flex;
  gap: 8px;
  justify-content: space-between;
}

.battery-energy .energy-item {
  flex: 1;
  background: #1a1f2e;
  border-radius: 8px;
  padding: 6px;
  text-align: center;
}

/* Version shunt */
.shunt-energy {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid #2a2f3e;
}
```

---

5. Page de visualisation complète

Fichier : src/pages/VisualisationComplete.jsx

```jsx
import { ReactFlow, useNodesState, useEdgesState, Background, Controls, MiniMap } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import BatteryNode from '../components/nodes/BatteryNode';
import ET112Node from '../components/nodes/ET112Node';
import SwitchNode from '../components/nodes/SwitchNode';
import ShuntNode from '../components/nodes/ShuntNode';
import '../styles/energyCommon.css';

// Déclaration des types de nœuds
const nodeTypes = {
  battery: BatteryNode,
  et112: ET112Node,
  switch: SwitchNode,
  shunt: ShuntNode,
};

// Configuration des nœuds - TOUS AVEC MESURES D'ÉNERGIE
const initialNodes = [
  // 1. SHUNT - Mesure principale
  {
    id: 'shunt-main',
    type: 'shunt',
    position: { x: 250, y: 250 },
    data: {
      label: 'Shunt Victron',
      status: 'Décharge en cours',
      power: -1664,
      soc: 90.2,
      voltage: 52.81,
      current: -31.5,
      timeRemaining: 13.10,
      handles: { left: true, right: true },
      // ÉNERGIE MESURÉE
      energyDay: 24.5,
      energyWeek: 168.2,
      energyMonth: 720.5,
      energyTotal: 12500.0
    }
  },

  // 2. BATTERIE - Stockage
  {
    id: 'battery-360ah',
    type: 'battery',
    position: { x: 250, y: 480 },
    data: {
      label: 'BMS-360Ah',
      soc: 92,
      voltage: 52.8,
      current: -17.3,
      temperature: 14.0,
      power: -910,
      // ÉNERGIE STOCKÉE
      energyImported: 1250.5,   // kWh chargés
      energyExported: 890.3,    // kWh déchargés
      energyTotal: 2140.8       // kWh traités
    }
  },

  // 3. SWITCH - Interrupteur commandable
  {
    id: 'tongou-switch',
    type: 'switch',
    position: { x: 500, y: 250 },
    data: {
      label: 'Tongou Switch',
      deviceId: 'tongou_3BC764',
      time: '20:17:48',
      isOn: true,
      power: 2.0,
      voltage: 231.0,
      current: 0.04,
      cosPhi: 0.26,
      // ÉNERGIE CONSOMMÉE
      today: 4.26,
      yesterday: 2.62,
      total: 42.3,
      handles: { left: true, right: true }
    }
  },

  // 4. ET112 - Compteur final
  {
    id: 'et112-final',
    type: 'et112',
    position: { x: 750, y: 250 },
    data: {
      label: 'ET112',
      deviceId: '0x07',
      time: '20:09:42',
      power: 1664.0,
      voltage: 230.4,
      current: 7.23,
      type: 'load',
      // ÉNERGIE IMPORT/EXPORT
      imported: 760.30,
      exported: 0.00,
      handles: { left: true, right: false }
    }
  },

  // 5. Charge finale (nœud simple)
  {
    id: 'load-final',
    type: 'default',
    position: { x: 950, y: 250 },
    data: { label: '⚡ Charge AC' }
  }
];

// Connexions entre les nœuds
const initialEdges = [
  // Shunt → Switch
  { id: 'e1', source: 'shunt-main', target: 'tongou-switch', sourceHandle: 'right-output', targetHandle: 'left-input' },
  // Switch → ET112
  { id: 'e2', source: 'tongou-switch', target: 'et112-final', sourceHandle: 'right-output', targetHandle: 'left-input' },
  // ET112 → Charge
  { id: 'e3', source: 'et112-final', target: 'load-final', sourceHandle: 'right-output' },
  // Batterie ↔ Shunt (flux bidirectionnel)
  { id: 'e4', source: 'battery-360ah', target: 'shunt-main', targetHandle: 'left-input' }
];

function VisualisationComplete() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Mise à jour temps réel de l'énergie
  const updateEnergyData = () => {
    setNodes((nds) =>
      nds.map((node) => {
        // Incrémentation des compteurs énergie
        if (node.type === 'battery') {
          return {
            ...node,
            data: {
              ...node.data,
              energyImported: node.data.energyImported + 0.01,
              energyTotal: node.data.energyTotal + 0.01
            }
          };
        }
        if (node.type === 'shunt') {
          return {
            ...node,
            data: {
              ...node.data,
              energyDay: node.data.energyDay + 0.05,
              energyWeek: node.data.energyWeek + 0.05,
              energyMonth: node.data.energyMonth + 0.05,
              energyTotal: node.data.energyTotal + 0.05
            }
          };
        }
        if (node.type === 'switch') {
          return {
            ...node,
            data: {
              ...node.data,
              today: node.data.today + 0.01,
              total: node.data.total + 0.01
            }
          };
        }
        if (node.type === 'et112') {
          return {
            ...node,
            data: {
              ...node.data,
              imported: node.data.imported + 0.02
            }
          };
        }
        return node;
      })
    );
  };

  // Simulation de consommation temps réel
  setInterval(updateEnergyData, 3000);

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
        🔋 Visualisation Énergétique | Tous les compteurs sont en kWh
      </div>

      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
        minZoom={0.5}
        maxZoom={1.5}
        defaultViewport={{ x: 0, y: 0, zoom: 0.8 }}
      >
        <Background />
        <Controls />
        <MiniMap 
          nodeColor={(node) => {
            switch(node.type) {
              case 'battery': return '#4caf50';
              case 'shunt': return '#2196f3';
              case 'switch': return '#ff9800';
              case 'et112': return '#9c27b0';
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

6. Point d'entrée de l'application

Fichier : src/App.jsx

```jsx
import VisualisationComplete from './pages/VisualisationComplete';

function App() {
  return <VisualisationComplete />;
}

export default App;
```

---

7. Récapitulatif des mesures d'énergie par type de nœud

Type de nœud Champs énergie ajoutés Unité
BatteryNode energyImported, energyExported, energyTotal kWh
ShuntNode energyDay, energyWeek, energyMonth, energyTotal kWh
SwitchNode today, yesterday, total (existants) kWh
ET112Node imported, exported (existants) kWh

---

8. Structure finale des fichiers

```
src/
├── components/
│   └── nodes/
│       ├── BatteryNode.jsx      (avec énergie)
│       ├── ET112Node.jsx         (avec énergie)
│       ├── SwitchNode.jsx        (avec énergie)
│       └── ShuntNode.jsx         (avec énergie)
├── pages/
│   └── VisualisationComplete.jsx
├── styles/
│   ├── batteryAnimations.css
│   ├── et112Animations.css
│   ├── switchAnimations.css
│   ├── shuntAnimations.css
│   └── energyCommon.css          (NOUVEAU - styles communs)
└── App.jsx
```

---

9. Processus complet d'installation

```bash
# 1. Créer le projet
npm create vite@latest energie-viz -- --template react
cd energie-viz

# 2. Installer React Flow
npm install @xyflow/react

# 3. Créer la structure des dossiers
mkdir -p src/components/nodes
mkdir -p src/pages
mkdir -p src/styles

# 4. Créer tous les fichiers (copier les codes ci-dessus)

# 5. Lancer l'application
npm run dev
```

---

10. Visualisation attendue

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  🔋 Visualisation Énergétique | Tous les compteurs sont en kWh              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                    ┌──────────────┐     ┌──────────────┐    ┌─────────────┐│
│                    │    Shunt     │────▶│    Switch    │───▶│    ET112    ││
│                    │  -1664 W     │     │    ON 2.0W   │    │  1664.0 W   ││
│                    │  90.2%       │     │  Auj:4.26kWh │    │ Imp:760kWh  ││
│                    │  Jour:24.5   │     │  Tot:42.3kWh │    │             ││
│                    └──────┬───────┘     └──────────────┘    └──────┬──────┘│
│                           │                                        │       │
│                           │                                        ▼       │
│                    ┌──────┴───────┐                          ┌─────────────┐│
│                    │   Batterie   │                          │   Charge    ││
│                    │   92% -910W  │                          │    AC       ││
│                    │  Imp:1250kWh │                          └─────────────┘│
│                    │  Exp:890kWh  │                                            │
│                    └──────────────┘                                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

11. Évolutions possibles

Fonctionnalité Description
Export CSV Télécharger l'historique des compteurs
Graphiques Courbes d'évolution de l'énergie
Alertes Seuils de consommation/dépassement
API REST Sauvegarde des données en base
MQTT Communication temps réel avec équipements

---

Fin du document - Vous disposez maintenant d'une visualisation complète avec tous les types de nœuds intégrant des mesures d'énergie.
