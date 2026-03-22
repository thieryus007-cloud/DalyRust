#!/bin/bash
# =============================================================================
# Install script — Irradiance RS485 Modbus RTU → MQTT service
# À exécuter sur Pi5 depuis ~/Daly-BMS-Rust/
# =============================================================================
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_NAME="irradiance-rs485"

echo "=== Installation ${SERVICE_NAME} ==="

# 1. Dépendances Python
echo "1. Installation des dépendances Python..."
pip3 install --user pyserial paho-mqtt

# 2. Ajouter pi5compute au groupe dialout (accès /dev/ttyUSB*)
echo "2. Ajout au groupe dialout..."
sudo usermod -a -G dialout pi5compute
echo "   IMPORTANT : déconnectez-vous et reconnectez-vous pour que le groupe prenne effet."
echo "   (ou exécutez: newgrp dialout)"

# 3. Installer l'unité systemd
echo "3. Installation du service systemd..."
sudo cp "${SCRIPT_DIR}/${SERVICE_NAME}.service" /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now "${SERVICE_NAME}"

echo ""
echo "=== Installation terminée ==="
echo ""
echo "Commandes de diagnostic :"
echo "  systemctl status ${SERVICE_NAME}"
echo "  journalctl -u ${SERVICE_NAME} -f"
echo ""
echo "Vérifier MQTT (sur Pi5) :"
echo "  mosquitto_sub -h localhost -p 1883 -t 'santuario/irradiance/raw' -v"
echo ""
echo "Port série actuel :"
ls -la /dev/ttyUSB* 2>/dev/null || echo "  Aucun ttyUSB trouvé"
echo ""
echo "Si le port est ttyUSB0 (et non ttyUSB1), éditer SERIAL_PORT dans :"
echo "  ${SCRIPT_DIR}/irradiance_reader.py"
