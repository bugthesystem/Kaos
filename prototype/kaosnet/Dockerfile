# KaosNet Game Server Dockerfile
# Multi-stage build for minimal image size
#
# Transport: kaos-rudp (Reliable UDP with NAK/ACK, congestion control)
# Protocol: KaosTalk

# Build stage
FROM rust:1.83-bookworm AS builder

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY kaos ./kaos
COPY kaos-rudp ./kaos-rudp
COPY kaos-ipc ./kaos-ipc
COPY kaos-test-support ./kaos-test-support
COPY kaos-driver ./kaos-driver
COPY kaos-archive ./kaos-archive
COPY kaos-ws ./kaos-ws
COPY kaos-http ./kaos-http
COPY prototype/kaosnet ./prototype/kaosnet

# Build with all features
RUN cargo build --release -p kaosnet --features "lua,console"

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary (when we have an example server binary)
# COPY --from=builder /build/target/release/kaosnet-server /app/

# Copy Lua scripts directory (mount or copy your scripts)
RUN mkdir -p /app/scripts

# Default ports
# 7350 - Game server (kaos-rudp over UDP - reliable game traffic)
# 7351 - Console API (HTTP - admin dashboard)
EXPOSE 7350/udp
EXPOSE 7351

# Environment variables
ENV KAOS_BIND=0.0.0.0:7350
ENV KAOS_CONSOLE_BIND=0.0.0.0:7351
ENV KAOS_LUA_SCRIPTS=/app/scripts
ENV KAOS_LOG_LEVEL=info

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:7351/health || exit 1

# Default command (placeholder - update when server binary exists)
CMD ["echo", "KaosNet server binary not yet built - use as library"]
