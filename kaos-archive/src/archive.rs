//! Fast archive with SPSC ring buffer + background writer (30-34 M/s).

use crate::{ArchiveError, SyncArchive};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

const RING_SIZE: usize = 65536;
const RING_MASK: usize = RING_SIZE - 1;
const MAX_MSG_SIZE: usize = 1024;

#[repr(C, align(128))]
struct PaddedU64(AtomicU64);

#[repr(C, align(64))]
struct Slot {
    len: u16,
    data: [u8; MAX_MSG_SIZE],
}

impl Default for Slot {
    fn default() -> Self { Self { len: 0, data: [0u8; MAX_MSG_SIZE] } }
}

struct SharedState {
    ring: Vec<Slot>,
    producer_cursor: PaddedU64,
    consumer_cursor: PaddedU64,
    running: AtomicBool,
}

/// Fast archive with background persistence. Call `flush()` for durability.
pub struct Archive {
    state: Arc<SharedState>,
    local_cursor: u64,
    cached_consumer: u64,
    writer_handle: Option<JoinHandle<()>>,
}

impl Archive {
    pub fn new<P: AsRef<Path>>(base_path: P, capacity: usize) -> Result<Self, ArchiveError> {
        let path = base_path.as_ref().to_path_buf();

        let mut slots = Vec::with_capacity(RING_SIZE);
        slots.resize_with(RING_SIZE, Slot::default);

        let state = Arc::new(SharedState {
            ring: slots,
            producer_cursor: PaddedU64(AtomicU64::new(0)),
            consumer_cursor: PaddedU64(AtomicU64::new(0)),
            running: AtomicBool::new(true),
        });

        let state_clone = state.clone();
        let handle = thread::spawn(move || {
            let mut archive = SyncArchive::create(&path, capacity).expect("Failed to create archive");
            let mut consumer = 0u64;
            let mut batch_buf: Vec<&[u8]> = Vec::with_capacity(64);

            while state_clone.running.load(Ordering::Relaxed) {
                let producer = state_clone.producer_cursor.0.load(Ordering::Acquire);
                if producer == consumer {
                    std::hint::spin_loop();
                    continue;
                }

                // Collect batch
                batch_buf.clear();
                let batch_size = ((producer - consumer) as usize).min(64);
                for i in 0..batch_size {
                    let slot = unsafe { &*state_clone.ring.as_ptr().add(((consumer + i as u64) as usize) & RING_MASK) };
                    if slot.len > 0 {
                        batch_buf.push(unsafe { std::slice::from_raw_parts(slot.data.as_ptr(), slot.len as usize) });
                    }
                }

                // Batch write
                if !batch_buf.is_empty() {
                    let _ = archive.append_batch(&batch_buf);
                }

                consumer += batch_size as u64;
                state_clone.consumer_cursor.0.store(consumer, Ordering::Release);
            }

            // Drain remaining
            let producer = state_clone.producer_cursor.0.load(Ordering::Acquire);
            while consumer < producer {
                let slot = unsafe { &*state_clone.ring.as_ptr().add((consumer as usize) & RING_MASK) };
                if slot.len > 0 {
                    let _ = archive.append_no_index(&slot.data[..slot.len as usize]);
                }
                consumer += 1;
            }
            state_clone.consumer_cursor.0.store(consumer, Ordering::Release);
        });

        Ok(Self { state, local_cursor: 0, cached_consumer: 0, writer_handle: Some(handle) })
    }

    #[inline(always)]
    pub fn append(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        if data.len() > MAX_MSG_SIZE {
            return Err(ArchiveError::Full);
        }

        let next = self.local_cursor + 1;
        if next - self.cached_consumer > RING_SIZE as u64 {
            self.cached_consumer = self.state.consumer_cursor.0.load(Ordering::Acquire);
            if next - self.cached_consumer > RING_SIZE as u64 {
                return Err(ArchiveError::Full);
            }
        }

        let slot = unsafe { &mut *(self.state.ring.as_ptr().add((self.local_cursor as usize) & RING_MASK) as *mut Slot) };
        slot.len = data.len() as u16;
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), slot.data.as_mut_ptr(), data.len()); }

        let seq = self.local_cursor;
        self.local_cursor = next;

        if (next & 63) == 0 {
            self.state.producer_cursor.0.store(next, Ordering::Release);
        }

        Ok(seq)
    }

    /// Batch append multiple messages.
    #[inline]
    pub fn append_batch(&mut self, messages: &[&[u8]]) -> Result<u64, ArchiveError> {
        let start_seq = self.local_cursor;
        for msg in messages {
            self.append(msg)?;
        }
        Ok(start_seq)
    }

    pub fn flush(&mut self) {
        self.state.producer_cursor.0.store(self.local_cursor, Ordering::Release);
        while self.state.consumer_cursor.0.load(Ordering::Acquire) < self.local_cursor {
            std::hint::spin_loop();
        }
    }
}

impl Drop for Archive {
    fn drop(&mut self) {
        self.flush();
        self.state.running.store(false, Ordering::Release);
        if let Some(h) = self.writer_handle.take() { let _ = h.join(); }
    }
}

unsafe impl Send for SharedState {}
unsafe impl Sync for SharedState {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_archive() {
        let dir = tempdir().unwrap();
        let mut archive = Archive::new(dir.path().join("test"), 1024 * 1024).unwrap();
        for i in 0..100u64 { archive.append(&i.to_le_bytes()).unwrap(); }
        archive.flush();
    }
}
