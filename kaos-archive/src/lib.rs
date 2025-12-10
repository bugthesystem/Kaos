//! High-performance message archive using memory-mapped files.
//!
//! ```rust,no_run
//! use kaos_archive::Archive;
//!
//! let mut archive = Archive::create("/tmp/messages", 1024 * 1024).unwrap();
//! let seq = archive.append(b"hello world").unwrap();
//! let msg = archive.read(seq).unwrap();
//! ```

use kaos::crc32::crc32_simd;
use memmap2::{ MmapMut, MmapOptions };
use std::fs::{ File, OpenOptions };
use std::io;
use std::path::Path;
use std::sync::atomic::{ AtomicU64, Ordering };

/// Log file header (64 bytes, cache-line aligned)
#[repr(C, align(64))]
struct LogHeader {
    magic: u64, // "KAOSLOG\0"
    version: u32,
    _reserved: u32,
    write_pos: AtomicU64, // Current write position
    msg_count: AtomicU64, // Total messages
    _pad: [u8; 32],
}

const MAGIC: u64 = 0x004b414f534c4f47; // "KAOSLOG\0"
const HEADER_SIZE: usize = 64;

/// Index entry (16 bytes)
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct IndexEntry {
    offset: u64, // Offset in log file
    length: u32, // Message length
    _pad: u32,
}

/// Message frame header (8 bytes)
#[repr(C)]
struct FrameHeader {
    length: u32, // Payload length
    checksum: u32, // CRC32 of payload
}

const FRAME_HEADER_SIZE: usize = 8;

/// Memory-mapped message archive.
pub struct Archive {
    log_mmap: MmapMut,
    index_mmap: MmapMut,
    _log_file: File,
    _index_file: File,
    capacity: usize,
    // Local state cache (avoid atomic reads on hot path)
    write_pos: usize,
    msg_count: u64,
    // Cached pointers (avoid repeated as_mut_ptr calls)
    log_base: *mut u8,
    idx_base: *mut u8,
    idx_len: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("io: {0}")] Io(#[from] io::Error),
    #[error("archive full")]
    Full,
    #[error("invalid sequence: {0}")] InvalidSequence(u64),
    #[error("corrupted: checksum mismatch")]
    Corrupted,
    #[error("invalid magic")]
    InvalidMagic,
}

impl Archive {
    /// Create a new archive with given capacity.
    pub fn create<P: AsRef<Path>>(base_path: P, capacity: usize) -> Result<Self, ArchiveError> {
        let base = base_path.as_ref();
        let log_path = base.with_extension("log");
        let index_path = base.with_extension("idx");

        // Create log file
        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&log_path)?;
        log_file.set_len(capacity as u64)?;

        // Create index file (16 bytes per entry, estimate 64-byte avg message)
        let index_capacity = (capacity / 64) * std::mem::size_of::<IndexEntry>();
        let index_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;
        index_file.set_len(index_capacity as u64)?;

        // Memory map
        let mut log_mmap = unsafe { MmapOptions::new().map_mut(&log_file)? };
        let mut index_mmap = unsafe { MmapOptions::new().map_mut(&index_file)? };

        // Initialize header
        let header = unsafe { &mut *(log_mmap.as_mut_ptr() as *mut LogHeader) };
        header.magic = MAGIC;
        header.version = 1;
        header.write_pos.store(HEADER_SIZE as u64, Ordering::Release);
        header.msg_count.store(0, Ordering::Release);

        let log_base = log_mmap.as_mut_ptr();
        let idx_base = index_mmap.as_mut_ptr();
        let idx_len = index_mmap.len();

        Ok(Self {
            log_mmap,
            index_mmap,
            _log_file: log_file,
            _index_file: index_file,
            capacity,
            write_pos: HEADER_SIZE,
            msg_count: 0,
            log_base,
            idx_base,
            idx_len,
        })
    }

    /// Open an existing archive.
    pub fn open<P: AsRef<Path>>(base_path: P) -> Result<Self, ArchiveError> {
        let base = base_path.as_ref();
        let log_path = base.with_extension("log");
        let index_path = base.with_extension("idx");

        let log_file = OpenOptions::new().read(true).write(true).open(&log_path)?;
        let index_file = OpenOptions::new().read(true).write(true).open(&index_path)?;

        let capacity = log_file.metadata()?.len() as usize;
        let mut log_mmap = unsafe { MmapOptions::new().map_mut(&log_file)? };
        let mut index_mmap = unsafe { MmapOptions::new().map_mut(&index_file)? };

        // Verify magic and read state
        let header = unsafe { &*(log_mmap.as_ptr() as *const LogHeader) };
        if header.magic != MAGIC {
            return Err(ArchiveError::InvalidMagic);
        }
        let write_pos = header.write_pos.load(Ordering::Relaxed) as usize;
        let msg_count = header.msg_count.load(Ordering::Relaxed);

        let log_base = log_mmap.as_mut_ptr();
        let idx_base = index_mmap.as_mut_ptr();
        let idx_len = index_mmap.len();

        Ok(Self {
            log_mmap,
            index_mmap,
            _log_file: log_file,
            _index_file: index_file,
            capacity,
            write_pos,
            msg_count,
            log_base,
            idx_base,
            idx_len,
        })
    }

    /// Append a message. Returns sequence number.
    #[inline]
    pub fn append(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        self.append_with_options(data, true, true)
    }

    /// Fast append without CRC32 checksum (use when data integrity verified elsewhere).
    /// Still maintains index for random access.
    #[inline]
    pub fn append_unchecked(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        self.append_with_options(data, false, true) // no CRC, yes index
    }

    /// Append without CRC or index. Faster but no random access.
    #[inline]
    pub fn append_no_index(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        self.append_with_options(data, false, false)
    }

    /// Raw append: no bounds check, no index, no CRC. Minimal overhead.
    /// # Safety
    /// Caller must ensure capacity is sufficient.
    #[inline(always)]
    pub unsafe fn append_raw(&mut self, data: &[u8]) -> u64 {
        let seq = self.msg_count;
        let pos = self.write_pos;
        let base = self.log_base.add(pos);

        // Write length (4 bytes) + data directly
        std::ptr::write_unaligned(base as *mut u32, data.len() as u32);
        std::ptr::copy_nonoverlapping(data.as_ptr(), base.add(4), data.len());

        self.write_pos = pos + 4 + data.len();
        self.msg_count = seq + 1;
        seq
    }

    /// Batch append multiple messages. No bounds check, no index, no CRC.
    /// # Safety
    /// Caller must ensure capacity is sufficient for all messages.
    #[inline(always)]
    pub unsafe fn append_batch_raw(&mut self, messages: &[&[u8]]) -> u64 {
        let start_seq = self.msg_count;
        let mut pos = self.write_pos;
        let base = self.log_base;

        for data in messages {
            let dst = base.add(pos);
            std::ptr::write_unaligned(dst as *mut u32, data.len() as u32);
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst.add(4), data.len());
            pos += 4 + data.len();
        }

        self.write_pos = pos;
        self.msg_count += messages.len() as u64;
        start_seq
    }

    /// Append with configurable options.
    #[inline(always)]
    fn append_with_options(
        &mut self,
        data: &[u8],
        compute_crc: bool,
        update_index: bool,
    ) -> Result<u64, ArchiveError> {
        let seq = self.msg_count;
        let pos = self.write_pos;
        let new_pos = pos + FRAME_HEADER_SIZE + data.len();

        if new_pos > self.capacity {
            return Err(ArchiveError::Full);
        }

        unsafe {
            let base = self.log_base.add(pos);

            // Write header (8 bytes: length + checksum) using unaligned writes
            let checksum = if compute_crc { crc32_simd(data) } else { 0 };
            std::ptr::write_unaligned(base as *mut u32, data.len() as u32);
            std::ptr::write_unaligned(base.add(4) as *mut u32, checksum);

            // Write payload
            std::ptr::copy_nonoverlapping(data.as_ptr(), base.add(FRAME_HEADER_SIZE), data.len());

            // Update index using cached pointer and bit shift
            if update_index {
                let idx_pos = (seq as usize) << 4; // * 16
                if idx_pos + 16 <= self.idx_len {
                    let idx_ptr = self.idx_base.add(idx_pos);
                    std::ptr::write_unaligned(idx_ptr as *mut u64, pos as u64);
                    std::ptr::write_unaligned(idx_ptr.add(8) as *mut u32, data.len() as u32);
                }
            }
        }

        // Update local cache
        self.write_pos = new_pos;
        self.msg_count = seq + 1;

        // Periodically sync to mmap header (every 1024 messages)
        if (seq & 0x3ff) == 0 {
            let header_ptr = self.log_mmap.as_mut_ptr() as *mut LogHeader;
            unsafe {
                (*header_ptr).write_pos.store(new_pos as u64, Ordering::Relaxed);
                (*header_ptr).msg_count.store(seq + 1, Ordering::Release);
            }
        }

        Ok(seq)
    }

    /// Read a message by sequence number. Zero-copy.
    pub fn read(&self, seq: u64) -> Result<&[u8], ArchiveError> {
        if seq >= self.msg_count {
            return Err(ArchiveError::InvalidSequence(seq));
        }

        let idx_pos = (seq as usize) * std::mem::size_of::<IndexEntry>();
        let entry = unsafe { &*(self.index_mmap.as_ptr().add(idx_pos) as *const IndexEntry) };

        let offset = entry.offset as usize;
        let length = entry.length as usize;

        // Read checksum from frame header (may be unaligned, so read directly from bytes)
        let checksum_offset = offset + 4; // checksum is at offset 4 in FrameHeader (after length u32)
        let checksum_bytes = &self.log_mmap[checksum_offset..checksum_offset + 4];
        let checksum = u32::from_ne_bytes([
            checksum_bytes[0],
            checksum_bytes[1],
            checksum_bytes[2],
            checksum_bytes[3],
        ]);
        let data = &self.log_mmap[offset + FRAME_HEADER_SIZE..offset + FRAME_HEADER_SIZE + length];

        // Verify checksum
        if crc32_simd(data) != checksum {
            return Err(ArchiveError::Corrupted);
        }

        Ok(data)
    }

    /// Read without checksum verification (faster).
    pub fn read_unchecked(&self, seq: u64) -> Result<&[u8], ArchiveError> {
        if seq >= self.msg_count {
            return Err(ArchiveError::InvalidSequence(seq));
        }

        let idx_pos = (seq as usize) * std::mem::size_of::<IndexEntry>();
        let entry = unsafe { &*(self.index_mmap.as_ptr().add(idx_pos) as *const IndexEntry) };

        let offset = entry.offset as usize;
        let length = entry.length as usize;

        Ok(&self.log_mmap[offset + FRAME_HEADER_SIZE..offset + FRAME_HEADER_SIZE + length])
    }

    /// Replay messages in range.
    pub fn replay<F>(&self, from: u64, to: u64, mut handler: F) -> Result<u64, ArchiveError>
        where F: FnMut(u64, &[u8])
    {
        let mut count = 0;
        for seq in from..to {
            match self.read_unchecked(seq) {
                Ok(data) => {
                    handler(seq, data);
                    count += 1;
                }
                Err(ArchiveError::InvalidSequence(_)) => {
                    break;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(count)
    }

    /// Number of messages.
    pub fn len(&self) -> u64 {
        self.msg_count
    }

    /// Is archive empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Bytes used.
    pub fn bytes_used(&self) -> u64 {
        self.header().write_pos.load(Ordering::Acquire)
    }

    /// Flush to disk.
    pub fn flush(&self) -> Result<(), ArchiveError> {
        self.log_mmap.flush()?;
        self.index_mmap.flush()?;
        Ok(())
    }

    fn header(&self) -> &LogHeader {
        unsafe { &*(self.log_mmap.as_ptr() as *const LogHeader) }
    }
}

impl Archive {
    /// Sync cached state to mmap header.
    pub fn sync(&mut self) {
        let header_ptr = self.log_mmap.as_mut_ptr() as *mut LogHeader;
        unsafe {
            (*header_ptr).write_pos.store(self.write_pos as u64, Ordering::Relaxed);
            (*header_ptr).msg_count.store(self.msg_count, Ordering::Release);
        }
    }
}

impl Drop for Archive {
    fn drop(&mut self) {
        self.sync();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_append_read() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test");

        let mut archive = Archive::create(&path, 1024 * 1024).unwrap();

        let seq0 = archive.append(b"hello").unwrap();
        let seq1 = archive.append(b"world").unwrap();

        assert_eq!(seq0, 0);
        assert_eq!(seq1, 1);
        assert_eq!(archive.len(), 2);

        assert_eq!(archive.read(0).unwrap(), b"hello");
        assert_eq!(archive.read(1).unwrap(), b"world");
    }

    #[test]
    fn test_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test");

        {
            let mut archive = Archive::create(&path, 1024 * 1024).unwrap();
            archive.append(b"persistent").unwrap();
            archive.flush().unwrap();
        }

        let archive = Archive::open(&path).unwrap();
        assert_eq!(archive.len(), 1);
        assert_eq!(archive.read(0).unwrap(), b"persistent");
    }

    #[test]
    fn test_replay() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test");

        let mut archive = Archive::create(&path, 1024 * 1024).unwrap();
        for i in 0..100u64 {
            archive.append(&i.to_le_bytes()).unwrap();
        }

        let mut replayed = Vec::new();
        archive
            .replay(10, 20, |seq, data| {
                replayed.push((seq, data.to_vec()));
            })
            .unwrap();

        assert_eq!(replayed.len(), 10);
        assert_eq!(replayed[0].0, 10);
        assert_eq!(replayed[9].0, 19);
    }
}
