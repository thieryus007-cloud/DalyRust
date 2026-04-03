GUIDE DE PERSONNALISATION - Styles des NodeTypes

Version : 1.0
Date : Avril 2026

---

1. Stratégie recommandée

Deux approches possibles :

Approche Description Quand l'utiliser
Variables CSS globales Définir des couleurs, tailles, polices dans un fichier commun Pour une charte graphique cohérente
Props individuelles Modifier chaque nœud via ses data Pour des variations spécifiques

---

2. Solution 1 : Variables CSS globales (Recommandée)

Étape 1 : Créer un fichier CSS commun

Fichier : src/components/nodes/common.css

```css
/* VARIABLES GLOBALES - Modifiez UN SEUL endroit pour tout changer */
:root {
  /* Tailles des nœuds */
  --node-min-width: 260px;
  --node-padding: 16px;
  --node-border-radius: 20px;
  --node-border-width: 2px;
  
  /* Polices */
  --font-family: 'Segoe UI', 'Roboto', monospace;
  --font-size-title: 13px;
  --font-size-value-large: 36px;
  --font-size-value: 13px;
  --font-size-label: 9px;
  --font-size-small: 8px;
  
  /* Espacements */
  --spacing-small: 8px;
  --spacing-medium: 12px;
  --spacing-large: 16px;
  
  /* Ombres */
  --shadow-hover: 0 4px 12px rgba(0,0,0,0.3);
  --shadow-glow: 0 0 8px;
  
  /* Arrière-plans */
  --bg-dark: #0d1117;
  --bg-card: #1a1f2e;
  --bg-card-light: #252a3e;
  
  /* Couleurs fonctionnelles */
  --color-charge: #4caf50;
  --color-decharge: #f44336;
  --color-idle: #ff9800;
  --color-production: #2196f3;
  --color-text-primary: #fff;
  --color-text-secondary: #ddd;
  --color-text-muted: #888;
  --color-border: #2a2f3e;
}

/* Application aux nœuds */
.node-common {
  font-family: var(--font-family);
  transition: all 0.3s ease;
}

.node-common:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-hover);
}
```

Étape 2 : Importer le fichier commun dans chaque nœud

Exemple pour MPPTNode.jsx :

```jsx
import { Handle, Position } from '@xyflow/react';
import './common.css';  // ← À ajouter en premier
import './mpptAnimations.css';

const MPPTNode = ({ id, data }) => {
  // ... reste du code
```

Étape 3 : Utiliser les variables dans chaque CSS

Exemple - batteryAnimations.css modifié :

```css
.battery-node {
  min-width: var(--node-min-width);
  padding: var(--node-padding);
  border-radius: var(--node-border-radius);
  border: var(--node-border-width) solid;
  font-family: var(--font-family);
  background: var(--bg-dark);
}

.battery-node:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-hover);
}

.battery-header {
  font-size: var(--font-size-title);
  color: var(--color-text-primary);
}

.soc-value {
  font-size: var(--font-size-value-large);
  font-weight: bold;
}

.metric-label {
  font-size: var(--font-size-label);
  color: var(--color-text-muted);
}

.metric-value {
  font-size: var(--font-size-value);
  color: var(--color-text-secondary);
}
```

---

3. Solution 2 : Props individuelles par nœud

Modifier un nœud spécifique via ses data

Dans VisualisationComplete.jsx :

```jsx
const initialNodes = [
  {
    id: 'mppt-chargeur',
    type: 'mppt',
    position: { x: 100, y: 100 },
    data: {
      label: 'Chargeur PV',
      totalPower: 1169,
      mppts: [...],
      // ⬇️ STYLES PERSONNALISÉS pour CE nœud uniquement
      style: {
        minWidth: '300px',
        backgroundColor: '#1a2a1a',
        borderColor: '#ff9800'
      },
      className: 'custom-mppt',
      fontSize: '14px'
    }
  }
];
```

Adapter le composant pour utiliser ces props

Dans MPPTNode.jsx :

```jsx
const MPPTNode = ({ id, data }) => {
  const {
    // ... autres props
    style = {},
    className = ''
  } = data;

  return (
    <div 
      className={`mppt-node ${className}`}
      style={{
        ...style,  // Applique les styles personnalisés
        // Les styles par défaut sont écrasés par les props
      }}
    >
      {/* ... reste du JSX */}
    </div>
  );
};
```

---

4. Tableau récapitulatif - Variables à modifier

Ce que vous voulez changer Variable CSS à modifier
Largeur de TOUS les nœuds --node-min-width
Bordures --node-border-radius, --node-border-width
Police principale --font-family
Taille des titres --font-size-title
Taille des grandes valeurs (SOC, puissance) --font-size-value-large
Taille des métriques --font-size-value
Taille des labels --font-size-label
Couleur de charge --color-charge
Couleur de décharge --color-decharge
Couleur de production --color-production
Fond des cartes --bg-dark, --bg-card

---

5. Exemple complet - Changer la police pour tout le projet

Étape 1 : Importer une police Google dans index.html

```html
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;700&display=swap" rel="stylesheet">
```

Étape 2 : Modifier la variable dans common.css

```css
:root {
  --font-family: 'Inter', 'Segoe UI', monospace;
}
```

Étape 3 : Tous les nœuds héritent automatiquement de la nouvelle police

---

6. Exemple - Agrandir tous les nœuds

Modifiez UNE SEULE variable dans common.css :

```css
:root {
  --node-min-width: 320px;  /* Au lieu de 260px */
  --node-padding: 20px;      /* Au lieu de 16px */
}
```

---

7. Structure finale recommandée

```
src/components/nodes/
├── common.css                 ← VARIABLES GLOBALES (à créer)
├── BatteryNode.jsx
├── batteryAnimations.css      ← Utilise les variables
├── ET112Node.jsx
├── et112Animations.css
├── SwitchNode.jsx
├── switchAnimations.css
├── ShuntNode.jsx
├── shuntAnimations.css
├── MeteoNode.jsx
├── meteoAnimations.css
├── TemperatureNode.jsx
├── temperatureAnimations.css
├── MPPTNode.jsx
└── mpptAnimations.css
```

---

8. Ajout de common.css dans chaque composant

Modèle pour TOUS les fichiers JSX :

```jsx
import { Handle, Position } from '@xyflow/react';
import './common.css';  // ← TOUJOURS en premier
import './nomDuStyle.css';

// ... reste du code
```

---

9. Résumé - Ce que vous devez faire

1. Créer src/components/nodes/common.css avec les variables ci-dessus
2. Ajouter import './common.css'; dans chaque fichier JSX (en premier)
3. Modifier chaque fichier CSS pour remplacer les valeurs fixes par var(--nom-variable)
4. Toutes les modifications se feront désormais dans un seul fichier : common.css

---

Avec cette méthode, vous modifiez l'apparence de TOUS les nœuds en changeant UNE SEULE variable.
