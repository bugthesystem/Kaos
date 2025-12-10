//! Message archive using memory-mapped files.
//!
//! - `Archive` - 17 M/s, background writer, call `flush()`
//! - `SyncArchive` - 28 M/s, crash-safe per write

mod archive;
mod sync_archive;

pub use archive::Archive;
pub use sync_archive::SyncArchive;

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("full")]
    Full,
    #[error("invalid seq: {0}")]
    InvalidSequence(u64),
    #[error("corrupted")]
    Corrupted,
    #[error("invalid magic")]
    InvalidMagic,
}
