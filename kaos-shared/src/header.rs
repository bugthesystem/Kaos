//! RUDP packet header.

use crate::crc32::{crc32, crc32_incremental};
use crate::MessageType;
use bytemuck::{Pod, Zeroable};

/// RUDP packet header size in bytes.
pub const HEADER_SIZE: usize = 24;

/// RUDP packet header (24 bytes).
///
/// Layout:
/// ```text
/// Offset  Size  Field
/// 0       4     session_id
/// 4       8     sequence
/// 12      1     msg_type
/// 13      1     flags
/// 14      2     payload_len
/// 16      4     timestamp
/// 20      4     checksum
/// ```
///
/// The checksum covers the entire header (with checksum field zeroed) plus payload.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PacketHeader {
    /// Session identifier (0 for initial handshake)
    pub session_id: u32,
    /// Packet sequence number
    pub sequence: u64,
    /// Message type (see [`MessageType`])
    pub msg_type: u8,
    /// Flags (0x01 = unreliable/no-retransmit)
    pub flags: u8,
    /// Payload length in bytes
    pub payload_len: u16,
    /// Timestamp (milliseconds, used for RTT calculation)
    pub timestamp: u32,
    /// CRC32 checksum of header + payload
    pub checksum: u32,
}

impl PacketHeader {
    /// Size of the header in bytes.
    pub const SIZE: usize = HEADER_SIZE;

    /// Create a new packet header.
    #[inline]
    pub fn new(sequence: u64, msg_type: MessageType, payload_len: usize) -> Self {
        Self {
            session_id: 0,
            sequence,
            msg_type: msg_type as u8,
            flags: 0,
            payload_len: payload_len as u16,
            timestamp: 0,
            checksum: 0,
        }
    }

    /// Create a new packet header with current timestamp.
    pub fn new_with_timestamp(sequence: u64, msg_type: MessageType, payload_len: usize) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| (d.as_millis() & 0xFFFFFFFF) as u32)
            .unwrap_or(0);

        Self {
            session_id: 0,
            sequence,
            msg_type: msg_type as u8,
            flags: 0,
            payload_len: payload_len as u16,
            timestamp,
            checksum: 0,
        }
    }

    /// Serialize header to bytes.
    #[inline]
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.session_id.to_le_bytes());
        buf[4..12].copy_from_slice(&self.sequence.to_le_bytes());
        buf[12] = self.msg_type;
        buf[13] = self.flags;
        buf[14..16].copy_from_slice(&self.payload_len.to_le_bytes());
        buf[16..20].copy_from_slice(&self.timestamp.to_le_bytes());
        buf[20..24].copy_from_slice(&self.checksum.to_le_bytes());
        buf
    }

    /// Parse header from bytes.
    ///
    /// Returns `None` if buffer is too small.
    #[inline]
    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < HEADER_SIZE {
            return None;
        }
        Some(Self {
            session_id: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
            sequence: u64::from_le_bytes([
                buf[4], buf[5], buf[6], buf[7], buf[8], buf[9], buf[10], buf[11],
            ]),
            msg_type: buf[12],
            flags: buf[13],
            payload_len: u16::from_le_bytes([buf[14], buf[15]]),
            timestamp: u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]),
            checksum: u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]),
        })
    }

    /// Parse header from packet buffer, also returning the payload slice.
    ///
    /// Returns `None` if buffer is too small or payload_len doesn't match.
    #[inline]
    pub fn from_packet(buf: &[u8]) -> Option<(Self, &[u8])> {
        let header = Self::from_bytes(buf)?;
        let payload_len = { header.payload_len } as usize;

        if buf.len() < HEADER_SIZE + payload_len {
            return None;
        }

        Some((header, &buf[HEADER_SIZE..HEADER_SIZE + payload_len]))
    }

    /// Calculate and set the CRC32 checksum.
    ///
    /// Checksum covers header (with checksum field zeroed) + payload.
    pub fn calculate_checksum(&mut self, payload: &[u8]) {
        self.checksum = 0;
        let header_bytes = self.to_bytes();
        let header_crc = crc32(&header_bytes);
        self.checksum = crc32_incremental(header_crc, payload);
    }

    /// Verify the checksum against the payload.
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        let stored_checksum = { self.checksum };

        let mut header_copy = *self;
        header_copy.checksum = 0;
        let header_bytes = header_copy.to_bytes();
        let header_crc = crc32(&header_bytes);
        let computed = crc32_incremental(header_crc, payload);

        stored_checksum == computed
    }

    /// Get the message type as enum.
    #[inline]
    pub fn message_type(&self) -> MessageType {
        MessageType::from_u8_lossy(self.msg_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(std::mem::size_of::<PacketHeader>(), HEADER_SIZE);
    }

    #[test]
    fn test_header_roundtrip() {
        let mut header = PacketHeader::new(42, MessageType::Data, 100);
        header.session_id = 12345;
        header.timestamp = 9999;
        header.calculate_checksum(b"test payload");

        let bytes = header.to_bytes();
        let parsed = PacketHeader::from_bytes(&bytes).unwrap();

        assert_eq!({ parsed.session_id }, 12345);
        assert_eq!({ parsed.sequence }, 42);
        assert_eq!({ parsed.msg_type }, MessageType::Data as u8);
        assert_eq!({ parsed.payload_len }, 100);
        assert_eq!({ parsed.timestamp }, 9999);
        assert_eq!({ parsed.checksum }, { header.checksum });
    }

    #[test]
    fn test_checksum_verification() {
        let payload = b"hello world";
        let mut header = PacketHeader::new(1, MessageType::Data, payload.len());
        header.calculate_checksum(payload);

        assert!(header.verify_checksum(payload));
        assert!(!header.verify_checksum(b"wrong payload"));
    }

    #[test]
    fn test_from_packet() {
        let payload = b"test data";
        let mut header = PacketHeader::new(5, MessageType::Data, payload.len());
        header.calculate_checksum(payload);

        let mut packet = Vec::with_capacity(HEADER_SIZE + payload.len());
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(payload);

        let (parsed_header, parsed_payload) = PacketHeader::from_packet(&packet).unwrap();
        assert_eq!({ parsed_header.sequence }, 5);
        assert_eq!(parsed_payload, payload);
    }

    #[test]
    fn test_handshake_header() {
        let header = PacketHeader::new(0, MessageType::Handshake, 0);
        assert_eq!({ header.msg_type }, 5);
        assert_eq!(header.message_type(), MessageType::Handshake);
    }
}
