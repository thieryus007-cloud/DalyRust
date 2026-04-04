import { ReactFlow, useNodesState, useEdgesState, Background, Controls } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import BatteryNode from '../components/nodes/BatteryNode';
import MPPTNode from '../components/nodes/MPPTNode';
import ShuntNode from '../components/nodes/ShuntNode';
import SwitchNode from '../components/nodes/SwitchNode';
import ET112Node from '../components/nodes/ET112Node';
import MeteoNode from '../components/nodes/MeteoNode';
import TemperatureNode from '../components/nodes/TemperatureNode';

// Types de nœuds personnalisés - TOUS
const nodeTypes = {
  battery: BatteryNode,
  mppt: MPPTNode,
  shunt: ShuntNode,
  switch: SwitchNode,
  et112: ET112Node,
  meteo: MeteoNode,
  temperature: TemperatureNode,
};

// Données initiales - TOUS les nodes
const initialNodes = [
  // 1. MPPT (chargeur solaire)
  {
    id: 'mppt-chargeur',
    type: 'mppt',
    position: { x: 100, y: 50 },
    data: {
      label: 'Chargeur PV',
      totalPower: 1169,
      mppts: [
        { id: 'MPPT-273', voltage: 98.70, current: 1.9, power: 777 },
        { id: 'MPPT-289', voltage: 98.71, current: 4.3, power: 423 }
      ],
      energyToday: 12.5,
      energyTotal: 3450,
      handles: { bottom: true }
    }
  },

  // 2. Shunt (mesure principale)
  {
    id: 'shunt-main',
    type: 'shunt',
    position: { x: 350, y: 50 },
    data: {
      label: 'Shunt Principal',
      power: 1600,
      voltage: 52.85,
      current: -30.4,
      soc: 93,
      energyDay: 24.5,
      energyWeek: 168.2,
      energyTotal: 12500
    }
  },

  // 3. Batterie 360Ah
  {
    id: 'battery-360ah',
    type: 'battery',
    position: { x: 150, y: 250 },
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

  // 4. Batterie 320Ah
  {
    id: 'battery-320ah',
    type: 'battery',
    position: { x: 450, y: 250 },
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

  // 5. Switch (interrupteur)
  {
    id: 'tongou-switch',
    type: 'switch',
    position: { x: 650, y: 100 },
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
      total: 42.3,
      handles: { left: true, right: true }
    }
  },

  // 6. ET112 (compteur)
  {
    id: 'et112-final',
    type: 'et112',
    position: { x: 880, y: 100 },
    data: {
      label: 'ET112',
      deviceId: '0x07',
      time: '20:09:42',
      power: 1580,
      voltage: 230.4,
      current: 6.86,
      type: 'load',
      imported: 760.30,
      exported: 0.00,
      handles: { left: true }
    }
  },

  // 7. Météo (irradiance)
  {
    id: 'meteo-station',
    type: 'meteo',
    position: { x: 100, y: 450 },
    data: {
      label: 'Station Solaire',
      irradiance: 850,
      productionTotal: 31,
      productionLast24h: 30.6,
      productionDay: 31,
      lastUpdate: 'il y a quelques secondes'
    }
  },

  // 8. Température
  {
    id: 'temp-station',
    type: 'temperature',
    position: { x: 350, y: 450 },
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

// Connexions entre les nodes
const initialEdges = [
  // MPPT → Shunt
  { id: 'e1', source: 'mppt-chargeur', target: 'shunt-main', animated: true, style: { stroke: '#4caf50', strokeWidth: 2 } },
  
  // Batterie 360Ah → Shunt (handle haut)
  { id: 'e2', source: 'battery-360ah', sourceHandle: 'top-output', target: 'shunt-main', targetHandle: 'bottom-input', animated: true, style: { stroke: '#f44336', strokeWidth: 2 } },
  
  // Batterie 320Ah → Shunt (handle haut)
  { id: 'e3', source: 'battery-320ah', sourceHandle: 'top-output', target: 'shunt-main', targetHandle: 'bottom-input', animated: true, style: { stroke: '#f44336', strokeWidth: 2 } },
  
  // Shunt → Switch
  { id: 'e4', source: 'shunt-main', target: 'tongou-switch', animated: true, style: { stroke: '#ff9800', strokeWidth: 2 } },
  
  // Switch → ET112
  { id: 'e5', source: 'tongou-switch', target: 'et112-final', animated: true, style: { stroke: '#ff9800', strokeWidth: 2 } }
];

// Simulation de données temps réel pour TOUS les nodes
const simulateData = () => {
  const time = Date.now() / 1000;
  const variation = 0.9 + Math.sin(time / 10) * 0.1;
  const isDischarging = Math.sin(time / 20) > 0;

  return {
    mppt: {
      totalPower: Math.round(1169 * variation),
      mppts: [
        { ...initialNodes[0].data.mppts[0], power: Math.round(777 * variation), current: +(1.9 * variation).toFixed(1) },
        { ...initialNodes[0].data.mppts[1], power: Math.round(423 * variation), current: +(4.3 * variation).toFixed(1) }
      ]
    },
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
    meteo: {
      irradiance: 500 + Math.random() * 400,
      productionDay: 31 + Math.random() * 2
    },
    temperature: {
      temp: 22.5 + (Math.random() - 0.5) * 2
    }
  };
};

function VisualisationComplete() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Mise à jour temps réel toutes les 2 secondes
  setInterval(() => {
    const newData = simulateData();
    const isDischarging = newData.battery360.current < 0;

    setNodes((nds) =>
      nds.map((node) => {
        switch (node.id) {
          case 'mppt-chargeur':
            return { ...node, data: { ...node.data, totalPower: newData.mppt.totalPower, mppts: newData.mppt.mppts } };
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

    // Mise à jour des couleurs des edges
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
        🔋 TOUS les nodes : MPPT | 2 Batteries | Shunt | Switch | ET112 | Météo | Température
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
