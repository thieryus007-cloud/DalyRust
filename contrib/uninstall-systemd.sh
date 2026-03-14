#!/usr/bin/env bash
# =============================================================================
# uninstall-systemd.sh
# Désinstallation propre du service systemd daly-bms
# À exécuter en root ou avec sudo
# =============================================================================

set -euo pipefail

# ──────────────────────────────────────────────────────────────────────────────
# Chemins (doivent correspondre à install-systemd.sh)
# ──────────────────────────────────────────────────────────────────────────────

SERVICE_NAME="daly-bms"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

BINARY_PATH="/usr/local/bin/daly-bms-server"
BINARY_CLI_PATH="/usr/local/bin/daly-bms-cli"   # optionnel

CONFIG_DIR="/etc/daly-bms"
DATA_DIR="/var/lib/daly-bms"

USER="dalybms"
GROUP="dalybms"

# ──────────────────────────────────────────────────────────────────────────────
# Vérifications préalables
# ──────────────────────────────────────────────────────────────────────────────

if [[ $EUID -ne 0 ]]; then
    echo "Ce script doit être exécuté en root ou avec sudo."
    exit 1
fi

echo "Script de désinstallation du service ${SERVICE_NAME}"
echo "---------------------------------------------------"
echo ""

if [[ ! -f "$SERVICE_FILE" ]]; then
    echo "→ Service $SERVICE_FILE n'existe pas. Rien à désinstaller."
else
    echo "→ Service trouvé : $SERVICE_FILE"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Arrêt et désactivation du service
# ──────────────────────────────────────────────────────────────────────────────

if systemctl is-active --quiet "$SERVICE_NAME"; then
    echo "→ Arrêt du service..."
    systemctl stop "$SERVICE_NAME"
fi

if systemctl is-enabled --quiet "$SERVICE_NAME"; then
    echo "→ Désactivation du service..."
    systemctl disable "$SERVICE_NAME"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Suppression du fichier service
# ──────────────────────────────────────────────────────────────────────────────

if [[ -f "$SERVICE_FILE" ]]; then
    echo "→ Suppression de $SERVICE_FILE"
    rm -f "$SERVICE_FILE"
    systemctl daemon-reload
    systemctl reset-failed "$SERVICE_NAME" 2>/dev/null || true
else
    echo "→ Fichier service déjà absent"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Suppression optionnelle du binaire
# ──────────────────────────────────────────────────────────────────────────────

read -p "Voulez-vous aussi supprimer le binaire $BINARY_PATH ? (o/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Oo]$ ]]; then
    if [[ -f "$BINARY_PATH" ]]; then
        echo "→ Suppression de $BINARY_PATH"
        rm -f "$BINARY_PATH"
    fi
    if [[ -f "$BINARY_CLI_PATH" ]]; then
        echo "→ Suppression de $BINARY_CLI_PATH (CLI optionnelle)"
        rm -f "$BINARY_CLI_PATH"
    fi
else
    echo "→ Binaire conservé"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Suppression optionnelle des répertoires (config et data)
# ──────────────────────────────────────────────────────────────────────────────

read -p "Voulez-vous supprimer le répertoire de configuration $CONFIG_DIR ? (o/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Oo]$ ]]; then
    if [[ -d "$CONFIG_DIR" ]]; then
        echo "→ Suppression de $CONFIG_DIR"
        rm -rf "$CONFIG_DIR"
    fi
else
    echo "→ Configuration conservée"
fi

read -p "Voulez-vous supprimer le répertoire de données $DATA_DIR (alertes SQLite, etc.) ? (o/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Oo]$ ]]; then
    if [[ -d "$DATA_DIR" ]]; then
        echo "→ Suppression de $DATA_DIR"
        rm -rf "$DATA_DIR"
    fi
else
    echo "→ Données conservées"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Suppression optionnelle de l'utilisateur/groupe
# ──────────────────────────────────────────────────────────────────────────────

read -p "Voulez-vous supprimer l'utilisateur et le groupe $USER:$GROUP ? (o/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Oo]$ ]]; then
    if id "$USER" &>/dev/null; then
        echo "→ Suppression utilisateur $USER"
        userdel -r "$USER" 2>/dev/null || true
    fi
    if getent group "$GROUP" &>/dev/null; then
        echo "→ Suppression groupe $GROUP"
        groupdel "$GROUP" 2>/dev/null || true
    fi
else
    echo "→ Utilisateur et groupe conservés"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Nettoyage systemd et vérification finale
# ──────────────────────────────────────────────────────────────────────────────

systemctl daemon-reload
systemctl reset-failed

echo ""
echo "Désinstallation terminée."
echo ""
echo "Vérifications finales :"
echo "  - systemctl status $SERVICE_NAME     → devrait indiquer 'not found' ou 'inactive'"
echo "  - ls $BINARY_PATH                    → devrait échouer si supprimé"
echo "  - ls $CONFIG_DIR                     → devrait être absent si supprimé"
echo ""
echo "Si vous avez utilisé nginx, pensez à supprimer /etc/nginx/sites-enabled/dalybms si nécessaire."
echo "Logs restants : journalctl -u $SERVICE_NAME (peut être vide après suppression)"
