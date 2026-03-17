#!/bin/bash
# =============================================================================
# install-venus.sh — Déploiement de daly-bms-venus sur Venus OS (NanoPi/GX)
# =============================================================================
#
# Ce script installe les binaires Rust sur un Victron GX tournant Venus OS.
# Les fichiers sont placés dans /data/ qui est PERSISTANT après mise à jour firmware.
#
# Prérequis :
#   - Accès SSH au GX (ssh root@<gx-ip>)
#   - Binaires cross-compilés pour ARM64 (aarch64-unknown-linux-gnu)
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

# Binaires cross-compilés (ARM64)
TARGET="aarch64-unknown-linux-gnu"
RELEASE_DIR="target/${TARGET}/release"

echo "=== Déploiement daly-bms-venus sur Venus OS ${GX_IP} ==="

# Vérifier que les binaires existent
for bin in daly-bms-server daly-bms-venus; do
    if [ ! -f "${RELEASE_DIR}/${bin}" ]; then
        echo "ERREUR: ${RELEASE_DIR}/${bin} introuvable."
        echo "Lancer d'abord: make build-venus"
        exit 1
    fi
done

echo "1. Création des répertoires sur le GX..."
ssh "${GX_SSH}" "mkdir -p ${INSTALL_DIR} ${SERVICE_DIR}/daly-bms-server ${SERVICE_DIR}/daly-bms-venus"

echo "2. Copie des binaires..."
scp "${RELEASE_DIR}/daly-bms-server" "${GX_SSH}:${INSTALL_DIR}/"
scp "${RELEASE_DIR}/daly-bms-venus"  "${GX_SSH}:${INSTALL_DIR}/"
ssh "${GX_SSH}" "chmod +x ${INSTALL_DIR}/daly-bms-server ${INSTALL_DIR}/daly-bms-venus"

echo "3. Copie de la configuration..."
# Utiliser Config.toml local si pas déjà présent sur le GX
if ! ssh "${GX_SSH}" "test -f ${INSTALL_DIR}/config.toml" 2>/dev/null; then
    scp "Config.toml" "${GX_SSH}:${INSTALL_DIR}/config.toml"
    echo "   config.toml copié (éditer ${INSTALL_DIR}/config.toml si nécessaire)"
else
    echo "   config.toml existant conservé"
fi

echo "4. Installation des services runit..."
scp "nanoPi/sv/daly-bms-server/run" "${GX_SSH}:${SERVICE_DIR}/daly-bms-server/run"
scp "nanoPi/sv/daly-bms-venus/run"  "${GX_SSH}:${SERVICE_DIR}/daly-bms-venus/run"
ssh "${GX_SSH}" "chmod +x ${SERVICE_DIR}/daly-bms-server/run ${SERVICE_DIR}/daly-bms-venus/run"

echo "5. Activation des services (symlinks dans /service/)..."
ssh "${GX_SSH}" "
    # Activer daly-bms-server
    if [ ! -L ${ACTIVE_DIR}/daly-bms-server ]; then
        ln -s ${SERVICE_DIR}/daly-bms-server ${ACTIVE_DIR}/daly-bms-server
        echo '   daly-bms-server activé'
    else
        echo '   daly-bms-server déjà actif'
    fi

    # Activer daly-bms-venus
    if [ ! -L ${ACTIVE_DIR}/daly-bms-venus ]; then
        ln -s ${SERVICE_DIR}/daly-bms-venus ${ACTIVE_DIR}/daly-bms-venus
        echo '   daly-bms-venus activé'
    else
        echo '   daly-bms-venus déjà actif'
    fi
"

echo ""
echo "=== Installation terminée ! ==="
echo ""
echo "Vérification des services :"
echo "  ssh ${GX_SSH} 'sv status ${ACTIVE_DIR}/daly-bms-server'"
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
echo "  ssh ${GX_SSH} 'sv restart ${ACTIVE_DIR}/daly-bms-server'"
echo "  ssh ${GX_SSH} 'sv restart ${ACTIVE_DIR}/daly-bms-venus'"
