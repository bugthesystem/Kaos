#!/bin/bash
# NUMA benchmark script for Docker

echo "╔═══════════════════════════════════════════╗"
echo "║  Kaos NUMA Benchmark                      ║"
echo "╚═══════════════════════════════════════════╝"
echo

# Check NUMA topology
echo "=== NUMA Topology ==="
if command -v numactl &> /dev/null; then
    numactl --hardware 2>/dev/null || echo "NUMA not available (single node)"
else
    echo "numactl not installed"
fi
echo

# Check CPU info
echo "=== CPU Info ==="
nproc
cat /proc/cpuinfo | grep "model name" | head -1
echo

# Run affinity tests
echo "=== Affinity Tests ==="
cargo test -p kaos affinity --release 2>&1 | tail -10
echo

# Run core benchmark
echo "=== Core Benchmark ==="
cargo bench -p kaos --bench bench_core -- --noplot 2>&1 | grep -E "time:|thrpt:|Benchmarking"
echo

# Run with CPU pinning if available
if command -v numactl &> /dev/null; then
    echo "=== Benchmark with NUMA binding (node 0) ==="
    numactl --cpunodebind=0 --membind=0 cargo bench -p kaos --bench bench_core -- --noplot 2>&1 | grep -E "time:|thrpt:|Benchmarking"
fi

echo
echo "Done!"

