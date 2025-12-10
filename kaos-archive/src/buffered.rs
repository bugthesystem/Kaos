//! Buffered file archive - bypasses mmap for higher throughput.

use crate::ArchiveError;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

const BUFFER_SIZE: usize = 256 * 1024; // 256KB buffer

/// Buffered file archive (no mmap).
pub struct BufferedArchive {
    writer: BufWriter<File>,
    msg_count: u64,
}

impl BufferedArchive {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, ArchiveError> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path.as_ref().with_extension("log"))?;

        Ok(Self {
            writer: BufWriter::with_capacity(BUFFER_SIZE, file),
            msg_count: 0,
        })
    }

    #[inline]
    pub fn append(&mut self, data: &[u8]) -> Result<u64, ArchiveError> {
        let len = data.len() as u32;
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(&[0u8; 4])?; // checksum placeholder
        self.writer.write_all(data)?;
        let seq = self.msg_count;
        self.msg_count += 1;
        Ok(seq)
    }

    pub fn flush(&mut self) -> Result<(), ArchiveError> {
        self.writer.flush()?;
        Ok(())
    }

    pub fn len(&self) -> u64 { self.msg_count }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_buffered() {
        let dir = tempdir().unwrap();
        let mut archive = BufferedArchive::create(dir.path().join("test")).unwrap();
        for i in 0..1000u64 {
            archive.append(&i.to_le_bytes()).unwrap();
        }
        archive.flush().unwrap();
        assert_eq!(archive.len(), 1000);
    }
}

