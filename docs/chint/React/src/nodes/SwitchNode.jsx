import React from 'react';
import { Handle, Position } from 'reactflow';

export default function SwitchNode({ data }) {
  const { 
    sw1, sw2, middleOFF, telecomActive,
    onTelecomOn, onTelecomOff, onForceOff 
  } = data;
  
  return (
    <div style={{ 
      padding: '16px', 
      minWidth: '320px',
      border: '1px solid #e2e8f0',
      borderRadius: '16px',
      background: '#fff'
    }}>
      <Handle type="target" position={Position.Left} style={{ top: '30%', background: '#3b82f6' }} />
      <Handle type="target" position={Position.Left} style={{ top: '70%', background: '#3b82f6' }} />
      <Handle type="source" position={Position.Right} style={{ background: '#3b82f6' }} />
      
      <div style={{ 
        fontWeight: 700, 
        marginBottom: '16px', 
        display: 'flex', 
        alignItems: 'center', 
        gap: '8px',
        fontSize: '16px'
      }}>
        🔄 État Commutation
        <span style={{ 
          fontSize: '11px', padding: '4px 10px', borderRadius: '20px',
          background: telecomActive ? '#22c55e' : '#e2e8f0',
          color: telecomActive ? 'white' : '#64748b',
          marginLeft: 'auto',
          display: 'flex',
          alignItems: 'center',
          gap: '4px'
        }}>
          {telecomActive ? '📡 ON' : '🔒 OFF'}
        </span>
      </div>
      
      <div style={{ fontSize: '12px', color: '#64748b', display: 'flex', flexDirection: 'column', gap: '8px', marginBottom: '16px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span>SW1 Onduleur</span>
          <strong style={{ color: sw1 === 'Fermé' ? '#16a34a' : '#64748b' }}>{sw1}</strong>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span>SW2 Réseau</span>
          <strong style={{ color: sw2 === 'Fermé' ? '#16a34a' : '#64748b' }}>{sw2}</strong>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <span>middleOFF</span>
          <strong>{middleOFF}</strong>
        </div>
      </div>
      
      <div style={{ 
        paddingTop: '12px', 
        borderTop: '1px solid #e2e8f0',
        display: 'flex',
        flexDirection: 'column',
        gap: '10px'
      }}>
        <div style={{ display: 'flex', gap: '10px' }}>
          <button
            onClick={onTelecomOn}
            disabled={!onTelecomOn || telecomActive}
            style={{
              flex: 1,
              padding: '10px 16px',
              border: 'none',
              borderRadius: '20px',
              background: onTelecomOn && !telecomActive ? '#22c55e' : '#e2e8f0',
              color: onTelecomOn && !telecomActive ? 'white' : '#94a3b8',
              fontWeight: 600,
              fontSize: '12px',
              cursor: onTelecomOn && !telecomActive ? 'pointer' : 'not-allowed',
              opacity: onTelecomOn && !telecomActive ? 1 : 0.6
            }}
          >
            📡 Activer
          </button>
          <button
            onClick={onTelecomOff}
            disabled={!onTelecomOff || !telecomActive}
            style={{
              flex: 1,
              padding: '10px 16px',
              border: 'none',
              borderRadius: '20px',
              background: onTelecomOff && telecomActive ? '#ef4444' : '#e2e8f0',
              color: onTelecomOff && telecomActive ? 'white' : '#94a3b8',
              fontWeight: 600,
              fontSize: '12px',
              cursor: onTelecomOff && telecomActive ? 'pointer' : 'not-allowed',
              opacity: onTelecomOff && telecomActive ? 1 : 0.6
            }}
          >
            🔒 Désactiver
          </button>
        </div>
        <button
          onClick={onForceOff}
          disabled={!onForceOff || !telecomActive}
          style={{
            padding: '10px 16px',
            border: 'none',
            borderRadius: '20px',
            background: onForceOff && telecomActive ? '#ef4444' : '#f1f5f9',
            color: onForceOff && telecomActive ? 'white' : '#94a3b8',
            fontWeight: 600,
            fontSize: '12px',
            cursor: onForceOff && telecomActive ? 'pointer' : 'not-allowed',
            opacity: onForceOff && telecomActive ? 1 : 0.6
          }}
        >
          ⏹️ Forcer OFF
        </button>
      </div>
      
      <div style={{ 
        fontSize: '10px', 
        color: '#d97706', 
        marginTop: '10px',
        fontStyle: 'italic',
        display: 'flex',
        alignItems: 'center',
        gap: '4px'
      }}>
        ⚠️ "Forcer OFF" et "Direct" nécessitent télécommande active
      </div>
    </div>
  );
}