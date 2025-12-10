//! Synchronous archive - crash-safe per write.

use crate::ArchiveError;
use kaos::crc32::crc32_simd;
use memmap2::{MmapMut, MmapOptions};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

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
const FRAME_HEADER_SIZE: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct IndexEntry {
    offset: u64,
    length: u32,
    _pad: u32,
}

/// Synchronous mmap archive. Crash-safe but slower than `Archive`.
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
    pub fn create<P: AsRef<Path>>(base_path: P, capacity: usize) -> Result<Self, ArchiveError> {
        let base = base_path.as_ref();

        let log_file = OpenOptions::new()
            .read(true).write(true).create(true).truncate(true)
            .open(base.with_extension("log"))?;
        log_file.set_len(capacity as u64)?;

        let index_capacity = (capacity / 64) * 16;
        let index_file = OpenOptions::new()
            .read(true).write(true).create(true).truncate(true)
            .open(base.with_extension("idx"))?;
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
            log_mmap, index_mmap, _log_file: log_file, _index_file: index_file,
            capacity, write_pos: HEADER_SIZE, msg_count: 0,
            log_base, idx_base, idx_len,
        })
    }

    pub fn open<P: AsRef<Path>>(base_path: P) -> Result<Self, ArchiveError> {
        let base = base_path.as_ref();

        let log_file = OpenOptions::new().read(true).write(true).open(base.with_extension("log"))?;
        let index_file = OpenOptions::new().read(true).write(true).open(base.with_extension("idx"))?;

        let capacity = log_file.metadata()?.len() as usize;
        let mut log_mmap = unsafe { MmapOptions::new().map_mut(&log_file)? };
        let mut index_mmap = unsafe { MmapOptions::new().map_mut(&index_file)? };

        let header = unsafe { &*(log_mmap.as_ptr() as *const LogHeader) };
        if header.magic != MAGIC {
            return Err(ArchiveError::InvalidMagic);
        }

        Ok(Self {
            write_pos: header.write_pos.load(Ordering::Relaxed) as usize,
            msg_count: header.msg_count.load(Ordering::Relaxed),
            log_base: log_mmap.as_mut_ptr(),
            idx_base: index_mmap.as_mut_ptr(),
            idx_len: index_mmap.len(),
            log_mmap, index_mmap, _log_file: log_file, _index_file: index_file, capacity,
        })
    }

    // ─── Append (safe) ───────────────────────────────────────────────────────

    /// Append with CRC32 and index (safest, slowest).
    #[inline]
    pub fn append(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        self.append_inner(data, true, true)
    }

    /// Append without CRC32 (faster, still indexed).
    #[inline]
    pub fn append_no_crc(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        self.append_inner(data, false, true)
    }

    /// Append without CRC32 or index (fastest safe).
    #[inline]
    pub fn append_no_index(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        self.append_inner(data, false, false)
    }

    #[inline(always)]
    fn append_inner(&mut self, data: &[u8], crc: bool, index: bool) -> Result<u64, ArchiveError> {
        let seq = self.msg_count;
        let pos = self.write_pos;
        let new_pos = pos + FRAME_HEADER_SIZE + data.len();

        if new_pos > self.capacity {
            return Err(ArchiveError::Full);
        }

        unsafe { self.write_frame(pos, seq, data, crc, index); }

        self.write_pos = new_pos;
        self.msg_count = seq + 1;

        if (seq & 0x3ff) == 0 {
            self.sync_header();
        }

        Ok(seq)
    }

    // ─── Append (unsafe) ─────────────────────────────────────────────────────

    /// Append without bounds check. Caller must ensure capacity.
    /// # Safety
    /// - `write_pos + 8 + data.len()` must not exceed capacity
    #[inline(always)]
    pub unsafe fn append_unchecked(&mut self, data: &[u8]) -> u64 {
        let seq = self.msg_count;
        let pos = self.write_pos;

        self.write_frame(pos, seq, data, false, false);

        self.write_pos = pos + FRAME_HEADER_SIZE + data.len();
        self.msg_count = seq + 1;
        seq
    }

    #[inline(always)]
    unsafe fn write_frame(&mut self, pos: usize, seq: u64, data: &[u8], crc: bool, index: bool) {
        let base = self.log_base.add(pos);
        let checksum = if crc { crc32_simd(data) } else { 0 };
        std::ptr::write_unaligned(base as *mut u32, data.len() as u32);
        std::ptr::write_unaligned(base.add(4) as *mut u32, checksum);
        std::ptr::copy_nonoverlapping(data.as_ptr(), base.add(FRAME_HEADER_SIZE), data.len());

        if index {
            let idx_pos = (seq as usize) << 4;
            if idx_pos + 16 <= self.idx_len {
                let idx_ptr = self.idx_base.add(idx_pos);
                std::ptr::write_unaligned(idx_ptr as *mut u64, pos as u64);
                std::ptr::write_unaligned(idx_ptr.add(8) as *mut u32, data.len() as u32);
            }
        }
    }

    // ─── Read (safe) ─────────────────────────────────────────────────────────

    /// Read with CRC32 verification.
    pub fn read(&self, seq: u64) -> Result<&[u8], ArchiveError> {
        if seq >= self.msg_count {
            return Err(ArchiveError::InvalidSequence(seq));
        }

        let entry = unsafe { &*(self.index_mmap.as_ptr().add((seq as usize) * 16) as *const IndexEntry) };
        let offset = entry.offset as usize;
        let length = entry.length as usize;
        let data = &self.log_mmap[offset + FRAME_HEADER_SIZE..offset + FRAME_HEADER_SIZE + length];

        let checksum = u32::from_ne_bytes(self.log_mmap[offset + 4..offset + 8].try_into().unwrap());
        if crc32_simd(data) != checksum {
            return Err(ArchiveError::Corrupted);
        }

        Ok(data)
    }

    /// Read without CRC32 verification (faster).
    pub fn read_no_verify(&self, seq: u64) -> Result<&[u8], ArchiveError> {
        if seq >= self.msg_count {
            return Err(ArchiveError::InvalidSequence(seq));
        }

        let entry = unsafe { &*(self.index_mmap.as_ptr().add((seq as usize) * 16) as *const IndexEntry) };
        let offset = entry.offset as usize;
        Ok(&self.log_mmap[offset + FRAME_HEADER_SIZE..offset + FRAME_HEADER_SIZE + entry.length as usize])
    }

    // ─── Read (unsafe) ───────────────────────────────────────────────────────

    /// Read without bounds check. Caller must ensure seq < msg_count.
    /// # Safety
    /// - `seq` must be a valid sequence number
    #[inline(always)]
    pub unsafe fn read_unchecked(&self, seq: u64) -> &[u8] {
        let entry = &*(self.index_mmap.as_ptr().add((seq as usize) * 16) as *const IndexEntry);
        let offset = entry.offset as usize;
        &self.log_mmap[offset + FRAME_HEADER_SIZE..offset + FRAME_HEADER_SIZE + entry.length as usize]
    }

    // ─── Utility ─────────────────────────────────────────────────────────────

    pub fn len(&self) -> u64 { self.msg_count }
    pub fn is_empty(&self) -> bool { self.msg_count == 0 }

    pub fn flush(&self) -> Result<(), ArchiveError> {
        self.log_mmap.flush()?;
        self.index_mmap.flush()?;
        Ok(())
    }

    fn sync_header(&mut self) {
        let header = unsafe { &mut *(self.log_mmap.as_mut_ptr() as *mut LogHeader) };
        header.write_pos.store(self.write_pos as u64, Ordering::Relaxed);
        header.msg_count.store(self.msg_count, Ordering::Release);
    }
}

impl Drop for SyncArchive {
    fn drop(&mut self) { self.sync_header(); }
}

unsafe impl Send for SyncArchive {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sync_archive() {
        let dir = tempdir().unwrap();
        let mut archive = SyncArchive::create(dir.path().join("test"), 1024 * 1024).unwrap();
        
        // Safe API with CRC
        let seq = archive.append(b"hello").unwrap();
        assert_eq!(archive.read(seq).unwrap(), b"hello");
        
        // Safe API without CRC
        let seq2 = archive.append_no_crc(b"world").unwrap();
        assert_eq!(archive.read_no_verify(seq2).unwrap(), b"world");
        
        // Unsafe read (seq must be valid)
        unsafe {
            assert_eq!(archive.read_unchecked(seq), b"hello");
            assert_eq!(archive.read_unchecked(seq2), b"world");
        }
    }
}
