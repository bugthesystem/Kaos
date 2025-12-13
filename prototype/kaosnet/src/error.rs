//! Error types for KaosNet.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, KaosNetError>;

#[derive(Error, Debug)]
pub enum KaosNetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {reason}")]
    Protocol { reason: String },

    #[error("Session not found: {id}")]
    SessionNotFound { id: u64 },

    #[error("Room not found: {id}")]
    RoomNotFound { id: String },

    #[error("Room full: {id}")]
    RoomFull { id: String },

    #[error("Authentication failed")]
    AuthFailed,

    #[error("Lua error: {0}")]
    #[cfg(feature = "lua")]
    Lua(#[from] mlua::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl KaosNetError {
    pub fn protocol(reason: impl Into<String>) -> Self {
        Self::Protocol {
            reason: reason.into(),
        }
    }

    pub fn session_not_found(id: u64) -> Self {
        Self::SessionNotFound { id }
    }

    pub fn room_not_found(id: impl Into<String>) -> Self {
        Self::RoomNotFound { id: id.into() }
    }

    pub fn room_full(id: impl Into<String>) -> Self {
        Self::RoomFull { id: id.into() }
    }
}
