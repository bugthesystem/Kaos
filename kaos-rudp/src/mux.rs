//! Mux-Key Multiplexed RUDP Server
//!
//! Routes UDP packets to different game handlers based on a mux_key prefix (u32).
//! This allows multiple games/services to share a single UDP port.
//!
//! ## Protocol
//!
//! Each packet has a 4-byte mux_key prefix:
//! ```text
//! +----------+------------------+
//! | mux_key  |    payload       |
//! | (4 bytes)| (RUDP protocol)  |
//! +----------+------------------+
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use kaos_rudp::{MuxRudpServer, GameHandler};
//!
//! // Define game handlers
//! struct AsteroidsHandler;
//! impl GameHandler for AsteroidsHandler {
//!     fn on_connect(&mut self, client: SocketAddr) { }
//!     fn on_message(&mut self, client: SocketAddr, data: &[u8]) { }
//!     fn on_disconnect(&mut self, client: SocketAddr) { }
//! }
//!
//! let mut server = MuxRudpServer::bind("0.0.0.0:7354")?;
//! server.register(0x00000001, Box::new(AsteroidsHandler));
//! server.register(0x00000002, Box::new(RacingHandler));
//!
//! loop {
//!     server.poll();
//!     // Game logic...
//! }
//! ```

use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::congestion::CongestionController;
use crate::header::{MessageType, ReliableUdpHeader};
use crate::window::BitmapWindow;
use kaos::disruptor::{MessageRingBuffer, RingBufferConfig, RingBufferEntry};

/// Socket buffer size (2MB for reasonable throughput)
const SOCKET_BUFFER_SIZE: i32 = 2 * 1024 * 1024;

/// Default window size per client
const DEFAULT_WINDOW_SIZE: usize = 256;

/// Client timeout (30 seconds)
const DEFAULT_CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Max receive buffer size (8KB - handles most game packets, larger need fragmentation)
/// UDP max is 65535 but typical game packets are under MTU (1500 bytes)
const RECV_BUFFER_SIZE: usize = 8192;

/// Mux key size in bytes (u32 = 4 bytes)
const MUX_KEY_SIZE: usize = 4;

/// Max packets per poll batch (pre-allocated)
const MAX_POLL_BATCH: usize = 64;

/// Pooled buffer for zero-allocation packet handling
struct PooledBuffer {
    /// Pre-allocated buffers
    buffers: Vec<Vec<u8>>,
    /// Buffer lengths (valid data in each buffer)
    lengths: Vec<usize>,
    /// Source addresses
    addrs: Vec<SocketAddr>,
    /// Number of active packets
    count: usize,
}

impl PooledBuffer {
    fn new(capacity: usize, buffer_size: usize) -> Self {
        Self {
            buffers: (0..capacity).map(|_| vec![0u8; buffer_size]).collect(),
            lengths: vec![0; capacity],
            addrs: vec!["0.0.0.0:0".parse().unwrap(); capacity],
            count: 0,
        }
    }

    fn clear(&mut self) {
        self.count = 0;
    }
}

/// Handler trait for multiplexed connections.
/// Implement this for each service/game type that shares the UDP port.
///
/// Note: This trait does NOT require Send + Sync. MuxRudpServer is designed
/// for single-threaded use - all handler callbacks are invoked synchronously
/// during poll(). For multi-threaded scenarios, implement your own synchronization.
pub trait MuxHandler {
    /// Called when a new client connects with this mux_key
    fn on_connect(&mut self, client: SocketAddr);

    /// Called when a message is received from a client
    fn on_message(&mut self, client: SocketAddr, data: &[u8]);

    /// Called when a client disconnects
    fn on_disconnect(&mut self, client: SocketAddr);

    /// Called each tick (optional, for periodic updates)
    fn on_tick(&mut self) {}
}

/// Backward compatibility alias
#[deprecated(since = "2.0.0", note = "Use MuxHandler instead")]
pub type GameHandler = dyn MuxHandler;

/// Per-client state for multiplexed connections
#[allow(dead_code)]
struct MuxClientState {
    /// Which game/service this client is connected to (mux_key)
    mux_key: u32,
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

impl MuxClientState {
    fn new(addr: SocketAddr, mux_key: u32, window_size: usize) -> io::Result<Self> {
        let config = RingBufferConfig::new(window_size)
            .map_err(|e| io::Error::other(format!("Invalid window size: {}", e)))?
            .with_consumers(1)
            .map_err(|e| io::Error::other(format!("Config error: {}", e)))?;

        let send_window = MessageRingBuffer::new(config)
            .map_err(|e| io::Error::other(format!("RingBuffer error: {}", e)))?;

        let nak_addr = SocketAddr::new(addr.ip(), addr.port().wrapping_add(1));

        Ok(Self {
            mux_key,
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
}

/// Multiplexed RUDP server - routes packets by mux_key
pub struct MuxRudpServer {
    /// Main data socket
    socket: Arc<UdpSocket>,
    /// NAK/ACK socket (port + 1)
    nak_socket: Arc<UdpSocket>,
    /// Per-client state (addr -> state)
    clients: HashMap<SocketAddr, MuxClientState>,
    /// Mux handlers (mux_key -> handler)
    handlers: HashMap<u32, Box<dyn MuxHandler>>,
    /// Server local address
    local_addr: SocketAddr,
    /// Window size for new clients
    window_size: usize,
    /// Client timeout
    client_timeout: Duration,
    /// Pre-allocated packet buffer pool (zero-allocation receive)
    packet_pool: PooledBuffer,
    /// Pending new clients (addr, mux_key)
    pending_accepts: Vec<(SocketAddr, u32)>,
    /// Messages to deliver - pooled buffer indices instead of Vec<u8>
    /// Format: (mux_key, addr, pool_index, length)
    pending_message_indices: Vec<(u32, SocketAddr, usize, usize)>,
    /// Message delivery pool (pre-allocated for dispatch)
    message_pool: PooledBuffer,
}

impl MuxRudpServer {
    /// Bind to an address and create a new multiplexed RUDP server
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        Self::bind_with_window(addr, DEFAULT_WINDOW_SIZE)
    }

    /// Bind with custom window size
    pub fn bind_with_window<A: ToSocketAddrs>(addr: A, window_size: usize) -> io::Result<Self> {
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
            handlers: HashMap::new(),
            local_addr,
            window_size,
            client_timeout: DEFAULT_CLIENT_TIMEOUT,
            packet_pool: PooledBuffer::new(MAX_POLL_BATCH, RECV_BUFFER_SIZE),
            pending_accepts: Vec::new(),
            pending_message_indices: Vec::with_capacity(MAX_POLL_BATCH),
            message_pool: PooledBuffer::new(MAX_POLL_BATCH * 4, RECV_BUFFER_SIZE),
        })
    }

    /// Register a handler for a specific mux_key
    pub fn register(&mut self, mux_key: u32, handler: Box<dyn MuxHandler>) {
        self.handlers.insert(mux_key, handler);
    }

    /// Unregister a handler
    pub fn unregister(&mut self, mux_key: u32) -> Option<Box<dyn MuxHandler>> {
        self.handlers.remove(&mux_key)
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

    /// Get clients for a specific mux_key
    pub fn clients_for_mux_key(&self, mux_key: u32) -> impl Iterator<Item = &SocketAddr> {
        self.clients
            .iter()
            .filter(move |(_, state)| state.mux_key == mux_key)
            .map(|(addr, _)| addr)
    }

    /// Poll for incoming packets and route to handlers
    pub fn poll(&mut self) {
        self.poll_data_socket();
        self.poll_nak_socket();
        self.cleanup_timed_out();
        self.dispatch_messages();
    }

    /// Call tick on all handlers
    pub fn tick(&mut self) {
        for handler in self.handlers.values_mut() {
            handler.on_tick();
        }
    }

    /// Poll data socket for incoming packets (zero-allocation)
    fn poll_data_socket(&mut self) {
        // Clear the packet pool for this batch
        self.packet_pool.clear();

        // Receive into pre-allocated pool buffers
        loop {
            if self.packet_pool.count >= MAX_POLL_BATCH {
                break; // Pool full
            }
            let idx = self.packet_pool.count;
            match self.socket.recv_from(&mut self.packet_pool.buffers[idx]) {
                Ok((len, src_addr)) => {
                    if len < MUX_KEY_SIZE {
                        continue; // Need at least mux_key (4 bytes)
                    }
                    self.packet_pool.addrs[idx] = src_addr;
                    self.packet_pool.lengths[idx] = len;
                    self.packet_pool.count += 1;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        // Process packets from pool
        // We need to iterate by index to satisfy borrow checker
        let count = self.packet_pool.count;
        for i in 0..count {
            let src_addr = self.packet_pool.addrs[i];
            let len = self.packet_pool.lengths[i];
            // Copy buffer pointer to stack before mutable borrow
            // The buffer contents are valid for the duration of this loop
            let buf_ptr = self.packet_pool.buffers[i].as_ptr();
            // Safety: buf_ptr points to valid memory in packet_pool.buffers[i]
            // which is not modified by handle_packet_ref
            let data = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
            self.handle_packet_ref(src_addr, data);
        }
    }

    /// Handle an incoming packet with mux_key prefix (4 bytes) - zero-copy version
    fn handle_packet_ref(&mut self, src_addr: SocketAddr, data: &[u8]) {
        if data.len() < MUX_KEY_SIZE {
            return;
        }

        let mux_key = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let payload = &data[MUX_KEY_SIZE..];

        // Check if this mux_key is registered
        if !self.handlers.contains_key(&mux_key) {
            // Unknown mux_key - ignore
            return;
        }

        // Ensure client exists
        let is_new = !self.clients.contains_key(&src_addr);
        if is_new {
            if let Ok(state) = MuxClientState::new(src_addr, mux_key, self.window_size) {
                self.clients.insert(src_addr, state);
                self.pending_accepts.push((src_addr, mux_key));
            } else {
                return;
            }
        }

        let client = self.clients.get_mut(&src_addr).unwrap();

        // Verify mux_key matches (client can't switch games)
        if client.mux_key != mux_key {
            return;
        }

        client.touch();

        // If payload is too small for RUDP header, treat as raw data
        if payload.len() < ReliableUdpHeader::SIZE {
            client
                .recv_window
                .insert(client.recv_window.last_delivered_seq() + 1, payload);
            return;
        }

        // Parse RUDP header
        if let Some((header, msg_payload)) =
            ReliableUdpHeader::from_packet_with_payload_check(payload)
        {
            match header.msg_type {
                t if t == MessageType::Data as u8 => {
                    if header.verify_checksum(msg_payload) {
                        client.recv_window.insert(header.sequence, msg_payload);
                        self.send_ack_to(src_addr, header.sequence);
                    }
                }
                t if t == MessageType::Ack as u8 => {
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
                    client.congestion.on_loss();
                    self.retransmit_for_client(src_addr, header.sequence);
                }
                t if t == MessageType::Handshake as u8 => {
                    // Handshake received - advance receive window past the handshake sequence
                    // so we expect the first data packet (handshake seq + 1)
                    let handshake_seq = header.sequence;
                    if handshake_seq == 0 {
                        // Client sends handshake with seq 0, then data starts at seq 1
                        // We need to advance the window to expect seq 1
                        client.recv_window.advance_expected(1);
                    }
                    // Send ACK for handshake
                    self.send_ack_to(src_addr, handshake_seq);
                }
                _ => {}
            }
        }
    }

    /// Poll NAK socket
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

                    let client_addr =
                        SocketAddr::new(src_addr.ip(), src_addr.port().wrapping_sub(1));

                    if let Some((header, payload)) =
                        ReliableUdpHeader::from_packet_with_payload_check(&buf[..len])
                    {
                        let seq = header.sequence;
                        match header.msg_type {
                            t if t == MessageType::Ack as u8 => {
                                ack_events.push((client_addr, seq));
                            }
                            t if t == MessageType::Nak as u8 => {
                                if payload.len() >= 16 {
                                    let start_seq =
                                        u64::from_le_bytes(payload[0..8].try_into().unwrap());
                                    let end_seq =
                                        u64::from_le_bytes(payload[8..16].try_into().unwrap());
                                    nak_events.push((client_addr, start_seq, end_seq));
                                } else {
                                    nak_events.push((client_addr, seq, seq));
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

    /// Retransmit a single packet
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

    /// Retransmit a range of packets
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
        let mut disconnected: Vec<(u32, SocketAddr)> = Vec::new();

        self.clients.retain(|addr, client| {
            let timed_out = client.is_timed_out(timeout);
            if timed_out {
                disconnected.push((client.mux_key, *addr));
            }
            !timed_out
        });

        // Notify handlers of disconnects
        for (mux_key, addr) in disconnected {
            if let Some(handler) = self.handlers.get_mut(&mux_key) {
                handler.on_disconnect(addr);
            }
        }
    }

    /// Dispatch pending messages to handlers (zero-allocation path)
    fn dispatch_messages(&mut self) {
        // Process new connections
        for (addr, mux_key) in self.pending_accepts.drain(..) {
            if let Some(handler) = self.handlers.get_mut(&mux_key) {
                handler.on_connect(addr);
            }
        }

        // Clear message pool for this dispatch cycle
        self.message_pool.clear();
        self.pending_message_indices.clear();

        // First pass: collect messages into pool (avoids borrow issues)
        let client_addrs: Vec<SocketAddr> = self.clients.keys().copied().collect();
        for addr in client_addrs {
            if let Some(client) = self.clients.get_mut(&addr) {
                let mux_key = client.mux_key;

                // Collect messages into pool
                client.recv_window.deliver_in_order_with(|data| {
                    let pool_idx = self.message_pool.count;
                    if pool_idx < self.message_pool.buffers.len() && data.len() <= RECV_BUFFER_SIZE
                    {
                        self.message_pool.buffers[pool_idx][..data.len()].copy_from_slice(data);
                        self.message_pool.lengths[pool_idx] = data.len();
                        self.message_pool.count += 1;
                        self.pending_message_indices
                            .push((mux_key, addr, pool_idx, data.len()));
                    }
                });

                // Send NAKs for gaps
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
        }

        // Second pass: dispatch to handlers from pool (zero-copy)
        for (mux_key, addr, pool_idx, len) in self.pending_message_indices.drain(..) {
            if let Some(handler) = self.handlers.get_mut(&mux_key) {
                let data = &self.message_pool.buffers[pool_idx][..len];
                handler.on_message(addr, data);
            }
        }
    }

    /// Send data to a client (with mux_key prefix)
    pub fn send(&mut self, client_addr: &SocketAddr, data: &[u8]) -> io::Result<u64> {
        let client = self
            .clients
            .get_mut(client_addr)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "Client not found"))?;

        if !client.open {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Client disconnected",
            ));
        }

        if !client.congestion.can_send() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Congestion window full",
            ));
        }

        let mux_key = client.mux_key;
        let seq = client.next_send_seq;
        let mut header = ReliableUdpHeader::new(0, seq, MessageType::Data, data.len() as u16);
        header.calculate_checksum(data);

        // Build packet with mux_key prefix (4 bytes)
        let mut packet = Vec::with_capacity(MUX_KEY_SIZE + ReliableUdpHeader::SIZE + data.len());
        packet.extend_from_slice(&mux_key.to_le_bytes());
        packet.extend_from_slice(bytemuck::bytes_of(&header));
        packet.extend_from_slice(data);

        // Store in send window for retransmit
        if let Some((slot_seq, slots)) = client.send_window.try_claim_slots(1) {
            slots[0].set_sequence(slot_seq);
            slots[0].set_data(&packet);
            client.send_window.publish_batch(slot_seq, 1);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Send window full",
            ));
        }

        // Send
        self.socket.send_to(&packet, *client_addr)?;
        client.congestion.on_send();
        client.next_send_seq = seq.wrapping_add(1);

        Ok(seq)
    }

    /// Broadcast to all clients with a specific mux_key
    pub fn broadcast(&mut self, mux_key: u32, data: &[u8]) -> usize {
        let addrs: Vec<SocketAddr> = self
            .clients
            .iter()
            .filter(|(_, c)| c.mux_key == mux_key)
            .map(|(a, _)| *a)
            .collect();

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
        if let Some(client) = self.clients.remove(client_addr) {
            if let Some(handler) = self.handlers.get_mut(&client.mux_key) {
                handler.on_disconnect(*client_addr);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        connects: Vec<SocketAddr>,
        messages: Vec<(SocketAddr, Vec<u8>)>,
        disconnects: Vec<SocketAddr>,
    }

    impl TestHandler {
        fn new() -> Self {
            Self {
                connects: Vec::new(),
                messages: Vec::new(),
                disconnects: Vec::new(),
            }
        }
    }

    impl MuxHandler for TestHandler {
        fn on_connect(&mut self, client: SocketAddr) {
            self.connects.push(client);
        }

        fn on_message(&mut self, client: SocketAddr, data: &[u8]) {
            self.messages.push((client, data.to_vec()));
        }

        fn on_disconnect(&mut self, client: SocketAddr) {
            self.disconnects.push(client);
        }
    }

    #[test]
    fn test_mux_server_bind() {
        let server = MuxRudpServer::bind("127.0.0.1:0").unwrap();
        assert_eq!(server.client_count(), 0);
        assert!(server.local_addr().port() > 0);
    }

    #[test]
    fn test_mux_server_register() {
        let mut server = MuxRudpServer::bind("127.0.0.1:0").unwrap();
        server.register(0x00000001, Box::new(TestHandler::new()));
        server.register(0x00000002, Box::new(TestHandler::new()));

        // Both should be registered
        assert!(server.handlers.contains_key(&0x00000001));
        assert!(server.handlers.contains_key(&0x00000002));
    }

    #[test]
    fn test_mux_server_poll_empty() {
        let mut server = MuxRudpServer::bind("127.0.0.1:0").unwrap();
        server.register(0x00000001, Box::new(TestHandler::new()));
        server.poll(); // Should not panic
        assert_eq!(server.client_count(), 0);
    }
}
