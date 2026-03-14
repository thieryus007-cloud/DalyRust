#!/usr/bin/env bash
# =============================================================================
# install-systemd.sh
# Installation et activation du service systemd pour daly-bms-server (Rust)
# À exécuter en root ou avec sudo
# =============================================================================

set -euo pipefail

# ──────────────────────────────────────────────────────────────────────────────
# Configuration des chemins (adapte selon ton installation)
# ──────────────────────────────────────────────────────────────────────────────

BINARY_SRC="target/release/daly-bms-server"                  # chemin relatif après cargo build --release
BINARY_DEST="/usr/local/bin/daly-bms-server"

SERVICE_SRC="contrib/daly-bms.service"
SERVICE_DEST="/etc/systemd/system/daly-bms.service"

CONFIG_EXAMPLE="config.example.toml"
CONFIG_DEST="/etc/daly-bms/config.toml"

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

if [[ ! -f "$BINARY_SRC" ]]; then
    echo "Erreur : Binaire non trouvé à $BINARY_SRC"
    echo "Exécutez d'abord : cargo build --release"
    exit 1
fi

if [[ ! -f "$SERVICE_SRC" ]]; then
    echo "Erreur : Fichier service non trouvé à $SERVICE_SRC"
    exit 1
fi

# ──────────────────────────────────────────────────────────────────────────────
# Création des répertoires et utilisateur/groupe
# ──────────────────────────────────────────────────────────────────────────────

echo "→ Création utilisateur/groupe $USER:$GROUP (si inexistant)"
if ! id "$USER" &>/dev/null; then
    useradd -r -s /usr/sbin/nologin -d /nonexistent "$USER" || true
fi
groupadd "$GROUP" 2>/dev/null || true
usermod -aG dialout "$USER"   # pour accès /dev/ttyUSB*

mkdir -p "$CONFIG_DIR" "$DATA_DIR"
chown -R "$USER:$GROUP" "$CONFIG_DIR" "$DATA_DIR"
chmod 750 "$CONFIG_DIR" "$DATA_DIR"

# ──────────────────────────────────────────────────────────────────────────────
# Installation du binaire
# ──────────────────────────────────────────────────────────────────────────────

echo "→ Installation du binaire vers $BINARY_DEST"
install -m 755 -o root -g root "$BINARY_SRC" "$BINARY_DEST"

# ──────────────────────────────────────────────────────────────────────────────
# Configuration TOML
# ──────────────────────────────────────────────────────────────────────────────

if [[ ! -f "$CONFIG_DEST" ]]; then
    echo "→ Copie de la configuration par défaut"
    cp "$CONFIG_EXAMPLE" "$CONFIG_DEST"
    chown "$USER:$GROUP" "$CONFIG_DEST"
    chmod 640 "$CONFIG_DEST"
    echo "→ Veuillez éditer $CONFIG_DEST avant de démarrer le service"
else
    echo "→ Configuration existante : $CONFIG_DEST (non écrasée)"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Installation du service systemd
# ──────────────────────────────────────────────────────────────────────────────

echo "→ Installation du fichier service"
cp "$SERVICE_SRC" "$SERVICE_DEST"
chmod 644 "$SERVICE_DEST"

# Recharge systemd
systemctl daemon-reload

# ──────────────────────────────────────────────────────────────────────────────
# Activation et démarrage
# ──────────────────────────────────────────────────────────────────────────────

echo "→ Activation et démarrage du service daly-bms"
systemctl enable daly-bms
systemctl restart daly-bms   # restart au cas où déjà lancé

# Petit délai pour que le service démarre
sleep 2

# ──────────────────────────────────────────────────────────────────────────────
# Vérification finale
# ──────────────────────────────────────────────────────────────────────────────

echo ""
echo "Statut du service :"
systemctl status daly-bms --no-pager -l

echo ""
echo "Dernières lignes de logs :"
journalctl -u daly-bms -n 40 --no-pager

echo ""
echo "Installation terminée."
echo "Prochaines étapes :"
echo "  1. Éditez   : sudo nano $CONFIG_DEST"
echo "  2. Testez   : curl http://localhost:8000/api/v1/system/status"
echo "  3. Dashboard: http://dalybms.local/"
echo "  4. Logs     : journalctl -u daly-bms -f"
