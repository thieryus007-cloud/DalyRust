import React, { useState, useCallback, useEffect, useRef } from 'react';
import ReactFlow, { 
  Background, Controls, useNodesState, useEdgesState 
} from 'reactflow';
import 'reactflow/dist/style.css';
import SourceNode from './nodes/SourceNode';
import SwitchNode from './nodes/SwitchNode';

const nodeTypes = {
  source: SourceNode,
  commutator: SwitchNode,
};

const initialNodes = [
  { 
    id: 'onduleur', 
    type: 'source', 
    position: { x: 100, y: 150 },
    data: { 
      title: '🔋 Onduleur', 
      voltage: '236 V', 
      status: 'Connecté',
      isActive: false,
      onForce: () => {},
      onDirect: () => {}
    } 
  },
  { 
    id: 'switch', 
    type: 'commutator', 
    position: { x: 450, y: 200 },
    data: { 
      sw1: 'Fermé', 
      sw2: 'Ouvert', 
      middleOFF: 'Désactivé',
      telecomActive: false,
      onTelecomOn: () => {},
      onTelecomOff: () => {},
      onForceOff: () => {}
    } 
  },
  { 
    id: 'reseau', 
    type: 'source', 
    position: { x: 100, y: 400 },
    data: { 
      title: '⚡ Réseau', 
      voltage: '235 V', 
      status: 'Connecté',
      isActive: false,
      onForce: () => {},
      onDirect: () => {}
    } 
  },
];

const initialEdges = [
  { id: 'e1', source: 'onduleur', target: 'switch', animated: false },
  { id: 'e2', source: 'reseau', target: 'switch', animated: false },
];

export default function App() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const [refreshCount, setRefreshCount] = useState(0);
  const [isConnected, setIsConnected] = useState(false);
  
  // Refs pour éviter les dépendances circulaires
  const nodesRef = useRef(nodes);
  const edgesRef = useRef(edges);
  nodesRef.current = nodes;
  edgesRef.current = edges;

  // Fonctions de commande
  const handleForceSource = useCallback(async (cmd, sourceName) => {
    console.log(`🔋 Tentative de forçage ${sourceName}...`);
    try {
      const resp = await fetch(`/api/${cmd}`, { method: 'POST' });
      const data = await resp.json();
      
      if (data.success) {
        console.log(`✅ ${sourceName} forcé avec succès`);
        setTimeout(refreshData, 500);
      } else {
        console.error(`❌ Échec forçage ${sourceName}:`, data.error);
        alert(`Erreur: ${data.error || 'Échec de la commande'}`);
      }
    } catch (e) {
      console.error(`❌ Erreur réseau:`, e);
      alert(`Erreur de connexion: ${e.message}`);
    }
  }, []);

  const handleTelecom = useCallback(async (cmd) => {
    console.log(`📡 Commande télécommande: ${cmd}`);
    try {
      const resp = await fetch(`/api/${cmd}`, { method: 'POST' });
      const data = await resp.json();
      
      if (data.success) {
        console.log(`✅ Télécommande ${cmd === 'remote_on' ? 'activée' : 'désactivée'}`);
        setTimeout(refreshData, 500);
      } else {
        console.error(`❌ Échec télécommande:`, data.error);
        alert(`Erreur: ${data.error || 'Échec de la commande'}`);
      }
    } catch (e) {
      console.error(`❌ Erreur réseau:`, e);
      alert(`Erreur de connexion: ${e.message}`);
    }
  }, []);

  const handleForceOff = useCallback(async () => {
    console.log('⏹️ Forçage OFF...');
    try {
      const resp = await fetch('/api/force_double', { method: 'POST' });
      const data = await resp.json();
      
      if (data.success) {
        console.log('✅ Forçage OFF réussi');
        setTimeout(refreshData, 500);
      } else {
        console.error('❌ Échec forçage OFF:', data.error);
        alert(`Erreur: ${data.error || 'Échec de la commande'}`);
      }
    } catch (e) {
      console.error('❌ Erreur réseau:', e);
      alert(`Erreur de connexion: ${e.message}`);
    }
  }, []);

  const refreshData = useCallback(async () => {
    try {
      const resp = await fetch('/api/read_all');
      const data = await resp.json();

      if (data.success) {
        setRefreshCount(prev => prev + 1);
        setIsConnected(true);

        const sw1 = data.values.sw1 || "";
        const sw2 = data.values.sw2 || "";
        const middleOFF = data.values.middleOFF || "";
        
        let sourceActive = "Aucune";
        if (sw1.includes("Fermé")) {
          sourceActive = "Onduleur";
        } else if (sw2.includes("Fermé")) {
          sourceActive = "Réseau";
        } else if (middleOFF.includes("Fermé") || middleOFF.includes("Active")) {
          sourceActive = "OFF";
        }

        const updatedNodes = nodesRef.current.map(node => {
          if (node.id === 'onduleur') {
            return {
              ...node,
              data: {
                ...node.data,
                voltage: data.values.v1a || '---',
                status: (parseInt(data.values.v1a) || 0) > 50 ? 'Connecté' : 'Déconnecté',
                isActive: sourceActive === 'Onduleur',
                onForce: () => handleForceSource('force_source1', 'Onduleur'),
                onDirect: () => handleForceSource('force_source1', 'Onduleur')
              }
            };
          }
          if (node.id === 'reseau') {
            return {
              ...node,
              data: {
                ...node.data,
                voltage: data.values.v2a || '---',
                status: (parseInt(data.values.v2a) || 0) > 50 ? 'Connecté' : 'Déconnecté',
                isActive: sourceActive === 'Réseau',
                onForce: () => handleForceSource('force_source2', 'Réseau'),
                onDirect: () => handleForceSource('force_source2', 'Réseau')
              }
            };
          }
          if (node.id === 'switch') {
            return {
              ...node,
              data: {
                ...node.data,
                sw1: data.values.sw1 || '---',
                sw2: data.values.sw2 || '---',
                middleOFF: data.values.middleOFF || '---',
                telecomActive: data.values.swRemote === "📡 Activé",
                onTelecomOn: () => handleTelecom('remote_on'),
                onTelecomOff: () => handleTelecom('remote_off'),
                onForceOff: handleForceOff
              }
            };
          }
          return node;
        });

        setNodes(updatedNodes);

        const updatedEdges = edgesRef.current.map(edge => {
          const isActive = (edge.source === 'onduleur' && sourceActive === 'Onduleur') ||
                          (edge.source === 'reseau' && sourceActive === 'Réseau');
          return {
            ...edge,
            animated: isActive,
            style: { 
              stroke: isActive ? '#22c55e' : '#94a3b8',
              strokeWidth: isActive ? 3 : 2
            }
          };
        });

        setEdges(updatedEdges);
      }
    } catch (e) {
      console.error('Erreur de connexion:', e);
      setIsConnected(false);
    }
  }, [handleForceSource, handleTelecom, handleForceOff]);

  useEffect(() => {
    refreshData();
    const interval = setInterval(refreshData, 5000);
    return () => clearInterval(interval);
  }, [refreshData]);

  const onConnect = useCallback((params) => {
    setEdges(eds => [...eds, params]);
  }, []);

  return (
    <div style={{ width: '100vw', height: '100vh' }}>
      {/* Header */}
      <div style={{
        position: 'absolute', top: 20, left: 20, right: 20, zIndex: 10,
        background: 'white', padding: '16px 24px', borderRadius: '16px',
        boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        flexWrap: 'wrap', gap: '12px'
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div style={{
            width: '14px', height: '14px', borderRadius: '50%',
            background: isConnected ? '#22c55e' : '#ef4444'
          }} />
          <strong>{isConnected ? '✅ Connecté' : '❌ Déconnecté'}</strong>
          <span style={{ fontSize: '13px', color: '#64748b' }}>
            Actus: {refreshCount}
          </span>
          <span style={{
            fontSize: '11px', padding: '4px 10px', borderRadius: '20px',
            background: '#fbbf24', color: '#0f172a', fontWeight: 'bold'
          }}>
            BN
          </span>
        </div>
        
        <button 
          onClick={refreshData}
          style={{
            padding: '8px 16px', borderRadius: '20px', border: 'none',
            background: 'linear-gradient(135deg, #3b82f6, #2563eb)',
            color: 'white', fontWeight: 600, cursor: 'pointer'
          }}
        >
          🔄 Actualiser
        </button>
      </div>
      
      {/* ReactFlow */}
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        nodeTypes={nodeTypes}
        fitView
        attributionPosition="bottom-right"
        style={{ marginTop: 100 }}
      >
        <Background color="#cbd5e1" gap={20} />
        <Controls />
      </ReactFlow>
    </div>
  );
}