//! # kaos-rudp
//!
//! Reliable UDP transport built on kaos ring buffers.
//!
//! ## Features
//!
//! - **Composable**: Trait-based layers - combine reliability, archiving, multicast
//! - **Fast**: 3M+ messages/sec on localhost
//! - **Reliable**: NAK-based retransmission for loss recovery
//! - **Minimal allocations**: Pre-allocated buffers in hot path
//!
//! ## Transport Trait
//!
//! All transports implement `Transport` trait for composability:
//!
//! ```rust,ignore
//! use kaos_rudp::{Transport, RudpTransport};
//!
//! fn send_messages<T: Transport>(transport: &mut T) {
//!     transport.send(b"hello").unwrap();
//! }
//! ```
//!
//! ## Protocol
//!
//! - Sequence numbering for ordering
//! - NAK (Negative Acknowledgment) for retransmission requests
//! - Sliding window flow control
//! - Multicast-friendly (no ACKs required)

use kaos::disruptor::{MessageRingBuffer, RingBufferConfig, RingBufferEntry};
use std::cell::RefCell;
use std::net::{SocketAddr, UdpSocket};

// Buffer sizes for UDP I/O
/// Max send buffer (64KB = max UDP payload with some headroom)
const SEND_BUFFER_SIZE: usize = 65536;
/// Large message assembly buffer (4KB typical MTU-friendly size)
const LARGE_MSG_SIZE: usize = 4096;
/// Per-packet receive buffer (64KB to handle large game state payloads)
const RECV_PACKET_SIZE: usize = 65536;
/// Batch size for recvmmsg (16 packets per syscall - reduced due to larger buffers)
const RECV_BATCH_SIZE: usize = 16;
/// Socket buffer size (8MB for high throughput)
const SOCKET_BUFFER_SIZE: i32 = 8 * 1024 * 1024;

thread_local! {
    static SEND_BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(SEND_BUFFER_SIZE));
    static LARGE_MSG_BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(LARGE_MSG_SIZE));
    static RECV_BUFFERS: RefCell<Vec<[u8; RECV_PACKET_SIZE]>> = RefCell::new(vec![[0u8; RECV_PACKET_SIZE]; RECV_BATCH_SIZE]);
    static RECV_LENS: RefCell<Vec<usize>> = RefCell::new(vec![0usize; RECV_BATCH_SIZE]);
}

mod header;
mod transport;

#[cfg(feature = "archive")]
pub mod archived;
pub mod congestion;
#[cfg(feature = "driver")]
pub mod driver;
pub mod multicast;
mod sendmmsg;
pub mod server;
mod window;

pub use header::{FastHeader, MessageType, ReliableUdpHeader, FAST_HEADER_MAGIC, FLAG_NO_CRC};

// Tracing macros - no-op when feature disabled
#[cfg(feature = "tracing")]
macro_rules! trace_debug { ($($arg:tt)*) => { tracing::debug!($($arg)*) } }
#[cfg(not(feature = "tracing"))]
macro_rules! trace_debug { ($($arg:tt)*) => {} }

#[cfg(feature = "tracing")]
macro_rules! trace_warn { ($($arg:tt)*) => { tracing::warn!($($arg)*) } }
#[cfg(not(feature = "tracing"))]
macro_rules! trace_warn { ($($arg:tt)*) => {} }

// Core trait - all transports implement this
pub use transport::{Archived, BatchTransport, Reliable, Transport};

#[cfg(feature = "archive")]
pub use archived::{ArchivedError, ArchivedTransport};
use congestion::CongestionController;
pub use congestion::CongestionController as Congestion;
#[cfg(feature = "driver")]
pub use driver::DriverTransport;
use kaos::{record_backpressure, record_receive, record_retransmit, record_send};
pub use multicast::{MulticastSocket, MulticastTransport};
pub use server::{RudpServer, RudpServerClient};
use window::BitmapWindow;

/// Reliable UDP transport with ring buffer for retransmission.
pub struct RudpTransport {
    socket: std::sync::Arc<UdpSocket>,
    nak_socket: UdpSocket,
    send_window: MessageRingBuffer,
    recv_window: BitmapWindow,
    window_size: usize,
    next_send_seq: u64,
    acked_seq: u64,
    remote_addr: SocketAddr,
    remote_nak_addr: SocketAddr,
    congestion: CongestionController,
    /// Last send timestamp for RTT measurement
    last_send_time: std::time::Instant,
    /// Last NAK send time for backoff
    last_nak_time: std::time::Instant,
    /// Pending retransmits (limited queue)
    retransmit_queue: std::collections::VecDeque<u64>,
    #[cfg(target_os = "linux")]
    _batch_sender: sendmmsg::BatchSender,
    #[cfg(target_os = "linux")]
    _batch_receiver: sendmmsg::BatchReceiver,
}

#[derive(Debug, Clone)]
pub struct ReliableUdpConfig {
    pub local_addr: String,
    pub remote_addr: String,
    pub window_size: usize,
}

impl Default for ReliableUdpConfig {
    fn default() -> Self {
        Self {
            local_addr: "127.0.0.1:0".to_string(),
            remote_addr: "127.0.0.1:0".to_string(),
            window_size: 1024,
        }
    }
}

impl RudpTransport {
    pub fn new(
        bind_addr: SocketAddr,
        remote_addr: SocketAddr,
        window_size: usize,
    ) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_nonblocking(true)?;

        // Get actual bound port (important when bind_addr uses port 0)
        let actual_addr = socket.local_addr()?;

        // Create NAK socket on actual_port+1
        let nak_port = actual_addr.port() + 1;
        let nak_bind_addr = SocketAddr::new(actual_addr.ip(), nak_port);
        let nak_socket = UdpSocket::bind(nak_bind_addr)?;
        nak_socket.set_nonblocking(true)?;

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

        // Remote NAK port is also port+1
        let remote_nak_addr = SocketAddr::new(remote_addr.ip(), remote_addr.port() + 1);

        let config = RingBufferConfig::new(window_size)
            .map_err(|e| std::io::Error::other(format!("Invalid window size: {}", e)))?
            .with_consumers(1)
            .map_err(|e| std::io::Error::other(format!("Config error: {}", e)))?;

        let send_window = MessageRingBuffer::new(config)
            .map_err(|e| std::io::Error::other(format!("RingBuffer error: {}", e)))?;

        Ok(Self {
            socket: std::sync::Arc::new(socket),
            nak_socket,
            send_window,
            recv_window: BitmapWindow::new(window_size, 0),
            window_size,
            next_send_seq: 0,
            acked_seq: 0,
            remote_addr,
            remote_nak_addr,
            congestion: CongestionController::new(64, window_size as u32),
            last_send_time: std::time::Instant::now(),
            last_nak_time: std::time::Instant::now(),
            retransmit_queue: std::collections::VecDeque::with_capacity(64),
            #[cfg(target_os = "linux")]
            _batch_sender: sendmmsg::BatchSender::new(64),
            #[cfg(target_os = "linux")]
            _batch_receiver: sendmmsg::BatchReceiver::new(64, 65536),
        })
    }

    pub fn auto(config: ReliableUdpConfig) -> std::io::Result<Self> {
        let bind_addr: std::net::SocketAddr = config.local_addr.parse().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid local_addr: {}", e),
            )
        })?;
        let remote_addr: std::net::SocketAddr = config.remote_addr.parse().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid remote_addr: {}", e),
            )
        })?;
        Self::new(bind_addr, remote_addr, config.window_size)
    }

    pub fn send(&mut self, data: &[u8]) -> std::io::Result<u64> {
        // Congestion control: check if we can send
        if !self.congestion.can_send() {
            record_backpressure();
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Congestion window full",
            ));
        }

        let seq = self.next_send_seq;
        let mut header = ReliableUdpHeader::new(0, seq, MessageType::Data, data.len() as u16);
        header.calculate_checksum(data);

        const MAX_STACK_SIZE: usize = 256;
        let total_len = ReliableUdpHeader::SIZE + data.len();
        let mut stack_buf = [0u8; MAX_STACK_SIZE];

        let packet: &[u8] = if total_len <= MAX_STACK_SIZE {
            // Safe: ReliableUdpHeader derives Pod
            stack_buf[..ReliableUdpHeader::SIZE].copy_from_slice(bytemuck::bytes_of(&header));
            stack_buf[ReliableUdpHeader::SIZE..total_len].copy_from_slice(data);
            &stack_buf[..total_len]
        } else {
            return LARGE_MSG_BUFFER.with(|buf_cell| {
                let mut buffer = buf_cell.borrow_mut();
                buffer.clear();
                // Safe: ReliableUdpHeader derives Pod
                buffer.extend_from_slice(bytemuck::bytes_of(&header));
                buffer.extend_from_slice(data);

                if let Some((slot_seq, slots)) = self.send_window.try_claim_slots(1) {
                    slots[0].set_sequence(slot_seq);
                    slots[0].set_data(&buffer);
                    self.send_window.publish_batch(slot_seq, 1);

                    self.socket.send_to(&buffer, self.remote_addr)?;
                    self.congestion.on_send();
                    self.last_send_time = std::time::Instant::now();
                    record_send(buffer.len() as u64);
                    self.next_send_seq = self.next_send_seq.wrapping_add(1);
                    Ok(seq)
                } else {
                    record_backpressure();
                    Err(std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        "Send window full",
                    ))
                }
            });
        };

        if let Some((slot_seq, slots)) = self.send_window.try_claim_slots(1) {
            slots[0].set_sequence(slot_seq);
            slots[0].set_data(packet);
            self.send_window.publish_batch(slot_seq, 1);

            self.socket.send_to(packet, self.remote_addr)?;
            self.congestion.on_send();
            self.last_send_time = std::time::Instant::now();
            record_send(packet.len() as u64);
            self.next_send_seq = self.next_send_seq.wrapping_add(1);
            Ok(seq)
        } else {
            record_backpressure();
            Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Send window full",
            ))
        }
    }

    pub fn send_batch(&mut self, data: &[&[u8]]) -> std::io::Result<usize> {
        self.send_batch_ultra(data)
    }

    ///  Batch send with minimal header and no CRC or timestamp
    #[inline]
    pub fn send_batch_ultra(&mut self, data: &[&[u8]]) -> std::io::Result<usize> {
        let batch_size = data.len();
        if batch_size == 0 {
            return Ok(0);
        }

        if let Some((slot_seq, _slots)) = self.send_window.try_claim_slots(batch_size) {
            let actual = batch_size;

            SEND_BUFFER.with(|buf_cell| {
                let mut buf = buf_cell.borrow_mut();
                buf.clear();

                // Pre-size buffer
                let estimated_size = actual * (FastHeader::SIZE + 64);
                let current_cap = buf.capacity();
                if current_cap < estimated_size {
                    buf.reserve(estimated_size - current_cap);
                }

                for (i, msg) in data.iter().take(actual).enumerate() {
                    let seq = (slot_seq + (i as u64)) as u32;

                    // Minimal 8-byte header
                    let header = FastHeader::new(seq, msg.len());
                    // Safe: FastHeader derives Pod
                    buf.extend_from_slice(bytemuck::bytes_of(&header));
                    buf.extend_from_slice(msg);
                }

                let _ = self.socket.send_to(&buf, self.remote_addr);
            });

            self.send_window.publish_batch(slot_seq, actual);
            // Keep messages for retransmission - only advance on ACK
            self.next_send_seq = slot_seq + (actual as u64);
            Ok(actual)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Send window full",
            ))
        }
    }

    /// Queue a retransmit (paced - not sent immediately)
    pub fn queue_retransmit(&mut self, lost_seq: u64) {
        const MAX_PENDING: usize = 64;
        if self.retransmit_queue.len() < MAX_PENDING {
            // Avoid duplicates
            if !self.retransmit_queue.contains(&lost_seq) {
                self.retransmit_queue.push_back(lost_seq);
            }
        }
    }

    /// Process pending retransmits (call in event loop)
    /// Returns number of packets retransmitted
    pub fn process_retransmits(&mut self) -> usize {
        const MAX_PER_CALL: usize = 8; // Limit to prevent flood
        let mut count = 0;
        
        while count < MAX_PER_CALL {
            if let Some(seq) = self.retransmit_queue.pop_front() {
                self.retransmit_now(seq);
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Retransmit immediately (internal)
    fn retransmit_now(&mut self, lost_seq: u64) {
        let slots = self.send_window.peek_batch(0, self.window_size);
        if let Some(slot) = slots.iter().find(|s| s.sequence() == lost_seq) {
            let pkt_data = slot.data();
            if !pkt_data.is_empty() {
                record_retransmit();
                let _ = self.socket.send_to(pkt_data, self.remote_addr);
            }
        }
    }

    /// Legacy: immediate retransmit (for backward compatibility)
    pub fn retransmit(&mut self, lost_seq: u64) {
        self.retransmit_now(lost_seq);
    }

    pub fn send_batch_nak(&self, start_seq: u64, end_seq: u64) {
        let mut packet = Vec::with_capacity(ReliableUdpHeader::SIZE + 16);
        let payload = [start_seq.to_le_bytes(), end_seq.to_le_bytes()].concat();
        let mut header = ReliableUdpHeader::new(0, start_seq, MessageType::Nak, 16);
        header.calculate_checksum(&payload);
        // Safe: ReliableUdpHeader derives Pod
        packet.extend_from_slice(bytemuck::bytes_of(&header));
        packet.extend_from_slice(&payload);

        
        trace_debug!(
            "[NAK-SEND] Sending batch NAK for seq {}-{} to {}",
            start_seq, end_seq, self.remote_nak_addr
        );
        if let Err(_e) = self.nak_socket.send_to(&packet, self.remote_nak_addr) {
            
            trace_warn!("[NAK-SEND-ERROR] Failed to send NAK: {}", _e);
        } else {
            
            trace_debug!("[NAK-SEND-OK] Batch NAK sent successfully");
        }
    }

    /// Send ACK to confirm receipt up to a sequence number
    pub fn send_ack(&self, acked_seq: u64) {
        let mut header = ReliableUdpHeader::new(0, acked_seq, MessageType::Ack, 0);
        header.calculate_checksum(&[]);
        // Safe: ReliableUdpHeader derives Pod
        let packet = bytemuck::bytes_of(&header);

        
        trace_debug!(
            "[ACK-SEND] Sending ACK for seq {} to {}",
            acked_seq, self.remote_nak_addr
        );
        let _ = self.nak_socket.send_to(packet, self.remote_nak_addr);
    }

    /// Process incoming ACKs and advance send window
    pub fn process_acks(&mut self) {
        let mut buf = [0u8; 256];

        loop {
            match self.nak_socket.recv_from(&mut buf) {
                Ok((len, _)) => {
                    if len < ReliableUdpHeader::SIZE {
                        continue;
                    }

                    if let Some((header, _payload)) =
                        ReliableUdpHeader::from_packet_with_payload_check(&buf[..len])
                    {
                        if header.msg_type == (MessageType::Ack as u8) {
                            let acked = header.sequence;
                            if acked > self.acked_seq {
                                // Count newly acknowledged packets
                                let newly_acked = acked.saturating_sub(self.acked_seq);
                                
                                
                                trace_debug!(
                                    "[ACK-RECV] ACK seq {}, {} packets acked",
                                    acked, newly_acked
                                );
                                
                                // Call on_ack() for EACH acked packet
                                for _ in 0..newly_acked {
                                    self.congestion.on_ack();
                                }
                                
                                // Measure RTT (approximate: time since last send)
                                let rtt_us = self.last_send_time.elapsed().as_micros() as u64;
                                if rtt_us > 0 && rtt_us < 1_000_000 {
                                    self.congestion.update_rtt(rtt_us);
                                }
                                
                                self.acked_seq = acked;
                                self.send_window.advance_consumer(0, acked);
                            }
                        } else if header.msg_type == (MessageType::Nak as u8) {
                            // Handle NAK - queue for paced retransmit
                            self.congestion.on_loss();
                            let sequence = header.sequence;
                            self.queue_retransmit(sequence);
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(_) => {
                    break;
                }
            }
        }
    }

    /// Process incoming NAKs and retransmit as needed
    pub fn process_naks(&mut self) {
        let mut buf = [0u8; 2048];
        let mut _nak_count = 0;
        let mut _retransmit_count = 0;

        loop {
            match self.nak_socket.recv_from(&mut buf) {
                Ok((len, _src)) => {
                    _nak_count += 1;

                    if len < ReliableUdpHeader::SIZE {
                        
                        trace_debug!(
                            "[NAK] Received invalid NAK (too short: {} bytes) from {:?}",
                            len, _src
                        );
                        continue;
                    }

                    if let Some((header, payload)) =
                        ReliableUdpHeader::from_packet_with_payload_check(&buf[..len])
                    {
                        let msg_type = header.msg_type;

                        if msg_type != (MessageType::Nak as u8) {
                            
                            trace_debug!("[NAK] Received non-NAK message type: {}", msg_type);
                            continue;
                        }

                        let sequence = header.sequence;

                        if payload.len() >= 16 && payload.len() % 16 == 0 {
                            let range_count = payload.len() / 16;
                            
                            trace_debug!(
                                "[NAK] Received batch NAK from {} with {} ranges",
                                _src, range_count
                            );

                            for i in 0..range_count {
                                let offset = i * 16;
                                let start_seq = u64::from_le_bytes(
                                    payload[offset..offset + 8].try_into().unwrap(),
                                );
                                let end_seq = u64::from_le_bytes(
                                    payload[offset + 8..offset + 16].try_into().unwrap(),
                                );
                                let count = (end_seq - start_seq + 1) as usize;

                                
                                trace_debug!(
                                    "[NAK] Range {}: seq {}-{} ({} packets)",
                                    i, start_seq, end_seq, count
                                );
                                self.retransmit_batch(start_seq, end_seq);
                                _retransmit_count += count;
                            }
                        } else {
                            
                            trace_debug!(
                                "[NAK] Received single NAK from {} for seq {}",
                                _src, sequence
                            );
                            self.retransmit(sequence);
                            _retransmit_count += 1;
                        }
                    } else {
                        
                        trace_debug!("[NAK] Failed to parse NAK header");
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(_e) => {
                    
                    trace_debug!("[NAK] Socket error: {}", _e);
                    break;
                }
            }
        }
    }

    /// Retransmit a batch of lost packets (on batch NAK)
    pub fn retransmit_batch(&mut self, start_seq: u64, end_seq: u64) {
        self.congestion.on_loss(); // Loss event triggers congestion control
        let slots = self.send_window.peek_batch(0, self.window_size);
        for slot in slots.iter().filter(|s| {
            let seq = s.sequence();
            seq >= start_seq && seq <= end_seq
        }) {
            let pkt_data = slot.data();
            if !pkt_data.is_empty() {
                record_retransmit();
                let _ = self.socket.send_to(pkt_data, self.remote_addr);
            }
        }
    }

    /// Get congestion window size
    pub fn congestion_window(&self) -> u32 {
        self.congestion.window_size()
    }

    /// Get packets in flight
    pub fn in_flight(&self) -> u32 {
        self.congestion.in_flight()
    }

    /// Callback-based delivery: process each message with the provided closure.
    pub fn receive_batch_with<F: FnMut(&[u8])>(&mut self, max_count: usize, mut f: F) {
        RECV_BUFFERS.with(|bufs_cell| {
            RECV_LENS.with(|lens_cell| {
                let mut bufs = bufs_cell.borrow_mut();
                let mut lens = lens_cell.borrow_mut();

                let max_recv = max_count.min(bufs.len());
                let mut n = 0;
                for i in 0..max_recv {
                    match self.socket.recv_from(&mut bufs[i]) {
                        Ok((len, _)) => {
                            lens[i] = len;
                            n += 1;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(_e) => {
                            
                            trace_warn!("[ERR] Socket error: {}", _e);
                            break;
                        }
                    }
                }
                for i in 0..n {
                    let len = lens[i];
                    let data = &bufs[i][..len];

                    if len < FastHeader::SIZE {
                        continue;
                    }

                    // Detect format via magic bit in first u32
                    let first_u32 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                    let is_fast = (first_u32 & FAST_HEADER_MAGIC) != 0;

                    if is_fast {
                        // Parse FastHeader format (ultra-fast path)
                        let mut offset = 0;
                        while offset + FastHeader::SIZE <= len {
                            let frame_len_raw = u32::from_le_bytes([
                                data[offset],
                                data[offset + 1],
                                data[offset + 2],
                                data[offset + 3],
                            ]);
                            let frame_len = (frame_len_raw & !FAST_HEADER_MAGIC) as usize;
                            let seq = u32::from_le_bytes([
                                data[offset + 4],
                                data[offset + 5],
                                data[offset + 6],
                                data[offset + 7],
                            ]) as u64;

                            if frame_len >= FastHeader::SIZE && offset + frame_len <= len {
                                let payload = &data[offset + FastHeader::SIZE..offset + frame_len];
                                self.recv_window.insert(seq, payload);
                                offset += frame_len;
                            } else {
                                break;
                            }
                        }
                    } else if first_u32 >= (ReliableUdpHeader::SIZE as u32)
                        && first_u32 < 2000
                        && 4 + (first_u32 as usize) <= len
                    {
                        // Parse standard batch format (fast path with 24-byte header)
                        let mut offset = 0;
                        while offset + 4 <= len {
                            let pkt_len = u32::from_le_bytes([
                                data[offset],
                                data[offset + 1],
                                data[offset + 2],
                                data[offset + 3],
                            ]) as usize;
                            offset += 4;

                            if pkt_len >= ReliableUdpHeader::SIZE && offset + pkt_len <= len {
                                if let Some(header) = ReliableUdpHeader::from_bytes(
                                    &data[offset..offset + ReliableUdpHeader::SIZE],
                                ) {
                                    let payload_len = header.payload_len as usize;
                                    let flags = header.flags;
                                    if pkt_len >= ReliableUdpHeader::SIZE + payload_len {
                                        let payload = &data[offset + ReliableUdpHeader::SIZE
                                            ..offset + ReliableUdpHeader::SIZE + payload_len];
                                        let valid = (flags & FLAG_NO_CRC) != 0
                                            || header.verify_checksum(payload);
                                        if valid {
                                            self.recv_window.insert(header.sequence, payload);
                                        }
                                    }
                                }
                                offset += pkt_len;
                            } else {
                                break;
                            }
                        }
                    } else {
                        // Parse single packet format
                        if let Some(header) =
                            ReliableUdpHeader::from_bytes(&data[..ReliableUdpHeader::SIZE])
                        {
                            let payload_len = header.payload_len as usize;
                            if len >= ReliableUdpHeader::SIZE + payload_len {
                                let payload = &data[ReliableUdpHeader::SIZE
                                    ..ReliableUdpHeader::SIZE + payload_len];
                                let checksum_ok = header.verify_checksum(payload);
                                if checksum_ok {
                                    self.recv_window.insert(header.sequence, payload);
                                }
                            }
                        }
                    }
                }
                self.recv_window.deliver_in_order_with(|msg| {
                    record_receive(msg.len() as u64);
                    f(msg);
                });

                // Send ACK for highest delivered sequence
                let last_delivered = self.recv_window.last_delivered_seq();
                if last_delivered > 0 {
                    self.send_ack(last_delivered);
                }

                // NAK backoff: limit to once per RTT
                let nak_interval = std::time::Duration::from_micros(self.congestion.rtt_us().max(1000));
                if self.last_nak_time.elapsed() >= nak_interval {
                    self.recv_window.send_batch_naks_for_gaps(|start, end| {
                        self.send_batch_nak(start, end);
                    });
                    self.last_nak_time = std::time::Instant::now();
                }
            });
        });
    }

    /// Get reference to the underlying socket
    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }

    /// Get reference to the NAK socket
    pub fn nak_socket(&self) -> &UdpSocket {
        &self.nak_socket
    }

    /// Get remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

// Trait implementations for composability
impl Transport for RudpTransport {
    fn send(&mut self, data: &[u8]) -> std::io::Result<u64> {
        RudpTransport::send(self, data)
    }

    fn receive<F: FnMut(&[u8])>(&mut self, mut handler: F) -> usize {
        let mut count = 0;
        RudpTransport::receive_batch_with(self, 64, |msg| {
            handler(msg);
            count += 1;
        });
        count
    }
}

impl BatchTransport for RudpTransport {
    fn send_batch(&mut self, data: &[&[u8]]) -> std::io::Result<usize> {
        RudpTransport::send_batch(self, data)
    }
}

impl Reliable for RudpTransport {
    fn retransmit_pending(&mut self) -> std::io::Result<usize> {
        // NAK-based: retransmission triggered by receive
        Ok(0)
    }

    fn acked_sequence(&self) -> u64 {
        self.acked_seq
    }
}
