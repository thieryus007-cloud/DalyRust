Voici tous les fichiers impactés par ces deux modifications de connexion.

---

1. Fichier : src/components/nodes/ShuntNode.jsx

Modification : Ajout d'un handle sur le côté DROIT pour la connexion avec MPPT

```jsx
import { Handle, Position } from '@xyflow/react';

const ShuntNode = ({ id, data }) => {
  const {
    label = 'Shunt',
    power = 0,
    voltage = 0,
    current = 0,
    soc = 0
  } = data;

  const isCharging = current > 0;
  const isDischarging = current < 0;
  const flowColor = isCharging ? '#4caf50' : (isDischarging ? '#f44336' : '#ff9800');

  return (
    <div 
      className="shunt-node"
      style={{
        backgroundColor: '#ffffff',
        borderRadius: '12px',
        padding: '8px',
        minWidth: '140px',
        border: '1.5px solid',
        borderColor: flowColor,
        fontFamily: 'Segoe UI, monospace',
        boxShadow: `0 0 4px ${flowColor}`
      }}
    >
      {/* Handle BAS - reçoit des batteries */}
      <Handle 
        type="target"
        position={Position.Bottom}
        id="bottom-input"
        style={{ 
          background: flowColor,
          width: '10px',
          height: '10px',
          bottom: '-5px'
        }}
      />

      {/* Handle DROIT - connexion vers MPPT */}
      <Handle 
        type="source"
        position={Position.Right}
        id="right-output"
        style={{ 
          background: flowColor,
          width: '10px',
          height: '10px',
          right: '-5px'
        }}
      />

      <div style={{ textAlign: 'center', marginBottom: '8px' }}>
        <span style={{ fontSize: '9px', color: '#888' }}>{label}</span>
        <div style={{ fontSize: '22px', fontWeight: 'bold', color: flowColor }}>
          {Math.abs(power).toFixed(0)} W
        </div>
      </div>

      <div style={{ textAlign: 'center', marginBottom: '8px' }}>
        <div style={{ position: 'relative', width: '60px', height: '60px', margin: '0 auto' }}>
          <svg viewBox="0 0 100 100" style={{ width: '100%', height: '100%', transform: 'rotate(-90deg)' }}>
            <circle cx="50" cy="50" r="45" fill="none" stroke="#e0e0e0" strokeWidth="8" />
            <circle 
              cx="50" cy="50" r="45" fill="none" stroke={flowColor} strokeWidth="8"
              strokeDasharray={`${(soc / 100) * 283} 283`}
              strokeLinecap="round"
            />
          </svg>
          <div style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)', fontSize: '11px', fontWeight: 'bold' }}>
            {soc}%
          </div>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '5px' }}>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '4px', textAlign: 'center' }}>
          <div style={{ fontSize: '6px', color: '#888' }}>TENSION</div>
          <div style={{ fontSize: '9px', fontWeight: 'bold' }}>{voltage.toFixed(2)} V</div>
        </div>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '4px', textAlign: 'center' }}>
          <div style={{ fontSize: '6px', color: '#888' }}>COURANT</div>
          <div style={{ fontSize: '9px', fontWeight: 'bold', color: flowColor }}>{Math.abs(current).toFixed(1)} A</div>
        </div>
      </div>
    </div>
  );
};

export default ShuntNode;
```

---

2. Fichier : src/components/nodes/MPPTNode.jsx

Modification : Ajout d'un handle sur le côté GAUCHE pour la connexion avec Shunt

```jsx
import { Handle, Position } from '@xyflow/react';

const MPPTNode = ({ id, data }) => {
  const {
    label = 'Chargeur PV',
    totalPower = 0,
    mppts = [],
    energyToday = 0,
    energyTotal = 0,
    efficiency = 0
  } = data;

  const totalColor = totalPower > 0 ? '#4caf50' : '#888';

  return (
    <div 
      className="mppt-node"
      style={{
        backgroundColor: '#ffffff',
        borderRadius: '12px',
        padding: '8px',
        minWidth: '140px',
        border: '1.5px solid',
        borderColor: totalColor,
        fontFamily: 'Segoe UI, monospace',
        boxShadow: totalPower > 0 ? `0 0 4px ${totalColor}` : 'none'
      }}
    >
      {/* Handle GAUCHE - reçoit du Shunt */}
      <Handle 
        type="target"
        position={Position.Left}
        id="left-input"
        style={{ 
          background: totalColor,
          width: '10px',
          height: '10px',
          left: '-5px'
        }}
      />

      <div style={{ display: 'flex', alignItems: 'center', gap: '5px', marginBottom: '6px', paddingBottom: '4px', borderBottom: '1px solid #e0e0e0' }}>
        <span style={{ fontSize: '12px' }}>☀️</span>
        <span style={{ fontWeight: 'bold', color: '#333', fontSize: '8px', flex: 1 }}>{label}</span>
        {totalPower > 0 && (
          <span style={{ fontSize: '6px', padding: '1px 5px', borderRadius: '10px', backgroundColor: totalColor, color: 'white' }}>ACTIF</span>
        )}
      </div>

      <div style={{ textAlign: 'center', marginBottom: '8px', padding: '5px', background: '#f5f5f5', borderRadius: '10px' }}>
        <span style={{ fontSize: '22px', fontWeight: 'bold', color: totalColor }}>{totalPower}</span>
        <span style={{ fontSize: '8px', marginLeft: '2px' }}>W</span>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', marginBottom: '8px' }}>
        {mppts.map((mppt, index) => {
          const mpptColor = mppt.power > 0 ? '#4caf50' : '#888';
          return (
            <div key={mppt.id || index} style={{ background: '#f5f5f5', borderRadius: '8px', padding: '5px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
                <span style={{ fontSize: '7px', fontWeight: 'bold', color: '#ff9800', fontFamily: 'monospace' }}>{mppt.id}</span>
                <span style={{ fontSize: '9px', fontWeight: 'bold', color: mpptColor }}>{mppt.power} W</span>
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '5px' }}>
                <div style={{ background: '#ffffff', borderRadius: '5px', padding: '3px', textAlign: 'center', border: '1px solid #e0e0e0' }}>
                  <div style={{ fontSize: '5px', color: '#888' }}>Tension</div>
                  <div style={{ fontSize: '8px', fontWeight: 'bold' }}>{mppt.voltage.toFixed(2)} V</div>
                </div>
                <div style={{ background: '#ffffff', borderRadius: '5px', padding: '3px', textAlign: 'center', border: '1px solid #e0e0e0' }}>
                  <div style={{ fontSize: '5px', color: '#888' }}>Courant</div>
                  <div style={{ fontSize: '8px', fontWeight: 'bold', color: mpptColor }}>{mppt.current.toFixed(1)} A</div>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {(energyToday > 0 || energyTotal > 0) && (
        <div style={{ display: 'flex', gap: '6px', marginTop: '6px', paddingTop: '5px', borderTop: '1px solid #e0e0e0' }}>
          {energyToday > 0 && (
            <div style={{ flex: 1, background: '#f5f5f5', borderRadius: '5px', padding: '3px', textAlign: 'center' }}>
              <div style={{ fontSize: '5px', color: '#888' }}>Aujourd'hui</div>
              <div style={{ fontSize: '7px', fontWeight: 'bold' }}>{energyToday.toFixed(1)} kWh</div>
            </div>
          )}
          {energyTotal > 0 && (
            <div style={{ flex: 1, background: '#f5f5f5', borderRadius: '5px', padding: '3px', textAlign: 'center' }}>
              <div style={{ fontSize: '5px', color: '#888' }}>Total</div>
              <div style={{ fontSize: '7px', fontWeight: 'bold', color: '#ff9800' }}>{energyTotal.toFixed(0)} kWh</div>
            </div>
          )}
        </div>
      )}

      {efficiency > 0 && (
        <div style={{ marginTop: '6px' }}>
          <div style={{ background: '#e0e0e0', borderRadius: '6px', height: '4px', overflow: 'hidden', marginBottom: '3px' }}>
            <div style={{ width: `${efficiency}%`, height: '4px', background: totalColor, borderRadius: '6px' }} />
          </div>
          <div style={{ fontSize: '6px', color: '#888', textAlign: 'center' }}>Rendement {efficiency}%</div>
        </div>
      )}
    </div>
  );
};

export default MPPTNode;
```

---

3. Fichier : src/components/nodes/SwitchNode.jsx

Modification : Ajout d'un handle sur le côté DROIT pour la connexion avec ET112

```jsx
import { useState } from 'react';
import { Handle, Position } from '@xyflow/react';

const SwitchNode = ({ id, data, onToggle }) => {
  const [isOn, setIsOn] = useState(data.isOn !== undefined ? data.isOn : true);

  const {
    label = 'Tongou Switch',
    deviceId = 'tongou_3BC764',
    time = '20:17:48',
    power = 0,
    voltage = 0,
    current = 0,
    cosPhi = 0,
    today = 0,
    yesterday = 0,
    total = 0
  } = data;

  const switchColor = isOn ? '#4caf50' : '#f44336';
  const statusText = isOn ? 'ON' : 'OFF';

  const handleToggle = () => {
    const newState = !isOn;
    setIsOn(newState);
    if (onToggle) onToggle(id, newState);
  };

  return (
    <div 
      className="switch-node"
      style={{
        backgroundColor: '#ffffff',
        borderRadius: '12px',
        padding: '8px',
        minWidth: '140px',
        border: '1.5px solid',
        borderColor: switchColor,
        fontFamily: 'Segoe UI, monospace',
        boxShadow: isOn ? `0 0 4px ${switchColor}` : 'none',
        opacity: isOn ? 1 : 0.7
      }}
    >
      {/* Handle GAUCHE - reçoit du Shunt */}
      <Handle 
        type="target"
        position={Position.Left}
        id="left-input"
        style={{ 
          background: switchColor,
          width: '10px',
          height: '10px',
          left: '-5px'
        }}
      />

      {/* Handle DROIT - connexion vers ET112 */}
      <Handle 
        type="source"
        position={Position.Right}
        id="right-output"
        style={{ 
          background: switchColor,
          width: '10px',
          height: '10px',
          right: '-5px'
        }}
      />

      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '6px' }}>
        <div style={{ padding: '2px 6px', borderRadius: '10px', fontSize: '7px', fontWeight: 'bold', backgroundColor: switchColor, color: 'white' }}>
          {statusText}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '3px' }}>
          <span style={{ fontSize: '10px' }}>🔌</span>
          <span style={{ fontWeight: 'bold', color: '#333', fontSize: '8px' }}>{label}</span>
        </div>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '6px', fontSize: '6px', color: '#999', fontFamily: 'monospace' }}>
        <span>{deviceId}</span>
        <span>{time}</span>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '8px', padding: '4px 0', borderTop: '1px solid #e0e0e0', borderBottom: '1px solid #e0e0e0' }}>
        <button 
          onClick={handleToggle}
          style={{
            width: '36px',
            height: '18px',
            background: isOn ? '#4caf50' : '#ccc',
            borderRadius: '18px',
            border: 'none',
            cursor: 'pointer',
            position: 'relative',
            transition: 'background 0.2s ease',
            padding: 0
          }}
        >
          <span style={{
            position: 'absolute',
            width: '14px',
            height: '14px',
            background: 'white',
            borderRadius: '50%',
            top: '2px',
            left: isOn ? '20px' : '2px',
            transition: 'left 0.2s ease'
          }} />
        </button>
        <span style={{ fontSize: '7px', fontWeight: 'bold', color: switchColor }}>
          {isOn ? 'COMMANDÉ ON' : 'COMMANDÉ OFF'}
        </span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '4px', marginBottom: '8px' }}>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '3px', textAlign: 'center' }}>
          <div style={{ fontSize: '5px', color: '#888' }}>PUISSANCE</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold', color: switchColor }}>{power.toFixed(1)} W</div>
        </div>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '3px', textAlign: 'center' }}>
          <div style={{ fontSize: '5px', color: '#888' }}>TENSION</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold' }}>{voltage.toFixed(1)} V</div>
        </div>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '3px', textAlign: 'center' }}>
          <div style={{ fontSize: '5px', color: '#888' }}>COURANT</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold' }}>{current.toFixed(2)} A</div>
        </div>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '3px', textAlign: 'center' }}>
          <div style={{ fontSize: '5px', color: '#888' }}>COS Φ</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold' }}>{cosPhi.toFixed(2)}</div>
        </div>
      </div>

      <div style={{ display: 'flex', gap: '6px', marginBottom: '6px', padding: '4px', background: '#f5f5f5', borderRadius: '8px' }}>
        <div style={{ flex: 1, textAlign: 'center' }}>
          <div style={{ fontSize: '5px', color: '#999' }}>AUJOURD'HUI</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold' }}>{today.toFixed(2)} kWh</div>
        </div>
        <div style={{ flex: 1, textAlign: 'center' }}>
          <div style={{ fontSize: '5px', color: '#999' }}>HIER</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold' }}>{yesterday.toFixed(2)} kWh</div>
        </div>
        <div style={{ flex: 1, textAlign: 'center', borderLeft: '1px solid #ddd' }}>
          <div style={{ fontSize: '5px', color: '#999' }}>TOTAL</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold', color: '#ff9800' }}>{total.toFixed(1)} kWh</div>
        </div>
      </div>

      <div style={{ textAlign: 'right', fontSize: '7px', color: '#ff9800', cursor: 'pointer', paddingTop: '4px', borderTop: '1px solid #e0e0e0' }}>
        Details →
      </div>
    </div>
  );
};

export default SwitchNode;
```

---

4. Fichier : src/components/nodes/ET112Node.jsx

Modification : Ajout d'un handle sur le côté GAUCHE pour la connexion avec Switch

```jsx
import { Handle, Position } from '@xyflow/react';

const ET112Node = ({ id, data }) => {
  const {
    label = 'ET112',
    deviceId = '0x07',
    time = '20:09:42',
    power = 0,
    voltage = 230.4,
    current = 0,
    type = 'load',
    imported = 0,
    exported = 0
  } = data;

  const flowColor = power > 0 ? '#4caf50' : '#ff9800';

  return (
    <div 
      className="et112-node"
      style={{
        backgroundColor: '#ffffff',
        borderRadius: '12px',
        padding: '8px',
        minWidth: '140px',
        border: '1px solid #e0e0e0',
        fontFamily: 'Segoe UI, monospace',
        boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
      }}
    >
      {/* Handle GAUCHE - reçoit du Switch */}
      <Handle 
        type="target"
        position={Position.Left}
        id="left-input"
        style={{ 
          background: flowColor,
          width: '10px',
          height: '10px',
          left: '-5px'
        }}
      />

      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '6px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          <span style={{ fontSize: '12px' }}>📊</span>
          <span style={{ fontWeight: 'bold', color: '#333', fontSize: '8px' }}>{label}</span>
        </div>
        <div style={{ background: '#ff3b30', color: 'white', fontSize: '6px', fontWeight: 'bold', padding: '2px 5px', borderRadius: '10px', display: 'flex', alignItems: 'center', gap: '3px' }}>
          <span style={{ width: '4px', height: '4px', background: 'white', borderRadius: '50%', animation: 'livePulse 1s infinite' }}></span>
          LIVE
        </div>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px', fontSize: '7px', color: '#999', fontFamily: 'monospace' }}>
        <span>{deviceId}</span>
        <span>{time}</span>
      </div>

      <div style={{ textAlign: 'center', marginBottom: '8px', padding: '5px', background: '#f5f5f5', borderRadius: '8px' }}>
        <div style={{ fontSize: '7px', color: '#888', letterSpacing: '0.5px' }}>PUISSANCE</div>
        <div style={{ fontSize: '16px', fontWeight: 'bold', color: flowColor }}>{Math.abs(power).toFixed(1)} W</div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '5px', marginBottom: '8px' }}>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '4px', textAlign: 'center' }}>
          <div style={{ fontSize: '6px', color: '#888' }}>TENSION</div>
          <div style={{ fontSize: '9px', fontWeight: 'bold' }}>{voltage.toFixed(1)} V</div>
        </div>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '4px', textAlign: 'center' }}>
          <div style={{ fontSize: '6px', color: '#888' }}>COURANT</div>
          <div style={{ fontSize: '9px', fontWeight: 'bold' }}>{current.toFixed(2)} A</div>
        </div>
        <div style={{ gridColumn: 'span 2', background: '#f5f5f5', borderRadius: '6px', padding: '4px', textAlign: 'center' }}>
          <div style={{ fontSize: '6px', color: '#888' }}>TYPE</div>
          <div style={{ fontSize: '9px', fontWeight: 'bold', color: '#ff9800', fontFamily: 'monospace', textTransform: 'uppercase' }}>{type}</div>
        </div>
      </div>

      <div style={{ display: 'flex', gap: '6px', marginBottom: '6px', paddingTop: '5px', borderTop: '1px solid #e0e0e0' }}>
        <div style={{ flex: 1, textAlign: 'center' }}>
          <div style={{ fontSize: '5px', color: '#999' }}>📥 IMPORTÉE</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold' }}>{imported.toFixed(2)} kWh</div>
        </div>
        <div style={{ flex: 1, textAlign: 'center' }}>
          <div style={{ fontSize: '5px', color: '#999' }}>📤 EXPORTÉE</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold' }}>{exported.toFixed(2)} kWh</div>
        </div>
      </div>

      <div style={{ textAlign: 'right', fontSize: '7px', color: '#ff9800', cursor: 'pointer', paddingTop: '4px', borderTop: '1px solid #e0e0e0' }}>
        Détails →
      </div>

      <style>{`
        @keyframes livePulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.4; }
        }
      `}</style>
    </div>
  );
};

export default ET112Node;
```

---

5. Fichier : src/pages/VisualisationComplete.jsx

Modification : Mise à jour des positions et des edges pour les nouvelles connexions

```jsx
import { ReactFlow, useNodesState, useEdgesState, Background, Controls } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import BatteryNode from '../components/nodes/BatteryNode';
import MPPTNode from '../components/nodes/MPPTNode';
import ShuntNode from '../components/nodes/ShuntNode';
import SwitchNode from '../components/nodes/SwitchNode';
import ET112Node from '../components/nodes/ET112Node';
import MeteoNode from '../components/nodes/MeteoNode';
import TemperatureNode from '../components/nodes/TemperatureNode';

const nodeTypes = {
  battery: BatteryNode,
  mppt: MPPTNode,
  shunt: ShuntNode,
  switch: SwitchNode,
  et112: ET112Node,
  meteo: MeteoNode,
  temperature: TemperatureNode,
};

const initialNodes = [
  // MPPT (à gauche du Shunt)
  {
    id: 'mppt-chargeur',
    type: 'mppt',
    position: { x: 80, y: 100 },
    data: {
      label: 'Chargeur PV',
      totalPower: 1169,
      mppts: [
        { id: 'MPPT-273', voltage: 98.70, current: 1.9, power: 777 },
        { id: 'MPPT-289', voltage: 98.71, current: 4.3, power: 423 }
      ],
      energyToday: 12.5,
      energyTotal: 3450
    }
  },

  // Shunt (centre)
  {
    id: 'shunt-main',
    type: 'shunt',
    position: { x: 300, y: 100 },
    data: {
      label: 'Shunt Principal',
      power: 1600,
      voltage: 52.85,
      current: -30.4,
      soc: 93
    }
  },

  // Switch (à droite du Shunt)
  {
    id: 'tongou-switch',
    type: 'switch',
    position: { x: 550, y: 100 },
    data: {
      label: 'Tongou Switch',
      deviceId: 'tongou_3BC764',
      time: '20:17:48',
      isOn: true,
      power: 1600,
      voltage: 231.0,
      current: 6.9,
      cosPhi: 0.95,
      today: 4.26,
      yesterday: 2.62,
      total: 42.3
    }
  },

  // ET112 (à droite du Switch)
  {
    id: 'et112-final',
    type: 'et112',
    position: { x: 780, y: 100 },
    data: {
      label: 'ET112',
      deviceId: '0x07',
      time: '20:09:42',
      power: 1580,
      voltage: 230.4,
      current: 6.86,
      type: 'load',
      imported: 760.30,
      exported: 0.00
    }
  },

  // Batterie 360Ah (en bas à gauche)
  {
    id: 'battery-360ah',
    type: 'battery',
    position: { x: 150, y: 320 },
    data: {
      label: 'BMS-360Ah',
      soc: 92,
      voltage: 52.8,
      current: -17.3,
      temperature: 14.0,
      power: -910,
      energyImported: 1250,
      energyExported: 890
    }
  },

  // Batterie 320Ah (en bas à droite)
  {
    id: 'battery-320ah',
    type: 'battery',
    position: { x: 450, y: 320 },
    data: {
      label: 'BMS-320Ah',
      soc: 94,
      voltage: 52.9,
      current: -13.1,
      temperature: 16.0,
      power: -690,
      energyImported: 1100,
      energyExported: 780
    }
  },

  // Météo (en bas à gauche)
  {
    id: 'meteo-station',
    type: 'meteo',
    position: { x: 80, y: 500 },
    data: {
      label: 'Station Solaire',
      irradiance: 850,
      productionTotal: 31,
      productionLast24h: 30.6,
      productionDay: 31,
      lastUpdate: 'il y a quelques secondes'
    }
  },

  // Température (en bas à droite)
  {
    id: 'temp-station',
    type: 'temperature',
    position: { x: 350, y: 500 },
    data: {
      label: 'Station Météo',
      temperature: 22.5,
      humidity: 53,
      pressure: 1012,
      tempMin24h: 18.5,
      tempMax24h: 26.5,
      lastUpdate: 'il y a quelques secondes'
    }
  }
];

// Connexions avec les nouveaux handles
const initialEdges = [
  // MPPT (côté DROIT) → Shunt (côté GAUCHE)
  { 
    id: 'e-mppt-shunt', 
    source: 'mppt-chargeur', 
    sourceHandle: 'right-output',
    target: 'shunt-main', 
    targetHandle: 'left-input',
    animated: true, 
    style: { stroke: '#4caf50', strokeWidth: 2 } 
  },
  
  // Batterie 360Ah → Shunt (côté BAS)
  { 
    id: 'e-battery360-shunt', 
    source: 'battery-360ah', 
    sourceHandle: 'top-output', 
    target: 'shunt-main', 
    targetHandle: 'bottom-input', 
    animated: true, 
    style: { stroke: '#f44336', strokeWidth: 2 } 
  },
  
  // Batterie 320Ah → Shunt (côté BAS)
  { 
    id: 'e-battery320-shunt', 
    source: 'battery-320ah', 
    sourceHandle: 'top-output', 
    target: 'shunt-main', 
    targetHandle: 'bottom-input', 
    animated: true, 
    style: { stroke: '#f44336', strokeWidth: 2 } 
  },
  
  // Shunt (côté DROIT) → Switch (côté GAUCHE)
  { 
    id: 'e-shunt-switch', 
    source: 'shunt-main', 
    sourceHandle: 'right-output', 
    target: 'tongou-switch', 
    targetHandle: 'left-input', 
    animated: true, 
    style: { stroke: '#ff9800', strokeWidth: 2 } 
  },
  
  // Switch (côté DROIT) → ET112 (côté GAUCHE)
  { 
    id: 'e-switch-et112', 
    source: 'tongou-switch', 
    sourceHandle: 'right-output', 
    target: 'et112-final', 
    targetHandle: 'left-input', 
    animated: true, 
    style: { stroke: '#ff9800', strokeWidth: 2 } 
  }
];

// Simulation de données temps réel
const simulateData = () => {
  const time = Date.now() / 1000;
  const variation = 0.9 + Math.sin(time / 10) * 0.1;
  const isDischarging = Math.sin(time / 20) > 0;

  return {
    mppt: { totalPower: Math.round(1169 * variation) },
    battery360: {
      current: isDischarging ? -17.3 * variation : +15.0 * variation,
      power: isDischarging ? -910 * variation : +790 * variation,
      soc: Math.max(0, Math.min(100, 92 + (isDischarging ? -0.1 : +0.1)))
    },
    battery320: {
      current: isDischarging ? -13.1 * variation : +12.0 * variation,
      power: isDischarging ? -690 * variation : +630 * variation,
      soc: Math.max(0, Math.min(100, 94 + (isDischarging ? -0.08 : +0.08)))
    },
    shunt: {
      current: (isDischarging ? -30.4 : +27.0) * variation,
      power: (isDischarging ? -1600 : +1420) * variation,
      soc: (92 + 94) / 2
    },
    switchState: isDischarging,
    meteo: { irradiance: 500 + Math.random() * 400, productionDay: 31 + Math.random() * 2 },
    temperature: { temp: 22.5 + (Math.random() - 0.5) * 2 }
  };
};

function VisualisationComplete() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  setInterval(() => {
    const newData = simulateData();
    const isDischarging = newData.battery360.current < 0;

    setNodes((nds) =>
      nds.map((node) => {
        switch (node.id) {
          case 'mppt-chargeur':
            return { ...node, data: { ...node.data, totalPower: newData.mppt.totalPower } };
          case 'battery-360ah':
            return { ...node, data: { ...node.data, current: newData.battery360.current, power: newData.battery360.power, soc: newData.battery360.soc } };
          case 'battery-320ah':
            return { ...node, data: { ...node.data, current: newData.battery320.current, power: newData.battery320.power, soc: newData.battery320.soc } };
          case 'shunt-main':
            return { ...node, data: { ...node.data, current: newData.shunt.current, power: newData.shunt.power, soc: newData.shunt.soc } };
          case 'tongou-switch':
            return { ...node, data: { ...node.data, isOn: newData.switchState, power: Math.abs(newData.shunt.power) } };
          case 'et112-final':
            return { ...node, data: { ...node.data, power: Math.abs(newData.shunt.power), current: Math.abs(newData.shunt.power) / 230.4 } };
          case 'meteo-station':
            return { ...node, data: { ...node.data, irradiance: newData.meteo.irradiance, productionDay: newData.meteo.productionDay } };
          case 'temp-station':
            return { ...node, data: { ...node.data, temperature: newData.temperature.temp } };
          default:
            return node;
        }
      })
    );

    setEdges((eds) =>
      eds.map((edge) => {
        let strokeColor = '#ff9800';
        if (edge.source === 'mppt-chargeur') strokeColor = '#4caf50';
        if (edge.source === 'battery-360ah' || edge.source === 'battery-320ah') strokeColor = isDischarging ? '#f44336' : '#4caf50';
        if (edge.target === 'tongou-switch' || edge.target === 'et112-final') strokeColor = isDischarging ? '#f44336' : '#4caf50';
        return { ...edge, style: { stroke: strokeColor, strokeWidth: 2 } };
      })
    );
  }, 2000);

  return (
    <div style={{ width: '100vw', height: '100vh', backgroundColor: '#f0f2f5' }}>
      <div style={{
        position: 'absolute',
        top: 10,
        left: 10,
        zIndex: 10,
        background: '#fff',
        padding: '6px 12px',
        borderRadius: 8,
        color: '#333',
        fontSize: 12,
        fontFamily: 'monospace',
        boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
      }}>
        🔌 MPPT → Shunt (Droit/Gauche) | Switch → ET112 (Droit/Gauche)
      </div>

      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
        defaultViewport={{ x: 0, y: 0, zoom: 0.6 }}
      >
        <Background color="#ccc" gap={16} />
        <Controls />
      </ReactFlow>
    </div>
  );
}

export default VisualisationComplete;
```

---

Récapitulatif des modifications

Fichier Modification
ShuntNode.jsx Ajout handle DROIT (right-output) pour MPPT
MPPTNode.jsx Ajout handle GAUCHE (left-input) pour Shunt
SwitchNode.jsx Handle DROIT existant conservé pour ET112
ET112Node.jsx Ajout handle GAUCHE (left-input) pour Switch
VisualisationComplete.jsx Nouvelles positions et edges mis à jour

---

Schéma des connexions final

```
                    MPPT                    Switch                  ET112
                   (GAUCHE) ←─── (DROIT)   (GAUCHE) ←─── (DROIT)   (GAUCHE)
                      ↑                         ↑
                      │                         │
                      ↓                         ↓
    Batterie 360Ah ──→ Shunt ←── Batterie 320Ah
                      (BAS)
```
