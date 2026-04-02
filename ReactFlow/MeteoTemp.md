---

DOCUMENTATION TECHNIQUE - NodeType Meteo & Temperature pour React Flow

Version : 1.0
Date : Avril 2026
Statut : Template réutilisable

---

1. Objectif

Créer deux nouveaux nœuds React Flow personnalisés pour la supervision météorologique :

NodeTypeMeteo (partie haute)

· Affichage de l'irradiance solaire (W/m²)
· Affichage de la production énergétique (kWh)
· Graphique simplifié des dernières 24h
· Statistiques de production du jour

NodeTypeTemperature (partie basse)

· Affichage de la température extérieure
· Humidité relative (%)
· Pression barométrique (hPa)
· Min/Max des dernières 24h
· Horodatage

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
│       ├── MeteoNode.jsx          (à créer)
│       └── TemperatureNode.jsx    (à créer)
├── pages/
│   └── VisualisationComplete.jsx
└── styles/
    ├── meteoAnimations.css
    └── temperatureAnimations.css
```

---

4. Code complet - NodeTypeMeteo

Fichier : src/components/nodes/MeteoNode.jsx

```jsx
import { Handle, Position } from '@xyflow/react';
import './meteoAnimations.css';

const MeteoNode = ({ id, data }) => {
  const {
    label = 'Station Solaire',
    irradiance = 1.0,           // W/m²
    productionTotal = 31,       // kWh production totale
    productionLast24h = 30.6,   // kWh dernières 24h
    productionDay = 31,         // kWh production du jour
    lastUpdate = 'il y a quelques secondes',
    // Configuration des handles (optionnel)
    handles = {
      bottom: false,  // handle bas pour connexion
      top: false      // handle haut
    }
  } = data;

  // Calcul de la hauteur de la barre de production (max 50 kWh)
  const barHeight = Math.min((productionLast24h / 50) * 60, 60);
  
  // Données pour le mini graphique (simulées)
  const hourlyData = [2.1, 3.5, 5.2, 8.1, 12.4, 18.7, 22.3, 30.6, 28.4, 24.1, 18.2, 12.5];

  return (
    <div className="meteo-node">
      {/* Handles optionnels */}
      {handles.top && <Handle type="target" position={Position.Top} />}
      {handles.bottom && <Handle type="source" position={Position.Bottom} />}

      {/* En-tête */}
      <div className="meteo-header">
        <span className="meteo-icon">☀️</span>
        <span className="meteo-label">{label}</span>
      </div>

      {/* Irradiance principale */}
      <div className="meteo-irradiance">
        <span className="irradiance-value">{irradiance.toFixed(1)}</span>
        <span className="irradiance-unit">W/m²</span>
      </div>

      {/* Production totale */}
      <div className="meteo-production-total">
        <span className="production-sign">-</span>
        <span className="production-value">{productionTotal}</span>
        <span className="production-unit">kWh</span>
      </div>

      {/* Mini graphique 24h */}
      <div className="meteo-chart">
        <div className="chart-header">
          <span>Dernières 24 h</span>
          <span className="chart-value">{productionLast24h.toFixed(1)} kWh</span>
        </div>
        <div className="chart-bars">
          {hourlyData.slice(0, 12).map((value, idx) => (
            <div 
              key={idx} 
              className="chart-bar"
              style={{ height: `${(value / 35) * 40}px` }}
            />
          ))}
        </div>
      </div>

      {/* Production du jour */}
      <div className="meteo-production-day">
        <span className="day-label">Production du jour</span>
        <span className="day-value">{productionDay} kWh</span>
      </div>

      {/* Dernière mise à jour */}
      <div className="meteo-update">
        Dernière mise à jour<br />
        {lastUpdate}
      </div>
    </div>
  );
};

export default MeteoNode;
```

---

5. Fichier CSS - NodeTypeMeteo

Fichier : src/components/nodes/meteoAnimations.css

```css
.meteo-node {
  min-width: 220px;
  background: linear-gradient(135deg, #1a1a2e 0%, #0d1117 100%);
  border-radius: 20px;
  padding: 16px;
  border: 1px solid #2a2f3e;
  font-family: 'Segoe UI', monospace;
  text-align: center;
  transition: all 0.3s ease;
}

.meteo-node:hover {
  transform: translateY(-2px);
  border-color: #ff9800;
  box-shadow: 0 0 12px rgba(255, 152, 0, 0.3);
}

/* En-tête */
.meteo-header {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-bottom: 16px;
}

.meteo-icon {
  font-size: 24px;
}

.meteo-label {
  font-size: 12px;
  color: #888;
  letter-spacing: 1px;
}

/* Irradiance */
.meteo-irradiance {
  margin-bottom: 8px;
}

.irradiance-value {
  font-size: 42px;
  font-weight: bold;
  color: #ff9800;
}

.irradiance-unit {
  font-size: 14px;
  color: #888;
  margin-left: 4px;
}

/* Production totale */
.meteo-production-total {
  margin-bottom: 20px;
}

.production-sign {
  font-size: 28px;
  font-weight: bold;
  color: #ff9800;
}

.production-value {
  font-size: 28px;
  font-weight: bold;
  color: #ff9800;
}

.production-unit {
  font-size: 12px;
  color: #888;
  margin-left: 4px;
}

/* Graphique */
.meteo-chart {
  background: #1a1f2e;
  border-radius: 12px;
  padding: 10px;
  margin-bottom: 16px;
}

.chart-header {
  display: flex;
  justify-content: space-between;
  font-size: 10px;
  color: #888;
  margin-bottom: 10px;
}

.chart-value {
  color: #ff9800;
  font-weight: bold;
}

.chart-bars {
  display: flex;
  align-items: flex-end;
  gap: 4px;
  height: 50px;
}

.chart-bar {
  flex: 1;
  background: #ff9800;
  border-radius: 2px;
  transition: height 0.3s ease;
  opacity: 0.7;
}

.chart-bar:hover {
  opacity: 1;
}

/* Production du jour */
.meteo-production-day {
  display: flex;
  justify-content: space-between;
  background: #1a1f2e;
  border-radius: 10px;
  padding: 8px 12px;
  margin-bottom: 12px;
}

.day-label {
  font-size: 10px;
  color: #888;
}

.day-value {
  font-size: 14px;
  font-weight: bold;
  color: #ff9800;
}

/* Mise à jour */
.meteo-update {
  font-size: 9px;
  color: #555;
  text-align: center;
  line-height: 1.4;
}
```

---

6. Code complet - NodeTypeTemperature

Fichier : src/components/nodes/TemperatureNode.jsx

```jsx
import { Handle, Position } from '@xyflow/react';
import './temperatureAnimations.css';

const TemperatureNode = ({ id, data }) => {
  const {
    label = 'Station Météo',
    temperature = 10.2,        // °C
    humidity = 53,              // %
    pressure = 1007.0,          // hPa
    tempMin24h = 8.5,           // °C min dernières 24h
    tempMax24h = 14.6,          // °C max dernières 24h
    lastUpdate = 'il y a quelques secondes',
    // Configuration des handles
    handles = {
      bottom: false,
      top: false
    }
  } = data;

  return (
    <div className="temperature-node">
      {/* Handles optionnels */}
      {handles.top && <Handle type="target" position={Position.Top} />}
      {handles.bottom && <Handle type="source" position={Position.Bottom} />}

      {/* En-tête */}
      <div className="temp-header">
        <span className="temp-icon">🌡️</span>
        <span className="temp-label">{label}</span>
      </div>

      {/* Température extérieure */}
      <div className="temp-main">
        <span className="temp-label-main">Température Extérieure</span>
        <div className="temp-value-container">
          <span className="temp-value">{temperature.toFixed(1)}</span>
          <span className="temp-unit">°C</span>
        </div>
      </div>

      {/* Humidité et Pression */}
      <div className="temp-metrics">
        <div className="metric-circle">
          <span className="metric-value">{humidity}%</span>
          <span className="metric-label">Humidité relative</span>
        </div>
        <div className="metric-circle">
          <span className="metric-value">{pressure.toFixed(1)}</span>
          <span className="metric-label">Pression barométrique</span>
          <span className="metric-sub">hPa</span>
        </div>
      </div>

      {/* Min/Max 24h */}
      <div className="temp-range">
        <div className="range-item">
          <span className="range-label">Dernières 24 h</span>
          <div className="range-values">
            <span className="range-min">{tempMin24h.toFixed(1)}°C min</span>
            <span className="range-max">{tempMax24h.toFixed(1)}°C max</span>
          </div>
        </div>
      </div>

      {/* Dernière mise à jour */}
      <div className="temp-update">
        Dernière mise à jour<br />
        {lastUpdate}
      </div>
    </div>
  );
};

export default TemperatureNode;
```

---

7. Fichier CSS - NodeTypeTemperature

Fichier : src/components/nodes/temperatureAnimations.css

```css
.temperature-node {
  min-width: 220px;
  background: linear-gradient(135deg, #1a2a3a 0%, #0d1520 100%);
  border-radius: 20px;
  padding: 16px;
  border: 1px solid #2a4a6a;
  font-family: 'Segoe UI', monospace;
  text-align: center;
  transition: all 0.3s ease;
}

.temperature-node:hover {
  transform: translateY(-2px);
  border-color: #2196f3;
  box-shadow: 0 0 12px rgba(33, 150, 243, 0.3);
}

/* En-tête */
.temp-header {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-bottom: 16px;
}

.temp-icon {
  font-size: 24px;
}

.temp-label {
  font-size: 12px;
  color: #888;
  letter-spacing: 1px;
}

/* Température principale */
.temp-main {
  margin-bottom: 20px;
}

.temp-label-main {
  display: block;
  font-size: 10px;
  color: #888;
  margin-bottom: 8px;
}

.temp-value-container {
  display: flex;
  align-items: baseline;
  justify-content: center;
  gap: 4px;
}

.temp-value {
  font-size: 48px;
  font-weight: bold;
  color: #2196f3;
}

.temp-unit {
  font-size: 16px;
  color: #888;
}

/* Métriques circulaires */
.temp-metrics {
  display: flex;
  gap: 16px;
  justify-content: center;
  margin-bottom: 20px;
}

.metric-circle {
  background: #1a2a3a;
  border-radius: 60px;
  padding: 10px 16px;
  text-align: center;
  min-width: 80px;
}

.metric-circle .metric-value {
  display: block;
  font-size: 20px;
  font-weight: bold;
  color: #4caf50;
}

.metric-circle .metric-label {
  display: block;
  font-size: 8px;
  color: #888;
}

.metric-sub {
  font-size: 8px;
  color: #666;
}

/* Plage de température */
.temp-range {
  background: #1a2a3a;
  border-radius: 12px;
  padding: 10px;
  margin-bottom: 12px;
}

.range-item {
  text-align: center;
}

.range-label {
  display: block;
  font-size: 9px;
  color: #888;
  margin-bottom: 6px;
}

.range-values {
  display: flex;
  justify-content: center;
  gap: 16px;
}

.range-min {
  font-size: 11px;
  color: #64b5f6;
}

.range-max {
  font-size: 11px;
  color: #ff8a65;
}

/* Mise à jour */
.temp-update {
  font-size: 9px;
  color: #555;
  text-align: center;
  line-height: 1.4;
}
```

---

8. Intégration dans la page de visualisation

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

const nodeTypes = {
  battery: BatteryNode,
  et112: ET112Node,
  switch: SwitchNode,
  shunt: ShuntNode,
  meteo: MeteoNode,
  temperature: TemperatureNode,
};

const initialNodes = [
  // NodeTypeMeteo - Irradiance et production
  {
    id: 'meteo-station',
    type: 'meteo',
    position: { x: 100, y: 50 },
    data: {
      label: 'Station Solaire',
      irradiance: 1.0,
      productionTotal: 31,
      productionLast24h: 30.6,
      productionDay: 31,
      lastUpdate: 'il y a quelques secondes'
    }
  },
  
  // NodeTypeTemperature - Conditions extérieures
  {
    id: 'temp-station',
    type: 'temperature',
    position: { x: 100, y: 300 },
    data: {
      label: 'Station Météo',
      temperature: 10.2,
      humidity: 53,
      pressure: 1007.0,
      tempMin24h: 8.5,
      tempMax24h: 14.6,
      lastUpdate: 'il y a quelques secondes'
    }
  },
  
  // Autres nœuds (batterie, shunt, etc.)
  // ... (conserver les nœuds existants)
];

function VisualisationComplete() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  // Simulation de mise à jour temps réel
  setInterval(() => {
    setNodes((nds) =>
      nds.map((node) => {
        if (node.type === 'meteo') {
          return {
            ...node,
            data: {
              ...node.data,
              irradiance: Math.random() * 800 + 100,
              productionDay: node.data.productionDay + 0.1
            }
          };
        }
        if (node.type === 'temperature') {
          return {
            ...node,
            data: {
              ...node.data,
              temperature: 10.2 + (Math.random() - 0.5) * 2,
              humidity: 53 + (Math.random() - 0.5) * 5
            }
          };
        }
        return node;
      })
    );
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
        <MiniMap />
      </ReactFlow>
    </div>
  );
}

export default VisualisationComplete;
```

---

9. Récapitulatif des données

NodeTypeMeteo

Champ Type Description Exemple
irradiance number Rayonnement solaire 1.0 W/m²
productionTotal number Production totale 31 kWh
productionLast24h number Production dernières 24h 30.6 kWh
productionDay number Production du jour 31 kWh
lastUpdate string Horodatage "il y a quelques secondes"

NodeTypeTemperature

Champ Type Description Exemple
temperature number Température extérieure 10.2 °C
humidity number Humidité relative 53 %
pressure number Pression barométrique 1007.0 hPa
tempMin24h number Température min 24h 8.5 °C
tempMax24h number Température max 24h 14.6 °C
lastUpdate string Horodatage "il y a quelques secondes"

---

10. Processus complet d'installation

```bash
# 1. Créer les fichiers
touch src/components/nodes/MeteoNode.jsx
touch src/components/nodes/TemperatureNode.jsx
touch src/styles/meteoAnimations.css
touch src/styles/temperatureAnimations.css

# 2. Copier les codes dans les fichiers correspondants

# 3. Mettre à jour VisualisationComplete.jsx avec les imports

# 4. Lancer l'application
npm run dev
```

---

11. Visualisation attendue

```
┌─────────────────────────────────────────────────────────────────┐
│  ┌─────────────────────┐                                        │
│  │ ☀️ Station Solaire   │                                        │
│  │                     │                                        │
│  │     1.0 W/m²        │                                        │
│  │                     │                                        │
│  │     -31 kWh         │                                        │
│  │                     │                                        │
│  │ Dernières 24 h      │                                        │
│  │ ▓▓▓▓▓▓▓▓▓▓ 30.6 kWh │                                        │
│  │                     │                                        │
│  │ Production du jour  │                                        │
│  │       31 kWh        │                                        │
│  └─────────────────────┘                                        │
│                                                                 │
│  ┌─────────────────────┐                                        │
│  │ 🌡️ Station Météo    │                                        │
│  │                     │                                        │
│  │   Température       │                                        │
│  │     10.2 °C         │                                        │
│  │                     │                                        │
│  │  53%     1007.0 hPa │                                        │
│  │                     │                                        │
│  │ Dernières 24 h      │                                        │
│  │ 8.5°C min  14.6°C max│                                       │
│  └─────────────────────┘                                        │
└─────────────────────────────────────────────────────────────────┘
```

---

Fin du document - Les NodeTypeMeteo et NodeTypeTemperature sont prêts à être intégrés dans votre visualisation énergétique complète.
