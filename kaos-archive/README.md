# kaos-archive

Persistent message archive with mmap.

## When to Use What

| Need | Use | Speed |
|------|-----|-------|
| **High-throughput writes** | `Archive` | 20 M/s |
| **Random reads by seq** | `MmapArchive` | ~1 M/s writes, 35ns reads |
| **Maximum write speed** | `MmapArchive::append_unchecked()` | 28 M/s |

**Rule:** Use `Archive` for writes, `MmapArchive` for reads.

## Usage

```rust
// High-throughput: fire-and-forget to background thread
use kaos_archive::Archive;
let mut archive = Archive::create("/tmp/log", 1_000_000_000)?;
archive.append(b"hello")?; // Non-blocking
archive.flush(); // Wait for persistence

// Random access: direct mmap (slower writes, instant reads)
use kaos_archive::MmapArchive;
let mut archive = MmapArchive::create("/tmp/log", 1_000_000_000)?;
archive.append(b"hello")?; // Blocking, crash-safe
let msg = archive.read(0)?; // Random read by sequence
```

## Why MmapArchive Writes Are Slow

1. **Page faults** — 1GB mmap, each new page faults (~1-10μs)
2. **Dual mmap** — log + index files thrash CPU cache
3. **Disk-backed** — OS page cache, not pure RAM

This is **intentional** — crash-safe durability has a cost.

## Performance (M1 Pro, 64B messages)

| Method | Throughput | Features |
|--------|-----------|----------|
| `Archive::append()` | 20 M/s | Async, buffered |
| `MmapArchive::append_unchecked()` | 28 M/s | No CRC/index/bounds |
| `MmapArchive::append()` | ~1 M/s | CRC + index + crash-safe |
| `MmapArchive::read()` | 35 ns | Random access |
