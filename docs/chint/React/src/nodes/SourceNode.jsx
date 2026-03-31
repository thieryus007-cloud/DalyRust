import React from 'react';
import { Handle, Position } from 'reactflow';

export default function SourceNode({ data }) {
  const { 
    title, voltage, maxVoltage, commutations, t1, 
    status, isActive, onForce, onDirect 
  } = data;
  
  return (
    <div style={{ 
      padding: '16px', 
      minWidth: '280px',
      border: isActive ? '2px solid #ef4444' : '1px solid #e2e8f0',
      borderRadius: '16px',
      background: '#fff',
      boxShadow: isActive ? '0 0 20px rgba(239,68,68,0.3)' : '0 4px 12px rgba(0,0,0,0.05)',
      animation: isActive ? 'blink-red 1.2s infinite' : 'none'
    }}>
      <Handle type="source" position={Position.Right} style={{ background: '#3b82f6' }} />
      
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '12px' }}>
        <div style={{ 
          width: '14px', height: '14px', borderRadius: '50%',
          background: status === 'Connecté' ? '#22c55e' : '#ef4444',
          boxShadow: status === 'Connecté' ? '0 0 8px #22c55e' : 'none'
        }} />
        <strong style={{ fontSize: '16px' }}>{title}</strong>
        <span style={{ 
          fontSize: '11px', padding: '4px 8px', borderRadius: '12px',
          background: status === 'Connecté' ? 'rgba(34,197,94,0.2)' : 'rgba(239,68,68,0.2)',
          color: status === 'Connecté' ? '#16a34a' : '#ef4444'
        }}>
          {status}
        </span>
      </div>
      
      <div style={{ fontSize: '13px', color: '#64748b', display: 'flex', flexDirection: 'column', gap: '8px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span>Phase A</span>
          <strong style={{ color: '#0f172a', fontFamily: 'monospace' }}>{voltage}</strong>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span>Max enregistré</span>
          <strong style={{ color: '#0f172a', fontFamily: 'monospace' }}>{maxVoltage}</strong>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span>Commutations</span>
          <strong style={{ color: '#0f172a', fontFamily: 'monospace' }}>{commutations}</strong>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span>T1 (transfert)</span>
          <strong style={{ color: '#d97706', fontFamily: 'monospace' }}>{t1}</strong>
        </div>
      </div>
      
      <div style={{ 
        marginTop: '12px', 
        paddingTop: '12px', 
        borderTop: '1px solid #e2e8f0',
        display: 'flex',
        gap: '8px'
      }}>
        <button
          onClick={onForce}
          style={{
            flex: 1,
            padding: '8px 12px',
            border: 'none',
            borderRadius: '20px',
            background: '#f59e0b',
            color: 'white',
            fontWeight: 600,
            fontSize: '11px',
            cursor: 'pointer'
          }}
        >
          🔋 Forcer (auto)
        </button>
        <button
          onClick={onDirect}
          style={{
            padding: '8px 12px',
            border: '1px solid rgba(59,130,246,0.4)',
            borderRadius: '20px',
            background: 'rgba(59,130,246,0.15)',
            color: '#1e40af',
            fontSize: '11px',
            cursor: 'pointer'
          }}
        >
          ⚡ Direct
        </button>
      </div>
      
      <div style={{ 
        fontSize: '10px', 
        color: '#d97706', 
        marginTop: '6px',
        fontStyle: 'italic'
      }}>
        💡 Auto = active/désactive la télécommande automatiquement
      </div>
      
      <Handle type="target" position={Position.Left} style={{ background: '#3b82f6' }} />
    </div>
  );
}