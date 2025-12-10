//! Async archive using optimized SPSC ring buffer + background writer.
//!
//! Single producer cursor, single consumer cursor - no per-slot atomics.

use crate::{Archive, ArchiveError};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

const RING_SIZE: usize = 65536;
const RING_MASK: usize = RING_SIZE - 1;
const MAX_MSG_SIZE: usize = 1024;

/// Cache-line padded cursor
#[repr(C, align(128))]
struct PaddedU64(AtomicU64);

/// Slot in the ring buffer (cache-line sized)
#[repr(C, align(64))]
struct Slot {
    len: u16,
    data: [u8; MAX_MSG_SIZE],
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            len: 0,
            data: [0u8; MAX_MSG_SIZE],
        }
    }
}

/// Shared state between producer and consumer
struct SharedState {
    ring: Vec<Slot>,
    producer_cursor: PaddedU64,  // Written by producer, read by consumer
    consumer_cursor: PaddedU64,  // Written by consumer, read by producer
    running: AtomicBool,
}

/// Async archive with SPSC ring buffer for fast appends.
pub struct AsyncArchive {
    state: Arc<SharedState>,
    local_cursor: u64,  // Local producer cursor (no atomics on hot path)
    cached_consumer: u64,  // Cached consumer position
    writer_handle: Option<JoinHandle<()>>,
}

impl AsyncArchive {
    /// Create a new async archive.
    pub fn new<P: AsRef<Path>>(base_path: P, capacity: usize) -> Result<Self, ArchiveError> {
        let path = base_path.as_ref().to_path_buf();

        // Pre-allocate ring buffer
        let mut slots = Vec::with_capacity(RING_SIZE);
        for _ in 0..RING_SIZE {
            slots.push(Slot::default());
        }

        let state = Arc::new(SharedState {
            ring: slots,
            producer_cursor: PaddedU64(AtomicU64::new(0)),
            consumer_cursor: PaddedU64(AtomicU64::new(0)),
            running: AtomicBool::new(true),
        });

        let state_clone = state.clone();

        // Background writer
        let handle = thread::spawn(move || {
            let mut archive = Archive::create(&path, capacity).expect("Failed to create archive");
            let mut local_consumer = 0u64;

            while state_clone.running.load(Ordering::Relaxed) {
                // Check how many slots available
                let producer = state_clone.producer_cursor.0.load(Ordering::Acquire);
                if producer == local_consumer {
                    std::hint::spin_loop();
                    continue;
                }

                // Process batch
                let available = (producer - local_consumer) as usize;
                let batch = available.min(64);

                for _ in 0..batch {
                    let idx = (local_consumer as usize) & RING_MASK;
                    let slot = unsafe { &*state_clone.ring.as_ptr().add(idx) };
                    let len = slot.len as usize;
                    if len > 0 {
                        let _ = archive.append_no_index(&slot.data[..len]);
                    }
                    local_consumer += 1;
                }

                // Publish consumer position
                state_clone.consumer_cursor.0.store(local_consumer, Ordering::Release);
            }

            // Drain remaining
            let producer = state_clone.producer_cursor.0.load(Ordering::Acquire);
            while local_consumer < producer {
                let idx = (local_consumer as usize) & RING_MASK;
                let slot = unsafe { &*state_clone.ring.as_ptr().add(idx) };
                let len = slot.len as usize;
                if len > 0 {
                    let _ = archive.append_no_index(&slot.data[..len]);
                }
                local_consumer += 1;
            }
            state_clone.consumer_cursor.0.store(local_consumer, Ordering::Release);
        });

        Ok(Self {
            state,
            local_cursor: 0,
            cached_consumer: 0,
            writer_handle: Some(handle),
        })
    }

    /// Append a message (non-blocking, writes to ring buffer).
    #[inline(always)]
    pub fn append(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        if data.len() > MAX_MSG_SIZE {
            return Err(ArchiveError::Full);
        }

        // Check if we have space (using cached consumer)
        let next = self.local_cursor + 1;
        if next - self.cached_consumer > RING_SIZE as u64 {
            // Update cache
            self.cached_consumer = self.state.consumer_cursor.0.load(Ordering::Acquire);
            if next - self.cached_consumer > RING_SIZE as u64 {
                return Err(ArchiveError::Full);
            }
        }

        // Write to slot
        let idx = (self.local_cursor as usize) & RING_MASK;
        let slot = unsafe { &mut *(self.state.ring.as_ptr().add(idx) as *mut Slot) };
        slot.len = data.len() as u16;
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), slot.data.as_mut_ptr(), data.len());
        }

        let seq = self.local_cursor;
        self.local_cursor = next;

        // Batch publish (every 64 messages)
        if (next & 63) == 0 {
            self.state.producer_cursor.0.store(next, Ordering::Release);
        }

        Ok(seq)
    }

    /// Wait for all pending messages to be archived.
    pub fn flush(&mut self) {
        // Publish any remaining
        self.state.producer_cursor.0.store(self.local_cursor, Ordering::Release);
        
        let target = self.local_cursor;
        while self.state.consumer_cursor.0.load(Ordering::Acquire) < target {
            std::hint::spin_loop();
        }
    }
}

impl Drop for AsyncArchive {
    fn drop(&mut self) {
        self.flush();
        self.state.running.store(false, Ordering::Release);
        if let Some(handle) = self.writer_handle.take() {
            let _ = handle.join();
        }
    }
}

// Safety: SharedState is thread-safe via atomics
unsafe impl Send for SharedState {}
unsafe impl Sync for SharedState {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_async_archive() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test");

        let mut archive = AsyncArchive::new(&path, 1024 * 1024).unwrap();

        for i in 0..100u64 {
            archive.append(&i.to_le_bytes()).unwrap();
        }

        archive.flush();
    }

    #[test]
    fn test_async_throughput() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test");

        let mut archive = AsyncArchive::new(&path, 1024 * 1024 * 1024).unwrap();
        let msg = [0u8; 64];
        
        // Warmup
        for _ in 0..10_000 {
            archive.append(&msg).unwrap();
        }
        archive.flush();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let count = 1_000_000u64;
        let start = std::time::Instant::now();
        for _ in 0..count {
            while archive.append(&msg).is_err() {
                std::hint::spin_loop();
            }
        }
        let elapsed = start.elapsed();

        let rate = count as f64 / elapsed.as_secs_f64() / 1_000_000.0;
        println!("AsyncArchive (optimized SPSC): {:.2} M/s", rate);

        archive.flush();
    }
}
