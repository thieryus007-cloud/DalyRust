Je respecte strictement votre demande. Voici tous les fichiers complets avec les modifications demandées.

---

1. Fichier : src/components/nodes/MeteoNode.jsx (sans handle)

```jsx
const MeteoNode = ({ id, data }) => {
  const {
    label = 'Station Solaire',
    irradiance = 850,
    productionTotal = 31,
    productionLast24h = 30.6,
    productionDay = 31,
    lastUpdate = 'il y a quelques secondes'
  } = data;

  return (
    <div 
      className="meteo-node"
      style={{
        backgroundColor: '#ffffff',
        borderRadius: '12px',
        padding: '8px',
        minWidth: '140px',
        border: '1px solid #e0e0e0',
        fontFamily: 'Segoe UI, monospace',
        textAlign: 'center',
        boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '4px', marginBottom: '6px' }}>
        <span style={{ fontSize: '14px' }}>☀️</span>
        <span style={{ fontSize: '8px', color: '#666' }}>{label}</span>
      </div>

      <div style={{ marginBottom: '4px' }}>
        <span style={{ fontSize: '22px', fontWeight: 'bold', color: '#ff9800' }}>{irradiance.toFixed(1)}</span>
        <span style={{ fontSize: '8px', color: '#999' }}> W/m²</span>
      </div>

      <div style={{ marginBottom: '8px' }}>
        <span style={{ fontSize: '16px', fontWeight: 'bold', color: '#ff9800' }}>-{productionTotal}</span>
        <span style={{ fontSize: '8px', color: '#999' }}> kWh</span>
      </div>

      <div style={{ background: '#f5f5f5', borderRadius: '8px', padding: '6px', marginBottom: '6px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '7px', color: '#888', marginBottom: '6px' }}>
          <span>Dernières 24 h</span>
          <span style={{ color: '#ff9800', fontWeight: 'bold' }}>{productionLast24h.toFixed(1)} kWh</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'flex-end', gap: '2px', height: '28px' }}>
          {[2.1, 3.5, 5.2, 8.1, 12.4, 18.7, 22.3, 30.6, 28.4, 24.1, 18.2, 12.5].slice(0, 8).map((val, idx) => (
            <div key={idx} style={{ flex: 1, background: '#ff9800', height: `${(val / 35) * 28}px`, borderRadius: '2px', opacity: 0.7 }} />
          ))}
        </div>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', background: '#f5f5f5', borderRadius: '6px', padding: '4px 6px', marginBottom: '6px' }}>
        <span style={{ fontSize: '7px', color: '#888' }}>Production du jour</span>
        <span style={{ fontSize: '9px', fontWeight: 'bold', color: '#ff9800' }}>{productionDay} kWh</span>
      </div>

      <div style={{ fontSize: '6px', color: '#aaa', textAlign: 'center' }}>
        Dernière mise à jour<br />{lastUpdate}
      </div>
    </div>
  );
};

export default MeteoNode;
```

---

2. Fichier : src/components/nodes/TemperatureNode.jsx (sans handle)

```jsx
const TemperatureNode = ({ id, data }) => {
  const {
    label = 'Station Météo',
    temperature = 22.5,
    humidity = 53,
    pressure = 1012,
    tempMin24h = 18.5,
    tempMax24h = 26.5,
    lastUpdate = 'il y a quelques secondes'
  } = data;

  return (
    <div 
      className="temperature-node"
      style={{
        backgroundColor: '#ffffff',
        borderRadius: '12px',
        padding: '8px',
        minWidth: '140px',
        border: '1px solid #e0e0e0',
        fontFamily: 'Segoe UI, monospace',
        textAlign: 'center',
        boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '4px', marginBottom: '6px' }}>
        <span style={{ fontSize: '14px' }}>🌡️</span>
        <span style={{ fontSize: '8px', color: '#666' }}>{label}</span>
      </div>

      <div style={{ marginBottom: '8px' }}>
        <div style={{ fontSize: '6px', color: '#888', marginBottom: '4px' }}>Température Extérieure</div>
        <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'center', gap: '2px' }}>
          <span style={{ fontSize: '24px', fontWeight: 'bold', color: '#2196f3' }}>{temperature.toFixed(1)}</span>
          <span style={{ fontSize: '8px', color: '#999' }}>°C</span>
        </div>
      </div>

      <div style={{ display: 'flex', gap: '8px', justifyContent: 'center', marginBottom: '8px' }}>
        <div style={{ background: '#f5f5f5', borderRadius: '30px', padding: '5px 8px', textAlign: 'center', minWidth: '55px' }}>
          <div style={{ fontSize: '12px', fontWeight: 'bold', color: '#4caf50' }}>{humidity}%</div>
          <div style={{ fontSize: '5px', color: '#888' }}>Humidité</div>
        </div>
        <div style={{ background: '#f5f5f5', borderRadius: '30px', padding: '5px 8px', textAlign: 'center', minWidth: '55px' }}>
          <div style={{ fontSize: '12px', fontWeight: 'bold', color: '#4caf50' }}>{pressure.toFixed(0)}</div>
          <div style={{ fontSize: '5px', color: '#888' }}>Pression</div>
          <div style={{ fontSize: '5px', color: '#999' }}>hPa</div>
        </div>
      </div>

      <div style={{ background: '#f5f5f5', borderRadius: '8px', padding: '5px', marginBottom: '6px' }}>
        <div style={{ fontSize: '5px', color: '#888', marginBottom: '3px' }}>Dernières 24 h</div>
        <div style={{ display: 'flex', justifyContent: 'center', gap: '10px' }}>
          <span style={{ fontSize: '7px', color: '#64b5f6' }}>{tempMin24h.toFixed(1)}°C min</span>
          <span style={{ fontSize: '7px', color: '#ff8a65' }}>{tempMax24h.toFixed(1)}°C max</span>
        </div>
      </div>

      <div style={{ fontSize: '5px', color: '#bbb', textAlign: 'center' }}>
        Dernière mise à jour<br />{lastUpdate}
      </div>
    </div>
  );
};

export default TemperatureNode;
```

---

3. Fichier : src/components/nodes/ATSNode.jsx (NOUVEAU)

```jsx
import { Handle, Position } from '@xyflow/react';

const ATSNode = ({ id, data }) => {
  const {
    label = 'ATS',
    status = 'Source Principale',
    power = 0
  } = data;

  return (
    <div 
      className="ats-node"
      style={{
        backgroundColor: '#ffffff',
        borderRadius: '12px',
        padding: '8px',
        minWidth: '120px',
        border: '2px solid #2196f3',
        fontFamily: 'Segoe UI, monospace',
        textAlign: 'center',
        boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
      }}
    >
      {/* Handle HAUT - pas de liaison pour le moment */}
      <Handle 
        type="target"
        position={Position.Top}
        id="top-input"
        style={{ 
          background: '#2196f3',
          width: '10px',
          height: '10px',
          top: '-5px'
        }}
      />

      {/* Handle DROIT - relié à ET112 */}
      <Handle 
        type="source"
        position={Position.Right}
        id="right-output"
        style={{ 
          background: '#2196f3',
          width: '10px',
          height: '10px',
          right: '-5px'
        }}
      />

      {/* Handle BAS - relié à Onduleur */}
      <Handle 
        type="source"
        position={Position.Bottom}
        id="bottom-output"
        style={{ 
          background: '#2196f3',
          width: '10px',
          height: '10px',
          bottom: '-5px'
        }}
      />

      <div style={{ fontSize: '14px', marginBottom: '4px' }}>🔄</div>
      <div style={{ fontWeight: 'bold', fontSize: '11px', color: '#2196f3' }}>{label}</div>
      <div style={{ fontSize: '8px', color: '#666' }}>{status}</div>
      <div style={{ fontSize: '10px', fontWeight: 'bold', marginTop: '4px' }}>{Math.abs(power).toFixed(0)} W</div>
    </div>
  );
};

export default ATSNode;
```

---

4. Fichier : src/components/nodes/OnduleurNode.jsx (NOUVEAU)

```jsx
import { Handle, Position } from '@xyflow/react';

const OnduleurNode = ({ id, data }) => {
  const {
    label = 'Onduleur',
    power = 0,
    voltage = 230,
    efficiency = 94
  } = data;

  const flowColor = power > 0 ? '#4caf50' : '#ff9800';

  return (
    <div 
      className="onduleur-node"
      style={{
        backgroundColor: '#ffffff',
        borderRadius: '12px',
        padding: '8px',
        minWidth: '120px',
        border: '1.5px solid',
        borderColor: flowColor,
        fontFamily: 'Segoe UI, monospace',
        textAlign: 'center',
        boxShadow: `0 0 4px ${flowColor}`
      }}
    >
      {/* Handle HAUT - reçoit de ATS */}
      <Handle 
        type="target"
        position={Position.Top}
        id="top-input"
        style={{ 
          background: flowColor,
          width: '10px',
          height: '10px',
          top: '-5px'
        }}
      />

      {/* Handle BAS - envoie vers Shunt */}
      <Handle 
        type="source"
        position={Position.Bottom}
        id="bottom-output"
        style={{ 
          background: flowColor,
          width: '10px',
          height: '10px',
          bottom: '-5px'
        }}
      />

      <div style={{ fontSize: '14px', marginBottom: '4px' }}>⚡</div>
      <div style={{ fontWeight: 'bold', fontSize: '10px', color: '#333' }}>{label}</div>
      <div style={{ fontSize: '18px', fontWeight: 'bold', color: flowColor }}>{Math.abs(power).toFixed(0)}</div>
      <div style={{ fontSize: '8px', color: '#888' }}>W</div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: '6px', paddingTop: '4px', borderTop: '1px solid #e0e0e0' }}>
        <div>
          <div style={{ fontSize: '6px', color: '#888' }}>Tension</div>
          <div style={{ fontSize: '8px', fontWeight: 'bold' }}>{voltage}V</div>
        </div>
        <div>
          <div style={{ fontSize: '6px', color: '#888' }}>Rendement</div>
          <div style={{ fontSize: '8px', fontWeight: 'bold', color: '#4caf50' }}>{efficiency}%</div>
        </div>
      </div>
    </div>
  );
};

export default OnduleurNode;
```

---

5. Fichier : src/components/nodes/ShuntNode.jsx

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
      {/* Handle HAUT - reçoit de l'Onduleur */}
      <Handle 
        type="target"
        position={Position.Top}
        id="top-input"
        style={{ 
          background: flowColor,
          width: '10px',
          height: '10px',
          top: '-5px'
        }}
      />

      {/* Handle DROIT - relié à MPPT */}
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

      {/* Handle BAS - relié aux Batteries */}
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

6. Fichier : src/components/nodes/MPPTNode.jsx

```jsx
import { Handle, Position } from '@xyflow/react';

const MPPTNode = ({ id, data }) => {
  const {
    label = 'Chargeur PV',
    totalPower = 0,
    mppts = [],
    energyToday = 0,
    energyTotal = 0
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
    </div>
  );
};

export default MPPTNode;
```

---

7. Fichier : src/components/nodes/ET112Node.jsx

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
      {/* Handle GAUCHE - reçoit de ATS */}
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

      {/* Handle DROIT - envoie vers Switch */}
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

8. Fichier : src/components/nodes/SwitchNode.jsx

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
      {/* Handle GAUCHE - reçoit de ET112 */}
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

9. Fichier : src/components/nodes/BatteryNode.jsx

```jsx
import { Handle, Position } from '@xyflow/react';

const BatteryNode = ({ id, data }) => {
  const {
    label = 'BMS',
    soc = 0,
    voltage = 0,
    current = 0,
    temperature = 0,
    power = 0,
    energyImported = 0,
    energyExported = 0
  } = data;

  const isCharging = current > 0;
  const isDischarging = current < 0;
  const currentColor = isCharging ? '#4caf50' : (isDischarging ? '#f44336' : '#ff9800');

  return (
    <div 
      className="battery-node"
      style={{ 
        borderColor: currentColor, 
        boxShadow: `0 0 4px ${currentColor}`,
        backgroundColor: '#ffffff',
        borderRadius: '12px',
        padding: '8px',
        minWidth: '140px',
        border: '1.5px solid',
        fontFamily: 'Segoe UI, monospace'
      }}
    >
      {/* Handle HAUT - connexion vers le Shunt */}
      <Handle 
        type="source"
        position={Position.Top}
        id="top-output"
        style={{ 
          background: currentColor,
          width: '10px',
          height: '10px',
          top: '-5px'
        }}
      />

      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '6px' }}>
        <span>🔋 {label}</span>
        <span style={{ color: currentColor, fontSize: '9px' }}>
          {isCharging ? 'CHARGE' : (isDischarging ? 'DÉCHARGE' : 'IDLE')}
        </span>
      </div>

      <div style={{ textAlign: 'center', marginBottom: '8px' }}>
        <span style={{ fontSize: '18px', fontWeight: 'bold' }}>{soc}%</span>
        <div style={{ background: '#e0e0e0', borderRadius: '6px', height: '5px', marginTop: '4px' }}>
          <div style={{ width: `${soc}%`, height: '5px', background: currentColor, borderRadius: '6px' }} />
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '5px', marginBottom: '8px' }}>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '4px', textAlign: 'center' }}>
          <div style={{ fontSize: '6px', color: '#888' }}>TENSION</div>
          <div style={{ fontSize: '9px', fontWeight: 'bold' }}>{voltage.toFixed(1)}V</div>
        </div>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '4px', textAlign: 'center' }}>
          <div style={{ fontSize: '6px', color: '#888' }}>COURANT</div>
          <div style={{ fontSize: '9px', fontWeight: 'bold', color: currentColor }}>{Math.abs(current).toFixed(1)}A</div>
        </div>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '4px', textAlign: 'center' }}>
          <div style={{ fontSize: '6px', color: '#888' }}>TEMP.</div>
          <div style={{ fontSize: '9px', fontWeight: 'bold' }}>{temperature.toFixed(1)}°C</div>
        </div>
        <div style={{ background: '#f5f5f5', borderRadius: '6px', padding: '4px', textAlign: 'center' }}>
          <div style={{ fontSize: '6px', color: '#888' }}>PUISSANCE</div>
          <div style={{ fontSize: '9px', fontWeight: 'bold', color: currentColor }}>{Math.abs(power).toFixed(0)}W</div>
        </div>
      </div>

      <div style={{ display: 'flex', gap: '4px', borderTop: '1px solid #e0e0e0', paddingTop: '5px' }}>
        <div style={{ flex: 1, background: '#f5f5f5', borderRadius: '5px', padding: '3px', textAlign: 'center' }}>
          <div style={{ fontSize: '5px', color: '#888' }}>IMPORTÉ</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold' }}>{energyImported.toFixed(0)} kWh</div>
        </div>
        <div style={{ flex: 1, background: '#f5f5f5', borderRadius: '5px', padding: '3px', textAlign: 'center' }}>
          <div style={{ fontSize: '5px', color: '#888' }}>EXPORTÉ</div>
          <div style={{ fontSize: '7px', fontWeight: 'bold' }}>{energyExported.toFixed(0)} kWh</div>
        </div>
      </div>
    </div>
  );
};

export default BatteryNode;
```

---

10. Fichier : src/pages/VisualisationComplete.jsx

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
import ATSNode from '../components/nodes/ATSNode';
import OnduleurNode from '../components/nodes/OnduleurNode';

const nodeTypes = {
  battery: BatteryNode,
  mppt: MPPTNode,
  shunt: ShuntNode,
  switch: SwitchNode,
  et112: ET112Node,
  meteo: MeteoNode,
  temperature: TemperatureNode,
  ats: ATSNode,
  onduleur: OnduleurNode,
};

const initialNodes = [
  // ===== EN HAUT A DROITE - Météo et Température (sans handles, centrés à droite) =====
  {
    id: 'meteo-station',
    type: 'meteo',
    position: { x: 900, y: 50 },
    data: {
      label: 'Station Solaire',
      irradiance: 850,
      productionTotal: 31,
      productionLast24h: 30.6,
      productionDay: 31,
      lastUpdate: 'il y a quelques secondes'
    }
  },
  {
    id: 'temp-station',
    type: 'temperature',
    position: { x: 900, y: 230 },
    data: {
      label: 'Station Météo',
      temperature: 22.5,
      humidity: 53,
      pressure: 1012,
      tempMin24h: 18.5,
      tempMax24h: 26.5,
      lastUpdate: 'il y a quelques secondes'
    }
  },

  // ===== LIGNE HAUTE - ATS, ET112, Switch (alignés de gauche à droite) =====
  {
    id: 'ats-main',
    type: 'ats',
    position: { x: 100, y: 50 },
    data: {
      label: 'ATS',
      status: 'Source Principale',
      power: 5000
    }
  },
  {
    id: 'et112-final',
    type: 'et112',
    position: { x: 320, y: 50 },
    data: {
      label: 'ET112',
      deviceId: '0x07',
      time: '20:09:42',
      power: 4800,
      voltage: 230.4,
      current: 20.8,
      type: 'load',
      imported: 760.30,
      exported: 0.00
    }
  },
  {
    id: 'tongou-switch',
    type: 'switch',
    position: { x: 540, y: 50 },
    data: {
      label: 'Tongou Switch',
      deviceId: 'tongou_3BC764',
      time: '20:17:48',
      isOn: true,
      power: 4750,
      voltage: 231.0,
      current: 20.6,
      cosPhi: 0.95,
      today: 4.26,
      yesterday: 2.62,
      total: 42.3
    }
  },

  // ===== LIGNE MILIEU - Onduleur (sous ATS) =====
  {
    id: 'onduleur-main',
    type: 'onduleur',
    position: { x: 100, y: 220 },
    data: {
      label: 'Onduleur',
      power: 4600,
      voltage: 230,
      efficiency: 94
    }
  },

  // ===== LIGNE BAS - Shunt, MPPT, Batteries =====
  {
    id: 'shunt-main',
    type: 'shunt',
    position: { x: 100, y: 400 },
    data: {
      label: 'Shunt Principal',
      power: 4500,
      voltage: 52.85,
      current: -85.0,
      soc: 85
    }
  },
  {
    id: 'mppt-chargeur',
    type: 'mppt',
    position: { x: 350, y: 400 },
    data: {
      label: 'Chargeur PV',
      totalPower: 2500,
      mppts: [
        { id: 'MPPT-273', voltage: 98.70, current: 12.7, power: 1250 },
        { id: 'MPPT-289', voltage: 98.71, current: 12.7, power: 1250 }
      ],
      energyToday: 12.5,
      energyTotal: 3450
    }
  },
  {
    id: 'battery-360ah',
    type: 'battery',
    position: { x: 100, y: 580 },
    data: {
      label: 'BMS-360Ah',
      soc: 92,
      voltage: 52.8,
      current: -45.0,
      temperature: 14.0,
      power: -2376,
      energyImported: 1250,
      energyExported: 890
    }
  },
  {
    id: 'battery-320ah',
    type: 'battery',
    position: { x: 320, y: 580 },
    data: {
      label: 'BMS-320Ah',
      soc: 94,
      voltage: 52.9,
      current: -40.0,
      temperature: 16.0,
      power: -2116,
      energyImported: 1100,
      energyExported: 780
    }
  }
];

// ===== CONNEXIONS =====
const initialEdges = [
  // ATS (côté DROIT) → ET112 (côté GAUCHE)
  { 
    id: 'e-ats-et112', 
    source: 'ats-main', 
    sourceHandle: 'right-output',
    target: 'et112-final', 
    targetHandle: 'left-input',
    animated: true, 
    style: { stroke: '#2196f3', strokeWidth: 2 } 
  },
  
  // ET112 (côté DROIT) → Switch (côté GAUCHE)
  { 
    id: 'e-et112-switch', 
    source: 'et112-final', 
    sourceHandle: 'right-output',
    target: 'tongou-switch', 
    targetHandle: 'left-input',
    animated: true, 
    style: { stroke: '#ff9800', strokeWidth: 2 } 
  },
  
  // ATS (côté BAS) → Onduleur (côté HAUT)
  { 
    id: 'e-ats-onduleur', 
    source: 'ats-main', 
    sourceHandle: 'bottom-output',
    target: 'onduleur-main', 
    targetHandle: 'top-input',
    animated: true, 
    style: { stroke: '#4caf50', strokeWidth: 2 } 
  },
  
  // Onduleur (côté BAS) → Shunt (côté HAUT)
  { 
    id: 'e-onduleur-shunt', 
    source: 'onduleur-main', 
    sourceHandle: 'bottom-output',
    target: 'shunt-main', 
    targetHandle: 'top-input',
    animated: true, 
    style: { stroke: '#4caf50', strokeWidth: 2 } 
  },
  
  // Shunt (côté DROIT) → MPPT (côté GAUCHE)
  { 
    id: 'e-shunt-mppt', 
    source: 'shunt-main', 
    sourceHandle: 'right-output',
    target: 'mppt-chargeur', 
    targetHandle: 'left-input',
    animated: true, 
    style: { stroke: '#4caf50', strokeWidth: 2 } 
  },
  
  // Shunt (côté BAS) → Batterie 360Ah (côté HAUT)
  { 
    id: 'e-shunt-battery360', 
    source: 'shunt-main', 
    sourceHandle: 'bottom-input',
    target: 'battery-360ah', 
    targetHandle: 'top-output',
    animated: true, 
    style: { stroke: '#f44336', strokeWidth: 2 } 
  },
  
  // Shunt (côté BAS) → Batterie 320Ah (côté HAUT)
  { 
    id: 'e-shunt-battery320', 
    source: 'shunt-main', 
    sourceHandle: 'bottom-input',
    target: 'battery-320ah', 
    targetHandle: 'top-output',
    animated: true, 
    style: { stroke: '#f44336', strokeWidth: 2 } 
  }
];

// Simulation de données temps réel
const simulateData = () => {
  const time = Date.now() / 1000;
  const variation = 0.9 + Math.sin(time / 10) * 0.1;
  const isDischarging = Math.sin(time / 20) > 0;

  return {
    atsPower: 5000 * variation,
    et112Power: 4800 * variation,
    switchPower: 4750 * variation,
    onduleurPower: 4600 * variation,
    shuntPower: (isDischarging ? -4500 : +4200) * variation,
    shuntCurrent: (isDischarging ? -85 : +80) * variation,
    battery360: {
      current: isDischarging ? -45 * variation : +42 * variation,
      power: isDischarging ? -2376 * variation : +2218 * variation,
      soc: Math.max(0, Math.min(100, 92 + (isDischarging ? -0.05 : +0.05)))
    },
    battery320: {
      current: isDischarging ? -40 * variation : +38 * variation,
      power: isDischarging ? -2116 * variation : +2010 * variation,
      soc: Math.max(0, Math.min(100, 94 + (isDischarging ? -0.04 : +0.04)))
    },
    mpptPower: 2500 * (0.8 + Math.random() * 0.4),
    switchState: !isDischarging
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
          case 'ats-main':
            return { ...node, data: { ...node.data, power: newData.atsPower } };
          case 'et112-final':
            return { ...node, data: { ...node.data, power: newData.et112Power, current: newData.et112Power / 230.4 } };
          case 'tongou-switch':
            return { ...node, data: { ...node.data, isOn: newData.switchState, power: newData.switchPower } };
          case 'onduleur-main':
            return { ...node, data: { ...node.data, power: newData.onduleurPower } };
          case 'shunt-main':
            return { ...node, data: { ...node.data, power: Math.abs(newData.shuntPower), current: Math.abs(newData.shuntCurrent), soc: (newData.battery360.soc + newData.battery320.soc) / 2 } };
          case 'battery-360ah':
            return { ...node, data: { ...node.data, current: newData.battery360.current, power: newData.battery360.power, soc: newData.battery360.soc } };
          case 'battery-320ah':
            return { ...node, data: { ...node.data, current: newData.battery320.current, power: newData.battery320.power, soc: newData.battery320.soc } };
          case 'mppt-chargeur':
            return { ...node, data: { ...node.data, totalPower: newData.mpptPower, mppts: node.data.mppts.map(m => ({ ...m, power: newData.mpptPower / 2, current: (newData.mpptPower / 2) / 98.7 })) } };
          default:
            return node;
        }
      })
    );

    setEdges((eds) =>
      eds.map((edge) => {
        let strokeColor = '#ff9800';
        if (edge.source === 'ats-main' || edge.source === 'onduleur-main') strokeColor = '#4caf50';
        if (edge.source === 'shunt-main' && edge.target === 'mppt-chargeur') strokeColor = '#4caf50';
        if (edge.source === 'shunt-main' && (edge.target === 'battery-360ah' || edge.target === 'battery-320ah')) strokeColor = isDischarging ? '#f44336' : '#4caf50';
        if (edge.source === 'et112-final' && edge.target === 'tongou-switch') strokeColor = '#ff9800';
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
        🔌 ATS → ET112 → Switch | ATS → Onduleur → Shunt → MPPT | Shunt → Batteries
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

Récapitulatif des fichiers créés/modifiés

Fichier Action
MeteoNode.jsx Modifié - sans handle
TemperatureNode.jsx Modifié - sans handle
ATSNode.jsx NOUVEAU - handles haut, droit, bas
OnduleurNode.jsx NOUVEAU - handles haut et bas
ShuntNode.jsx Modifié - handles haut, droit, bas
MPPTNode.jsx Modifié - handle gauche
ET112Node.jsx Modifié - handles gauche et droit
SwitchNode.jsx Modifié - handle gauche
BatteryNode.jsx Modifié - handle haut
VisualisationComplete.jsx Modifié - positions et edges

---

Schéma des connexions final

```
                    Météo (sans handle)     Température (sans handle)
                    (900,50)                 (900,230)
                    
                    ATS (100,50) ──DROIT──► ET112 (320,50) ──DROIT──► Switch (540,50)
                     │
                     │ BAS
                     ▼
                    Onduleur (100,220)
                     │
                     │ BAS
                     ▼
                    Shunt (100,400) ──DROIT──► MPPT (350,400)
                     │
                     ├── BAS ──► Batterie 360Ah (100,580)
                     │
                     └── BAS ──► Batterie 320Ah (320,580)
```
