#!/bin/bash
# =============================================================================
# install-venus.sh — Déploiement de daly-bms-venus sur Venus OS (NanoPi/GX)
# =============================================================================
#
# Ce script installe UNIQUEMENT le bridge D-Bus sur le NanoPi.
# Le service daly-bms-server (polling RS485 + dashboard HTTP) tourne sur le Pi5,
# pas sur le NanoPi.
#
# Prérequis :
#   - Accès SSH au GX (ssh root@<gx-ip>)
#   - Binaire cross-compilé pour ARM64 (aarch64-unknown-linux-gnu)
#     Commande : make build-venus
#
# Usage :
#   ./nanoPi/install-venus.sh <gx-ip>
#   ./nanoPi/install-venus.sh 192.168.1.120
#
# =============================================================================

set -euo pipefail

GX_IP="${1:-192.168.1.120}"
GX_USER="root"
GX_SSH="${GX_USER}@${GX_IP}"
INSTALL_DIR="/data/daly-bms"
SERVICE_DIR="/data/etc/sv"
ACTIVE_DIR="/service"

# Architecture : armv7 (NanoPi 32-bit) ou aarch64 (64-bit, défaut)
if [ "${ARCH:-}" = "armv7" ]; then
    TARGET="armv7-unknown-linux-gnueabihf"
else
    TARGET="aarch64-unknown-linux-gnu"
fi
RELEASE_DIR="target/${TARGET}/release"

echo "=== Déploiement daly-bms-venus sur Venus OS ${GX_IP} ==="

# Vérifier que le binaire existe
if [ ! -f "${RELEASE_DIR}/daly-bms-venus" ]; then
    echo "ERREUR: ${RELEASE_DIR}/daly-bms-venus introuvable."
    echo "Lancer d'abord: make build-venus"
    exit 1
fi

echo "1. Création des répertoires sur le GX..."
ssh "${GX_SSH}" "mkdir -p ${INSTALL_DIR} ${SERVICE_DIR}/daly-bms-venus"

echo "2. Suppression de daly-bms-server s'il est présent (ne doit pas tourner sur le NanoPi)..."
ssh "${GX_SSH}" "
    if [ -L ${ACTIVE_DIR}/daly-bms-server ]; then
        sv -d ${ACTIVE_DIR}/daly-bms-server 2>/dev/null || true
        rm -f ${ACTIVE_DIR}/daly-bms-server
        echo '   symlink /service/daly-bms-server supprimé'
    fi
    rm -f ${INSTALL_DIR}/daly-bms-server
    rm -rf ${SERVICE_DIR}/daly-bms-server
    echo '   daly-bms-server retiré du NanoPi'
"

echo "3. Copie du binaire daly-bms-venus..."
scp "${RELEASE_DIR}/daly-bms-venus" "${GX_SSH}:${INSTALL_DIR}/"
ssh "${GX_SSH}" "chmod +x ${INSTALL_DIR}/daly-bms-venus"

echo "4. Copie de la configuration..."
if ! ssh "${GX_SSH}" "test -f ${INSTALL_DIR}/config.toml" 2>/dev/null; then
    scp "Config.toml" "${GX_SSH}:${INSTALL_DIR}/config.toml"
    echo "   config.toml copié (éditer ${INSTALL_DIR}/config.toml si nécessaire)"
else
    echo "   config.toml existant conservé"
fi

echo "5. Installation du service runit daly-bms-venus..."
scp "nanoPi/sv/daly-bms-venus/run" "${GX_SSH}:${SERVICE_DIR}/daly-bms-venus/run"
ssh "${GX_SSH}" "chmod +x ${SERVICE_DIR}/daly-bms-venus/run"

echo "6. Activation du service (symlink dans /service/)..."
ssh "${GX_SSH}" "
    if [ ! -L ${ACTIVE_DIR}/daly-bms-venus ]; then
        ln -s ${SERVICE_DIR}/daly-bms-venus ${ACTIVE_DIR}/daly-bms-venus
        echo '   daly-bms-venus activé'
    else
        sv restart ${ACTIVE_DIR}/daly-bms-venus
        echo '   daly-bms-venus redémarré'
    fi
"

echo ""
echo "=== Installation terminée ! ==="
echo ""
echo "Vérification du service :"
echo "  ssh ${GX_SSH} 'sv status ${ACTIVE_DIR}/daly-bms-venus'"
echo ""
echo "Vérification D-Bus :"
echo "  ssh ${GX_SSH} 'dbus -y com.victronenergy.battery.mqtt_1 /Soc GetValue'"
echo "  ssh ${GX_SSH} 'dbus -y com.victronenergy.battery.mqtt_2 /Dc/0/Voltage GetValue'"
echo ""
echo "Logs :"
echo "  ssh ${GX_SSH} 'logread | grep daly'"
echo ""
echo "Redémarrage si nécessaire :"
echo "  ssh ${GX_SSH} 'sv restart ${ACTIVE_DIR}/daly-bms-venus'"
