# Vérification du Protocole d'Écriture - Registre 343

## Question
Lorsqu'on modifie le registre 343 (Protocol) avec la valeur 1, est-ce qu'on adresse bien les 8 bits LSB du registre ?

## Réponse : ✅ OUI, CONFORME

### Analyse Détaillée

#### 1. Encodage de la Valeur 1

Pour le registre 343 avec valeur 1 :

```
Valeur décimale : 1
Valeur 16 bits  : 0000000000000001
                  └──MSB──┘└──LSB──┘
                  00000000  00000001

MSB (bits 15-8) : 0x00
LSB (bits 7-0)  : 0x01
```

#### 2. Trame Modbus Générée

```
AA 10 01 57 00 01 02 00 01 F1 80

Décomposition :
- AA       : Start byte
- 10       : Function Write Multiple Registers
- 01 57    : Adresse registre 343 (0x0157)
- 00 01    : Quantité de registres (1)
- 02       : Nombre d'octets de données (2)
- 00 01    : Valeur à écrire (MSB=0x00, LSB=0x01)
           └─MSB  └─LSB
- F1 80    : CRC16 Modbus
```

#### 3. Conformité avec la Documentation

Selon `TinyBMS_service.ts` :

```typescript
export function buildWriteRegisterCommand(addr: number, value: number): Uint8Array {
    const buf = [
        0xAA,
        0x10,
        (addr >> 8) & 0xFF,
        addr & 0xFF,
        0x00,
        0x01,
        0x02,
        (value >> 8) & 0xFF,  // Data MSB
        value & 0xFF          // Data LSB
    ];
    // ...
}
```

**Notre implémentation** (tinybms.js ligne 154-155) :
```javascript
const dataBytes = Buffer.alloc(2);
dataBytes.writeUInt16BE(rawValue);
```

**Résultat** :
- `writeUInt16BE(1)` produit : `[0x00, 0x01]`
- Formule doc pour value=1 :
  - MSB = `(1 >> 8) & 0xFF` = `0x00` ✅
  - LSB = `1 & 0xFF` = `0x01` ✅

### 4. Tests Complémentaires

| Registre | Valeur | MSB | LSB | Trame Data | Conforme |
|----------|--------|-----|-----|------------|----------|
| 343 | 0 | 0x00 | 0x00 | `00 00` | ✅ |
| 343 | 1 | 0x00 | 0x01 | `00 01` | ✅ |
| 340 | 0 | 0x00 | 0x00 | `00 00` | ✅ |
| 340 | 1 | 0x00 | 0x01 | `00 01` | ✅ |
| 322 | 2000 | 0x07 | 0xD0 | `07 D0` | ✅ |
| 322 | 5000 | 0x13 | 0x88 | `13 88` | ✅ |

## Conclusion

**✅ La valeur 1 écrite au registre 343 adresse bien les 8 bits LSB (Least Significant Byte)**

L'implémentation actuelle :
1. Utilise `writeUInt16BE()` qui encode en Big Endian (MSB first)
2. Place correctement la valeur 1 dans le LSB (0x01)
3. Met le MSB à 0x00 (car 1 < 256)
4. Correspond exactement au protocole TinyBMS Communication Rev D
5. Envoie la trame Modbus correctement formatée

### Format Big Endian (Modbus Standard)

```
Registre UINT16 (16 bits)
┌──────────────┬──────────────┐
│   MSB (H)    │   LSB (L)    │
│  Bits 15-8   │  Bits 7-0    │
│    0x00      │    0x01      │
└──────────────┴──────────────┘
     Octet 0        Octet 1
```

Pour la valeur 1 :
- Les bits 7-0 (LSB) contiennent : `00000001` (= 1)
- Les bits 15-8 (MSB) contiennent : `00000000` (= 0)

**La réponse est donc : OUI, absolument conforme ! ✅**
