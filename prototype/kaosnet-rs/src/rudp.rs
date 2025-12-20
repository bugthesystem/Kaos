//! RUDP (Reliable UDP) client for low-latency real-time communication.
//!
//! This module provides a synchronous RUDP client that can be used alongside
//! the async WebSocket connection for game-specific low-latency data.
//!
//! RUDP is better for:
//! - Game state updates (position, rotation, velocity)
//! - Time-critical actions (shooting, dodging)
//! - High-frequency updates (60+ times per second)
//!
//! WebSocket is better for:
//! - Chat messages
//! - Matchmaking
//! - Reliable RPCs
//! - Less frequent updates

use serde::{Deserialize, Serialize};
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

/// RUDP packet header size (matches kaos-rudp)
const HEADER_SIZE: usize = 24;

/// RUDP message types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Data = 0,
    Ack = 1,
    Nak = 2,
    Ping = 3,
    Pong = 4,
    Handshake = 5,
    Disconnect = 6,
}

impl From<u8> for MessageType {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Data,
            1 => Self::Ack,
            2 => Self::Nak,
            3 => Self::Ping,
            4 => Self::Pong,
            5 => Self::Handshake,
            6 => Self::Disconnect,
            _ => Self::Data,
        }
    }
}

/// RUDP packet header (matches kaos-rudp ReliableUdpHeader)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    pub version: u8,
    pub msg_type: u8,
    pub flags: u8,
    pub reserved: u8,
    pub stream_id: u32,
    pub sequence: u64,
    pub payload_len: u16,
    pub checksum: u16,
    pub timestamp: u32,
}

impl PacketHeader {
    /// Create a new packet header
    pub fn new(seq: u64, msg_type: MessageType, payload_len: usize) -> Self {
        Self {
            version: 1,
            msg_type: msg_type as u8,
            flags: 0,
            reserved: 0,
            stream_id: 0,
            sequence: seq,
            payload_len: payload_len as u16,
            checksum: 0,
            timestamp: 0,
        }
    }

    /// Calculate checksum for payload
    pub fn calculate_checksum(&mut self, payload: &[u8]) {
        let mut sum: u32 = 0;
        for byte in payload {
            sum = sum.wrapping_add(*byte as u32);
        }
        self.checksum = (sum & 0xFFFF) as u16;
    }

    /// Convert header to bytes
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0] = self.version;
        buf[1] = self.msg_type;
        buf[2] = self.flags;
        buf[3] = self.reserved;
        buf[4..8].copy_from_slice(&self.stream_id.to_le_bytes());
        buf[8..16].copy_from_slice(&self.sequence.to_le_bytes());
        buf[16..18].copy_from_slice(&self.payload_len.to_le_bytes());
        buf[18..20].copy_from_slice(&self.checksum.to_le_bytes());
        buf[20..24].copy_from_slice(&self.timestamp.to_le_bytes());
        buf
    }

    /// Parse header from bytes
    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < HEADER_SIZE {
            return None;
        }
        Some(Self {
            version: buf[0],
            msg_type: buf[1],
            flags: buf[2],
            reserved: buf[3],
            stream_id: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
            sequence: u64::from_le_bytes([buf[8], buf[9], buf[10], buf[11], buf[12], buf[13], buf[14], buf[15]]),
            payload_len: u16::from_le_bytes([buf[16], buf[17]]),
            checksum: u16::from_le_bytes([buf[18], buf[19]]),
            timestamp: u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]),
        })
    }
}

/// Configuration for RUDP client
#[derive(Debug, Clone)]
pub struct RudpConfig {
    /// Local address to bind to (0 = auto-assign port)
    pub bind_addr: SocketAddr,
    /// Server address to connect to
    pub server_addr: SocketAddr,
    /// Send window size (number of packets)
    pub window_size: usize,
    /// Read timeout
    pub read_timeout: Option<Duration>,
    /// Write timeout
    pub write_timeout: Option<Duration>,
}

impl Default for RudpConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:0".parse().unwrap(),
            server_addr: "127.0.0.1:7351".parse().unwrap(),
            window_size: 256,
            read_timeout: Some(Duration::from_millis(100)),
            write_timeout: Some(Duration::from_millis(1000)),
        }
    }
}

/// RUDP client for low-latency game communication.
///
/// # Example
///
/// ```rust,no_run
/// use kaosnet_rs::RudpClient;
/// use std::net::SocketAddr;
///
/// fn main() -> std::io::Result<()> {
///     let server: SocketAddr = "127.0.0.1:7351".parse().unwrap();
///     let mut client = RudpClient::connect(server)?;
///
///     // Send match data
///     client.send(1, b"player position update")?;
///
///     // Receive data
///     client.receive(|op_code, data| {
///         println!("Received op={}, data len={}", op_code, data.len());
///     });
///
///     Ok(())
/// }
/// ```
pub struct RudpClient {
    socket: UdpSocket,
    server_addr: SocketAddr,
    sequence: u64,
    recv_buffer: Vec<u8>,
    connected: bool,
}

impl RudpClient {
    /// Connect to a RUDP server
    pub fn connect(server_addr: SocketAddr) -> io::Result<Self> {
        Self::connect_with_config(RudpConfig {
            server_addr,
            ..Default::default()
        })
    }

    /// Connect with custom configuration
    pub fn connect_with_config(config: RudpConfig) -> io::Result<Self> {
        let socket = UdpSocket::bind(config.bind_addr)?;
        socket.set_nonblocking(true)?;
        socket.set_read_timeout(config.read_timeout)?;
        socket.set_write_timeout(config.write_timeout)?;

        let mut client = Self {
            socket,
            server_addr: config.server_addr,
            sequence: 0,
            recv_buffer: vec![0u8; 65536],
            connected: false,
        };

        // Send handshake
        client.send_handshake()?;
        client.connected = true;

        Ok(client)
    }

    /// Send handshake packet
    fn send_handshake(&mut self) -> io::Result<()> {
        let header = PacketHeader::new(self.sequence, MessageType::Handshake, 0);
        self.sequence += 1;
        let packet = header.to_bytes();
        self.socket.send_to(&packet, self.server_addr)?;
        Ok(())
    }

    /// Send data with an op code
    pub fn send(&mut self, _op_code: i64, data: &[u8]) -> io::Result<usize> {
        self.send_raw(data)
    }

    /// Send raw data (without op code wrapper)
    pub fn send_raw(&mut self, data: &[u8]) -> io::Result<usize> {
        let mut header = PacketHeader::new(self.sequence, MessageType::Data, data.len());
        header.calculate_checksum(data);
        self.sequence += 1;

        let mut packet = Vec::with_capacity(HEADER_SIZE + data.len());
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(data);

        self.socket.send_to(&packet, self.server_addr)
    }

    /// Send unreliable data (fire and forget, no sequence tracking)
    pub fn send_unreliable(&self, data: &[u8]) -> io::Result<usize> {
        let mut header = PacketHeader::new(0, MessageType::Data, data.len());
        header.flags = 0x01; // Unreliable flag
        header.calculate_checksum(data);

        let mut packet = Vec::with_capacity(HEADER_SIZE + data.len());
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(data);

        self.socket.send_to(&packet, self.server_addr)
    }

    /// Receive data, calling handler for each received message
    pub fn receive<F: FnMut(i64, &[u8])>(&mut self, mut handler: F) -> usize {
        let mut count = 0;

        loop {
            match self.socket.recv_from(&mut self.recv_buffer) {
                Ok((len, _addr)) => {
                    if len < HEADER_SIZE {
                        continue;
                    }

                    if let Some(header) = PacketHeader::from_bytes(&self.recv_buffer[..len]) {
                        let msg_type = MessageType::from(header.msg_type);

                        match msg_type {
                            MessageType::Data => {
                                let payload = &self.recv_buffer[HEADER_SIZE..len];
                                // Extract op code if present (first 8 bytes as i64)
                                if payload.len() >= 8 {
                                    let op_code = i64::from_le_bytes([
                                        payload[0], payload[1], payload[2], payload[3],
                                        payload[4], payload[5], payload[6], payload[7],
                                    ]);
                                    handler(op_code, &payload[8..]);
                                } else {
                                    handler(0, payload);
                                }
                                count += 1;
                            }
                            MessageType::Pong => {
                                // Ping response received
                            }
                            _ => {}
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        count
    }

    /// Send a ping to measure RTT
    pub fn ping(&mut self) -> io::Result<()> {
        let mut header = PacketHeader::new(self.sequence, MessageType::Ping, 0);
        header.timestamp = (Instant::now().elapsed().as_millis() & 0xFFFFFFFF) as u32;
        self.sequence += 1;

        let packet = header.to_bytes();
        self.socket.send_to(&packet, self.server_addr)?;
        Ok(())
    }

    /// Disconnect from the server
    pub fn disconnect(&mut self) -> io::Result<()> {
        if self.connected {
            let header = PacketHeader::new(self.sequence, MessageType::Disconnect, 0);
            let packet = header.to_bytes();
            self.socket.send_to(&packet, self.server_addr)?;
            self.connected = false;
        }
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get local address
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Get server address
    pub fn server_addr(&self) -> SocketAddr {
        self.server_addr
    }
}

impl Drop for RudpClient {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

/// Match data for sending game state over RUDP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDataPacket {
    pub match_id: String,
    pub op_code: i64,
    pub data: Vec<u8>,
}

impl MatchDataPacket {
    /// Create a new match data packet
    pub fn new(match_id: impl Into<String>, op_code: i64, data: Vec<u8>) -> Self {
        Self {
            match_id: match_id.into(),
            op_code,
            data,
        }
    }

    /// Encode to bytes for sending
    pub fn encode(&self) -> Vec<u8> {
        // Simple encoding: match_id length (2 bytes) + match_id + op_code (8 bytes) + data
        let mut buf = Vec::with_capacity(2 + self.match_id.len() + 8 + self.data.len());
        let match_id_bytes = self.match_id.as_bytes();
        buf.extend_from_slice(&(match_id_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(match_id_bytes);
        buf.extend_from_slice(&self.op_code.to_le_bytes());
        buf.extend_from_slice(&self.data);
        buf
    }

    /// Decode from bytes
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 10 {
            return None;
        }
        let match_id_len = u16::from_le_bytes([data[0], data[1]]) as usize;
        if data.len() < 2 + match_id_len + 8 {
            return None;
        }
        let match_id = String::from_utf8(data[2..2 + match_id_len].to_vec()).ok()?;
        let op_code = i64::from_le_bytes([
            data[2 + match_id_len], data[3 + match_id_len], data[4 + match_id_len], data[5 + match_id_len],
            data[6 + match_id_len], data[7 + match_id_len], data[8 + match_id_len], data[9 + match_id_len],
        ]);
        let payload = data[10 + match_id_len..].to_vec();
        Some(Self {
            match_id,
            op_code,
            data: payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_header_roundtrip() {
        let mut header = PacketHeader::new(42, MessageType::Data, 100);
        header.calculate_checksum(b"test data");

        let bytes = header.to_bytes();
        let parsed = PacketHeader::from_bytes(&bytes).unwrap();

        // Use block expressions to copy packed fields to avoid unaligned references
        assert_eq!({ parsed.version }, 1);
        assert_eq!({ parsed.msg_type }, MessageType::Data as u8);
        assert_eq!({ parsed.sequence }, 42);
        assert_eq!({ parsed.payload_len }, 100);
    }

    #[test]
    fn test_match_data_packet_roundtrip() {
        let packet = MatchDataPacket::new("match-123", 42, b"player position".to_vec());
        let encoded = packet.encode();
        let decoded = MatchDataPacket::decode(&encoded).unwrap();

        assert_eq!(decoded.match_id, "match-123");
        assert_eq!(decoded.op_code, 42);
        assert_eq!(decoded.data, b"player position");
    }
}
