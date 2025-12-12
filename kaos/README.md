# kaos

Lock-free ring buffers (LMAX Disruptor pattern).

## Usage

```rust
use kaos::disruptor::{RingBuffer, Slot8};

let mut ring = RingBuffer::<Slot8>::new(1024)?;

// Producer: claim, write, publish
if let Some((seq, slots)) = ring.try_claim_slots(10, 0) {
    for (i, slot) in slots.iter_mut().enumerate() {
        slot.value = i as u64;
    }
    ring.publish(seq + slots.len() as u64);
}

// Consumer: read, advance
let batch = ring.get_read_batch(0, 10);
for slot in batch {
    println!("{}", slot.value);
}
ring.update_consumer(batch.len() as u64);
```

## Ring Buffers

| Type | Pattern | Use Case |
|------|---------|----------|
| `RingBuffer<T>` | SPSC | Fastest, single thread each side |
| `BroadcastRingBuffer<T>` | SPSC fan-out | Multiple consumers see ALL messages |
| `SharedRingBuffer<T>` | SPSC (mmap) | IPC between processes |
| `MpscRingBuffer<T>` | MPSC | Multiple producers, one consumer |
| `SpmcRingBuffer<T>` | SPMC | One producer, multiple consumers |
| `MpmcRingBuffer<T>` | MPMC | Full flexibility (slowest) |

## Slot Types

| Type | Size | Alignment |
|------|------|-----------|
| `Slot8` | 8B | 8B |
| `Slot16` | 16B | 16B |
| `Slot32` | 32B | 32B |
| `Slot64` | 64B | 64B |
| `MessageSlot` | 1KB+ | 128B (cache-line) |

## Thread Affinity (Linux)

```rust
use kaos::affinity::{pin_to_core, numa_available};

if numa_available() {
    pin_to_core(0)?;  // Pin to CPU 0
}
```

## License

MIT OR Apache-2.0
