//! # kaos-ws
//!
//! WebSocket transport for Kaos messaging.
//!
//! ## Features
//!
//! - **RFC 6455 compliant**: Full WebSocket protocol support via tungstenite
//! - **Composable**: Implements Transport trait for interchangeability with RUDP
//! - **Binary frames**: Optimized for game message payloads
//! - **Non-blocking**: Designed for game server tick loops
//!
//! ## Example
//!
//! ```rust,ignore
//! use kaos_ws::{WsServer, WsTransport};
//!
//! // Server accepts connections
//! let mut server = WsServer::bind("127.0.0.1:9000")?;
//!
//! // Accept a client
//! if let Some(transport) = server.accept()? {
//!     transport.send(b"hello")?;
//! }
//! ```

mod error;
mod server;
mod transport;

pub use error::{WsError, Result};
pub use server::WsServer;
pub use transport::WsTransport;

/// Transport trait - matches kaos-rudp interface
pub trait Transport {
    /// Send data, returns bytes sent
    fn send(&mut self, data: &[u8]) -> std::io::Result<usize>;

    /// Receive into callback, returns count received
    fn receive<F: FnMut(&[u8])>(&mut self, handler: F) -> usize;

    /// Flush pending operations
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    /// Check if connection is open
    fn is_open(&self) -> bool;

    /// Close the connection
    fn close(&mut self) -> std::io::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_server_client_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let addr_clone = addr;
        let server_thread = thread::spawn(move || {
            let mut server = WsServer::bind(addr_clone).unwrap();
            server.set_nonblocking(false).unwrap();

            // Accept one client (non-blocking set by accept)
            let mut transport = server.accept().unwrap().unwrap();

            // Poll for message (non-blocking)
            let mut received = Vec::new();
            for _ in 0..100 {
                transport.receive(|data| {
                    received.extend_from_slice(data);
                });
                if !received.is_empty() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }

            // Echo back
            transport.send(&received).unwrap();
            transport.close().unwrap();
        });

        // Give server time to start
        thread::sleep(Duration::from_millis(50));

        // Client connects (non-blocking after connect)
        let mut client = WsTransport::connect(&format!("ws://{}", addr)).unwrap();

        // Send message
        client.send(b"hello world").unwrap();

        // Poll for response
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

        assert_eq!(response, b"hello world");
        client.close().unwrap();
        server_thread.join().unwrap();
    }

    #[test]
    fn test_binary_frames() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let addr_clone = addr;
        let server_thread = thread::spawn(move || {
            let mut server = WsServer::bind(addr_clone).unwrap();
            server.set_nonblocking(false).unwrap();

            let mut transport = server.accept().unwrap().unwrap();

            // Poll for message
            let mut received = Vec::new();
            for _ in 0..100 {
                transport.receive(|data| {
                    received.extend_from_slice(data);
                });
                if !received.is_empty() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }

            // Verify binary data integrity
            assert_eq!(received.len(), 256);
            for (i, &byte) in received.iter().enumerate() {
                assert_eq!(byte, i as u8);
            }

            transport.send(&received).unwrap();
            transport.close().unwrap();
        });

        thread::sleep(Duration::from_millis(50));

        let mut client = WsTransport::connect(&format!("ws://{}", addr)).unwrap();

        // Send binary data 0-255
        let data: Vec<u8> = (0u8..=255).collect();
        client.send(&data).unwrap();

        // Poll for response
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

        assert_eq!(response, data);
        client.close().unwrap();
        server_thread.join().unwrap();
    }
}
