//! Message archive using memory-mapped files.
//!
//! - `Archive` - background writer, call `flush()`
//! - `MmapArchive` - faster but crash-safe per write

mod archive;
mod mmap_archive;

pub use archive::Archive;
pub use mmap_archive::MmapArchive;

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
