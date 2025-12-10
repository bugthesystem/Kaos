# kaos-archive

High-performance message archive.

## Archive Types

| Type | Throughput | Use Case |
|------|-----------|----------|
| `BufferedArchive` | **32 M/s** | Fastest append, no random read |
| `SyncArchive` | 28 M/s | Random read via mmap |
| `Archive` | 17 M/s | Async with background writer |

## Performance vs Aeron (M1 Pro)

| Archive | Throughput |
|---------|-----------|
| **Kaos BufferedArchive** | **32 M/s** |
| Kaos SyncArchive | 28 M/s |
| Aeron Archive | 15 M/s |

**Kaos is 2x faster than Aeron.**

## Usage

```rust
use kaos_archive::{BufferedArchive, SyncArchive};

// Fastest (write-only, no random read)
let mut archive = BufferedArchive::create("/tmp/log")?;
archive.append(b"hello")?;
archive.flush()?;

// With random read support
let mut archive = SyncArchive::create("/tmp/log", 1024 * 1024)?;
archive.append(b"hello")?;
let msg = archive.read(0)?;
```
