# Kaos.io Game Server Dockerfile
# Multi-stage build for small final image
#
# Features:
# - WebSocket game transport (port 7351)
# - RUDP game transport (port 7354/udp)
# - Console API with auth (port 7350)
# - Console UI (React admin dashboard)
# - Prometheus metrics endpoint (port 9090)
# - PostgreSQL storage
# - Lua scripting with hooks

# Console UI build stage
FROM node:20-alpine AS console-builder

WORKDIR /app/console-ui

# Copy package files first for better caching
COPY prototype/console-ui/package*.json ./

# Install dependencies
RUN npm ci

# Copy source files
COPY prototype/console-ui ./

# Build production bundle
RUN npm run build

# Rust build stage
FROM rust:1.83-bookworm AS builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY kaos ./kaos
COPY kaos-rudp ./kaos-rudp
COPY kaos-ipc ./kaos-ipc
COPY kaos-test-support ./kaos-test-support
COPY kaos-driver ./kaos-driver
COPY kaos-archive ./kaos-archive
COPY prototype/kaos-ws ./prototype/kaos-ws
COPY prototype/kaos-http ./prototype/kaos-http
COPY prototype/kaosnet ./prototype/kaosnet
COPY prototype/kaosnet-rs ./prototype/kaosnet-rs
COPY prototype/examples/kaos_io ./prototype/examples/kaos_io
COPY prototype/examples/kaos_asteroids ./prototype/examples/kaos_asteroids

# Build release binaries (server + bot)
RUN cargo build --release -p kaos-io --bin kaos-io-server --bin kaos-io-bot

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    procps \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/kaos-io-server /app/kaos-io-server
COPY --from=builder /app/target/release/kaos-io-bot /app/kaos-io-bot

# Copy web client
COPY prototype/examples/kaos_io/web /app/web

# Copy SDK bundle for SDK version of web client
COPY prototype/kaosnet-js/dist/index.global.js /app/web/kaosnet.js

# Copy console UI build
COPY --from=console-builder /app/console-ui/dist /app/console

# Copy Lua scripts
COPY prototype/examples/kaos_io/scripts /app/scripts

# Ports
# 7350 - Console API (HTTP) + Console UI
# 7351 - WebSocket game
EXPOSE 7350
EXPOSE 7351

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:7350/api/status || exit 1

CMD ["/app/kaos-io-server"]
