# =============================================================================
# DalyBMS Server — Dockerfile multi-stage
# =============================================================================
# Build stage : Rust officiel (cache des dépendances optimisé)
# Runtime stage : Debian slim (minimal, ~50 MB final)
#
# Build local :
#   docker build -t dalybms-server:latest .
#
# Build cross (Pi5 arm64) depuis x86_64 :
#   docker buildx build --platform linux/arm64 -t dalybms-server:arm64 .

# =============================================================================
# Stage 1 — Builder
# =============================================================================
FROM rust:latest-slim-bookworm AS builder

# Dépendances système nécessaires pour les crates natives (rusqlite bundled, openssl)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# ── Astuce : copier d'abord les Cargo.toml pour cacher les deps ──────────────
# Cela évite de recompiler toutes les dépendances à chaque changement de code.
COPY Cargo.toml Cargo.lock ./
COPY crates/daly-bms-core/Cargo.toml    crates/daly-bms-core/
COPY crates/daly-bms-server/Cargo.toml  crates/daly-bms-server/
COPY crates/daly-bms-cli/Cargo.toml     crates/daly-bms-cli/

# Créer des fichiers src stub pour que cargo fetch/check fonctionne
RUN mkdir -p crates/daly-bms-core/src \
             crates/daly-bms-server/src \
             crates/daly-bms-cli/src && \
    echo "fn main() {}" > crates/daly-bms-server/src/main.rs && \
    echo "fn main() {}" > crates/daly-bms-cli/src/main.rs && \
    touch crates/daly-bms-core/src/lib.rs

# Pré-compiler les dépendances (layer cacheable)
RUN cargo build --release --bin daly-bms-server 2>/dev/null || true

# ── Copier les vraies sources et compiler ────────────────────────────────────
COPY crates/ crates/

# Forcer la recompilation des crates locaux
RUN touch crates/daly-bms-core/src/lib.rs \
          crates/daly-bms-server/src/main.rs

RUN cargo build --release --bin daly-bms-server

# =============================================================================
# Stage 2 — Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Dépendances runtime minimales
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Utilisateur non-root pour la sécurité
RUN useradd -m -u 1000 -s /bin/sh dalybms

# Répertoires de données
RUN mkdir -p /etc/daly-bms /var/lib/daly-bms && \
    chown -R dalybms:dalybms /var/lib/daly-bms

# Binaire compilé
COPY --from=builder /build/target/release/daly-bms-server /usr/local/bin/daly-bms-server

# Config exemple (peut être surchargée via volume)
COPY Config.toml /etc/daly-bms/config.toml.example

USER dalybms

# Port API HTTP
EXPOSE 8000

# Healthcheck (vérifie que l'API répond)
HEALTHCHECK --interval=30s --timeout=10s --start-period=15s --retries=3 \
    CMD curl -f http://localhost:8000/api/v1/system/status || exit 1

# Variables d'environnement par défaut
ENV DALY_CONFIG=/etc/daly-bms/config.toml
ENV RUST_LOG=info

# Par défaut : mode simulation (surcharger avec "" pour hardware)
ENTRYPOINT ["/usr/local/bin/daly-bms-server"]
CMD ["--simulate"]
