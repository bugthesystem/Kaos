//! High-performance message archive using memory-mapped files.
//!
//! Two implementations:
//! - `Archive` (default) - 30-34 M/s, async with background writer
//! - `SyncArchive` - 22 M/s, sync writes, crash-safe per write
//!
//! ```rust,no_run
//! use kaos_archive::Archive;
//!
//! let mut archive = Archive::new("/tmp/messages", 1024 * 1024).unwrap();
//! archive.append(b"hello world").unwrap();
//! archive.flush(); // Wait for persistence
//! ```

mod archive;
mod sync_archive;

pub use archive::Archive;
pub use sync_archive::SyncArchive;

use std::io;

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
