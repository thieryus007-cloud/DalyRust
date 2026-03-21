#!/bin/bash
# =============================================================================
# install-venus.sh — Déploiement de daly-bms-venus sur Venus OS (NanoPi/GX)
# =============================================================================
#
# Usage :
#   ./nanoPi/install-venus.sh <gx-ip>
#   ARCH=armv7 ./nanoPi/install-venus.sh 192.168.1.120
#
# Prérequis (une seule fois) :
#   ssh-copy-id root@192.168.1.120   # pour ne plus jamais saisir de mot de passe
#
# =============================================================================

set -euo pipefail

GX_IP="${1:-192.168.1.120}"
GX_USER="root"
GX_SSH="${GX_USER}@${GX_IP}"
INSTALL_DIR="/data/daly-bms"
SERVICE_DIR="/data/etc/sv"
ACTIVE_DIR="/service"

# SSH ControlMaster : une seule connexion / une seule auth pour tout le script
SSH_SOCKET="/tmp/ssh-nanopi-$$.sock"
SSH_OPTS="-o ControlMaster=auto -o ControlPath=${SSH_SOCKET} -o ControlPersist=60 -o ConnectTimeout=10"
SCP_OPTS="-o ControlMaster=auto -o ControlPath=${SSH_SOCKET} -o ControlPersist=60 -o ConnectTimeout=10"

cleanup() {
    ssh ${SSH_OPTS} -O exit "${GX_SSH}" 2>/dev/null || true
}
trap cleanup EXIT

# Architecture : armv7 (NanoPi 32-bit) ou aarch64 (64-bit, défaut)
if [ "${ARCH:-}" = "armv7" ]; then
    TARGET="armv7-unknown-linux-gnueabihf"
else
    TARGET="aarch64-unknown-linux-gnu"
fi
RELEASE_DIR="target/${TARGET}/release"

echo "=== Déploiement daly-bms-venus sur Venus OS ${GX_IP} (${TARGET}) ==="

if [ ! -f "${RELEASE_DIR}/daly-bms-venus" ]; then
    echo "ERREUR: ${RELEASE_DIR}/daly-bms-venus introuvable."
    echo "Lancer d'abord: make build-venus-armv7"
    exit 1
fi

# Ouvre la connexion SSH maîtresse (une seule demande de mot de passe)
echo "Connexion SSH..."
ssh ${SSH_OPTS} -fN "${GX_SSH}"

echo "1. Création des répertoires sur le GX..."
ssh ${SSH_OPTS} "${GX_SSH}" "mkdir -p ${INSTALL_DIR} ${SERVICE_DIR}/daly-bms-venus"

echo "2. Arrêt du service daly-bms-venus avant mise à jour du binaire..."
ssh ${SSH_OPTS} "${GX_SSH}" "
    if [ -e ${ACTIVE_DIR}/daly-bms-venus ]; then
        svc -d ${ACTIVE_DIR}/daly-bms-venus 2>/dev/null || true
        sleep 1
        echo '   service daly-bms-venus stoppé'
    fi
"

echo "3. Suppression de daly-bms-server s'il est présent (ne doit pas tourner sur le NanoPi)..."
ssh ${SSH_OPTS} "${GX_SSH}" "
    if [ -L ${ACTIVE_DIR}/daly-bms-server ]; then
        svc -d ${ACTIVE_DIR}/daly-bms-server 2>/dev/null || true
        rm -f ${ACTIVE_DIR}/daly-bms-server
        echo '   symlink /service/daly-bms-server supprimé'
    fi
    rm -f ${INSTALL_DIR}/daly-bms-server
    rm -rf ${SERVICE_DIR}/daly-bms-server
    echo '   daly-bms-server retiré du NanoPi'
"

echo "4. Copie du binaire daly-bms-venus..."
scp ${SCP_OPTS} "${RELEASE_DIR}/daly-bms-venus" "${GX_SSH}:${INSTALL_DIR}/"
ssh ${SSH_OPTS} "${GX_SSH}" "chmod +x ${INSTALL_DIR}/daly-bms-venus"

echo "5. Copie de la configuration..."
if ! ssh ${SSH_OPTS} "${GX_SSH}" "test -f ${INSTALL_DIR}/config.toml" 2>/dev/null; then
    scp ${SCP_OPTS} "Config.toml" "${GX_SSH}:${INSTALL_DIR}/config.toml"
    echo "   config.toml copié (éditer ${INSTALL_DIR}/config.toml si nécessaire)"
else
    echo "   config.toml existant conservé"
fi

echo "6. Installation du run script daemontools..."
scp ${SCP_OPTS} "nanoPi/sv/daly-bms-venus/run" "${GX_SSH}:${SERVICE_DIR}/daly-bms-venus/run"
ssh ${SSH_OPTS} "${GX_SSH}" "chmod +x ${SERVICE_DIR}/daly-bms-venus/run"

echo "7. Activation / redémarrage du service..."
ssh ${SSH_OPTS} "${GX_SSH}" "
    if [ ! -L ${ACTIVE_DIR}/daly-bms-venus ] && [ ! -d ${ACTIVE_DIR}/daly-bms-venus ]; then
        ln -s ${SERVICE_DIR}/daly-bms-venus ${ACTIVE_DIR}/daly-bms-venus
        echo '   daly-bms-venus activé'
    else
        svc -u ${ACTIVE_DIR}/daly-bms-venus
        echo '   daly-bms-venus redémarré'
    fi
    sleep 2
    svstat ${ACTIVE_DIR}/daly-bms-venus
"

echo ""
echo "=== Déploiement terminé ! ==="
echo ""
echo "Vérification D-Bus :"
echo "  ssh ${GX_SSH} 'dbus -y com.victronenergy.battery.mqtt_1 /Soc GetValue'"
echo "  ssh ${GX_SSH} 'dbus -y com.victronenergy.battery.mqtt_2 /Soc GetValue'"
