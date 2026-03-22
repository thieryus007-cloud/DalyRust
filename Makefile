# =============================================================================
# DalyBMS — Rust Edition — Makefile
# =============================================================================
# Usage :
#   make up       → démarrer l'infra Docker (Mosquitto, InfluxDB, Grafana, Node-RED)
#   make build    → compiler en release (x86_64)
#   make build-arm → compiler pour aarch64 (Raspberry Pi CM5 / NanoPi)
#   make run      → lancer le serveur en dev
#   make test     → lancer les tests unitaires
#   make install  → installer le binaire et le service systemd
#   make lint     → clippy + fmt check

CARGO      := cargo
BINARY     := daly-bms-server
CLI        := daly-bms-cli
TARGET_ARM    := aarch64-unknown-linux-gnu
TARGET_ARMV7  := armv7-unknown-linux-gnueabihf
RELEASE_DIR := target/release
ARM_RELEASE_DIR := target/$(TARGET_ARM)/release
ARMV7_RELEASE_DIR := target/$(TARGET_ARMV7)/release

# =============================================================================
# Infrastructure Docker
# =============================================================================

.PHONY: up down restart logs reset reset-influx ps

up:
	docker compose -f docker-compose.infra.yml up -d
	@echo "✓ Infra démarrée — MQTT:1883 InfluxDB:8086 Grafana:3001 Node-RED:1880"

down:
	docker compose -f docker-compose.infra.yml down

restart:
	docker compose -f docker-compose.infra.yml restart

logs:
	docker compose -f docker-compose.infra.yml logs -f

reset:
	docker compose -f docker-compose.infra.yml down -v
	@echo "⚠ Volumes supprimés — données InfluxDB/Grafana/Node-RED effacées"

# Reset uniquement InfluxDB (conserve Grafana config + Node-RED)
# Utile pour repartir avec une base vierge sans perdre les dashboards Grafana
reset-influx:
	docker compose -f docker-compose.infra.yml stop influxdb
	docker volume rm $$(docker volume ls -q | grep influxdb) 2>/dev/null || true
	docker compose -f docker-compose.infra.yml up -d influxdb
	@echo "✓ InfluxDB réinitialisé — token conservé depuis .env"

ps:
	docker compose -f docker-compose.infra.yml ps

# =============================================================================
# Compilation
# =============================================================================

.PHONY: build build-arm build-arm-v7 build-cli build-venus build-venus-arm build-venus-armv7 build-venus-v7 install-venus install-venus-v7

VENUS_BIN  := dbus-mqtt-venus

build:
	$(CARGO) build --release --bin $(BINARY)
	@echo "✓ Binaire : $(RELEASE_DIR)/$(BINARY)"

build-arm:
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
	  $(CARGO) build --release --target $(TARGET_ARM) --bin $(BINARY)
	@echo "✓ Binaire ARM : $(ARM_RELEASE_DIR)/$(BINARY)"

build-cli:
	$(CARGO) build --release --bin $(CLI)

# Phase 3 — Venus OS D-Bus bridge
build-venus:
	$(CARGO) build --release --bin $(VENUS_BIN)
	@echo "✓ Binaire Venus : $(RELEASE_DIR)/$(VENUS_BIN)"

build-venus-arm:
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
	  $(CARGO) build --release --target $(TARGET_ARM) --bin $(VENUS_BIN) --bin $(BINARY)
	@echo "✓ Binaires ARM Venus OS :"
	@echo "  $(ARM_RELEASE_DIR)/$(BINARY)"
	@echo "  $(ARM_RELEASE_DIR)/$(VENUS_BIN)"

build-arm-v7:
	CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc \
	  $(CARGO) build --release --target $(TARGET_ARMV7) --bin $(BINARY)
	@echo "✓ Binaire ARMv7 : $(ARMV7_RELEASE_DIR)/$(BINARY)"

build-venus-armv7 build-venus-v7:
	CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc \
	  $(CARGO) build --release --target $(TARGET_ARMV7) --bin $(VENUS_BIN) --bin $(BINARY)
	@echo "✓ Binaires ARMv7 Venus OS :"
	@echo "  $(ARMV7_RELEASE_DIR)/$(BINARY)"
	@echo "  $(ARMV7_RELEASE_DIR)/$(VENUS_BIN)"

# Déploiement sur Venus OS (remplacer GX_IP par l'IP de votre GX)
GX_IP ?= 192.168.1.120
install-venus: build-venus-arm
	./nanoPi/install-venus.sh $(GX_IP)

# Déploiement sur Venus OS armv7l (NanoPi 32-bit)
install-venus-v7: build-venus-armv7
	ARCH=armv7 ./nanoPi/install-venus.sh $(GX_IP)

build-all:
	$(CARGO) build --release

# =============================================================================
# Développement
# =============================================================================

.PHONY: run run-debug

run:
	RUST_LOG=info $(CARGO) run --release --bin $(BINARY)

run-debug:
	RUST_LOG=debug $(CARGO) run --bin $(BINARY)

cli:
	$(CARGO) run --bin $(CLI) -- $(ARGS)

# =============================================================================
# Tests
# =============================================================================

.PHONY: test test-core test-verbose

test:
	$(CARGO) test --workspace

test-core:
	$(CARGO) test -p daly-bms-core

test-verbose:
	$(CARGO) test --workspace -- --nocapture

# =============================================================================
# Qualité
# =============================================================================

.PHONY: lint fmt check

lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

fmt:
	$(CARGO) fmt --all

check:
	$(CARGO) check --workspace
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --workspace

# =============================================================================
# Installation (systemd)
# =============================================================================

.PHONY: install uninstall

install: build
	sudo bash contrib/install-systemd.sh

uninstall:
	sudo bash contrib/uninstall-systemd.sh

# =============================================================================
# Cross-compile + déploiement SSH vers le Pi
# =============================================================================

PI_HOST ?= pi5compute@192.168.1.141
PI_BIN_PATH ?= /usr/local/bin/daly-bms-server

.PHONY: deploy

deploy: build-arm
	scp $(ARM_RELEASE_DIR)/$(BINARY) $(PI_HOST):/tmp/$(BINARY)
	ssh $(PI_HOST) "sudo install -m 755 /tmp/$(BINARY) $(PI_BIN_PATH) && sudo systemctl restart daly-bms && sudo systemctl status daly-bms --no-pager -l"
	@echo "✓ Déployé sur $(PI_HOST)"

# =============================================================================
# Node-RED — Déploiement des flows depuis git
# =============================================================================

NODERED_URL ?= http://localhost:1880

.PHONY: deploy-nodered

deploy-nodered:
	@echo "Déploiement des flows Node-RED depuis flux-nodered/ ..."
	NODERED_URL=$(NODERED_URL) python3 contrib/deploy-nodered-flows.py

# =============================================================================
# Dashboard (React)
# =============================================================================

.PHONY: dashboard-dev dashboard-build

dashboard-dev:
	cd dashboard && npm run dev

dashboard-build:
	cd dashboard && npm run build

# =============================================================================
# Documentation
# =============================================================================

.PHONY: doc

doc:
	$(CARGO) doc --workspace --no-deps --open

# =============================================================================
# Nettoyage
# =============================================================================

.PHONY: clean

clean:
	$(CARGO) clean

.DEFAULT_GOAL := help

.PHONY: help
help:
	@echo ""
	@echo "DalyBMS Rust Edition — Commandes disponibles :"
	@echo ""
	@echo "  Infrastructure Docker :"
	@echo "    make up            Démarrer Mosquitto + InfluxDB + Grafana + Node-RED"
	@echo "    make down          Arrêter l'infra"
	@echo "    make logs          Voir les logs Docker"
	@echo "    make ps            État des containers"
	@echo ""
	@echo "  Compilation :"
	@echo "    make build         Compiler pour l'architecture locale"
	@echo "    make build-arm     Cross-compiler pour aarch64 (Pi)"
	@echo "    make build-all     Compiler tous les binaires"
	@echo ""
	@echo "  Développement :"
	@echo "    make run           Lancer le serveur (release)"
	@echo "    make run-debug     Lancer en mode debug (RUST_LOG=debug)"
	@echo "    make cli ARGS='--help'  Lancer le CLI"
	@echo ""
	@echo "  Tests & Qualité :"
	@echo "    make test          Tests unitaires"
	@echo "    make lint          Clippy"
	@echo "    make fmt           Format code"
	@echo "    make check         Check + fmt + lint"
	@echo ""
	@echo "  Déploiement :"
	@echo "    make install       Installer le service systemd"
	@echo "    make deploy        Déployer daly-bms-server sur pi5compute@192.168.1.141"
	@echo "    make deploy-nodered  Pousser flows Node-RED depuis git vers Node-RED (port 1880)"
	@echo "    make install-venus-v7  Déployer dbus-mqtt-venus sur NanoPi (armv7)"
	@echo ""
