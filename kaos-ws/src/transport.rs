//! WebSocket transport implementation.

use std::io;
use std::net::TcpStream;

use tungstenite::protocol::WebSocket;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, client};

use crate::Transport;

/// WebSocket transport - handles both client and server connections
pub enum WsTransport {
    /// Client connection (may have TLS)
    Client {
        ws: WebSocket<MaybeTlsStream<TcpStream>>,
        closed: bool,
    },
    /// Server connection (plain TCP)
    Server {
        ws: WebSocket<TcpStream>,
        closed: bool,
    },
}

impl WsTransport {
    /// Connect to a WebSocket server
    pub fn connect(url: &str) -> crate::Result<Self> {
        let (ws, _response) = client::connect(url)?;

        // Set non-blocking for game loop compatibility
        if let MaybeTlsStream::Plain(ref stream) = ws.get_ref() {
            stream.set_nonblocking(true)?;
        }

        Ok(Self::Client { ws, closed: false })
    }

    /// Create from server-accepted socket
    pub(crate) fn from_server_socket(ws: WebSocket<TcpStream>) -> Self {
        Self::Server { ws, closed: false }
    }

    /// Set blocking mode
    pub fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
        match self {
            Self::Client { ws, .. } => {
                if let MaybeTlsStream::Plain(ref stream) = ws.get_ref() {
                    stream.set_nonblocking(nonblocking)?;
                }
            }
            Self::Server { ws, .. } => {
                ws.get_ref().set_nonblocking(nonblocking)?;
            }
        }
        Ok(())
    }

    fn is_closed(&self) -> bool {
        match self {
            Self::Client { closed, .. } => *closed,
            Self::Server { closed, .. } => *closed,
        }
    }

    fn set_closed(&mut self) {
        match self {
            Self::Client { closed, .. } => *closed = true,
            Self::Server { closed, .. } => *closed = true,
        }
    }

    fn can_write(&self) -> bool {
        match self {
            Self::Client { ws, .. } => ws.can_write(),
            Self::Server { ws, .. } => ws.can_write(),
        }
    }
}

impl Transport for WsTransport {
    fn send(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.is_closed() {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "connection closed"));
        }

        let msg = Message::Binary(data.to_vec().into());
        let result = match self {
            Self::Client { ws, .. } => ws.send(msg),
            Self::Server { ws, .. } => ws.send(msg),
        };

        result.map_err(ws_to_io)?;
        Ok(data.len())
    }

    fn receive<F: FnMut(&[u8])>(&mut self, mut handler: F) -> usize {
        if self.is_closed() {
            return 0;
        }

        let mut count = 0;

        loop {
            let read_result = match self {
                Self::Client { ws, .. } => ws.read(),
                Self::Server { ws, .. } => ws.read(),
            };

            match read_result {
                Ok(msg) => {
                    match msg {
                        Message::Binary(data) => {
                            handler(&data);
                            count += 1;
                        }
                        Message::Text(text) => {
                            handler(text.as_bytes());
                            count += 1;
                        }
                        Message::Ping(data) => {
                            let pong = Message::Pong(data);
                            let _ = match self {
                                Self::Client { ws, .. } => ws.send(pong),
                                Self::Server { ws, .. } => ws.send(pong),
                            };
                        }
                        Message::Pong(_) => {}
                        Message::Close(_) => {
                            self.set_closed();
                            break;
                        }
                        Message::Frame(_) => {}
                    }
                }
                Err(tungstenite::Error::Io(ref e)) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(_) => {
                    self.set_closed();
                    break;
                }
            }
        }

        count
    }

    fn flush(&mut self) -> io::Result<()> {
        let result = match self {
            Self::Client { ws, .. } => ws.flush(),
            Self::Server { ws, .. } => ws.flush(),
        };
        result.map_err(ws_to_io)
    }

    fn is_open(&self) -> bool {
        !self.is_closed() && self.can_write()
    }

    fn close(&mut self) -> io::Result<()> {
        if !self.is_closed() {
            let _ = match self {
                Self::Client { ws, .. } => ws.close(None),
                Self::Server { ws, .. } => ws.close(None),
            };
            self.set_closed();
        }
        Ok(())
    }
}

fn ws_to_io(e: tungstenite::Error) -> io::Error {
    match e {
        tungstenite::Error::Io(io_err) => io_err,
        tungstenite::Error::ConnectionClosed => {
            io::Error::new(io::ErrorKind::ConnectionReset, "WebSocket connection closed")
        }
        tungstenite::Error::AlreadyClosed => {
            io::Error::new(io::ErrorKind::NotConnected, "WebSocket already closed")
        }
        other => io::Error::new(io::ErrorKind::Other, other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::WouldBlock, "would block");
        let ws_err = tungstenite::Error::Io(io_err);
        let converted = ws_to_io(ws_err);
        assert_eq!(converted.kind(), io::ErrorKind::WouldBlock);
    }
}
