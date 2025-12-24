//! Adapter for MuxRudpServer with poll-based API
//!
//! Wraps MuxRudpServer to provide an API similar to RudpServer,
//! making it easy to integrate into existing game servers.
//!
//! # Example
//!
//! ```rust,ignore
//! use kaos_rudp::MuxRudpAdapter;
//!
//! let mut server = MuxRudpAdapter::bind("0.0.0.0:7354", 0x01)?;
//!
//! loop {
//!     server.poll();
//!
//!     // Accept new clients (just like RudpServer)
//!     while let Some(addr) = server.accept() {
//!         println!("Client connected: {}", addr);
//!     }
//!
//!     // Receive messages (just like RudpServer)
//!     for addr in server.clients().collect::<Vec<_>>() {
//!         server.receive(&addr, |data| {
//!             println!("Got {} bytes", data.len());
//!         });
//!     }
//!
//!     // Send messages (just like RudpServer)
//!     server.send(&client_addr, b"hello")?;
//! }
//! ```

use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};

use crate::mux::{GameHandler, MuxRudpServer};

/// Event types collected by the adapter
#[derive(Debug, Clone)]
enum MuxEvent {
    Connect(SocketAddr),
    Message(SocketAddr, Vec<u8>),
    Disconnect(SocketAddr),
}

/// Internal handler that collects events into a queue
struct QueuedGameHandler {
    events: Arc<Mutex<Vec<MuxEvent>>>,
}

impl QueuedGameHandler {
    fn new(events: Arc<Mutex<Vec<MuxEvent>>>) -> Self {
        Self { events }
    }
}

impl GameHandler for QueuedGameHandler {
    fn on_connect(&mut self, client: SocketAddr) {
        if let Ok(mut events) = self.events.lock() {
            events.push(MuxEvent::Connect(client));
        }
    }

    fn on_message(&mut self, client: SocketAddr, data: &[u8]) {
        if let Ok(mut events) = self.events.lock() {
            events.push(MuxEvent::Message(client, data.to_vec()));
        }
    }

    fn on_disconnect(&mut self, client: SocketAddr) {
        if let Ok(mut events) = self.events.lock() {
            events.push(MuxEvent::Disconnect(client));
        }
    }
}

/// Adapter that wraps MuxRudpServer with a poll-based API like RudpServer.
///
/// This allows existing game servers to use MuxRudpServer without changing
/// their poll-based architecture.
pub struct MuxRudpAdapter {
    server: MuxRudpServer,
    game_id: u8,
    events: Arc<Mutex<Vec<MuxEvent>>>,
    /// Pending new clients
    pending_accepts: Vec<SocketAddr>,
    /// Per-client message queues
    client_messages: HashMap<SocketAddr, Vec<Vec<u8>>>,
    /// Connected clients
    clients: Vec<SocketAddr>,
}

impl MuxRudpAdapter {
    /// Bind to an address and create a new multiplexed RUDP server adapter.
    ///
    /// The `game_id` is the 1-byte prefix that clients must include in packets.
    pub fn bind<A: ToSocketAddrs>(addr: A, game_id: u8) -> io::Result<Self> {
        Self::bind_with_window(addr, game_id, 256)
    }

    /// Bind with custom window size.
    pub fn bind_with_window<A: ToSocketAddrs>(
        addr: A,
        game_id: u8,
        window_size: usize,
    ) -> io::Result<Self> {
        let mut server = MuxRudpServer::bind_with_window(addr, window_size)?;
        let events = Arc::new(Mutex::new(Vec::new()));

        // Register handler for this game_id
        server.register(game_id, Box::new(QueuedGameHandler::new(Arc::clone(&events))));

        Ok(Self {
            server,
            game_id,
            events,
            pending_accepts: Vec::new(),
            client_messages: HashMap::new(),
            clients: Vec::new(),
        })
    }

    /// Get server's local address.
    pub fn local_addr(&self) -> SocketAddr {
        self.server.local_addr()
    }

    /// Get the game_id this adapter is handling.
    pub fn game_id(&self) -> u8 {
        self.game_id
    }

    /// Get number of connected clients.
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Get iterator over client addresses.
    pub fn clients(&self) -> impl Iterator<Item = &SocketAddr> {
        self.clients.iter()
    }

    /// Check if a client is connected.
    pub fn has_client(&self, addr: &SocketAddr) -> bool {
        self.clients.contains(addr)
    }

    /// Accept a pending new client (returns addr if any).
    pub fn accept(&mut self) -> Option<SocketAddr> {
        self.pending_accepts.pop()
    }

    /// Poll for incoming packets and process events.
    ///
    /// This must be called regularly in the game loop.
    pub fn poll(&mut self) {
        // Poll the underlying MuxRudpServer
        self.server.poll();

        // Process collected events
        let events: Vec<MuxEvent> = {
            let mut events_guard = self.events.lock().unwrap();
            std::mem::take(&mut *events_guard)
        };

        for event in events {
            match event {
                MuxEvent::Connect(addr) => {
                    if !self.clients.contains(&addr) {
                        self.clients.push(addr);
                        self.client_messages.insert(addr, Vec::new());
                        self.pending_accepts.push(addr);
                    }
                }
                MuxEvent::Message(addr, data) => {
                    if let Some(queue) = self.client_messages.get_mut(&addr) {
                        queue.push(data);
                    }
                }
                MuxEvent::Disconnect(addr) => {
                    self.clients.retain(|a| *a != addr);
                    self.client_messages.remove(&addr);
                }
            }
        }
    }

    /// Call tick on the game handler.
    pub fn tick(&mut self) {
        self.server.tick();
    }

    /// Receive data from a client (delivers in-order).
    ///
    /// Returns the number of messages delivered.
    pub fn receive<F: FnMut(&[u8])>(&mut self, client_addr: &SocketAddr, mut handler: F) -> usize {
        let mut count = 0;
        if let Some(queue) = self.client_messages.get_mut(client_addr) {
            for msg in queue.drain(..) {
                handler(&msg);
                count += 1;
            }
        }
        count
    }

    /// Send data to a client.
    pub fn send(&mut self, client_addr: &SocketAddr, data: &[u8]) -> io::Result<u64> {
        self.server.send(client_addr, data)
    }

    /// Broadcast data to all clients of this game.
    pub fn broadcast(&mut self, data: &[u8]) -> usize {
        self.server.broadcast(self.game_id, data)
    }

    /// Disconnect a client.
    pub fn disconnect(&mut self, client_addr: &SocketAddr) {
        self.server.disconnect(client_addr);
        self.clients.retain(|a| a != client_addr);
        self.client_messages.remove(client_addr);
    }

    /// Register an additional game handler for a different game_id.
    ///
    /// This allows running multiple games on the same port.
    pub fn register_game(&mut self, game_id: u8, handler: Box<dyn GameHandler>) {
        self.server.register(game_id, handler);
    }

    /// Get the underlying MuxRudpServer for advanced usage.
    pub fn inner(&self) -> &MuxRudpServer {
        &self.server
    }

    /// Get mutable reference to the underlying MuxRudpServer.
    pub fn inner_mut(&mut self) -> &mut MuxRudpServer {
        &mut self.server
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_bind() {
        let adapter = MuxRudpAdapter::bind("127.0.0.1:0", 0x01).unwrap();
        assert_eq!(adapter.client_count(), 0);
        assert_eq!(adapter.game_id(), 0x01);
        assert!(adapter.local_addr().port() > 0);
    }

    #[test]
    fn test_adapter_poll_empty() {
        let mut adapter = MuxRudpAdapter::bind("127.0.0.1:0", 0x01).unwrap();
        adapter.poll(); // Should not panic
        assert_eq!(adapter.client_count(), 0);
        assert!(adapter.accept().is_none());
    }

    #[test]
    fn test_adapter_has_client() {
        let adapter = MuxRudpAdapter::bind("127.0.0.1:0", 0x01).unwrap();
        let fake_addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();
        assert!(!adapter.has_client(&fake_addr));
    }

    #[test]
    fn test_adapter_send_unknown_client() {
        let mut adapter = MuxRudpAdapter::bind("127.0.0.1:0", 0x01).unwrap();
        let unknown_addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();
        let result = adapter.send(&unknown_addr, b"test");
        assert!(result.is_err());
    }

    #[test]
    fn test_adapter_register_multiple_games() {
        use crate::GameHandler;

        struct DummyHandler;
        impl GameHandler for DummyHandler {
            fn on_connect(&mut self, _: SocketAddr) {}
            fn on_message(&mut self, _: SocketAddr, _: &[u8]) {}
            fn on_disconnect(&mut self, _: SocketAddr) {}
        }

        let mut adapter = MuxRudpAdapter::bind("127.0.0.1:0", 0x01).unwrap();

        // Register additional game on same port (should not panic)
        adapter.register_game(0x02, Box::new(DummyHandler));

        // Primary game_id should still be 0x01
        assert_eq!(adapter.game_id(), 0x01);
    }

    #[test]
    fn test_adapter_different_game_ids() {
        // Test that different game names produce different IDs
        let adapter1 = MuxRudpAdapter::bind("127.0.0.1:0", 0xe5).unwrap(); // kaos_io
        let adapter2 = MuxRudpAdapter::bind("127.0.0.1:0", 0xdb).unwrap(); // kaos_asteroids

        assert_ne!(adapter1.game_id(), adapter2.game_id());
        assert_eq!(adapter1.game_id(), 0xe5);
        assert_eq!(adapter2.game_id(), 0xdb);
    }

    #[test]
    fn test_adapter_client_iteration() {
        let adapter = MuxRudpAdapter::bind("127.0.0.1:0", 0x01).unwrap();

        // No clients initially
        let clients: Vec<_> = adapter.clients().collect();
        assert!(clients.is_empty());
    }

    #[test]
    fn test_adapter_receive_empty() {
        let mut adapter = MuxRudpAdapter::bind("127.0.0.1:0", 0x01).unwrap();
        let fake_addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();

        // No messages for non-existent client
        let count = adapter.receive(&fake_addr, |_data| {
            panic!("Should not receive any data");
        });
        assert_eq!(count, 0);
    }
}
