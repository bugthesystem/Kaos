//! Unified transport abstraction for kaosnet.
//!
//! Provides a common interface for different transport backends:
//! - WebSocket (kaos-ws) - for browser clients
//! - Reliable UDP (kaos-rudp) - for native low-latency clients

use std::io;
use std::net::SocketAddr;

/// Unique client identifier
pub type ClientId = u64;

/// A single client connection (transport-agnostic)
pub trait ClientTransport: Send {
    /// Send data to this client
    fn send(&mut self, data: &[u8]) -> io::Result<usize>;

    /// Receive data from this client, calling handler for each message
    fn receive<F: FnMut(&[u8])>(&mut self, handler: F) -> usize;

    /// Check if connection is still open
    fn is_open(&self) -> bool;

    /// Close the connection
    fn close(&mut self) -> io::Result<()>;

    /// Get client's address (if available)
    fn peer_addr(&self) -> Option<SocketAddr> {
        None
    }
}

/// A server that accepts client connections
pub trait TransportServer: Send {
    /// The type of transport for accepted connections
    type Client: ClientTransport;

    /// Try to accept a new connection (non-blocking)
    fn accept(&mut self) -> io::Result<Option<Self::Client>>;

    /// Set blocking/non-blocking mode
    fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()>;

    /// Get server's local address
    fn local_addr(&self) -> io::Result<SocketAddr>;
}

// ============================================================================
// WebSocket Transport (kaos-ws)
// ============================================================================

/// Convert WsError to io::Error
fn ws_to_io(e: kaos_ws::WsError) -> io::Error {
    io::Error::other(e.to_string())
}

/// WebSocket server wrapper
pub struct WsServerTransport {
    inner: kaos_ws::WsServer,
}

impl WsServerTransport {
    pub fn bind(addr: impl std::net::ToSocketAddrs) -> io::Result<Self> {
        Ok(Self {
            inner: kaos_ws::WsServer::bind(addr).map_err(ws_to_io)?,
        })
    }
}

impl TransportServer for WsServerTransport {
    type Client = WsClientTransport;

    fn accept(&mut self) -> io::Result<Option<Self::Client>> {
        match self.inner.accept().map_err(ws_to_io)? {
            Some(transport) => Ok(Some(WsClientTransport { inner: transport })),
            None => Ok(None),
        }
    }

    fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
        self.inner.set_nonblocking(nonblocking)
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

/// WebSocket client transport wrapper
pub struct WsClientTransport {
    inner: kaos_ws::WsTransport,
}

impl WsClientTransport {
    /// Connect to a WebSocket server
    pub fn connect(url: &str) -> io::Result<Self> {
        Ok(Self {
            inner: kaos_ws::WsTransport::connect(url).map_err(ws_to_io)?,
        })
    }
}

impl ClientTransport for WsClientTransport {
    fn send(&mut self, data: &[u8]) -> io::Result<usize> {
        kaos_ws::Transport::send(&mut self.inner, data)
    }

    fn receive<F: FnMut(&[u8])>(&mut self, handler: F) -> usize {
        kaos_ws::Transport::receive(&mut self.inner, handler)
    }

    fn is_open(&self) -> bool {
        kaos_ws::Transport::is_open(&self.inner)
    }

    fn close(&mut self) -> io::Result<()> {
        kaos_ws::Transport::close(&mut self.inner)
    }
}

// ============================================================================
// RUDP Transport (kaos-rudp) - for point-to-point connections
// ============================================================================

/// RUDP transport wrapper for a single connection (point-to-point)
pub struct RudpClientTransport {
    inner: kaos_rudp::RudpTransport,
    open: bool,
}

impl RudpClientTransport {
    /// Create a new RUDP transport (client-side, connects to a specific server)
    pub fn new(
        bind_addr: SocketAddr,
        remote_addr: SocketAddr,
        window_size: usize,
    ) -> io::Result<Self> {
        Ok(Self {
            inner: kaos_rudp::RudpTransport::new(bind_addr, remote_addr, window_size)?,
            open: true,
        })
    }
}

impl ClientTransport for RudpClientTransport {
    fn send(&mut self, data: &[u8]) -> io::Result<usize> {
        kaos_rudp::Transport::send(&mut self.inner, data)?;
        Ok(data.len())
    }

    fn receive<F: FnMut(&[u8])>(&mut self, handler: F) -> usize {
        kaos_rudp::Transport::receive(&mut self.inner, handler)
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn close(&mut self) -> io::Result<()> {
        self.open = false;
        Ok(())
    }

    fn peer_addr(&self) -> Option<SocketAddr> {
        Some(self.inner.remote_addr())
    }
}

// ============================================================================
// RUDP Server - wraps kaos_rudp::RudpServer with MessageRingBuffer
// ============================================================================

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Re-export kaos-rudp types
pub use kaos_rudp::{ReliableUdpHeader, MessageType, RudpServer as KaosRudpServer};

/// Default window size for RUDP
const DEFAULT_WINDOW_SIZE: usize = 256;

/// Multi-client Reliable UDP server wrapper.
///
/// Wraps kaos_rudp::RudpServer which provides:
/// - MessageRingBuffer for per-client send windows (kaos disruptor)
/// - BitmapWindow for per-client receive windows
/// - NAK-based retransmission
/// - Congestion control
pub struct RudpServer {
    /// Inner kaos-rudp server with MessageRingBuffer per client
    inner: KaosRudpServer,
    /// Pending accepts (addresses)
    pending_accepts: VecDeque<SocketAddr>,
    /// Per-client receive queues (for TransportServer interface)
    client_queues: std::collections::HashMap<SocketAddr, Arc<Mutex<VecDeque<Vec<u8>>>>>,
    /// Client open state
    client_open: std::collections::HashMap<SocketAddr, Arc<Mutex<bool>>>,
}

impl RudpServer {
    /// Bind to an address and create a new reliable RUDP server
    pub fn bind(addr: impl std::net::ToSocketAddrs) -> io::Result<Self> {
        let inner = KaosRudpServer::bind(addr, DEFAULT_WINDOW_SIZE)?;
        Ok(Self {
            inner,
            pending_accepts: VecDeque::new(),
            client_queues: std::collections::HashMap::new(),
            client_open: std::collections::HashMap::new(),
        })
    }

    /// Set client timeout
    pub fn set_client_timeout(&mut self, timeout: Duration) {
        self.inner.set_client_timeout(timeout);
    }

    /// Get number of connected clients
    pub fn client_count(&self) -> usize {
        self.inner.client_count()
    }

    /// Broadcast data to all connected clients (with reliability)
    pub fn broadcast(&mut self, data: &[u8]) -> io::Result<usize> {
        Ok(self.inner.broadcast(data))
    }

    /// Remove timed-out clients
    pub fn cleanup_stale_clients(&mut self) {
        // Inner server handles cleanup during poll()
        self.inner.poll();
    }

    /// Poll and receive data from all clients into their queues
    fn poll_and_receive(&mut self) {
        // Poll the underlying server
        self.inner.poll();

        // Check for new clients
        while let Some(addr) = self.inner.accept() {
            self.pending_accepts.push_back(addr);
            self.client_queues.insert(addr, Arc::new(Mutex::new(VecDeque::new())));
            self.client_open.insert(addr, Arc::new(Mutex::new(true)));
        }

        // Receive data from all known clients into their queues
        let addrs: Vec<SocketAddr> = self.client_queues.keys().copied().collect();
        for addr in addrs {
            if let Some(queue) = self.client_queues.get(&addr) {
                let queue = queue.clone();
                self.inner.receive(&addr, |data| {
                    if let Ok(mut q) = queue.lock() {
                        q.push_back(data.to_vec());
                    }
                });
            }
        }

        // Clean up disconnected clients
        let disconnected: Vec<SocketAddr> = self.client_queues.keys()
            .filter(|addr| !self.inner.has_client(addr))
            .copied()
            .collect();
        for addr in disconnected {
            if let Some(open) = self.client_open.get(&addr) {
                if let Ok(mut o) = open.lock() {
                    *o = false;
                }
            }
            self.client_queues.remove(&addr);
        }
    }
}

impl TransportServer for RudpServer {
    type Client = RudpServerClient;

    fn accept(&mut self) -> io::Result<Option<Self::Client>> {
        // Poll and receive data
        self.poll_and_receive();

        // Return pending new clients
        if let Some(addr) = self.pending_accepts.pop_front() {
            if let Some(queue) = self.client_queues.get(&addr) {
                if let Some(open) = self.client_open.get(&addr) {
                    return Ok(Some(RudpServerClient {
                        remote_addr: addr,
                        recv_queue: queue.clone(),
                        open: open.clone(),
                        // Store server reference for sending
                        server_socket: self.inner.socket().try_clone()?,
                    }));
                }
            }
        }

        Ok(None)
    }

    fn set_nonblocking(&mut self, _nonblocking: bool) -> io::Result<()> {
        // Server is always non-blocking
        Ok(())
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.inner.local_addr())
    }
}

/// A reliable client connection from the RUDP server's perspective
pub struct RudpServerClient {
    remote_addr: SocketAddr,
    recv_queue: Arc<Mutex<VecDeque<Vec<u8>>>>,
    open: Arc<Mutex<bool>>,
    server_socket: std::net::UdpSocket,
}

impl ClientTransport for RudpServerClient {
    fn send(&mut self, data: &[u8]) -> io::Result<usize> {
        // Build reliable packet with kaos-rudp header
        // Note: For full reliability, the server's send() method should be used,
        // but we provide direct socket send for simplicity
        let mut header = ReliableUdpHeader::new(0, 0, MessageType::Data, data.len() as u16);
        header.calculate_checksum(data);

        let mut packet = Vec::with_capacity(ReliableUdpHeader::SIZE + data.len());
        packet.extend_from_slice(bytemuck::bytes_of(&header));
        packet.extend_from_slice(data);

        self.server_socket.send_to(&packet, self.remote_addr)
    }

    fn receive<F: FnMut(&[u8])>(&mut self, mut handler: F) -> usize {
        let mut count = 0;
        if let Ok(mut queue) = self.recv_queue.lock() {
            while let Some(data) = queue.pop_front() {
                handler(&data);
                count += 1;
            }
        }
        count
    }

    fn is_open(&self) -> bool {
        self.open.lock().map(|g| *g).unwrap_or(false)
    }

    fn close(&mut self) -> io::Result<()> {
        if let Ok(mut open) = self.open.lock() {
            *open = false;
        }
        Ok(())
    }

    fn peer_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_rudp_server_multi_client() {
        // Find two free ports (one for data, one for NAK)
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr = socket.local_addr().unwrap();
        drop(socket);
        // Also reserve the NAK port
        let nak_socket = UdpSocket::bind(SocketAddr::new(server_addr.ip(), server_addr.port() + 1)).unwrap();
        drop(nak_socket);

        let server_addr_clone = server_addr;
        let server_thread = thread::spawn(move || {
            let mut server = RudpServer::bind(server_addr_clone).unwrap();

            let mut clients: Vec<RudpServerClient> = Vec::new();
            let mut messages_received = 0;

            // Wait for clients and messages
            for _ in 0..200 {
                // Accept new clients
                while let Ok(Some(client)) = server.accept() {
                    clients.push(client);
                }

                // Receive from all clients and echo back
                for client in &mut clients {
                    let mut received: Vec<Vec<u8>> = Vec::new();
                    client.receive(|data| {
                        received.push(data.to_vec());
                    });
                    for data in received {
                        messages_received += 1;
                        let mut response = b"echo:".to_vec();
                        response.extend_from_slice(&data);
                        let _ = client.send(&response);
                    }
                }

                if messages_received >= 2 {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }

            (clients.len(), messages_received)
        });

        thread::sleep(Duration::from_millis(50));

        // Helper to build reliable packet with kaos-rudp header
        fn build_reliable_packet(seq: u64, data: &[u8]) -> Vec<u8> {
            let mut header = ReliableUdpHeader::new(0, seq, MessageType::Data, data.len() as u16);
            header.calculate_checksum(data);
            let mut packet = Vec::with_capacity(ReliableUdpHeader::SIZE + data.len());
            packet.extend_from_slice(bytemuck::bytes_of(&header));
            packet.extend_from_slice(data);
            packet
        }

        // Helper to extract payload from reliable packet
        fn extract_payload(packet: &[u8]) -> Option<Vec<u8>> {
            if packet.len() < ReliableUdpHeader::SIZE {
                return None;
            }
            ReliableUdpHeader::from_packet_with_payload_check(packet)
                .map(|(_, payload)| payload.to_vec())
        }

        // Client 1 - send with kaos-rudp header
        let client1 = UdpSocket::bind("127.0.0.1:0").unwrap();
        client1.set_nonblocking(true).unwrap();
        let packet1 = build_reliable_packet(0, b"hello from client1");
        client1.send_to(&packet1, server_addr).unwrap();

        // Client 2 - send with kaos-rudp header
        let client2 = UdpSocket::bind("127.0.0.1:0").unwrap();
        client2.set_nonblocking(true).unwrap();
        let packet2 = build_reliable_packet(0, b"hello from client2");
        client2.send_to(&packet2, server_addr).unwrap();

        // Wait for responses
        thread::sleep(Duration::from_millis(100));

        let mut buf1 = [0u8; 256];
        let mut buf2 = [0u8; 256];

        let mut response1 = None;
        let mut response2 = None;

        for _ in 0..50 {
            if response1.is_none() {
                if let Ok((len, _)) = client1.recv_from(&mut buf1) {
                    // Extract payload from reliable packet
                    response1 = extract_payload(&buf1[..len]);
                }
            }
            if response2.is_none() {
                if let Ok((len, _)) = client2.recv_from(&mut buf2) {
                    response2 = extract_payload(&buf2[..len]);
                }
            }
            if response1.is_some() && response2.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let (client_count, msg_count) = server_thread.join().unwrap();

        assert_eq!(client_count, 2, "Should have 2 clients");
        assert_eq!(msg_count, 2, "Should have received 2 messages");
        assert_eq!(response1, Some(b"echo:hello from client1".to_vec()));
        assert_eq!(response2, Some(b"echo:hello from client2".to_vec()));
    }

    #[test]
    fn test_ws_transport_server_client() {
        // Find a free port
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let addr_clone = addr;
        let server_thread = thread::spawn(move || {
            let mut server = WsServerTransport::bind(addr_clone).unwrap();
            server.set_nonblocking(false).unwrap();

            // Accept one client
            let mut client = server.accept().unwrap().unwrap();

            // Receive message
            let mut received = Vec::new();
            for _ in 0..100 {
                client.receive(|data| {
                    received.extend_from_slice(data);
                });
                if !received.is_empty() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }

            // Echo back
            client.send(&received).unwrap();
            client.close().unwrap();

            received
        });

        thread::sleep(Duration::from_millis(50));

        // Client connects
        let mut client = WsClientTransport::connect(&format!("ws://{}", addr)).unwrap();
        client.send(b"hello kaosnet").unwrap();

        // Receive response
        let mut response = Vec::new();
        for _ in 0..100 {
            client.receive(|data| {
                response.extend_from_slice(data);
            });
            if !response.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(response, b"hello kaosnet");
        client.close().unwrap();

        let server_received = server_thread.join().unwrap();
        assert_eq!(server_received, b"hello kaosnet");
    }
}
