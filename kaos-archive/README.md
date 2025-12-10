# kaos-archive

High-performance message archive using memory-mapped files.

## Features

- **Append-only log** - Sequential writes for max throughput
- **Zero-copy reads** - mmap returns direct pointers
- **CRC32 checksums** - Data integrity verification
- **Index file** - O(1) message lookup by sequence

## Archive Types

| Type | Throughput | Use Case |
|------|-----------|----------|
| `Archive` | 22 M/s | Sync writes, simple API, low latency |
| `AsyncArchive` | 30-34 M/s | Max throughput, background persistence |

**When to use which:**
- `Archive` — Simple, predictable latency, crash-safe on each write
- `AsyncArchive` — Higher throughput, trades latency for speed, must call `flush()`

## Performance (Apple M1 Pro)

| Operation | Size | Result |
|-----------|------|--------|
| Archive append | 64B | 22 M/s |
| AsyncArchive append | 64B | 30-34 M/s |
| Read (unchecked) | 64B | ~30 ns |

## Usage

### Sync Archive (simple, low latency)

```rust
use kaos_archive::Archive;

let mut archive = Archive::create("/tmp/messages", 1024 * 1024 * 1024)?;
let seq = archive.append(b"hello world")?;
let msg = archive.read(seq)?;
```

### Async Archive (max throughput)

```rust
use kaos_archive::AsyncArchive;

let mut archive = AsyncArchive::new("/tmp/messages", 1024 * 1024 * 1024)?;
archive.append(b"hello world")?;
archive.flush(); // Wait for persistence
```

## File Format

```
messages.log:
┌──────────────────┐
│ Header (64B)     │  magic, version, write_pos, msg_count
├──────────────────┤
│ Frame 0          │  length (4B) + checksum (4B) + payload
├──────────────────┤
│ Frame 1          │
├──────────────────┤
│ ...              │
└──────────────────┘

messages.idx:
┌──────────────────┐
│ Entry 0 (16B)    │  offset (8B) + length (4B) + pad
├──────────────────┤
│ Entry 1          │
├──────────────────┤
│ ...              │
└──────────────────┘
```
