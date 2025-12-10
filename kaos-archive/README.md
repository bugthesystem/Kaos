# kaos-archive

Message archive with mmap for random read access.

## Archive Types

| Type | Throughput | Use Case |
|------|-----------|----------|
| `Archive` | 17 M/s | Async, background writer |
| `SyncArchive` | 28 M/s | Sync, crash-safe per write |

## Usage

```rust
use kaos_archive::SyncArchive;

let mut archive = SyncArchive::create("/tmp/log", 1024 * 1024)?;
archive.append(b"hello")?;
let msg = archive.read(0)?; // Random read by sequence
```

## Performance (M1 Pro)

| Method | Throughput |
|--------|-----------|
| `append()` (CRC+index) | 20 M/s |
| `append_no_index()` | 28 M/s |
| `append_batch()` | 28 M/s |
| `read_no_verify()` | ~30 ns |
