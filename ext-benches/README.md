# External Benchmarks

Comparison benchmarks against other libraries.

## disruptor-rs-bench

Compares Kaos ring buffer against [disruptor-rs](https://crates.io/crates/disruptor).

```bash
cd disruptor-rs-bench && cargo bench
```

## aeron-java-bench

Compares Kaos against [Aeron](https://aeron.io/) (Java).

**Requirements:** [jbang](https://www.jbang.dev/)

### IPC Benchmark
```bash
cd aeron-java-bench && jbang AeronBench.java
```

### UDP Benchmark (two terminals)
```bash
# Terminal 1
jbang AeronBench.java recv

# Terminal 2
jbang AeronBench.java send
```

### Multicast Benchmark (two terminals)
```bash
# Aeron
jbang AeronMulticast.java recv
jbang AeronMulticast.java send

# Kaos (from project root)
cargo run -p kaos-rudp --release --example multicast_bench -- recv
cargo run -p kaos-rudp --release --example multicast_bench -- send
```

## Results (Apple M1 Pro)

| Benchmark | Throughput | Notes |
|-----------|------------|-------|
| Ring buffer (batch) | 2.2 G/s | 10M events |
| IPC (single) | 145 M/s | 8B messages |
| IPC (sustained) | 600 M/s | 100K batch |
