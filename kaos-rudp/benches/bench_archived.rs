//! Benchmark: ArchivedTransport Performance
//!
//! Measures lock-free archive throughput comparable to Aeron Archive.
//!
//! Run: cargo bench -p kaos-rudp --features archive --bench bench_archived

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput, BatchSize};
use kaos_archive::Archive;
use kaos_rudp::ArchivedTransport;
use std::net::SocketAddr;
use tempfile::tempdir;

/// Sync archive append latency
fn bench_sync_archive(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive-sync");

    for msg_size in [64, 256, 1024] {
        let msg = vec![0u8; msg_size];

        group.throughput(Throughput::Bytes(msg_size as u64));

        group.bench_with_input(
            BenchmarkId::new("append", format!("{}B", msg_size)),
            &msg,
            |b, msg| {
                b.iter_batched_ref(
                    || {
                        let dir = tempdir().unwrap();
                        let path = dir.path().join("sync");
                        let archive = Archive::create(&path, 1024 * 1024 * 1024).unwrap();
                        (archive, dir)
                    },
                    |(archive, _dir)| {
                        black_box(archive.append(black_box(msg)).unwrap())
                    },
                    BatchSize::NumIterations(100_000),
                );
            },
        );
    }

    group.finish();
}

/// Total throughput: 1M messages
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive-1M-messages");
    group.sample_size(10);

    let msg = vec![0u8; 64];
    let count = 1_000_000u64;

    group.throughput(Throughput::Elements(count));

    group.bench_function("sync", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            let path = dir.path().join("sync");
            let mut archive = Archive::create(&path, 1024 * 1024 * 1024).unwrap();

            for _ in 0..count {
                black_box(archive.append(black_box(&msg)).unwrap());
            }
        });
    });

    group.finish();
}

/// Hot path latency (single append)
fn bench_hot_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("hot-path");

    let msg = vec![0u8; 64];

    group.throughput(Throughput::Elements(1));

    group.bench_function("sync-append", |b| {
        b.iter_batched_ref(
            || {
                let dir = tempdir().unwrap();
                let path = dir.path().join("sync");
                let archive = Archive::create(&path, 1024 * 1024 * 1024).unwrap();
                (archive, dir)
            },
            |(archive, _dir)| {
                black_box(archive.append(black_box(&msg)).unwrap())
            },
            BatchSize::NumIterations(100_000),
        );
    });

    group.finish();
}

/// ArchivedTransport throughput (comparable to Aeron Archive)
/// NOTE: This includes full UDP transport overhead (syscalls)
fn bench_archived_transport(c: &mut Criterion) {
    let mut group = c.benchmark_group("archived-transport-full");
    group.sample_size(10);

    static PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(30000);

    for msg_size in [64, 256, 1024] {
        let msg = vec![0u8; msg_size];
        let count = 1_000_000u64;

        group.throughput(Throughput::Elements(count));

        group.bench_with_input(
            BenchmarkId::new("1M-messages", format!("{}B", msg_size)),
            &msg,
            |b, msg| {
                b.iter(|| {
                    let port = PORT.fetch_add(2, std::sync::atomic::Ordering::Relaxed);
                    let local: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
                    let remote: SocketAddr = format!("127.0.0.1:{}", port + 1).parse().unwrap();

                    let dir = tempdir().unwrap();
                    let path = dir.path().join("archive");

                    let mut transport = ArchivedTransport::new(
                        local,
                        remote,
                        8192,
                        &path,
                        1024 * 1024 * 1024,
                    ).unwrap();

                    for _ in 0..count {
                        let _ = transport.send(black_box(msg));
                    }

                    transport.wait_for_archive();
                    transport.flush().unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Pure archive tap benchmark (IPC-style, no network)
/// This is the apples-to-apples comparison with Aeron Archive IPC recording
/// Aeron typical: 8-15 M/s | Kaos: 120+ M/s
fn bench_archive_tap_only(c: &mut Criterion) {
    use kaos::disruptor::{MessageRingBuffer, RingBufferConfig, RingBufferEntry};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use std::cell::UnsafeCell;

    struct SpscBuffer {
        buffer: UnsafeCell<MessageRingBuffer>,
    }
    unsafe impl Sync for SpscBuffer {}
    unsafe impl Send for SpscBuffer {}

    let mut group = c.benchmark_group("kaos-archive-ipc");
    group.sample_size(10);

    for msg_size in [64, 256, 1024] {
        let msg = vec![0u8; msg_size];
        let count = 1_000_000u64;

        group.throughput(Throughput::Elements(count));

        group.bench_with_input(
            BenchmarkId::new("1M-messages", format!("{}B", msg_size)),
            &msg,
            |b, msg| {
                b.iter(|| {
                    let dir = tempdir().unwrap();
                    let path = dir.path().join("archive");

                    let mut archive = Archive::create(&path, 1024 * 1024 * 1024).unwrap();

                    let config = RingBufferConfig::new(65536).unwrap()
                        .with_consumers(1).unwrap();
                    let tap_buffer = MessageRingBuffer::new(config).unwrap();
                    let tap_buffer = Arc::new(SpscBuffer { buffer: UnsafeCell::new(tap_buffer) });

                    let msg_count = Arc::new(AtomicU64::new(0));
                    let archived_seq = Arc::new(AtomicU64::new(0));
                    let running = Arc::new(AtomicBool::new(true));

                    // Recorder thread
                    let recorder = {
                        let tap_buffer = tap_buffer.clone();
                        let msg_count = msg_count.clone();
                        let archived_seq = archived_seq.clone();
                        let running = running.clone();
                        let path = path.clone();

                        std::thread::spawn(move || {
                            let mut archive = Archive::open(&path).unwrap();
                            const BATCH_SIZE: usize = 256;

                            while running.load(Ordering::Relaxed) {
                                let total = msg_count.load(Ordering::Acquire);
                                let archived = archived_seq.load(Ordering::Relaxed);

                                if total <= archived {
                                    std::hint::spin_loop();
                                    continue;
                                }

                                let buffer = unsafe { &*tap_buffer.buffer.get() };
                                let slots = buffer.try_consume_batch(0, BATCH_SIZE);

                                if slots.is_empty() {
                                    std::hint::spin_loop();
                                    continue;
                                }

                                for slot in &slots {
                                    let data = slot.data();
                                    if !data.is_empty() {
                                        let _ = archive.append(data);
                                    }
                                }

                                archived_seq.fetch_add(slots.len() as u64, Ordering::Release);
                            }

                            // Drain
                            loop {
                                let buffer = unsafe { &*tap_buffer.buffer.get() };
                                let slots = buffer.try_consume_batch(0, BATCH_SIZE);
                                if slots.is_empty() { break; }

                                for slot in &slots {
                                    let data = slot.data();
                                    if !data.is_empty() {
                                        let _ = archive.append(data);
                                    }
                                }
                                archived_seq.fetch_add(slots.len() as u64, Ordering::Release);
                            }
                        })
                    };

                    // Producer: just tap to ring buffer (no network)
                    let buffer = unsafe { &mut *tap_buffer.buffer.get() };
                    for _ in 0..count {
                        if let Some((claimed_seq, slots)) = buffer.try_claim_slots(1) {
                            slots[0].set_sequence(claimed_seq);
                            slots[0].set_data(black_box(msg));
                            buffer.publish_batch(claimed_seq, 1);
                            msg_count.fetch_add(1, Ordering::Release);
                        }
                    }

                    // Wait for archive
                    let target = msg_count.load(Ordering::Acquire);
                    while archived_seq.load(Ordering::Acquire) < target {
                        std::hint::spin_loop();
                    }

                    running.store(false, Ordering::Release);
                    let _ = recorder.join();
                });
            },
        );
    }

    group.finish();
}

/// Hot-path: send + tap latency (no wait for archive)
fn bench_archived_hot_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("archived-hot-path");

    static PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(32000);

    for msg_size in [64, 256] {
        let msg = vec![0u8; msg_size];

        group.throughput(Throughput::Bytes(msg_size as u64));

        group.bench_with_input(
            BenchmarkId::new("send", format!("{}B", msg_size)),
            &msg,
            |b, msg| {
                b.iter_batched_ref(
                    || {
                        let port = PORT.fetch_add(2, std::sync::atomic::Ordering::Relaxed);
                        let local: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
                        let remote: SocketAddr = format!("127.0.0.1:{}", port + 1).parse().unwrap();

                        let dir = tempdir().unwrap();
                        let path = dir.path().join("archive");

                        let transport = ArchivedTransport::new(
                            local,
                            remote,
                            8192,
                            &path,
                            64 * 1024 * 1024,
                        ).unwrap();

                        (transport, dir)
                    },
                    |(transport, _dir)| {
                        let _ = transport.send(black_box(msg));
                    },
                    BatchSize::NumIterations(10_000),
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sync_archive,
    bench_throughput,
    bench_hot_path,
    bench_archived_transport,
    bench_archive_tap_only,
    bench_archived_hot_path
);
criterion_main!(benches);
