//! Error types for KaosNet client.

use thiserror::Error;

/// Result type for KaosNet operations.
pub type Result<T> = std::result::Result<T, Error>;

/// KaosNet client errors.
#[derive(Debug, Error)]
pub enum Error {
    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// WebSocket error.
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// Server returned an error response.
    #[error("Server error: {message}")]
    Server { message: String, code: Option<i32> },

    /// Authentication failed.
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// Session expired.
    #[error("Session expired")]
    SessionExpired,

    /// Not connected.
    #[error("Not connected to server")]
    NotConnected,

    /// Connection closed.
    #[error("Connection closed")]
    ConnectionClosed,

    /// Timeout waiting for response.
    #[error("Request timed out")]
    Timeout,

    /// Invalid URL.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Resource not found.
    #[error("Not found: {0}")]
    NotFound(String),
}

impl Error {
    /// Create a server error from a message.
    pub fn server(message: impl Into<String>) -> Self {
        Self::Server {
            message: message.into(),
            code: None,
        }
    }

    /// Create a server error with a code.
    pub fn server_with_code(message: impl Into<String>, code: i32) -> Self {
        Self::Server {
            message: message.into(),
            code: Some(code),
        }
    }

    /// Alias for server_with_code.
    pub fn server_code(message: impl Into<String>, code: i32) -> Self {
        Self::server_with_code(message, code)
    }
}
