//! Synchronous archive - crash-safe per write, simpler API.
//!
//! Use when:
//! - Need guaranteed persistence on each write
//! - Lower throughput is acceptable (22 M/s vs 30-34 M/s)
//! - Want simpler API without flush()

use crate::ArchiveError;
use kaos::crc32::crc32_simd;
use memmap2::{MmapMut, MmapOptions};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Log file header (64 bytes, cache-line aligned)
#[repr(C, align(64))]
struct LogHeader {
    magic: u64,
    version: u32,
    _reserved: u32,
    write_pos: AtomicU64,
    msg_count: AtomicU64,
    _pad: [u8; 32],
}

const MAGIC: u64 = 0x004b414f534c4f47; // "KAOSLOG\0"
const HEADER_SIZE: usize = 64;

/// Index entry (16 bytes)
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct IndexEntry {
    offset: u64,
    length: u32,
    _pad: u32,
}

const FRAME_HEADER_SIZE: usize = 8;

/// Synchronous memory-mapped archive.
/// 
/// Writes directly to mmap - crash-safe but slower than `Archive`.
pub struct SyncArchive {
    log_mmap: MmapMut,
    index_mmap: MmapMut,
    _log_file: File,
    _index_file: File,
    capacity: usize,
    write_pos: usize,
    msg_count: u64,
    log_base: *mut u8,
    idx_base: *mut u8,
    idx_len: usize,
}

impl SyncArchive {
    /// Create a new archive.
    pub fn create<P: AsRef<Path>>(base_path: P, capacity: usize) -> Result<Self, ArchiveError> {
        let base = base_path.as_ref();
        let log_path = base.with_extension("log");
        let index_path = base.with_extension("idx");

        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&log_path)?;
        log_file.set_len(capacity as u64)?;

        let index_capacity = (capacity / 64) * std::mem::size_of::<IndexEntry>();
        let index_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;
        index_file.set_len(index_capacity as u64)?;

        let mut log_mmap = unsafe { MmapOptions::new().map_mut(&log_file)? };
        let mut index_mmap = unsafe { MmapOptions::new().map_mut(&index_file)? };

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

    /// Append a message with CRC32 checksum.
    #[inline]
    pub fn append(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        self.append_with_options(data, true, true)
    }

    /// Append without CRC32 (faster).
    #[inline]
    pub fn append_unchecked(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        self.append_with_options(data, false, true)
    }

    /// Append without CRC or index.
    #[inline]
    pub fn append_no_index(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        self.append_with_options(data, false, false)
    }

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
            let checksum = if compute_crc { crc32_simd(data) } else { 0 };
            std::ptr::write_unaligned(base as *mut u32, data.len() as u32);
            std::ptr::write_unaligned(base.add(4) as *mut u32, checksum);
            std::ptr::copy_nonoverlapping(data.as_ptr(), base.add(FRAME_HEADER_SIZE), data.len());

            if update_index {
                let idx_pos = (seq as usize) << 4;
                if idx_pos + 16 <= self.idx_len {
                    let idx_ptr = self.idx_base.add(idx_pos);
                    std::ptr::write_unaligned(idx_ptr as *mut u64, pos as u64);
                    std::ptr::write_unaligned(idx_ptr.add(8) as *mut u32, data.len() as u32);
                }
            }
        }

        self.write_pos = new_pos;
        self.msg_count = seq + 1;

        if (seq & 0x3ff) == 0 {
            self.sync_header();
        }

        Ok(seq)
    }

    /// Read a message by sequence (zero-copy).
    pub fn read(&self, seq: u64) -> Result<&[u8], ArchiveError> {
        if seq >= self.msg_count {
            return Err(ArchiveError::InvalidSequence(seq));
        }

        let idx_pos = (seq as usize) * std::mem::size_of::<IndexEntry>();
        let entry = unsafe { &*(self.index_mmap.as_ptr().add(idx_pos) as *const IndexEntry) };

        let offset = entry.offset as usize;
        let length = entry.length as usize;

        let checksum_bytes = &self.log_mmap[offset + 4..offset + 8];
        let checksum = u32::from_ne_bytes([
            checksum_bytes[0],
            checksum_bytes[1],
            checksum_bytes[2],
            checksum_bytes[3],
        ]);
        let data = &self.log_mmap[offset + FRAME_HEADER_SIZE..offset + FRAME_HEADER_SIZE + length];

        if crc32_simd(data) != checksum {
            return Err(ArchiveError::Corrupted);
        }

        Ok(data)
    }

    /// Read without checksum verification.
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

    /// Number of messages.
    pub fn len(&self) -> u64 {
        self.msg_count
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Flush to disk.
    pub fn flush(&self) -> Result<(), ArchiveError> {
        self.log_mmap.flush()?;
        self.index_mmap.flush()?;
        Ok(())
    }

    fn sync_header(&mut self) {
        let header_ptr = self.log_mmap.as_mut_ptr() as *mut LogHeader;
        unsafe {
            (*header_ptr).write_pos.store(self.write_pos as u64, Ordering::Relaxed);
            (*header_ptr).msg_count.store(self.msg_count, Ordering::Release);
        }
    }
}

impl Drop for SyncArchive {
    fn drop(&mut self) {
        self.sync_header();
    }
}

// Safety: SyncArchive uses mmap which is thread-safe for reads
unsafe impl Send for SyncArchive {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sync_archive() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test");

        let mut archive = SyncArchive::create(&path, 1024 * 1024).unwrap();
        let seq = archive.append(b"hello").unwrap();
        assert_eq!(archive.read(seq).unwrap(), b"hello");
    }
}

