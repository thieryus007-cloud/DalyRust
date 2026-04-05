# 📚 INDEX DOCUMENTATION SYSTÈME — Dashboard Temps Réel
**Version:** 2.0  
**Date:** 2026-04-05  
**Branche:** `claude/realtime-metrics-dashboard-lUKF3`

---

## 🎯 PAR OÙ COMMENCER?

### Je veux...

#### 🚀 **Déployer le système sur Pi5**
→ Lire: [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) section "PROCÉDURE DÉPLOIEMENT RAPIDE"  
⏱️ Temps: **5 minutes**  
📋 Contient: Commandes exactes, vérifications, validation

#### 📖 **Comprendre comment ça marche**
→ Lire: [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 1-3  
⏱️ Temps: **15 minutes**  
📋 Contient: Architecture globale, flux de données, structures Rust

#### ➕ **Ajouter une nouvelle métrique du NanoPi (Victron D-Bus)**
→ Lire: [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 4  
⏱️ Temps: **30 minutes** (procédure pas-à-pas)  
📋 Contient: 10 étapes concrètes, exemples JSON, code Rust

#### ➕ **Ajouter une nouvelle métrique du Pi5**
→ Lire: [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 5  
⏱️ Temps: **20 minutes** (procédure pas-à-pas)  
📋 Contient: 6 étapes concrètes, polling loop, API endpoint

#### 🔧 **Dépanner un problème**
→ Lire: [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 7  
⏱️ Temps: **Variable selon problème**  
📋 Contient: Diagnostics MQTT, API, frontend, logs

#### 🧪 **Valider que tout fonctionne**
→ Lire: [IMPLEMENTATION_VERIFICATION.md](IMPLEMENTATION_VERIFICATION.md) section 7  
⏱️ Temps: **10 minutes**  
📋 Contient: 5 tests complets avec commandes exactes

#### 📊 **Voir ce qui a été fait**
→ Lire: [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md)  
⏱️ Temps: **10 minutes**  
📋 Contient: Résumé complet, commits, fichiers modifiés, validation

#### 🏗️ **Comprendre l'architecture complète**
→ Lire: [CLAUDE.md](CLAUDE.md) section 1-2 + [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 2  
⏱️ Temps: **20 minutes**  
📋 Contient: Diagrammes ASCII, réseau, structure git

---

## 📄 DOCUMENTS COMPLETS

### 1. CLAUDE.md
**Référence principale du projet (existant)**

**Contient:**
- Commandes Pi5, NanoPi, Claude Code (section 0)
- Architecture globale (section 1)
- Réseau & SSH (section 2)
- Repository GIT (section 3)
- Structure projet (section 4)
- Makefile commandes (section 5)
- Configuration production (section 6)
- Déploiement NanoPi (section 7)
- Services systemd (section 8)
- Stack Docker (section 9)
- Dépannage (section 10)
- Binaires & cibles (section 11)
- API endpoints (section 12)
- Topics MQTT (section 13)
- Dépendances Rust (section 14)
- Inventaire matériel D-Bus (section 19)

**Qui doit lire:** Tous les développeurs — **C'est la référence de base**

---

### 2. DASHBOARD_EXTENSION_GUIDE.md
**Guide complet du dashboard temps réel**

**Sections:**
1. **Vue d'ensemble du système** — Le problème initial et la solution
2. **Architecture détaillée** — Flux complet en 5 étapes (N-aPi → dashboard)
3. **Collecte de données** — Comment fonctionne chaque étape du pipeline
4. **Ajouter métrique NanoPi** — Procédure 10 étapes (générateur exemple)
5. **Ajouter métrique Pi5** — Procédure 6 étapes (température CPU exemple)
6. **Procédures détaillées** — Checklist générique, templates, patterns
7. **Dépannage** — Erreurs communes et solutions
8. **Cas d'usage réels** — Fronius, Shelly, Linky avec code

**Qui doit lire:**
- Développeurs ajoutant de nouvelles métriques
- Architectes système
- Intégrateurs d'appareils externes

---

### 3. IMPLEMENTATION_VERIFICATION.md
**Checklist complète et procédure de validation**

**Sections:**
1. **Checklist implémentation** — 15 catégories couvrant tout
2. **API endpoints** — Référence complète avec exemples
3. **Topics MQTT** — Payloads JSON détaillées
4. **Node mapping** — Correspondance nodes ReactFlow ↔ données
5. **Fichiers modifiés** — Liste complète des changements
6. **Déploiement** — Procédure sur Pi5 étape par étape
7. **Validation tests** — 5 tests concrets avec commandes exactes
8. **Hardware requis** — Appareils nécessaires pour chaque feature

**Qui doit lire:**
- DevOps faisant le déploiement
- QA/Testeurs validant le système
- Anyone voulant une checklist complète

---

### 4. IMPLEMENTATION_COMPLETE.md
**Résumé exécutif de l'implémentation**

**Sections:**
1. **Résumé exécutif** — Avant/après, solution, résultats
2. **Fichiers clés implémentés** — Tous les fichiers avec snippets
3. **Commits détaillés** — 20+ commits organisés par phase
4. **Validation checklist** — Code quality, architecture, frontend, deployment
5. **Procédure déploiement rapide** — 5 minutes sur Pi5
6. **Structure données final** — Topics MQTT, API endpoints, structures Rust
7. **Extension future** — Ce qu'on peut facilement ajouter
8. **Fichiers référence** — Tableau récapitulatif
9. **Tests effectués** — Unit tests, integration, manual tests
10. **Known limitations** — Réaliste sur dépendances
11. **Procédure merge à main** — Quand everything is ready

**Qui doit lire:**
- Managers/stakeholders voulant un résumé
- Développeurs voulant une vue globale rapide
- Team lead validant l'implémentation

---

### 5. DOCUMENTATION_INDEX.md
**Ce fichier — Guide de navigation**

---

## 🗂️ ORGANISATION PAR TÂCHE

### 📝 **TÂCHE: Première lecture du projet**
**Temps requis:** 1 heure

1. Lire [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) (15 min)
   → Comprendre ce qui a été fait, vue globale

2. Lire [CLAUDE.md](CLAUDE.md) sections 0-3 (20 min)
   → Comprendre architecture et commandes de base

3. Lire [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) sections 1-2 (25 min)
   → Comprendre le flux temps réel en détail

### 🚀 **TÂCHE: Déployer sur Pi5**
**Temps requis:** 10 minutes

1. Lire [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) → "PROCÉDURE DÉPLOIEMENT RAPIDE" (3 min)
2. Exécuter les commandes bash (5 min)
3. Vérifier le dashboard (2 min)

### 🔌 **TÂCHE: Intégrer un nouvel appareil Victron**
**Temps requis:** 45 minutes

1. Lire [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 4 (15 min)
   → Comprendre le processus complet

2. Créer Node-RED flow basé sur l'exemple (15 min)
3. Ajouter structures Rust suivant le template (10 min)
4. Compiler et tester (5 min)

### 🔌 **TÂCHE: Intégrer une API externe (Fronius, Shelly, etc.)**
**Temps requis:** 60 minutes

1. Lire [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 5 + cas d'usage réels (20 min)
2. Implémenter la structure Rust (15 min)
3. Ajouter le polling loop / HTTP client (15 min)
4. Créer l'API endpoint (5 min)
5. Mettre à jour le dashboard (5 min)

### 🧪 **TÂCHE: Valider une nouvelle métrique**
**Temps requis:** 15 minutes

1. Lire [IMPLEMENTATION_VERIFICATION.md](IMPLEMENTATION_VERIFICATION.md) section 7 (5 min)
2. Exécuter les tests (10 min)

### 🐛 **TÂCHE: Dépanner un problème**
**Temps requis:** Variable

1. Lire [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 7 (10 min)
2. Suivre le diagnostic spécifique (5-60 min selon le problème)

---

## 📊 DOCUMENTS PAR AUDIENCE

### 👨‍💻 **DÉVELOPPEUR BACKEND**
**Documents essentiels:**
- [CLAUDE.md](CLAUDE.md) — Référence complète
- [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) sections 1-3, 5-6 — Architecture et procédures
- [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) sections "Commits détaillés", "Structure données"

**Temps recommandé:** 2 heures (première lecture)

### 👨‍💼 **DÉVELOPPEUR FRONTEND**
**Documents essentiels:**
- [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) sections 1-2 — Architecture et flux
- [IMPLEMENTATION_VERIFICATION.md](IMPLEMENTATION_VERIFICATION.md) section 2 (API endpoints)
- [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) section "Structure données final"

**Temps recommandé:** 1 heure (première lecture)

### 🔧 **DEVOPS / ADMINISTRATEUR**
**Documents essentiels:**
- [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) section "Procédure déploiement rapide"
- [IMPLEMENTATION_VERIFICATION.md](IMPLEMENTATION_VERIFICATION.md) sections 6-7
- [CLAUDE.md](CLAUDE.md) sections 0, 5-9

**Temps recommandé:** 1.5 heure (première lecture + déploiement)

### 👔 **MANAGER / STAKEHOLDER**
**Documents essentiels:**
- [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) sections 1-3 — Résumé exécutif
- Ce fichier [DOCUMENTATION_INDEX.md](DOCUMENTATION_INDEX.md)

**Temps recommandé:** 30 minutes

### 📚 **QA / TESTEUR**
**Documents essentiels:**
- [IMPLEMENTATION_VERIFICATION.md](IMPLEMENTATION_VERIFICATION.md) sections 1, 7-8
- [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) section "Tests effectués"
- [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 7

**Temps recommandé:** 1.5 heure

---

## 🔄 FLUX DE TRAVAIL COMPLET

```
┌─────────────────────────────────────────────────────────────┐
│ 1. LECTURE INITIALE (1 heure)                               │
│    → IMPLEMENTATION_COMPLETE.md + CLAUDE.md basics           │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. DÉPLOIEMENT (10 minutes)                                 │
│    → Commandes Pi5 + import Node-RED flows                  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. VALIDATION (15 minutes)                                  │
│    → Test suite per IMPLEMENTATION_VERIFICATION.md           │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. EXTENSION (variable selon tâche)                         │
│    → Suivre guide spécifique par tâche (voir section ci-   │
│       dessus)                                                │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 5. DÉPANNAGE SI BESOIN (variable)                          │
│    → DASHBOARD_EXTENSION_GUIDE.md section 7                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔗 LIENS RAPIDES

### Documentation Existante
- [CLAUDE.md](CLAUDE.md) — Référence principale projet
- [Readme.md](Readme.md) — Documentation utilisateur générale

### Documentation Nouvelle
- [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) — 😍 **À lire en priorité pour extension**
- [IMPLEMENTATION_VERIFICATION.md](IMPLEMENTATION_VERIFICATION.md) — À lire avant déploiement
- [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) — À lire pour résumé
- [DOCUMENTATION_INDEX.md](DOCUMENTATION_INDEX.md) — Ce fichier

### Fichiers Code Clés
- `crates/daly-bms-server/src/state.rs` — Structures Venus
- `crates/daly-bms-server/src/bridges/mqtt.rs` — MQTT handlers
- `crates/daly-bms-server/src/api/system.rs` — API endpoints
- `crates/daly-bms-server/templates/visualization.html` — Dashboard
- `flux-nodered/inverter.json` — MultiPlus flow (NEW)
- `flux-nodered/smartshunt.json` — SmartShunt flow (NEW)

### Configurations
- `Config.toml` — Configuration production Pi5
- `nanoPi/config-nanopi.toml` — Configuration NanoPi
- `docker-compose.infra.yml` — Stack Docker

---

## ✅ CHECKLIST LECTURE

### ☐ Avant de déployer
- ☐ Lire [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) section "Résumé exécutif"
- ☐ Lire [IMPLEMENTATION_VERIFICATION.md](IMPLEMENTATION_VERIFICATION.md) section 6
- ☐ Comprendre les dépendances (section "Known limitations")

### ☐ Avant d'ajouter une métrique
- ☐ Lire le guide spécifique (section 4 ou 5 de DASHBOARD_EXTENSION_GUIDE.md)
- ☐ Préparer le code suivant le template
- ☐ Lire la section "Procédures détaillées"
- ☐ Consulter les cas d'usage réels pour patterns similaires

### ☐ Avant de dépanner
- ☐ Lire [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 7
- ☐ Collecter les logs (journalctl, docker logs, mosquitto_sub)
- ☐ Consulter la checklist de dépannage

---

## 📞 SUPPORT RAPIDE

| Problème | Solution |
|----------|----------|
| "Où commencer?" | → [Lire ce fichier](#par-où-commencer) |
| "Comment déployer?" | → [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md) + "Procédure déploiement rapide" |
| "Ajouter une métrique?" | → [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 4 ou 5 |
| "Ça ne marche pas" | → [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) section 7 |
| "API retourne null" | → DASHBOARD_EXTENSION_GUIDE.md section 7 → "Problème: Nouveau endpoint retourne connected: false" |
| "Dashboard affiche —" | → DASHBOARD_EXTENSION_GUIDE.md section 7 → "Problème: Dashboard affiche — au lieu de la valeur" |
| "MQTT ne publie pas" | → DASHBOARD_EXTENSION_GUIDE.md section 7 → "Problème: Nouveau endpoint retourne connected: false" |
| "Compiler échoue" | → DASHBOARD_EXTENSION_GUIDE.md section 7 → "Erreur commune: struct n'implémente pas Serialize" |

---

## 🎓 APPRENTISSAGE PROGRESSIF

### Niveau 1: Débutant
**Objectif:** Comprendre et déployer le système existant
1. Lire [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md)
2. Déployer en suivant "Procédure déploiement rapide"
3. Vérifier le dashboard fonctionne
**Temps:** 2 heures

### Niveau 2: Intermédiaire
**Objectif:** Ajouter une nouvelle métrique simple
1. Lire [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md) sections 1-3
2. Suivre procédure section 4 ou 5 (selon source données)
3. Tester et valider
**Temps:** 4 heures

### Niveau 3: Avancé
**Objectif:** Intégrer une API complexe et maîtriser le système
1. Étudier tous les documents
2. Lire le code Rust complet (state.rs, mqtt.rs, api/system.rs)
3. Implémenter intégration complexe (OAuth, authentification, etc.)
4. Contribuer au codebase
**Temps:** 8+ heures

---

## 🚀 PROCHAINES ÉTAPES

### Immédiatement
- ✅ Lire ce fichier
- ✅ Choisir votre rôle (dev, ops, etc.)
- ✅ Lire les documents recommandés

### Cette semaine
- Deploy sur Pi5 en suivant [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md)
- Valider en suivant [IMPLEMENTATION_VERIFICATION.md](IMPLEMENTATION_VERIFICATION.md)
- Vérifier que le dashboard affiche toutes les métriques

### Cette itération
- Ajouter 1-2 nouvelles métriques en suivant [DASHBOARD_EXTENSION_GUIDE.md](DASHBOARD_EXTENSION_GUIDE.md)
- Dépanner les issues trouvées pendant déploiement

### Long terme
- Intégrer API externes (Fronius, etc.)
- Ajouter contrôles (POST endpoints)
- Développer alertes et automations

---

**Document:** DOCUMENTATION_INDEX.md  
**Version:** 2.0  
**Mise à jour:** 2026-04-05  
**Status:** ✅ Complète et à jour
