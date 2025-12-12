# Kaos Roadmap

## Status

| Feature | Status |
|---------|--------|
| Lock-free ring buffers (SPSC/MPSC/SPMC/MPMC) | ✅ |
| Shared memory IPC (mmap) | ✅ |
| Media driver architecture | ✅ |
| Reliable UDP (NAK/ACK) | ✅ |
| Congestion control (AIMD) | ✅ |
| sendmmsg/recvmmsg (Linux) | ✅ |
| io_uring (Linux) | ✅ |
| AF_XDP kernel bypass | ⚠️ Compiles, needs testing |
| Tracing / Tracy profiler | ✅ |
| UDP multicast | ✅ |
| Message archive (sync + async) | ✅ |
| Thread affinity (Linux) | ⚠️ Experimental |
| NUMA detection | ⚠️ Experimental |
| NUMA-aware allocation | ❌ Planned |

## NUMA Support

### Done
- `pin_to_core(cpu)` - pin thread to CPU
- `pin_to_numa_node(node)` - pin thread to NUMA node
- `current_numa_node()` - get current node
- `numa_available()` - check NUMA support
- `numa_node_count()` - get node count

### TODO
- `RingBufferConfig::with_numa_node(n)` - allocate buffer on specific node
- `numa_alloc_on_node()` - low-level NUMA allocation
- Benchmark on real multi-socket hardware

## Testing

```bash
cargo test --workspace
cargo bench -p kaos
RUSTFLAGS="--cfg loom" cargo test -p kaos --test loom_ring_buffer --release
```
