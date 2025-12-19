# Kaos.io Game Server Dockerfile
# Multi-stage build for small final image
#
# Features:
# - WebSocket game transport (port 7351)
# - Console API with auth (port 7350)
# - PostgreSQL storage
# - Lua scripting

# Build stage
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
COPY prototype/examples/kaos_io ./prototype/examples/kaos_io
COPY prototype/examples/kaos_asteroids ./prototype/examples/kaos_asteroids

# Build release binary with all features
RUN cargo build --release -p kaos-io --bin kaos-io-server

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/kaos-io-server /app/kaos-io-server

# Copy web client
COPY prototype/examples/kaos_io/web /app/web

# Copy Lua scripts
COPY prototype/examples/kaos_io/scripts /app/scripts

# Ports
# 7350 - Console API (HTTP)
# 7351 - WebSocket game
EXPOSE 7350
EXPOSE 7351

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:7350/api/status || exit 1

CMD ["/app/kaos-io-server"]
