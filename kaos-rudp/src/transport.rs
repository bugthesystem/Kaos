//! Composable transport trait
//!
//! Enables layered transports: `Archived<Reliable<Udp>>`, `Driver<Reliable<Udp>>`, etc.

use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

/// Socket buffer size for client transports (4MB for high throughput)
const CLIENT_SOCKET_BUFFER_SIZE: i32 = 4 * 1024 * 1024;

use kaos_shared::{MessageType, PacketHeader, HEADER_SIZE, MUX_KEY_SIZE};

/// Core transport trait - all transports implement this
pub trait Transport {
    /// Send data, returns sequence number
    fn send(&mut self, data: &[u8]) -> io::Result<u64>;

    /// Receive into callback, returns count received
    fn receive<F: FnMut(&[u8])>(&mut self, handler: F) -> usize;

    /// Flush pending operations
    fn flush(&mut self) {}
}

/// Batch sending (optional optimization)
pub trait BatchTransport: Transport {
    fn send_batch(&mut self, data: &[&[u8]]) -> io::Result<usize>;
}

/// Reliability layer (NAK/ACK retransmission)
pub trait Reliable: Transport {
    fn retransmit_pending(&mut self) -> io::Result<usize>;
    fn acked_sequence(&self) -> u64;
}

/// Archiving layer (disk persistence)
pub trait Archived: Transport {
    fn archived_count(&self) -> u64;
    fn wait_for_archive(&self);
}

/// Configuration for ClientTransport
#[derive(Debug, Clone)]
pub struct ClientTransportConfig {
    /// Local address to bind to (0.0.0.0:0 = auto-assign)
    pub bind_addr: SocketAddr,
    /// Remote peer address
    pub peer_addr: SocketAddr,
    /// Read timeout for socket operations
    pub read_timeout: Option<Duration>,
    /// Write timeout for socket operations
    pub write_timeout: Option<Duration>,
    /// Mux key for multiplexed servers (4 bytes prefix on each packet)
    /// If None, no prefix is added (legacy mode)
    pub mux_key: Option<u32>,
}

impl Default for ClientTransportConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:0".parse().unwrap(),
            peer_addr: "127.0.0.1:7354".parse().unwrap(),
            read_timeout: Some(Duration::from_millis(100)),
            write_timeout: Some(Duration::from_millis(1000)),
            mux_key: None,
        }
    }
}

/// Client-side RUDP transport with optional multiplexing support.
///
/// This is a lightweight transport for game clients that handles:
/// - UDP socket management
/// - Packet header serialization/deserialization
/// - Mux key prefixing for multiplexed servers (u32)
/// - Sequence number tracking
///
/// This is different from the server-side `RudpTransport` which uses
/// ring buffers for high-performance server operations.
///
/// # Example
///
/// ```rust,no_run
/// use kaos_rudp::{ClientTransport, ClientTransportConfig, Transport};
/// use std::net::SocketAddr;
///
/// let peer: SocketAddr = "127.0.0.1:7354".parse().unwrap();
/// let mut transport = ClientTransport::connect(peer).unwrap();
///
/// // Send data (via Transport trait)
/// transport.send(b"hello").unwrap();
///
/// // Receive data (via Transport trait)
/// transport.receive(|data| {
///     println!("Received: {:?}", data);
/// });
/// ```
pub struct ClientTransport {
    socket: UdpSocket,
    /// NAK/ACK socket (local port + 1) for control messages
    #[allow(dead_code)]
    nak_socket: UdpSocket,
    peer_addr: SocketAddr,
    /// Peer's NAK address (peer port + 1)
    #[allow(dead_code)]
    peer_nak_addr: SocketAddr,
    sequence: u64,
    recv_buffer: Vec<u8>,
    /// Mux key for multiplexed servers (prepended to each packet)
    mux_key: Option<u32>,
    /// Connection state
    connected: bool,
}

impl ClientTransport {
    /// Connect to a peer (legacy mode without mux_key)
    pub fn connect(peer_addr: SocketAddr) -> io::Result<Self> {
        Self::connect_with_config(ClientTransportConfig {
            peer_addr,
            ..Default::default()
        })
    }

    /// Connect to a multiplexed peer with a mux_key (u32)
    ///
    /// The mux_key is a 4-byte prefix added to every packet sent.
    /// This allows multiple games/services to share the same UDP port.
    pub fn connect_mux(peer_addr: SocketAddr, mux_key: u32) -> io::Result<Self> {
        Self::connect_with_config(ClientTransportConfig {
            peer_addr,
            mux_key: Some(mux_key),
            ..Default::default()
        })
    }

    /// Connect with custom configuration
    pub fn connect_with_config(mut config: ClientTransportConfig) -> io::Result<Self> {
        // Ensure bind_addr uses the same IP version as peer_addr (IPv4/IPv6 matching)
        if config.bind_addr.ip().is_unspecified() {
            config.bind_addr = match config.peer_addr.ip() {
                IpAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
                IpAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
            };
        }

        eprintln!("[RUDP] Server address: {}", config.peer_addr);
        eprintln!("[RUDP] Binding main socket to: {}", config.bind_addr);

        let socket = UdpSocket::bind(config.bind_addr).map_err(|e| {
            eprintln!("[RUDP] Main socket bind FAILED: {}", e);
            e
        })?;
        socket.set_nonblocking(true)?;
        socket.set_read_timeout(config.read_timeout)?;
        socket.set_write_timeout(config.write_timeout)?;

        // Set large socket buffers for high throughput
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = socket.as_raw_fd();
            let buffer_size = CLIENT_SOCKET_BUFFER_SIZE;
            unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_SNDBUF,
                    &buffer_size as *const i32 as *const libc::c_void,
                    std::mem::size_of::<i32>() as u32,
                );
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_RCVBUF,
                    &buffer_size as *const i32 as *const libc::c_void,
                    std::mem::size_of::<i32>() as u32,
                );
            }
        }

        // Create NAK socket on local port + 1 for control messages
        // If port+1 is in use, let OS pick an available port (fallback)
        let local_addr = socket.local_addr()?;
        eprintln!("[RUDP] Main socket bound to: {}", local_addr);
        let nak_bind_addr = SocketAddr::new(local_addr.ip(), local_addr.port().wrapping_add(1));
        eprintln!("[RUDP] Binding NAK socket to: {}", nak_bind_addr);

        let nak_socket = match UdpSocket::bind(nak_bind_addr) {
            Ok(s) => {
                eprintln!("[RUDP] NAK socket bound successfully");
                s
            }
            Err(e) => {
                eprintln!("[RUDP] NAK socket bind failed ({}), trying fallback...", e);
                // Port+1 in use, let OS pick any available port
                let fallback_addr = SocketAddr::new(local_addr.ip(), 0);
                UdpSocket::bind(fallback_addr).map_err(|e2| {
                    eprintln!("[RUDP] Fallback NAK socket bind FAILED: {}", e2);
                    e2
                })?
            }
        };
        let nak_local = nak_socket.local_addr()?;
        eprintln!("[RUDP] NAK socket bound to: {}", nak_local);
        nak_socket.set_nonblocking(true)?;

        // Peer's NAK address is peer port + 1
        let peer_nak_addr = SocketAddr::new(
            config.peer_addr.ip(),
            config.peer_addr.port().wrapping_add(1),
        );

        let mut transport = Self {
            socket,
            nak_socket,
            peer_addr: config.peer_addr,
            peer_nak_addr,
            sequence: 0,
            recv_buffer: vec![0u8; 65536],
            mux_key: config.mux_key,
            connected: false,
        };

        // Send handshake
        transport.send_handshake()?;
        transport.connected = true;

        Ok(transport)
    }

    /// Send handshake packet to initiate connection
    fn send_handshake(&mut self) -> io::Result<()> {
        let header = PacketHeader::new(self.sequence, MessageType::Handshake, 0);
        self.sequence += 1;
        let header_bytes = header.to_bytes();

        let packet = self.prepend_mux_key(&header_bytes);
        eprintln!("[RUDP] Sending handshake to: {}", self.peer_addr);
        match self.socket.send_to(&packet, self.peer_addr) {
            Ok(n) => {
                eprintln!("[RUDP] Handshake sent ({} bytes)", n);
                Ok(())
            }
            Err(e) => {
                eprintln!("[RUDP] Handshake send_to FAILED: {}", e);
                Err(e)
            }
        }
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Send raw data with RUDP header
    pub fn send_raw(&mut self, data: &[u8]) -> io::Result<usize> {
        let mut header = PacketHeader::new(self.sequence, MessageType::Data, data.len());
        header.calculate_checksum(data);
        self.sequence += 1;

        let mut packet = self.create_packet_buffer(HEADER_SIZE + data.len());
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(data);

        self.socket.send_to(&packet, self.peer_addr)
    }

    /// Send unreliable data (fire and forget, no sequence tracking)
    pub fn send_unreliable(&self, data: &[u8]) -> io::Result<usize> {
        let mut header = PacketHeader::new(0, MessageType::Data, data.len());
        header.flags = 0x01; // Unreliable flag
        header.calculate_checksum(data);

        let mut packet = Vec::with_capacity(self.mux_prefix_len() + HEADER_SIZE + data.len());
        if let Some(mux_key) = self.mux_key {
            packet.extend_from_slice(&mux_key.to_le_bytes());
        }
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(data);

        self.socket.send_to(&packet, self.peer_addr)
    }

    /// Send an ACK to the peer for a received sequence
    fn send_ack(&self, seq: u64) {
        let mut header = PacketHeader::new(seq, MessageType::Ack, 0);
        header.calculate_checksum(&[]);
        let header_bytes = header.to_bytes();

        let packet = self.prepend_mux_key(&header_bytes);
        let _ = self.socket.send_to(&packet, self.peer_addr);
    }

    /// Send a ping (keep-alive)
    pub fn ping(&mut self) -> io::Result<()> {
        let mut header = PacketHeader::new(self.sequence, MessageType::Ping, 0);
        header.timestamp = (Instant::now().elapsed().as_millis() & 0xFFFFFFFF) as u32;
        self.sequence += 1;

        let packet = self.prepend_mux_key(&header.to_bytes());
        self.socket.send_to(&packet, self.peer_addr)?;
        Ok(())
    }

    /// Send disconnect packet
    pub fn disconnect(&mut self) -> io::Result<()> {
        let header = PacketHeader::new(self.sequence, MessageType::Disconnect, 0);
        let packet = self.prepend_mux_key(&header.to_bytes());
        self.socket.send_to(&packet, self.peer_addr)?;
        Ok(())
    }

    /// Get the mux_key if using multiplexed mode
    pub fn mux_key(&self) -> Option<u32> {
        self.mux_key
    }

    /// Get local address
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Get peer address
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Get current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    // Helper: calculate mux prefix length
    #[inline]
    fn mux_prefix_len(&self) -> usize {
        if self.mux_key.is_some() {
            MUX_KEY_SIZE
        } else {
            0
        }
    }

    // Helper: prepend mux_key to data if in mux mode
    fn prepend_mux_key(&self, data: &[u8]) -> Vec<u8> {
        if let Some(mux_key) = self.mux_key {
            let mut buf = Vec::with_capacity(MUX_KEY_SIZE + data.len());
            buf.extend_from_slice(&mux_key.to_le_bytes());
            buf.extend_from_slice(data);
            buf
        } else {
            data.to_vec()
        }
    }

    // Helper: create packet buffer with mux_key prefix space
    fn create_packet_buffer(&self, payload_size: usize) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.mux_prefix_len() + payload_size);
        if let Some(mux_key) = self.mux_key {
            buf.extend_from_slice(&mux_key.to_le_bytes());
        }
        buf
    }
}

impl Transport for ClientTransport {
    fn send(&mut self, data: &[u8]) -> io::Result<u64> {
        let seq = self.sequence;
        self.send_raw(data)?;
        Ok(seq)
    }

    fn receive<F: FnMut(&[u8])>(&mut self, mut handler: F) -> usize {
        let mut count = 0;
        let prefix_len = self.mux_prefix_len();
        let min_len = prefix_len + HEADER_SIZE;

        loop {
            match self.socket.recv_from(&mut self.recv_buffer) {
                Ok((len, _addr)) => {
                    if len < min_len {
                        continue;
                    }

                    // Skip mux_key prefix if in mux mode
                    let data = &self.recv_buffer[prefix_len..len];

                    if let Some(header) = PacketHeader::from_bytes(data) {
                        let msg_type = MessageType::from_u8_lossy(header.msg_type);
                        let seq = { header.sequence };

                        match msg_type {
                            MessageType::Data => {
                                let payload = &data[HEADER_SIZE..];
                                handler(payload);
                                count += 1;
                                // Send ACK back
                                self.send_ack(seq);
                            }
                            MessageType::Ping | MessageType::Pong => {
                                // Heartbeat (keep-alive)
                            }
                            MessageType::Ack => {
                                // ACK received
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
}

impl Drop for ClientTransport {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = ClientTransportConfig::default();
        assert_eq!(config.mux_key, None);
        assert_eq!(config.read_timeout, Some(Duration::from_millis(100)));
    }

    #[test]
    fn test_mux_key_config() {
        let config = ClientTransportConfig {
            mux_key: Some(0x12345678),
            ..Default::default()
        };
        assert_eq!(config.mux_key, Some(0x12345678));
    }
}
