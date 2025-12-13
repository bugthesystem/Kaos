//! Error types for kaos-ws.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, WsError>;

#[derive(Error, Debug)]
pub enum WsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Handshake failed")]
    HandshakeFailed,
}

impl WsError {
    pub fn invalid_url(url: impl Into<String>) -> Self {
        Self::InvalidUrl(url.into())
    }
}
