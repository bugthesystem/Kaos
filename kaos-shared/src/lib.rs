//! # kaos-shared
//!
//! Shared protocol types for Kaos networking.
//!
//! This crate provides the low-level transport types used by both
//! client SDKs and server implementations:
//!
//! - [`MessageType`]: RUDP message type discriminator
//! - [`PacketHeader`]: 24-byte RUDP packet header
//! - CRC32 checksum utilities
//!
//! ## Layer Diagram
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ kaostalk (Application Layer)            │
//! │ - ClientMessage, ServerMessage (JSON)   │
//! └────────────────────┬────────────────────┘
//!                      │
//! ┌────────────────────┴────────────────────┐
//! │ kaos-shared (Transport Layer)           │
//! │ - MessageType, PacketHeader (Binary)    │  ← This crate
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use kaos_shared::{MessageType, PacketHeader, HEADER_SIZE};
//!
//! // Create a data packet header
//! let mut header = PacketHeader::new(42, MessageType::Data, 100);
//! header.calculate_checksum(b"payload data");
//!
//! // Serialize to bytes
//! let bytes = header.to_bytes();
//! assert_eq!(bytes.len(), HEADER_SIZE);
//! ```

mod header;
mod message_type;

pub use header::{PacketHeader, HEADER_SIZE};
pub use message_type::MessageType;

/// Multiplexing key size in bytes (u32 = 4 bytes)
pub const MUX_KEY_SIZE: usize = 4;

/// Re-export CRC32 utilities
pub mod crc32 {
    pub use crc32fast::Hasher;

    /// Calculate CRC32 checksum for data
    #[inline]
    pub fn crc32(data: &[u8]) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(data);
        hasher.finalize()
    }

    /// Calculate CRC32 incrementally (continue from previous checksum)
    #[inline]
    pub fn crc32_incremental(initial_crc: u32, data: &[u8]) -> u32 {
        let mut hasher = Hasher::new_with_initial(initial_crc);
        hasher.update(data);
        hasher.finalize()
    }
}
