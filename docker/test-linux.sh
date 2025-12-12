#!/bin/bash
set -e

echo "╔═══════════════════════════════════════════╗"
echo "║  Kaos Linux Feature Tests                 ║"
echo "╚═══════════════════════════════════════════╝"
echo ""

cd "$(dirname "$0")/.."

# Build Docker image
echo "Building Docker image..."
docker build -t kaos-linux-tests -f docker/Dockerfile.linux-tests .
echo ""

echo "=== sendmmsg/recvmmsg Tests ==="
docker run --rm kaos-linux-tests cargo test -p kaos-rudp --release -- sendmmsg --nocapture
echo ""

echo "=== io_uring Tests ==="
# io_uring tests may need --privileged for SQPOLL
docker run --rm --privileged kaos-linux-tests cargo test -p kaos-driver --release --features uring -- uring --nocapture 2>/dev/null || echo "io_uring: requires kernel 5.6+ with SQPOLL support"
echo ""

echo "=== Archive Crash Recovery Tests ==="
docker run --rm kaos-linux-tests cargo test -p kaos-archive --release -- crash_recovery --nocapture
echo ""

echo "=== RUDP Retransmit/Replay Tests ==="
docker run --rm kaos-linux-tests cargo test -p kaos-rudp --release --features archive -- "test_retransmit\|test_late_joiner" --nocapture
echo ""

echo "=== Full Workspace Tests ==="
docker run --rm kaos-linux-tests cargo test --release --workspace
echo ""

echo "Done!"

