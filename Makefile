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
TARGET_ARM := aarch64-unknown-linux-gnu
RELEASE_DIR := target/release
ARM_RELEASE_DIR := target/$(TARGET_ARM)/release

# =============================================================================
# Infrastructure Docker
# =============================================================================

.PHONY: up down restart logs reset ps

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
	@echo "⚠ Volumes supprimés — données InfluxDB/Grafana effacées"

ps:
	docker compose -f docker-compose.infra.yml ps

# =============================================================================
# Compilation
# =============================================================================

.PHONY: build build-arm build-cli

build:
	$(CARGO) build --release --bin $(BINARY)
	@echo "✓ Binaire : $(RELEASE_DIR)/$(BINARY)"

build-arm:
	@command -v cross >/dev/null 2>&1 || cargo install cross
	cross build --release --target $(TARGET_ARM) --bin $(BINARY)
	@echo "✓ Binaire ARM : $(ARM_RELEASE_DIR)/$(BINARY)"

build-cli:
	$(CARGO) build --release --bin $(CLI)

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

PI_HOST ?= pi@dalybms.local
PI_BIN_PATH ?= /usr/local/bin/daly-bms-server

.PHONY: deploy

deploy: build-arm
	scp $(ARM_RELEASE_DIR)/$(BINARY) $(PI_HOST):$(PI_BIN_PATH)
	ssh $(PI_HOST) "sudo systemctl restart daly-bms"
	@echo "✓ Déployé sur $(PI_HOST)"

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
	@echo "    make deploy PI_HOST=pi@192.168.1.100  Déployer sur le Pi"
	@echo ""
