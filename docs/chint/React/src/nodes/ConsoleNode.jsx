import React, { useState } from 'react';
import { Handle, Position } from 'reactflow';

export default function ConsoleNode({ data }) {
  const [frame, setFrame] = useState('06 03 00 4F 00 01');
  const [autoCrc, setAutoCrc] = useState(true);
  const [response, setResponse] = useState('-- Aucune requête envoyée --');
  const [history, setHistory] = useState([]);

  const handleSend = async () => {
    let frameToSend = frame;
    if (autoCrc && data.onCalculateCrc) {
      frameToSend = data.onCalculateCrc(frame);
    }

    const result = await data.onSendFrame(frameToSend);
    
    if (result.success) {
      setResponse(`✅ Réponse (${result.response_length} octets)\n${result.response_hex}`);
      setHistory(prev => [{
        time: new Date().toLocaleTimeString('fr-FR'),
        send: frameToSend,
        recv: result.response_hex
      }, ...prev].slice(0, 10));
    } else {
      setResponse(`❌ ${result.error}`);
    }
  };

  const handleCalculate = () => {
    if (data.onCalculateCrc) {
      const withCrc = data.onCalculateCrc(frame);
      alert(`CRC calculé :\nSans CRC : ${frame}\nAvec CRC : ${withCrc}`);
    }
  };

  const handleClear = () => {
    setHistory([]);
    setResponse('-- Aucune requête envoyée --');
  };

  return (
    <div style={{
      padding: '16px',
      minWidth: '500px',
      border: '1px solid #3b82f6',
      borderRadius: '16px',
      background: '#fff'
    }}>
      <Handle type="target" position={Position.Top} style={{ background: '#3b82f6' }} />
      
      <h3 style={{ margin: '0 0 12px 0', color: '#1e40af', fontSize: '16px' }}>
        🔧 Console Modbus - Envoi de trame hexadécimale
      </h3>
      
      <div style={{ display: 'flex', gap: '12px', marginBottom: '12px', flexWrap: 'wrap', alignItems: 'center' }}>
        <input
          type="text"
          value={frame}
          onChange={(e) => setFrame(e.target.value)}
          placeholder="Ex: 06 03 00 4F 00 01"
          style={{
            flex: 2,
            minWidth: '250px',
            padding: '8px 12px',
            border: '1px solid #3b82f6',
            borderRadius: '8px',
            fontFamily: 'monospace',
            fontSize: '13px'
          }}
        />
        <label style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px' }}>
          <input
            type="checkbox"
            checked={autoCrc}
            onChange={(e) => setAutoCrc(e.target.checked)}
          />
          Ajouter CRC automatiquement
        </label>
        <button
          onClick={handleCalculate}
          style={{
            padding: '8px 16px',
            border: 'none',
            borderRadius: '8px',
            background: '#e2e8f0',
            color: '#0f172a',
            fontWeight: 'bold',
            cursor: 'pointer',
            fontFamily: 'monospace'
          }}
        >
          🔢 Calculer CRC
        </button>
        <button
          onClick={handleSend}
          style={{
            padding: '8px 16px',
            border: 'none',
            borderRadius: '8px',
            background: '#3b82f6',
            color: 'white',
            fontWeight: 'bold',
            cursor: 'pointer'
          }}
        >
          📨 ENVOYER
        </button>
        <button
          onClick={handleClear}
          style={{
            padding: '8px 16px',
            border: 'none',
            borderRadius: '8px',
            background: '#e2e8f0',
            color: '#0f172a',
            cursor: 'pointer'
          }}
        >
          🗑️ Effacer
        </button>
      </div>
      
      <div style={{
        background: '#f8fafc',
        borderRadius: '8px',
        padding: '12px',
        marginBottom: '12px',
        border: '1px solid #e2e8f0'
      }}>
        <div style={{ fontSize: '11px', color: '#64748b', marginBottom: '4px' }}>
          📥 Réponse reçue :
        </div>
        <div style={{
          fontFamily: 'monospace',
          fontSize: '12px',
          color: response.includes('✅') ? '#16a34a' : response.includes('❌') ? '#ef4444' : '#64748b',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-all'
        }}>
          {response}
        </div>
      </div>
      
      {history.length > 0 && (
        <div style={{
          background: '#f8fafc',
          borderRadius: '8px',
          padding: '12px',
          maxHeight: '120px',
          overflowY: 'auto',
          fontFamily: 'monospace',
          fontSize: '11px'
        }}>
          <div style={{ color: '#64748b', marginBottom: '8px' }}>
            📋 Historique des trames (envoi/réception)
          </div>
          {history.map((h, i) => (
            <div key={i} style={{ 
              padding: '4px 0', 
              borderBottom: i < history.length - 1 ? '1px solid #e2e8f0' : 'none'
            }}>
              <div style={{ color: '#1e40af' }}>📤 [{h.time}] {h.send}</div>
              <div style={{ color: '#16a34a' }}>📥 {h.recv}</div>
            </div>
          ))}
        </div>
      )}
      
      <div style={{
        fontSize: '11px',
        color: '#d97706',
        marginTop: '8px'
      }}>
        💡 Saisissez la trame sans CRC (adresse + fonction + registre + données). Le CRC sera calculé automatiquement si la case est cochée.
      </div>
      
      <Handle type="source" position={Position.Bottom} style={{ background: '#3b82f6' }} />
    </div>
  );
}