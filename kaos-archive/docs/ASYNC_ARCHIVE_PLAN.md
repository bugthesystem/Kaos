# AsyncArchive Implementation Plan

## Goal
Match/beat Aeron Archive by using async I/O model with our fast IPC primitives.

## Current State
- Sync archive: ~22 M/s (with index)
- Aeron archive: ~26 M/s (async model)
- Kaos IPC: **146 M/s** (our advantage)

## Architecture

### Aeron's Model
```
Publisher → IPC Publication (26 M/s) → Recorder (async) → Disk
```

### Our New Model
```
Publisher → Ring Buffer (146 M/s) → Archive Writer (background) → Disk
```

## Implementation Steps

### Step 1: AsyncArchive struct
- Ring buffer for fast publication
- Background thread for archive writes
- Non-blocking `append()` that writes to ring buffer

### Step 2: Archive Writer Thread
- Polls ring buffer
- Batch writes to mmap
- Updates index in batches

### Step 3: Power-of-2 Alignment
- Align message frames to cache lines
- Use power-of-2 buffer sizes

### Step 4: Batch Index Updates
- Update index every N messages
- Reduce index write overhead

## API

```rust
// Create async archive
let archive = AsyncArchive::new(
    "/tmp/archive",
    1024 * 1024 * 1024,  // 1GB capacity
    65536,                // Ring buffer size
)?;

// Fast, non-blocking append (goes to ring buffer)
archive.append(b"message")?;  // Returns immediately

// Wait for all pending writes
archive.flush();

// Read (from mmap, zero-copy)
let msg = archive.read(seq)?;
```

## Expected Performance
- Publisher throughput: **~146 M/s** (IPC speed)
- Archive persistence: async, doesn't block publisher
- Read latency: same as sync (~35 ns)

## Comparison

| Metric | Sync Archive | Async Archive | Aeron |
|--------|--------------|---------------|-------|
| Append | 22 M/s | 146 M/s | 26 M/s |
| Model | Sync | Async | Async |
| Blocking | Yes | No | No |

