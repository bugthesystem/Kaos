//! Wire protocol for KaosNet.

use bytemuck::{Pod, Zeroable};

/// Message opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    // Session lifecycle
    SessionStart = 0x01,
    SessionEnd = 0x02,
    Heartbeat = 0x03,
    SessionAck = 0x04,

    // Room operations
    RoomCreate = 0x10,
    RoomJoin = 0x11,
    RoomLeave = 0x12,
    RoomData = 0x13,
    RoomState = 0x14,
    RoomList = 0x15,

    // RPC
    Rpc = 0x20,
    RpcResponse = 0x21,

    // Errors
    Error = 0xFF,
}

impl TryFrom<u8> for Op {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match value {
            0x01 => Ok(Op::SessionStart),
            0x02 => Ok(Op::SessionEnd),
            0x03 => Ok(Op::Heartbeat),
            0x04 => Ok(Op::SessionAck),
            0x10 => Ok(Op::RoomCreate),
            0x11 => Ok(Op::RoomJoin),
            0x12 => Ok(Op::RoomLeave),
            0x13 => Ok(Op::RoomData),
            0x14 => Ok(Op::RoomState),
            0x15 => Ok(Op::RoomList),
            0x20 => Ok(Op::Rpc),
            0x21 => Ok(Op::RpcResponse),
            0xFF => Ok(Op::Error),
            _ => Err(()),
        }
    }
}

/// Message header (4 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Header {
    pub op: u8,
    pub flags: u8,
    pub len: u16,
}

impl Header {
    pub const SIZE: usize = std::mem::size_of::<Self>();

    #[inline]
    pub fn new(op: Op, len: u16) -> Self {
        Self {
            op: op as u8,
            flags: 0,
            len,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(*bytemuck::from_bytes::<Self>(&bytes[..Self::SIZE]))
    }
}

/// Message with header and payload
#[derive(Debug, Clone)]
pub struct Message {
    pub op: Op,
    pub flags: u8,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new(op: Op, payload: Vec<u8>) -> Self {
        Self {
            op,
            flags: 0,
            payload,
        }
    }

    pub fn heartbeat() -> Self {
        Self::new(Op::Heartbeat, Vec::new())
    }

    pub fn session_ack(session_id: u64) -> Self {
        Self::new(Op::SessionAck, session_id.to_le_bytes().to_vec())
    }

    pub fn error(code: u16, msg: &str) -> Self {
        let mut payload = Vec::with_capacity(2 + msg.len());
        payload.extend_from_slice(&code.to_le_bytes());
        payload.extend_from_slice(msg.as_bytes());
        Self::new(Op::Error, payload)
    }

    /// Encode to wire format
    pub fn encode(&self) -> Vec<u8> {
        let header = Header::new(self.op, self.payload.len() as u16);
        let mut buf = Vec::with_capacity(Header::SIZE + self.payload.len());
        buf.extend_from_slice(bytemuck::bytes_of(&header));
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Decode from wire format
    pub fn decode(data: &[u8]) -> Option<(Self, usize)> {
        let header = Header::from_bytes(data)?;
        let total = Header::SIZE + header.len as usize;
        if data.len() < total {
            return None;
        }
        let op = Op::try_from(header.op).ok()?;
        let payload = data[Header::SIZE..total].to_vec();
        Some((
            Self {
                op,
                flags: header.flags,
                payload,
            },
            total,
        ))
    }
}

/// Session start payload
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SessionStartPayload {
    pub version: u16,
    pub flags: u16,
}

impl SessionStartPayload {
    pub const SIZE: usize = std::mem::size_of::<Self>();
    pub const CURRENT_VERSION: u16 = 1;

    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            flags: 0,
        }
    }
}

impl Default for SessionStartPayload {
    fn default() -> Self {
        Self::new()
    }
}

/// Room join payload
#[derive(Debug, Clone)]
pub struct RoomJoinPayload {
    pub room_id: String,
    pub metadata: Vec<u8>,
}

impl RoomJoinPayload {
    pub fn encode(&self) -> Vec<u8> {
        let id_bytes = self.room_id.as_bytes();
        let mut buf = Vec::with_capacity(1 + id_bytes.len() + self.metadata.len());
        buf.push(id_bytes.len() as u8);
        buf.extend_from_slice(id_bytes);
        buf.extend_from_slice(&self.metadata);
        buf
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let id_len = data[0] as usize;
        if data.len() < 1 + id_len {
            return None;
        }
        let room_id = String::from_utf8(data[1..1 + id_len].to_vec()).ok()?;
        let metadata = data[1 + id_len..].to_vec();
        Some(Self { room_id, metadata })
    }
}

/// Room data payload (broadcast to peers in room)
#[derive(Debug, Clone)]
pub struct RoomDataPayload {
    pub room_id: String,
    pub sender: u64,
    pub data: Vec<u8>,
}

impl RoomDataPayload {
    pub fn encode(&self) -> Vec<u8> {
        let id_bytes = self.room_id.as_bytes();
        let mut buf = Vec::with_capacity(1 + id_bytes.len() + 8 + self.data.len());
        buf.push(id_bytes.len() as u8);
        buf.extend_from_slice(id_bytes);
        buf.extend_from_slice(&self.sender.to_le_bytes());
        buf.extend_from_slice(&self.data);
        buf
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 10 {
            return None;
        }
        let id_len = data[0] as usize;
        if data.len() < 1 + id_len + 8 {
            return None;
        }
        let room_id = String::from_utf8(data[1..1 + id_len].to_vec()).ok()?;
        let sender = u64::from_le_bytes(data[1 + id_len..1 + id_len + 8].try_into().ok()?);
        let payload_data = data[1 + id_len + 8..].to_vec();
        Some(Self {
            room_id,
            sender,
            data: payload_data,
        })
    }
}

/// RPC payload
#[derive(Debug, Clone)]
pub struct RpcPayload {
    pub id: u32,
    pub method: String,
    pub data: Vec<u8>,
}

impl RpcPayload {
    pub fn encode(&self) -> Vec<u8> {
        let method_bytes = self.method.as_bytes();
        let mut buf = Vec::with_capacity(4 + 1 + method_bytes.len() + self.data.len());
        buf.extend_from_slice(&self.id.to_le_bytes());
        buf.push(method_bytes.len() as u8);
        buf.extend_from_slice(method_bytes);
        buf.extend_from_slice(&self.data);
        buf
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }
        let id = u32::from_le_bytes(data[0..4].try_into().ok()?);
        let method_len = data[4] as usize;
        if data.len() < 5 + method_len {
            return None;
        }
        let method = String::from_utf8(data[5..5 + method_len].to_vec()).ok()?;
        let payload_data = data[5 + method_len..].to_vec();
        Some(Self {
            id,
            method,
            data: payload_data,
        })
    }
}

/// RPC response payload
#[derive(Debug, Clone)]
pub struct RpcResponsePayload {
    pub id: u32,
    pub success: bool,
    pub data: Vec<u8>,
}

impl RpcResponsePayload {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(5 + self.data.len());
        buf.extend_from_slice(&self.id.to_le_bytes());
        buf.push(if self.success { 1 } else { 0 });
        buf.extend_from_slice(&self.data);
        buf
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }
        let id = u32::from_le_bytes(data[0..4].try_into().ok()?);
        let success = data[4] != 0;
        let payload_data = data[5..].to_vec();
        Some(Self {
            id,
            success,
            data: payload_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_roundtrip() {
        let msg = Message::new(Op::Heartbeat, vec![1, 2, 3]);
        let encoded = msg.encode();
        let (decoded, len) = Message::decode(&encoded).unwrap();
        assert_eq!(len, encoded.len());
        assert_eq!(decoded.op, Op::Heartbeat);
        assert_eq!(decoded.payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_room_join_payload() {
        let payload = RoomJoinPayload {
            room_id: "test-room".to_string(),
            metadata: vec![1, 2, 3],
        };
        let encoded = payload.encode();
        let decoded = RoomJoinPayload::decode(&encoded).unwrap();
        assert_eq!(decoded.room_id, "test-room");
        assert_eq!(decoded.metadata, vec![1, 2, 3]);
    }

    #[test]
    fn test_rpc_payload() {
        let payload = RpcPayload {
            id: 42,
            method: "test_method".to_string(),
            data: vec![4, 5, 6],
        };
        let encoded = payload.encode();
        let decoded = RpcPayload::decode(&encoded).unwrap();
        assert_eq!(decoded.id, 42);
        assert_eq!(decoded.method, "test_method");
        assert_eq!(decoded.data, vec![4, 5, 6]);
    }
}
