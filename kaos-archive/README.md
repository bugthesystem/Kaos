# kaos-archive

High-performance message archive using memory-mapped files.

## Archive Types

| Type | Throughput | When to Use |
|------|-----------|-------------|
| `Archive` | 30-34 M/s | Default. Max throughput, call `flush()` |
| `SyncArchive` | 22 M/s | Crash-safe per write, no `flush()` needed |

## Usage

### Archive (default, fastest)

```rust
use kaos_archive::Archive;

let mut archive = Archive::new("/tmp/messages", 1024 * 1024 * 1024)?;
archive.append(b"hello")?;
archive.flush(); // Wait for persistence
```

### SyncArchive (crash-safe)

```rust
use kaos_archive::SyncArchive;

let mut archive = SyncArchive::create("/tmp/messages", 1024 * 1024 * 1024)?;
archive.append(b"hello")?; // Persisted immediately
```

## Performance (Apple M1 Pro)

| Operation | Result |
|-----------|--------|
| Archive append | 30-34 M/s |
| SyncArchive append | 22 M/s |
| Read (unchecked) | ~30 ns |

## Features

- **Append-only log** - Sequential writes
- **Zero-copy reads** - mmap pointers
- **CRC32 checksums** - Data integrity
- **Index file** - O(1) lookup
