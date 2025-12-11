<h1 align="center">KAOS</h1>

<p align="center">
  <img src="logo.svg" alt="Kaos Logo" width="120" height="120"/>
</p>

<p align="center">
  <strong>High-performance lock-free messaging for Rust</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#performance">Performance</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#architecture">Architecture</a>
</p>

---

## Overview

Kaos provides lock-free ring buffers for inter-thread, inter-process, and network communication. Built on the [LMAX Disruptor](https://lmax-exchange.github.io/disruptor/) and [Aeron](https://github.com/real-logic/aeron) high performance networking patterns with modern Rust.

> **Note:** Preview release. APIs may change.

## Crates

| Crate | Description |
|-------|-------------|
| **[kaos](./kaos)** | Lock-free ring buffers (SPSC, MPSC, SPMC, MPMC) |
| **[kaos-ipc](./kaos-ipc)** | Shared memory IPC via mmap |
| **[kaos-rudp](./kaos-rudp)** | Reliable UDP with NAK/ACK |
| **[kaos-archive](./kaos-archive)** | Persistent message archive (sync + async) |
| **[kaos-driver](./kaos-driver)** | Media driver for zero-syscall I/O |

## Features

| Category | Feature | Status |
|----------|---------|--------|
| **Core** | Lock-free ring buffers | ✅ |
| | SPSC, MPSC, SPMC, MPMC | ✅ |
| | Batch operations | ✅ |
| **IPC** | Shared memory (mmap) | ✅ |
| | Zero-copy reads | ✅ |
| **Network** | Reliable UDP | ✅ |
| | Congestion control (AIMD) | ✅ |
| **Archive** | Persistent message storage | ✅ |
| | Retransmission from disk | ✅ |
| | Late joiner replay | ✅ |
| **Linux** | sendmmsg/recvmmsg | ✅ |
| | io_uring | ✅ |
| | AF_XDP kernel bypass | ✅ |
| **Observability** | Tracing / Tracy | ✅ |

## Observability

Feature-gated via `tracing` crate. Zero-cost when disabled.

### Console Logs

```toml
kaos = { version = "0.1", features = ["tracing"] }
tracing-subscriber = "0.3"
```

```rust
tracing_subscriber::fmt::init();
```

```
2024-01-15T10:30:45 TRACE send bytes=64
2024-01-15T10:30:45 TRACE recv bytes=64
2024-01-15T10:30:46 WARN  backpressure
```

### Tracy Profiler

Real-time visualization of latency, throughput, and bottlenecks.

```toml
kaos = { version = "0.1", features = ["tracy"] }
```

```bash
brew install tracy                    # Install
cargo run -p kaos --example profile --features tracy --release  # Run profiler
tracy                                 # Connect → 127.0.0.1
```

See [PROFILING.md](./kaos/docs/PROFILING.md) for detailed guide.

## Performance

Measured on Apple M1 Pro (actual `cargo bench` results).

| Benchmark | M1 Pro |
|-----------|--------|
| Ring buffer (batch, 10M) | 2.2 G/s |
| Ring buffer (per-event, 1B) | 425 M/s |
| IPC single send (8B) | 147 M/s |
| IPC sustained (100K) | 595 M/s |
| RUDP (reliable UDP) | 3.7 M/s (vs Aeron 2.6 M/s) |
| Archive | 30-34 M/s (vs Aeron 26 M/s) |
| SyncArchive | 22 M/s |
| Archive read (64B) | 35 ns |

```bash
# Run benchmarks
cargo bench -p kaos --bench bench_trace -- "100M"
cd ext-benches/disruptor-rs-bench && cargo bench --bench bench_trace_events
cd ext-benches/disruptor-java-bench && mvn compile -q && \
  java -cp "target/classes:$(mvn dependency:build-classpath -q -Dmdep.outputFile=/dev/stdout)" \
  com.kaos.TraceEventsBenchmark
```

## API Selection Guide

| Use Case | Ring Buffer | Producer | Speed |
|----------|-------------|----------|-------|
| **Fastest (single producer)** | `RingBuffer` | `CachedProducer` | 2.1 G/s |
| **Broadcast (fan-out)** | `BroadcastRingBuffer` | direct | 1.1 G/s |
| **Multi-producer, single consumer** | `MpscRingBuffer` | `CachedMpscProducer` | 390 M/s |
| **Work distribution** | `SpmcRingBuffer` | direct | 1.1 G/s |
| **Full flexibility** | `MpmcRingBuffer` | `CachedMpmcProducer` | 30 M/s |

**When to use `Cached*` producers:**
- `CachedProducer` - Caches consumer position, avoids atomic loads on hot path
- `CachedMpscProducer` - Same caching + closure API for zero-copy writes
- `CachedMpmcProducer` - Same caching + closure API, essential for MPMC performance

**Rule of thumb:** Always prefer `Cached*` producers when available.

## Quick Start

### Batch API

```rust
use kaos::disruptor::{RingBuffer, Slot8};

let ring = RingBuffer::<Slot8>::new(1024)?;

// Producer: claim batch, write, publish
if let Some((seq, slots)) = ring.try_claim_slots(10, cursor) {
    for (i, slot) in slots.iter_mut().enumerate() {
        slot.value = i as u64;
    }
    ring.publish(seq + slots.len() as u64);
}

// Consumer: read batch, advance
let slots = ring.get_read_batch(0, 10);
ring.update_consumer(10);
```

### Per-Event API

```rust
use kaos::disruptor::{RingBuffer, Slot8, CachedProducer};

let ring = Arc::new(RingBuffer::<Slot8>::new(1024)?);
let mut producer = CachedProducer::new(ring.clone());

// Publish with in-place mutation
producer.publish(|slot| {
    slot.value = 42;
});
```

## Archived RUDP

Combine reliable UDP with persistent archive for:
- **Retransmission from disk** — When ring buffer wraps, retransmit from archive
- **Late joiner replay** — New subscribers catch up from any sequence
- **Crash recovery** — Resume from persisted state

```rust
use kaos_rudp::ArchivedTransport;

let mut transport = ArchivedTransport::new(
    "127.0.0.1:9000".parse().unwrap(),
    "127.0.0.1:9001".parse().unwrap(),
    65536,                    // Ring buffer window
    "/tmp/rudp-archive",      // Archive path
    1024 * 1024 * 1024,       // 1GB archive
).unwrap();

// Send — automatically archived for durability
transport.send(b"hello").unwrap();

// Retransmit from archive (even after ring buffer wrapped)
transport.retransmit_from_archive(sequence_number);

// Replay range for late joiner
transport.replay(0, 1000, |seq, msg| {
    println!("Replaying seq {}: {} bytes", seq, msg.len());
});
```

Enable with feature flag:
```toml
kaos-rudp = { version = "0.1", features = ["archive"] }
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        APPLICATION                          │
│            Producer ───► Ring Buffer ───► Consumer          │
└─────────────────────────────┬───────────────────────────────┘
                              │ Shared Memory (2.4 ns)
┌─────────────────────────────▼───────────────────────────────┐
│                       MEDIA DRIVER                          │
│       sendmmsg  │  io_uring  │  AF_XDP  │  Reliable UDP     │
└─────────────────────────────┬───────────────────────────────┘
                              │
           ┌──────────────────┴──────────────────┐
           │                                     │
           ▼                                     ▼
┌─────────────────────┐              ┌─────────────────────────┐
│      NETWORK        │              │       ARCHIVE           │
│                     │◄────NAK──────│  (retransmit/replay)    │
└─────────────────────┘              └─────────────────────────┘
```

## Testing

```bash
# Unit tests
cargo test --workspace

# Loom concurrency verification (exhaustive state exploration)
RUSTFLAGS="--cfg loom" cargo test -p kaos --test loom_ring_buffer --release

# Memory analysis (macOS)
leaks --atExit -- ./target/release/examples/spsc_basic

# Memory analysis (Linux)
cargo valgrind run --example spsc_basic -p kaos --release
```

**Loom** tests verify lock-free correctness by exploring all possible thread interleavings. See [kaos/tests/loom_ring_buffer.rs](./kaos/tests/loom_ring_buffer.rs).

**Profiling** guide with flamegraphs, valgrind, leaks and Instruments: [kaos/docs/PROFILING.md](./kaos/docs/PROFILING.md)

## Platform Support

| Platform | Status |
|----------|--------|
| macOS ARM64 | ✅ Tested |
| Linux x86_64 | ✅ Tested |
| Windows | Not supported |

## Design Principles

- **Lock-free** — Atomic sequences, no mutex contention
- **Zero-copy reads** — Consumers get direct slice access (writes copy to buffer)  
- **Cache-aligned** — 128-byte padding prevents false sharing
- **Batch operations** — Amortize synchronization overhead

## Glossary

| Term | Meaning |
|------|---------|
| **SPSC** | Single Producer, Single Consumer |
| **MPSC** | Multiple Producers, Single Consumer |
| **SPMC** | Single Producer, Multiple Consumers |
| **MPMC** | Multiple Producers, Multiple Consumers |
| **CAS** | Compare-And-Swap (atomic operation for lock-free coordination) |
| **IPC** | Inter-Process Communication |
| **mmap** | Memory-mapped file (shared memory) |
| **RUDP** | Reliable UDP (guaranteed delivery) |
| **NAK** | Negative Acknowledgment (request retransmit) |
| **ACK** | Acknowledgment (confirm receipt) |
| **AIMD** | Additive Increase, Multiplicative Decrease (congestion control) |
| **io_uring** | Linux async I/O interface |
| **AF_XDP** | Linux kernel bypass for networking |
| **sendmmsg** | Linux batched send syscall |

## License

MIT OR Apache-2.0

---

<p align="center">
  Inspired by <a href="https://lmax-exchange.github.io/disruptor/">LMAX Disruptor</a> and <a href="https://github.com/real-logic/aeron">Aeron</a>
</p>
