//! Zero-Copy Tap Archived Transport
//!
//! Uses a lock-free SPSC ring buffer for zero-copy recording.
//! Hot path: atomic store + memcpy to ring buffer slot (~5ns)
//!
//! ```text
//! Producer ───► tap_buffer (lock-free) ───► Network
//!                      │
//!                      └──► Recorder Thread (poll + archive)
//! ```

use crate::{FastHeader, ReliableUdpRingBufferTransport};
use kaos::disruptor::{MessageRingBuffer, RingBufferConfig, RingBufferEntry};
use kaos_archive::{Archive, ArchiveError};
use std::cell::UnsafeCell;
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Wrapper for SPSC access to ring buffer
struct SpscBuffer {
    buffer: UnsafeCell<MessageRingBuffer>,
}

unsafe impl Sync for SpscBuffer {}
unsafe impl Send for SpscBuffer {}

impl SpscBuffer {
    fn new(buffer: MessageRingBuffer) -> Self {
        Self {
            buffer: UnsafeCell::new(buffer),
        }
    }

    /// SAFETY: Only called from producer thread (ArchivedTransport.send)
    #[inline]
    #[allow(clippy::mut_from_ref)]
    unsafe fn producer(&self) -> &mut MessageRingBuffer {
        &mut *self.buffer.get()
    }

    #[inline]
    fn consumer(&self) -> &MessageRingBuffer {
        unsafe { &*self.buffer.get() }
    }
}

/// Archived RUDP transport with lock-free recording
///
/// Uses a background thread to archive messages without blocking the hot path.
/// Hot path cost: ~5ns (atomic store + memcpy to ring buffer slot)
pub struct ArchivedTransport {
    inner: ReliableUdpRingBufferTransport,
    tap_buffer: Arc<SpscBuffer>,
    producer_seq: u64,
    archive: Arc<std::sync::Mutex<Archive>>,
    running: Arc<AtomicBool>,
    recorder_handle: Option<JoinHandle<()>>,
    archived_seq: Arc<AtomicU64>,
    msg_count: Arc<AtomicU64>,
}

#[derive(Debug)]
pub enum ArchivedError {
    Io(io::Error),
    Archive(ArchiveError),
    BufferFull,
}

impl From<io::Error> for ArchivedError {
    fn from(e: io::Error) -> Self {
        ArchivedError::Io(e)
    }
}

impl From<ArchiveError> for ArchivedError {
    fn from(e: ArchiveError) -> Self {
        ArchivedError::Archive(e)
    }
}

impl std::fmt::Display for ArchivedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchivedError::Io(e) => write!(f, "IO: {}", e),
            ArchivedError::Archive(e) => write!(f, "Archive: {}", e),
            ArchivedError::BufferFull => write!(f, "Buffer full"),
        }
    }
}

impl std::error::Error for ArchivedError {}

impl ArchivedTransport {
    pub fn new<P: AsRef<Path>>(
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
        window_size: usize,
        archive_path: P,
        archive_capacity: usize,
    ) -> Result<Self, ArchivedError> {
        let inner = ReliableUdpRingBufferTransport::new(local_addr, remote_addr, window_size)?;
        let archive = Archive::create(archive_path, archive_capacity)?;

        let config = RingBufferConfig::new(window_size)
            .map_err(|e| io::Error::other(format!("Config: {}", e)))?
            .with_consumers(1)
            .map_err(|e| io::Error::other(format!("Config: {}", e)))?;

        let tap_buffer = MessageRingBuffer::new(config)
            .map_err(|e| io::Error::other(format!("RingBuffer: {}", e)))?;

        let tap_buffer = Arc::new(SpscBuffer::new(tap_buffer));
        let archived_seq = Arc::new(AtomicU64::new(0));
        let msg_count = Arc::new(AtomicU64::new(0));
        let archive = Arc::new(std::sync::Mutex::new(archive));
        let running = Arc::new(AtomicBool::new(true));

        let recorder_handle = {
            let tap_buffer = tap_buffer.clone();
            let msg_count = msg_count.clone();
            let archived_seq = archived_seq.clone();
            let archive = archive.clone();
            let running = running.clone();

            thread::Builder::new()
                .name("kaos-tap-recorder".into())
                .spawn(move || {
                    Self::recorder_loop(tap_buffer, msg_count, archived_seq, archive, running);
                })
                .expect("Failed to spawn recorder")
        };

        Ok(Self {
            inner,
            tap_buffer,
            producer_seq: 0,
            archive,
            running,
            recorder_handle: Some(recorder_handle),
            archived_seq,
            msg_count,
        })
    }

    fn recorder_loop(
        tap_buffer: Arc<SpscBuffer>,
        msg_count: Arc<AtomicU64>,
        archived_seq: Arc<AtomicU64>,
        archive: Arc<std::sync::Mutex<Archive>>,
        running: Arc<AtomicBool>,
    ) {
        const BATCH_SIZE: usize = 64;

        while running.load(Ordering::Relaxed) {
            let total = msg_count.load(Ordering::Acquire);
            let archived = archived_seq.load(Ordering::Relaxed);

            if total <= archived {
                std::hint::spin_loop();
                continue;
            }

            // Use try_consume_batch to read available messages
            let buffer = tap_buffer.consumer();
            let slots = buffer.try_consume_batch(0, BATCH_SIZE);

            if slots.is_empty() {
                std::hint::spin_loop();
                continue;
            }

            {
                let mut arch = archive.lock().unwrap();
                for slot in &slots {
                    let data = slot.data();
                    if !data.is_empty() {
                        let _ = arch.append(data);
                    }
                }
            }

            let new_archived = archived + slots.len() as u64;
            archived_seq.store(new_archived, Ordering::Release);
        }

        // Drain on shutdown
        loop {
            let buffer = tap_buffer.consumer();
            let slots = buffer.try_consume_batch(0, BATCH_SIZE);
            if slots.is_empty() {
                break;
            }

            {
                let mut arch = archive.lock().unwrap();
                for slot in &slots {
                    let data = slot.data();
                    if !data.is_empty() {
                        let _ = arch.append(data);
                    }
                }
                let _ = arch.flush();
            }

            let archived = archived_seq.load(Ordering::Relaxed);
            archived_seq.store(archived + slots.len() as u64, Ordering::Release);
        }
    }

    #[inline]
    pub fn send(&mut self, data: &[u8]) -> Result<u64, ArchivedError> {
        let seq = self.producer_seq;

        // Write to tap buffer (lock-free, ~5ns)
        let buffer = unsafe { self.tap_buffer.producer() };
        if let Some((claimed_seq, slots)) = buffer.try_claim_slots(1) {
            slots[0].set_sequence(claimed_seq);
            slots[0].set_data(data);
            buffer.publish_batch(claimed_seq, 1);
            self.producer_seq += 1;
            self.msg_count.fetch_add(1, Ordering::Release);
        }

        self.inner.send(data)?;
        Ok(seq)
    }

    #[inline]
    pub fn send_batch(&mut self, messages: &[&[u8]]) -> Result<usize, ArchivedError> {
        if messages.is_empty() {
            return Ok(0);
        }

        let buffer = unsafe { self.tap_buffer.producer() };
        if let Some((claimed_seq, slots)) = buffer.try_claim_slots(messages.len()) {
            let count = slots.len();
            for (i, (slot, msg)) in slots.iter_mut().zip(messages.iter()).enumerate() {
                slot.set_sequence(claimed_seq + i as u64);
                slot.set_data(msg);
            }
            buffer.publish_batch(claimed_seq, count);
            self.producer_seq += count as u64;
            self.msg_count.fetch_add(count as u64, Ordering::Release);
        }

        let sent = self.inner.send_batch(messages)?;
        Ok(sent)
    }

    pub fn receive_batch_with<F>(&mut self, max_batch: usize, handler: F)
    where
        F: FnMut(&[u8]),
    {
        self.inner.receive_batch_with(max_batch, handler);
    }

    pub fn retransmit_from_archive(&self, seq: u64) -> Result<(), ArchivedError> {
        let archive = self.archive.lock().unwrap();
        if seq >= archive.len() {
            return Err(ArchivedError::Archive(ArchiveError::InvalidSequence(seq)));
        }

        let data = archive.read_unchecked(seq)?;
        let header = FastHeader::new(seq as u32, data.len());
        let mut packet = Vec::with_capacity(FastHeader::SIZE + data.len());
        packet.extend_from_slice(bytemuck::bytes_of(&header));
        packet.extend_from_slice(data);

        self.inner
            .socket()
            .send_to(&packet, self.inner.remote_addr())?;
        kaos::record_retransmit();
        Ok(())
    }

    pub fn replay<F>(&self, from: u64, to: u64, mut handler: F) -> Result<u64, ArchivedError>
    where
        F: FnMut(u64, &[u8]),
    {
        let archive = self.archive.lock().unwrap();
        let count = archive.replay(from, to, |seq, data| handler(seq, data))?;
        Ok(count)
    }

    pub fn wait_for_archive(&self) {
        let target = self.msg_count.load(Ordering::Acquire);
        while self.archived_seq.load(Ordering::Acquire) < target {
            std::hint::spin_loop();
        }
    }

    pub fn flush(&self) -> Result<(), ArchivedError> {
        self.wait_for_archive();
        let archive = self.archive.lock().unwrap();
        archive.flush()?;
        Ok(())
    }

    pub fn archive_len(&self) -> u64 {
        self.archived_seq.load(Ordering::Acquire)
    }

    pub fn pending(&self) -> u64 {
        self.msg_count
            .load(Ordering::Acquire)
            .saturating_sub(self.archived_seq.load(Ordering::Acquire))
    }

    pub fn inner(&self) -> &ReliableUdpRingBufferTransport {
        &self.inner
    }
}

impl Drop for ArchivedTransport {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(handle) = self.recorder_handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn get_test_addr(base: u16) -> SocketAddr {
        format!("127.0.0.1:{}", base).parse().unwrap()
    }

    #[test]
    fn test_send_archive() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("archive");

        let mut transport = ArchivedTransport::new(
            get_test_addr(21000),
            get_test_addr(21100),
            1024,
            &archive_path,
            1024 * 1024,
        )
        .unwrap();

        for i in 0..20 {
            let _ = transport.send(format!("tap-msg-{}", i).as_bytes());
        }

        transport.wait_for_archive();

        assert!(transport.archive_len() > 0);
        let archive = transport.archive.lock().unwrap();
        assert_eq!(archive.read_unchecked(0).unwrap(), b"tap-msg-0");
    }

    #[test]
    fn test_archive_performance() {
        let dir = tempdir().unwrap();
        let archive_path = dir.path().join("perf");

        let mut transport = ArchivedTransport::new(
            get_test_addr(21200),
            get_test_addr(21300),
            8192,
            &archive_path,
            64 * 1024 * 1024,
        )
        .unwrap();

        let msg = vec![0u8; 64];
        let count = 10_000;

        let start = std::time::Instant::now();
        for _ in 0..count {
            let _ = transport.send(&msg);
        }
        let send_time = start.elapsed();

        transport.wait_for_archive();
        let total_time = start.elapsed();

        println!(
            "ARCHIVED: {} msgs, send={:?}, total={:?}, rate={:.1}M/s",
            count,
            send_time,
            total_time,
            count as f64 / total_time.as_secs_f64() / 1_000_000.0
        );

        assert!(
            send_time.as_millis() < 100,
            "Send took too long: {:?}",
            send_time
        );
    }
}
