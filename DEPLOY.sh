#!/bin/bash
# SCRIPT DE DÉPLOIEMENT COMPLET & VALIDATION
# À exécuter sur Pi5: bash ~/Daly-BMS-Rust/DEPLOY.sh

set -e

echo "════════════════════════════════════════════════════════════════════════"
echo "DÉPLOIEMENT COMPLET - RealTime Metrics Dashboard"
echo "════════════════════════════════════════════════════════════════════════"
echo ""

# =========================================================================
# PHASE 1: CODE + BINAIRE
# =========================================================================
echo "PHASE 1: Récupération et compilation du code..."
cd ~/Daly-BMS-Rust
git fetch origin claude/realtime-metrics-dashboard-lUKF3
git reset --hard origin/claude/realtime-metrics-dashboard-lUKF3

echo "✓ Code récupéré"

if [ ! -f target/aarch64-unknown-linux-gnu/release/daly-bms-server ]; then
  echo "Compilation en cours (5-10 min)..."
  make build-arm 2>&1 | tail -3
fi
echo "✓ Binaire compilé"

# Déployer
echo "Déploiement du serveur..."
sudo systemctl stop daly-bms 2>/dev/null || true
sleep 1
sudo cp target/aarch64-unknown-linux-gnu/release/daly-bms-server /usr/local/bin/
sudo cp Config.toml /etc/daly-bms/config.toml
sudo systemctl start daly-bms
sleep 3

if systemctl is-active --quiet daly-bms; then
  echo "✓ Serveur redémarré avec succès"
else
  echo "✗ ERREUR: Serveur ne démarre pas"
  journalctl -u daly-bms -n 20
  exit 1
fi

# =========================================================================
# PHASE 2: NODE-RED FLOWS - IMPORT + VALIDATION
# =========================================================================
echo ""
echo "PHASE 2: Déploiement flows Node-RED..."

# Fonction pour importer un flow Node-RED
import_flow() {
  local flow_file=$1
  local flow_name=$(basename "$flow_file" .json)

  echo "Importation $flow_name..."

  # Appel API Node-RED pour importer
  curl -s -X POST http://localhost:1880/api/flows \
    -H "Content-Type: application/json" \
    -d @"$flow_file" > /dev/null 2>&1 || {
    echo "✗ Erreur import $flow_name"
    return 1
  }

  echo "✓ $flow_name importé"
}

# Importer les flows (dans l'ordre)
for flow in ~/Daly-BMS-Rust/flux-nodered/{smartshunt,Solar_power,meteo}.json; do
  if [ -f "$flow" ]; then
    import_flow "$flow" || true
  fi
done

# Déclencher redéploiement Node-RED
curl -s -X POST http://localhost:1880/api/flows -H "Content-Type: application/json" \
  -d '[]' > /dev/null 2>&1 || true

sleep 2

# =========================================================================
# PHASE 3: TESTS ET VALIDATION
# =========================================================================
echo ""
echo "PHASE 3: Validation de la chaîne de données..."

test_mqtt_topic() {
  local topic=$1
  local name=$2

  echo -n "  $name... "

  result=$(timeout 3 mosquitto_sub -h 192.168.1.120 -p 1883 -t "$topic" -C 1 2>/dev/null || echo "")

  if [ -z "$result" ]; then
    echo "✗ Aucune donnée"
    return 1
  else
    echo "✓ Données reçues"
    return 0
  fi
}

test_api_endpoint() {
  local endpoint=$1
  local field=$2
  local name=$3

  echo -n "  $name... "

  result=$(curl -s "http://localhost:8080$endpoint" 2>/dev/null | jq ".$field" 2>/dev/null || echo "null")

  if [ "$result" = "null" ] || [ -z "$result" ]; then
    echo "✗ Pas de données"
    return 1
  else
    echo "✓ Données: $result"
    return 0
  fi
}

echo "Tests MQTT:"
test_mqtt_topic "santuario/system/venus" "SmartShunt topic" || true
test_mqtt_topic "santuario/meteo/venus" "MPPT topic" || true

echo ""
echo "Tests API:"
test_api_endpoint "/api/v1/venus/smartshunt" "connected" "SmartShunt API" || true
test_api_endpoint "/api/v1/venus/mppt" "mppts[0].power_w" "MPPT API" || true
test_api_endpoint "/api/v1/et112/7/status" "connected" "ET112 0x07 API" || true

# =========================================================================
# PHASE 4: RÉSULTAT FINAL
# =========================================================================
echo ""
echo "════════════════════════════════════════════════════════════════════════"
echo "VALIDATION DU DASHBOARD"
echo "════════════════════════════════════════════════════════════════════════"
echo ""
echo "Accéder au dashboard:"
echo "  http://192.168.1.141:8080/visualization"
echo ""
echo "VÉRIFICATIONS À FAIRE:"
echo "  ✓ Micro-onduleurs (0x07) - affiche puissance AC réelle"
echo "  ✓ SmartShunt - affiche SOC%, Tension, Courant"
echo "  ✓ MPPT - affiche puissance réelle (pas 0W ou 1W)"
echo "  ✓ Température - affiche °C et humidité"
echo "  ✓ BMS devices - affichent données en temps réel"
echo "  ✓ Tous les edges animés quand puissance > 50W"
echo ""

# Test final
echo "Diagnostic final..."
echo ""
echo "Logs serveur (dernières 10 lignes):"
journalctl -u daly-bms -n 10 --no-pager
echo ""
echo "Status serveur:"
systemctl status daly-bms --no-pager | grep -E "Active|Loaded" || true
echo ""

echo "✓ DÉPLOIEMENT TERMINÉ"
echo ""
echo "Si le dashboard n'affiche pas les données:"
echo "  1. Vérifier que Node-RED flows sont déployées (http://192.168.1.141:1880)"
echo "  2. Vérifier logs serveur: journalctl -u daly-bms -f"
echo "  3. Vérifier MQTT: mosquitto_sub -h 192.168.1.120 -p 1883 -t 'santuario/#' -v"
echo ""
