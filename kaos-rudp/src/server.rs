//! Multi-client Reliable UDP Server
//!
//! A game server transport that handles multiple clients with full reliability:
//! - MessageRingBuffer for per-client send windows
//! - BitmapWindow for per-client receive windows
//! - NAK-based retransmission
//! - Congestion control
//!
//! ```rust,ignore
//! use kaos_rudp::RudpServer;
//!
//! let mut server = RudpServer::bind("0.0.0.0:7777", 256)?;
//!
//! loop {
//!     // Accept new clients
//!     server.poll();
//!
//!     // Process each client
//!     for client_id in server.clients() {
//!         server.receive(client_id, |data| {
//!             println!("Got {} bytes", data.len());
//!         });
//!         server.send(client_id, b"hello")?;
//!     }
//!
//!     // Flush sends and process retransmits
//!     server.flush();
//! }
//! ```

use crate::congestion::CongestionController;
use crate::header::{MessageType, ReliableUdpHeader};
use crate::window::BitmapWindow;
use kaos::disruptor::{MessageRingBuffer, RingBufferConfig, RingBufferEntry};
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Socket buffer size (8MB for high throughput)
const SOCKET_BUFFER_SIZE: i32 = 8 * 1024 * 1024;

/// Default window size per client
const DEFAULT_WINDOW_SIZE: usize = 256;

/// Client timeout (30 seconds)
const DEFAULT_CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Max receive buffer size
const RECV_BUFFER_SIZE: usize = 65536;

/// Per-client reliability state using kaos ring buffers
pub struct ClientState {
    /// Send window (MessageRingBuffer from kaos::disruptor)
    send_window: MessageRingBuffer,
    /// Receive window (BitmapWindow for ordered delivery)
    recv_window: BitmapWindow,
    /// Congestion controller
    congestion: CongestionController,
    /// Next send sequence number
    next_send_seq: u64,
    /// Highest ACKed sequence
    acked_seq: u64,
    /// Last activity time
    last_seen: Instant,
    /// Connection open flag
    open: bool,
    /// Client address
    addr: SocketAddr,
    /// NAK address (port + 1)
    nak_addr: SocketAddr,
    /// Window size
    window_size: usize,
}

impl ClientState {
    fn new(addr: SocketAddr, window_size: usize) -> io::Result<Self> {
        let config = RingBufferConfig::new(window_size)
            .map_err(|e| io::Error::other(format!("Invalid window size: {}", e)))?
            .with_consumers(1)
            .map_err(|e| io::Error::other(format!("Config error: {}", e)))?;

        let send_window = MessageRingBuffer::new(config)
            .map_err(|e| io::Error::other(format!("RingBuffer error: {}", e)))?;

        let nak_addr = SocketAddr::new(addr.ip(), addr.port().wrapping_add(1));

        Ok(Self {
            send_window,
            recv_window: BitmapWindow::new(window_size, 0),
            congestion: CongestionController::new(64, window_size as u32),
            next_send_seq: 0,
            acked_seq: 0,
            last_seen: Instant::now(),
            open: true,
            addr,
            nak_addr,
            window_size,
        })
    }

    fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    fn is_timed_out(&self, timeout: Duration) -> bool {
        self.last_seen.elapsed() > timeout
    }

    /// Get client address
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

/// Multi-client Reliable UDP Server with kaos ring buffers
pub struct RudpServer {
    /// Main data socket
    socket: Arc<UdpSocket>,
    /// NAK/ACK socket (port + 1)
    nak_socket: Arc<UdpSocket>,
    /// Per-client state (addr -> state)
    clients: HashMap<SocketAddr, ClientState>,
    /// Server local address
    local_addr: SocketAddr,
    /// Window size for new clients
    window_size: usize,
    /// Client timeout
    client_timeout: Duration,
    /// Receive buffer (reused)
    recv_buffer: Vec<u8>,
    /// Pending new clients
    pending_accepts: Vec<SocketAddr>,
}

impl RudpServer {
    /// Bind to an address and create a new RUDP server
    pub fn bind<A: ToSocketAddrs>(addr: A, window_size: usize) -> io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        let local_addr = socket.local_addr()?;
        socket.set_nonblocking(true)?;

        // Set large socket buffers
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = socket.as_raw_fd();
            let buffer_size = SOCKET_BUFFER_SIZE;
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

        // NAK socket on port + 1
        let nak_addr = SocketAddr::new(local_addr.ip(), local_addr.port() + 1);
        let nak_socket = UdpSocket::bind(nak_addr)?;
        nak_socket.set_nonblocking(true)?;

        Ok(Self {
            socket: Arc::new(socket),
            nak_socket: Arc::new(nak_socket),
            clients: HashMap::new(),
            local_addr,
            window_size,
            client_timeout: DEFAULT_CLIENT_TIMEOUT,
            recv_buffer: vec![0u8; RECV_BUFFER_SIZE],
            pending_accepts: Vec::new(),
        })
    }

    /// Bind with default window size
    pub fn bind_default<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        Self::bind(addr, DEFAULT_WINDOW_SIZE)
    }

    /// Set client timeout
    pub fn set_client_timeout(&mut self, timeout: Duration) {
        self.client_timeout = timeout;
    }

    /// Get server's local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Get number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Get iterator over client addresses
    pub fn clients(&self) -> impl Iterator<Item = &SocketAddr> {
        self.clients.keys()
    }

    /// Check if a client is connected
    pub fn has_client(&self, addr: &SocketAddr) -> bool {
        self.clients.contains_key(addr)
    }

    /// Accept a pending new client (returns addr if any)
    pub fn accept(&mut self) -> Option<SocketAddr> {
        self.pending_accepts.pop()
    }

    /// Poll for incoming packets and route to clients
    pub fn poll(&mut self) {
        self.poll_data_socket();
        self.poll_nak_socket();
        self.cleanup_timed_out();
    }

    /// Poll data socket for incoming packets
    fn poll_data_socket(&mut self) {
        // Collect packets first to avoid borrow issues
        let mut packets: Vec<(SocketAddr, Vec<u8>)> = Vec::new();

        loop {
            match self.socket.recv_from(&mut self.recv_buffer) {
                Ok((len, src_addr)) => {
                    packets.push((src_addr, self.recv_buffer[..len].to_vec()));
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        // Process packets
        for (src_addr, data) in packets {
            self.handle_packet(src_addr, data);
        }
    }

    /// Handle an incoming packet
    fn handle_packet(&mut self, src_addr: SocketAddr, data: Vec<u8>) {
        // Ensure client exists
        if !self.clients.contains_key(&src_addr) {
            if let Ok(state) = ClientState::new(src_addr, self.window_size) {
                self.clients.insert(src_addr, state);
                self.pending_accepts.push(src_addr);
            } else {
                return;
            }
        }

        let client = self.clients.get_mut(&src_addr).unwrap();
        client.touch();

        // Parse kaos-rudp header
        if data.len() < ReliableUdpHeader::SIZE {
            // Too small for header, treat as raw data for compatibility
            client.recv_window.insert(client.recv_window.last_delivered_seq() + 1, &data);
            return;
        }

        if let Some((header, payload)) = ReliableUdpHeader::from_packet_with_payload_check(&data) {
            match header.msg_type {
                t if t == MessageType::Data as u8 => {
                    // Verify checksum
                    if header.verify_checksum(payload) {
                        // Insert into receive window (handles ordering)
                        client.recv_window.insert(header.sequence, payload);

                        // Send ACK for received sequence
                        self.send_ack_to(src_addr, header.sequence);
                    }
                }
                t if t == MessageType::Ack as u8 => {
                    // ACK received - advance send window
                    let acked_seq = header.sequence;
                    if acked_seq > client.acked_seq {
                        let newly_acked = acked_seq - client.acked_seq;
                        for _ in 0..newly_acked {
                            client.congestion.on_ack();
                        }
                        client.acked_seq = acked_seq;
                        client.send_window.advance_consumer(0, acked_seq);
                    }
                }
                t if t == MessageType::Nak as u8 => {
                    // NAK received - retransmit requested packets
                    client.congestion.on_loss();
                    self.retransmit_for_client(src_addr, header.sequence);
                }
                _ => {}
            }
        }
    }

    /// Poll NAK socket for ACK/NAK messages
    fn poll_nak_socket(&mut self) {
        let mut buf = [0u8; 256];
        let mut ack_events: Vec<(SocketAddr, u64)> = Vec::new();
        let mut nak_events: Vec<(SocketAddr, u64, u64)> = Vec::new();

        loop {
            match self.nak_socket.recv_from(&mut buf) {
                Ok((len, src_addr)) => {
                    if len < ReliableUdpHeader::SIZE {
                        continue;
                    }

                    // Map NAK addr back to client addr (port - 1)
                    let client_addr = SocketAddr::new(src_addr.ip(), src_addr.port().wrapping_sub(1));

                    if let Some((header, payload)) =
                        ReliableUdpHeader::from_packet_with_payload_check(&buf[..len])
                    {
                        let seq = header.sequence;
                        match header.msg_type {
                            t if t == MessageType::Ack as u8 => {
                                ack_events.push((client_addr, seq));
                            }
                            t if t == MessageType::Nak as u8 => {
                                // Parse batch NAK payload
                                if payload.len() >= 16 {
                                    let start_seq = u64::from_le_bytes(
                                        payload[0..8].try_into().unwrap(),
                                    );
                                    let end_seq = u64::from_le_bytes(
                                        payload[8..16].try_into().unwrap(),
                                    );
                                    nak_events.push((client_addr, start_seq, end_seq));
                                } else {
                                    nak_events.push((client_addr, header.sequence, header.sequence));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        // Process ACK events
        for (client_addr, acked_seq) in ack_events {
            if let Some(client) = self.clients.get_mut(&client_addr) {
                // Update last_seen on ACK to prevent timeout
                client.touch();
                if acked_seq > client.acked_seq {
                    let newly_acked = acked_seq - client.acked_seq;
                    for _ in 0..newly_acked {
                        client.congestion.on_ack();
                    }
                    client.acked_seq = acked_seq;
                    client.send_window.advance_consumer(0, acked_seq);
                }
            }
        }

        // Process NAK events
        for (client_addr, start_seq, end_seq) in nak_events {
            if let Some(client) = self.clients.get_mut(&client_addr) {
                client.congestion.on_loss();
            }
            self.retransmit_range_for_client(client_addr, start_seq, end_seq);
        }
    }

    /// Send ACK to a client
    fn send_ack_to(&self, client_addr: SocketAddr, seq: u64) {
        let mut header = ReliableUdpHeader::new(0, seq, MessageType::Ack, 0);
        header.calculate_checksum(&[]);
        let packet = bytemuck::bytes_of(&header);

        let nak_addr = SocketAddr::new(client_addr.ip(), client_addr.port().wrapping_add(1));
        let _ = self.nak_socket.send_to(packet, nak_addr);
    }

    /// Retransmit a single packet for a client
    fn retransmit_for_client(&self, client_addr: SocketAddr, seq: u64) {
        if let Some(client) = self.clients.get(&client_addr) {
            let slots = client.send_window.peek_batch(0, client.window_size);
            if let Some(slot) = slots.iter().find(|s| s.sequence() == seq) {
                let pkt_data = slot.data();
                if !pkt_data.is_empty() {
                    let _ = self.socket.send_to(pkt_data, client_addr);
                }
            }
        }
    }

    /// Retransmit a range of packets for a client
    fn retransmit_range_for_client(&self, client_addr: SocketAddr, start_seq: u64, end_seq: u64) {
        if let Some(client) = self.clients.get(&client_addr) {
            let slots = client.send_window.peek_batch(0, client.window_size);
            for slot in slots.iter().filter(|s| {
                let seq = s.sequence();
                seq >= start_seq && seq <= end_seq
            }) {
                let pkt_data = slot.data();
                if !pkt_data.is_empty() {
                    let _ = self.socket.send_to(pkt_data, client_addr);
                }
            }
        }
    }

    /// Cleanup timed out clients
    fn cleanup_timed_out(&mut self) {
        let timeout = self.client_timeout;
        self.clients.retain(|_, client| !client.is_timed_out(timeout));
    }

    /// Send data to a client
    pub fn send(&mut self, client_addr: &SocketAddr, data: &[u8]) -> io::Result<u64> {
        let client = self.clients.get_mut(client_addr)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "Client not found"))?;

        if !client.open {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "Client disconnected"));
        }

        // Check congestion window
        if !client.congestion.can_send() {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "Congestion window full"));
        }

        let seq = client.next_send_seq;
        let mut header = ReliableUdpHeader::new(0, seq, MessageType::Data, data.len() as u16);
        header.calculate_checksum(data);

        // Build packet
        let mut packet = Vec::with_capacity(ReliableUdpHeader::SIZE + data.len());
        packet.extend_from_slice(bytemuck::bytes_of(&header));
        packet.extend_from_slice(data);

        // Store in send window for retransmission
        if let Some((slot_seq, slots)) = client.send_window.try_claim_slots(1) {
            slots[0].set_sequence(slot_seq);
            slots[0].set_data(&packet);
            client.send_window.publish_batch(slot_seq, 1);
        } else {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "Send window full"));
        }

        // Send immediately
        self.socket.send_to(&packet, *client_addr)?;
        client.congestion.on_send();
        client.next_send_seq = seq.wrapping_add(1);

        Ok(seq)
    }

    /// Receive data from a client (delivers in-order)
    pub fn receive<F: FnMut(&[u8])>(&mut self, client_addr: &SocketAddr, mut handler: F) -> usize {
        let mut count = 0;
        if let Some(client) = self.clients.get_mut(client_addr) {
            client.recv_window.deliver_in_order_with(|data| {
                handler(data);
                count += 1;
            });

            // Send NAKs for any gaps
            let nak_socket = self.nak_socket.clone();
            let nak_addr = client.nak_addr;
            client.recv_window.send_batch_naks_for_gaps(|start, end| {
                let mut packet = Vec::with_capacity(ReliableUdpHeader::SIZE + 16);
                let payload = [start.to_le_bytes(), end.to_le_bytes()].concat();
                let mut header = ReliableUdpHeader::new(0, start, MessageType::Nak, 16);
                header.calculate_checksum(&payload);
                packet.extend_from_slice(bytemuck::bytes_of(&header));
                packet.extend_from_slice(&payload);
                let _ = nak_socket.send_to(&packet, nak_addr);
            });
        }
        count
    }

    /// Broadcast data to all clients
    pub fn broadcast(&mut self, data: &[u8]) -> usize {
        let addrs: Vec<SocketAddr> = self.clients.keys().copied().collect();
        let mut sent = 0;
        for addr in addrs {
            if self.send(&addr, data).is_ok() {
                sent += 1;
            }
        }
        sent
    }

    /// Disconnect a client
    pub fn disconnect(&mut self, client_addr: &SocketAddr) {
        if let Some(client) = self.clients.get_mut(client_addr) {
            client.open = false;
        }
        self.clients.remove(client_addr);
    }

    /// Get client's congestion window size
    pub fn client_congestion_window(&self, client_addr: &SocketAddr) -> Option<u32> {
        self.clients.get(client_addr).map(|c| c.congestion.window_size())
    }

    /// Get client's packets in flight
    pub fn client_in_flight(&self, client_addr: &SocketAddr) -> Option<u32> {
        self.clients.get(client_addr).map(|c| c.congestion.in_flight())
    }

    /// Get underlying socket (for advanced use)
    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }
}

/// Client handle for easier per-client operations
pub struct RudpServerClient {
    addr: SocketAddr,
    recv_queue: Vec<Vec<u8>>,
}

impl RudpServerClient {
    /// Create a new client handle
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            recv_queue: Vec::new(),
        }
    }

    /// Get client address
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get peer address (same as addr)
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        Some(self.addr)
    }

    /// Push received data to the queue
    pub fn push_recv(&mut self, data: Vec<u8>) {
        self.recv_queue.push(data);
    }

    /// Pop received data from the queue
    pub fn pop_recv(&mut self) -> Option<Vec<u8>> {
        if self.recv_queue.is_empty() {
            None
        } else {
            Some(self.recv_queue.remove(0))
        }
    }

    /// Check if there's pending received data
    pub fn has_pending(&self) -> bool {
        !self.recv_queue.is_empty()
    }

    /// Get number of pending messages
    pub fn pending_count(&self) -> usize {
        self.recv_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_bind() {
        let server = RudpServer::bind_default("127.0.0.1:0").unwrap();
        assert_eq!(server.client_count(), 0);
        let addr = server.local_addr();
        assert!(addr.port() > 0);
    }

    #[test]
    fn test_client_state_creation() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let state = ClientState::new(addr, 64).unwrap();
        assert!(state.open);
        assert_eq!(state.next_send_seq, 0);
        assert_eq!(state.acked_seq, 0);
    }

    #[test]
    fn test_server_poll_empty() {
        let mut server = RudpServer::bind_default("127.0.0.1:0").unwrap();
        server.poll(); // Should not panic
        assert_eq!(server.client_count(), 0);
    }

    #[test]
    fn test_client_timeout() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mut state = ClientState::new(addr, 64).unwrap();

        // Immediately after creation, should not be timed out
        assert!(!state.is_timed_out(Duration::from_secs(30)));

        // With very short timeout, should be timed out
        std::thread::sleep(Duration::from_millis(10));
        assert!(state.is_timed_out(Duration::from_millis(1)));
    }

    #[test]
    fn test_send_to_unknown_client() {
        let mut server = RudpServer::bind_default("127.0.0.1:0").unwrap();
        let unknown_addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();

        let result = server.send(&unknown_addr, b"test");
        assert!(result.is_err());
    }
}
