//! WebSocket server for accepting connections.

use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};

use tungstenite::accept;
use tungstenite::protocol::WebSocket;

use crate::{Result, WsError, WsTransport};

/// WebSocket server that accepts incoming connections
pub struct WsServer {
    listener: TcpListener,
}

impl WsServer {
    /// Bind to an address
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener })
    }

    /// Set blocking mode for accept
    pub fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
        self.listener.set_nonblocking(nonblocking)
    }

    /// Get local address
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Accept a new WebSocket connection (non-blocking)
    pub fn accept(&mut self) -> Result<Option<WsTransport>> {
        match self.listener.accept() {
            Ok((stream, _addr)) => {
                stream.set_nonblocking(false)?; // Blocking for handshake

                let ws = accept(stream).map_err(|e| {
                    // Convert handshake error to WsError
                    match e {
                        tungstenite::HandshakeError::Interrupted(_) => WsError::HandshakeFailed,
                        tungstenite::HandshakeError::Failure(err) => WsError::WebSocket(err),
                    }
                })?;

                // Set non-blocking after handshake
                ws.get_ref().set_nonblocking(true)?;

                Ok(Some(WsTransport::from_server_socket(ws)))
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Accept raw WebSocket (for Autobahn testing)
    pub fn accept_raw(&mut self) -> Result<Option<WebSocket<TcpStream>>> {
        match self.listener.accept() {
            Ok((stream, _addr)) => {
                stream.set_nonblocking(false)?;

                let ws = accept(stream).map_err(|e| {
                    match e {
                        tungstenite::HandshakeError::Interrupted(_) => WsError::HandshakeFailed,
                        tungstenite::HandshakeError::Failure(err) => WsError::WebSocket(err),
                    }
                })?;

                Ok(Some(ws))
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }
}

/// Iterator over incoming connections
pub struct Incoming<'a> {
    server: &'a mut WsServer,
}

impl<'a> Iterator for Incoming<'a> {
    type Item = Result<WsTransport>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.server.accept() {
            Ok(Some(transport)) => Some(Ok(transport)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl WsServer {
    /// Returns an iterator over incoming connections
    pub fn incoming(&mut self) -> Incoming<'_> {
        Incoming { server: self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_bind() {
        let server = WsServer::bind("127.0.0.1:0").unwrap();
        assert!(server.local_addr().unwrap().port() > 0);
    }

    #[test]
    fn test_accept_no_client() {
        let mut server = WsServer::bind("127.0.0.1:0").unwrap();
        // Should return None (no clients)
        let result = server.accept().unwrap();
        assert!(result.is_none());
    }
}
