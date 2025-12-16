//! Error types for kaos-http.

use http::StatusCode;
use std::io;

/// Result type for kaos-http operations.
pub type Result<T> = std::result::Result<T, HttpError>;

/// HTTP server errors.
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// Hyper error.
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    /// HTTP error.
    #[error("http error: {0}")]
    Http(#[from] http::Error),

    /// JSON error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid address.
    #[error("invalid address: {0}")]
    InvalidAddr(String),

    /// Route not found.
    #[error("route not found: {method} {path}")]
    NotFound { method: String, path: String },

    /// Method not allowed.
    #[error("method not allowed: {method} {path}")]
    MethodNotAllowed { method: String, path: String },

    /// Bad request.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Unauthorized.
    #[error("unauthorized")]
    Unauthorized,

    /// Forbidden.
    #[error("forbidden")]
    Forbidden,

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl HttpError {
    /// Get HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            HttpError::NotFound { .. } => StatusCode::NOT_FOUND,
            HttpError::MethodNotAllowed { .. } => StatusCode::METHOD_NOT_ALLOWED,
            HttpError::BadRequest(_) => StatusCode::BAD_REQUEST,
            HttpError::Unauthorized => StatusCode::UNAUTHORIZED,
            HttpError::Forbidden => StatusCode::FORBIDDEN,
            HttpError::Json(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
