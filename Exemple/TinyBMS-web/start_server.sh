#!/bin/bash

# Script de lancement du serveur web TinyBMS
# Ce script démarre le serveur Node.js pour l'interface web TinyBMS

echo "=========================================="
echo "  Démarrage du serveur TinyBMS Web"
echo "=========================================="
echo ""

# Détection du répertoire du script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
cd "$SCRIPT_DIR"

# Vérification de Node.js
if ! command -v node &> /dev/null; then
    echo "❌ Erreur: Node.js n'est pas installé"
    echo "   Installez Node.js depuis https://nodejs.org/"
    exit 1
fi

# Vérification des dépendances
if [ ! -d "node_modules" ]; then
    echo "📦 Installation des dépendances npm..."
    npm install
    if [ $? -ne 0 ]; then
        echo "❌ Erreur lors de l'installation des dépendances"
        exit 1
    fi
fi

# Affichage des ports série disponibles
echo "🔍 Recherche des ports USB disponibles..."
PORTS=$(ls /dev/tty{USB,ACM}* 2>/dev/null)
if [ -z "$PORTS" ]; then
    echo "⚠️  Aucun port USB détecté pour le moment"
    echo "   Vous pourrez sélectionner le port depuis l'interface web"
    echo "   ou utiliser le mode SIMULATION"
else
    echo "✅ Ports USB détectés:"
    echo "$PORTS" | while read -r port; do
        echo "   - $port"
    done
fi

echo ""
echo "🚀 Démarrage du serveur..."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Interface web accessible à:"
echo "  👉 http://localhost:3000"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 Conseils:"
echo "   - Connectez le TinyBMS via USB avant de lancer l'interface"
echo "   - Sélectionnez 'SIMULATION' pour tester sans matériel"
echo "   - Utilisez Ctrl+C pour arrêter le serveur"
echo ""

# Démarrage du serveur
npm start
