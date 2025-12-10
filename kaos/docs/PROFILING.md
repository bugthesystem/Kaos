# Profiling & Memory Analysis

## Quick Start

```bash
cargo bench -p kaos --bench bench_core        # Core patterns
cargo bench -p kaos --bench bench_trace       # Trace events
```

## Tracy Profiler (Real-time)

Tracy provides real-time visualization of message flow, latency, and system performance.

### Setup

```bash
# Install Tracy
brew install tracy  # macOS

# Enable in Cargo.toml
kaos = { version = "0.1", features = ["tracy"] }
```

### Usage

```rust
fn main() {
    kaos::init_tracy();  // Initialize before any kaos operations
    // ... your app
}
```

```bash
# Terminal 1: Run your app
cargo run --release --features tracy

# Terminal 2: Open Tracy
tracy
# Click Connect → 127.0.0.1 → Connect
```

### What to Look For

| View | What It Shows | What to Look For |
|------|---------------|------------------|
| **Timeline** | Events over time | Message bursts, gaps, patterns |
| **Zones** | Function timing | Long `send`/`recv` operations |
| **Statistics** | Aggregated data | Mean/max latency, throughput |
| **Find Zone** | Search events | Filter by `send`, `recv`, `backpressure` |

### Key Metrics

| Metric | Good | Warning | Bad |
|--------|------|---------|-----|
| `send` latency | < 100ns | 100ns-1μs | > 1μs |
| `recv` latency | < 100ns | 100ns-1μs | > 1μs |
| `backpressure` events | Rare | Occasional | Frequent |
| `retransmit` events | 0 | < 1% | > 1% |

### Tracy Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Space` | Pause/resume |
| `F` | Find zone |
| `S` | Statistics |
| `Ctrl+F` | Search |
| `Mouse wheel` | Zoom timeline |
| `Click+drag` | Select time range |

### Troubleshooting

**"Incompatible protocol"**: Version mismatch. Use `tracing-tracy = "=0.11.0"` with brew's Tracy.

**No events visible**: Ensure `kaos::init_tracy()` is called before any kaos operations.

**High overhead**: Tracy adds ~50-100ns per event. Disable for production benchmarks.

## Memory Analysis

### macOS - leaks (Built-in)

```bash
# Build release
cargo build --release --example spsc_basic -p kaos

# Check for leaks at exit
leaks --atExit -- ./target/release/examples/spsc_basic

# Verbose output with allocation list
leaks --atExit --list -- ./target/release/examples/spsc_basic

# Export to Instruments for visualization
leaks --atExit --outputGraph=leak.memgraph -- ./target/release/examples/spsc_basic
open leak.memgraph
```

### macOS - valgrind (via Homebrew)

[LouisBrunner/valgrind-macos](https://github.com/LouisBrunner/valgrind-macos) provides macOS ARM support.

```bash
# Install (requires Xcode CLI tools)
brew tap LouisBrunner/valgrind
brew install --HEAD LouisBrunner/valgrind/valgrind

# Memory check
valgrind --leak-check=full ./target/release/examples/spsc_basic

# Generate suppressions for false positives
valgrind --gen-suppressions=all ./target/release/examples/spsc_basic
```

**Note:** macOS valgrind can be unstable. Use `leaks` for quick checks.

### Linux - valgrind

```bash
# Install
sudo apt install valgrind
cargo install cargo-valgrind

# Easy mode
cargo valgrind run --example spsc_basic -p kaos --release

# Direct usage
valgrind --leak-check=full --show-leak-kinds=all \
  ./target/release/examples/spsc_basic

# Cache analysis
valgrind --tool=cachegrind ./target/release/examples/spsc_basic
cg_annotate cachegrind.out.*

# Heap profiling
valgrind --tool=massif ./target/release/examples/spsc_basic
ms_print massif.out.*

# Thread errors
valgrind --tool=helgrind ./target/release/examples/spsc_basic
```

## CPU Profiling

### Flamegraph
```bash
cargo install flamegraph
cargo flamegraph --bench bench_core -o flame.svg
```

### perf (Linux)
```bash
perf stat cargo bench -p kaos --bench bench_core
perf record -g cargo bench && perf report
```

### Instruments (macOS)
```bash
xcrun xctrace record --template 'Time Profiler' \
  --launch -- ./target/release/examples/spsc_basic
```

### cargo-asm
```bash
cargo install cargo-asm
cd kaos && cargo asm --lib "kaos::disruptor::completion::CompletionTracker::try_claim"
```

## Assembly Analysis (ARM64 M1 Pro)

Critical path assembly verified with `cargo asm`. Hot path functions are inlined.

### CompletionTracker::try_claim (SPMC/MPMC consumer claim)

```asm
; cargo asm --lib "kaos::disruptor::completion::CompletionTracker::try_claim"
; 7 instructions in hot path - OPTIMAL

 mov     x9, x0
 mov     w0, #1
LBB11_1:
 ldr     x1, [x9]              ; Load current cursor
 cmp     x1, x8                ; Compare with limit
 b.hs    LBB11_4               ; If >= limit, return None
 add     x10, x1, #1           ; next = current + 1
 mov     x11, x1
 cas     x11, x10, [x9]        ; ARM64 native CAS instruction
 cmp     x11, x1               ; Check if CAS succeeded
 b.ne    LBB11_1               ; Retry loop if failed
 ret
LBB11_4:
 mov     x0, #0                ; Return None
 ret
```

### ARM64 Atomic Instructions Used

| Operation | Instruction | Ordering |
|-----------|-------------|----------|
| Load-Acquire | `ldar` | Acquire |
| Load-Acquire (relaxed) | `ldapr` | Acquire |
| Store-Release | `stlr` | Release |
| CAS | `cas` | Relaxed |
| CAS | `casa` | Acquire |
| CAS | `casl` | Release |
| CAS | `casal` | AcqRel |
| CRC32 | `crc32cx` | Hardware accelerated |

### Why Most Functions Don't Appear

RingBuffer::publish, try_claim_slots, etc. are **inlined** by the compiler.
This is optimal - no function call overhead in the hot path.

Only `CompletionTracker` methods appear because they're called through trait objects
or have `#[inline(never)]` for debugging.

To see inlined code, analyze benchmark binaries:
```bash
cargo build --release --bench bench_core -p kaos
objdump -d target/release/deps/bench_core-* | grep -A50 "run_bench"
```

## Verified Results (Apple M1 Pro)

| Component | Throughput | Memory |
|-----------|------------|--------|
| Ring buffer (batch) | 2.2 G/s | 0 leaks ✅ |
| Ring buffer (per-event) | 425 M/s | 0 leaks ✅ |
| IPC single | 147 M/s | 0 leaks ✅ |
| IPC sustained | 595 M/s | 0 leaks ✅ |
