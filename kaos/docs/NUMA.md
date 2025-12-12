# NUMA Optimization Guide

NUMA optimization can provide **60-100%+ throughput gains** on multi-socket servers.

## Quick Start

```rust
use kaos::affinity::{pin_to_core, pin_to_numa_node, numa_available};

if numa_available() {
    // Pin producer to core 0
    pin_to_core(0)?;
    
    // Or pin to entire NUMA node
    pin_to_numa_node(0)?;
}
```

## API

```rust
// Pin current thread to specific core
kaos::affinity::pin_to_core(0)?;

// Pin to NUMA node (any core on that node)
kaos::affinity::pin_to_numa_node(0)?;

// Get current NUMA node
let node = kaos::affinity::current_numa_node()?;

// Check NUMA support
if kaos::affinity::numa_available() { ... }

// Get node count
let nodes = kaos::affinity::numa_node_count();
```

## System Commands

```bash
# Check NUMA topology
numactl --hardware

# Run with NUMA binding
numactl --cpunodebind=0 --membind=0 ./kaos-app

# NIC IRQ affinity
echo 0-7 > /proc/irq/<NIC_IRQ>/smp_affinity_list

# Disable auto-balancing (optional)
echo 0 > /proc/sys/kernel/numa_balancing
```

## Docker

NUMA works in Docker **only if host has NUMA**:

```bash
# Check host
numactl --hardware

# Run with CPU/memory binding
docker run --cpuset-cpus="0-7" --cpuset-mems="0" kaos-bench
```

