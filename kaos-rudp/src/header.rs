//! Protocol headers for reliable UDP.

use bytemuck::{Pod, Zeroable};
use kaos::crc32;
use std::time::{SystemTime, UNIX_EPOCH};

/// Message types for reliable UDP protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Data = 0,
    Heartbeat = 1,
    Nak = 2,
    SessionStart = 3,
    SessionEnd = 4,
    Ack = 5,
}

impl TryFrom<u8> for MessageType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MessageType::Data),
            1 => Ok(MessageType::Heartbeat),
            2 => Ok(MessageType::Nak),
            3 => Ok(MessageType::SessionStart),
            4 => Ok(MessageType::SessionEnd),
            5 => Ok(MessageType::Ack),
            _ => Err(()),
        }
    }
}

/// Header flags
pub const FLAG_NO_CRC: u8 = 0x01;

/// Magic marker for FastHeader format
pub const FAST_HEADER_MAGIC: u32 = 0x80000000;

/// Minimal 8-byte header (Aeron-style)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FastHeader {
    pub frame_length: u32,
    pub sequence: u32,
}

impl FastHeader {
    pub const SIZE: usize = 8;

    #[inline(always)]
    pub fn new(sequence: u32, payload_len: usize) -> Self {
        Self {
            frame_length: FAST_HEADER_MAGIC | ((Self::SIZE + payload_len) as u32),
            sequence,
        }
    }
}

/// Full 24-byte header with CRC
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ReliableUdpHeader {
    pub session_id: u32,
    pub sequence: u64,
    pub msg_type: u8,
    pub flags: u8,
    pub payload_len: u16,
    pub timestamp: u32,
    pub checksum: u32,
}

impl ReliableUdpHeader {
    pub const SIZE: usize = std::mem::size_of::<Self>();

    pub fn new(session_id: u32, sequence: u64, msg_type: MessageType, payload_len: u16) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u32;

        Self {
            session_id,
            sequence,
            msg_type: msg_type as u8,
            flags: 0,
            payload_len,
            timestamp,
            checksum: 0,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(*bytemuck::from_bytes::<Self>(&bytes[..Self::SIZE]))
    }

    pub fn from_packet_with_payload_check(packet: &[u8]) -> Option<(&Self, &[u8])> {
        if packet.len() < Self::SIZE {
            return None;
        }
        let (header_bytes, payload) = packet.split_at(Self::SIZE);
        let header = bytemuck::from_bytes::<Self>(header_bytes);
        if payload.len() < (header.payload_len as usize) {
            return None;
        }
        let (actual_payload, _) = payload.split_at(header.payload_len as usize);
        Some((header, actual_payload))
    }

    pub fn calculate_checksum(&mut self, payload: &[u8]) {
        self.checksum = 0;
        let header_bytes = bytemuck::bytes_of(self);
        let mut crc = crc32::crc32_simd(header_bytes);
        crc = crc32::crc32_incremental(crc, payload);
        self.checksum = crc;
    }

    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        let mut temp = *self;
        temp.calculate_checksum(payload);
        temp.checksum == self.checksum
    }
}

