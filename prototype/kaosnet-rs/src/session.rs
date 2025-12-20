//! Session management for KaosNet client.

use crate::types::SessionData;
use std::time::{SystemTime, UNIX_EPOCH};

/// A user session with authentication token.
#[derive(Debug, Clone)]
pub struct Session {
    /// JWT access token.
    pub token: String,
    /// Refresh token for renewing the session.
    pub refresh_token: Option<String>,
    /// User ID.
    pub user_id: String,
    /// Username (if set).
    pub username: Option<String>,
    /// Token expiration time (Unix timestamp in seconds).
    pub expires_at: i64,
    /// Whether this is a newly created account.
    pub created: bool,
}

impl Session {
    /// Create a session from authentication response data.
    pub fn from_data(data: SessionData, new_account: bool) -> Self {
        Self {
            token: data.token,
            refresh_token: data.refresh_token,
            user_id: data.user_id,
            username: data.username,
            expires_at: data.expires_at,
            created: new_account,
        }
    }

    /// Check if the session token has expired.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        now >= self.expires_at
    }

    /// Check if the session will expire within the given number of seconds.
    pub fn expires_within(&self, seconds: i64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        now + seconds >= self.expires_at
    }

    /// Get the authorization header value.
    pub fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    /// Check if session can be refreshed.
    pub fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }

    /// Get display name (username or user_id).
    pub fn display_name(&self) -> &str {
        self.username.as_deref().unwrap_or(&self.user_id)
    }
}

impl std::fmt::Display for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Session {{ user: {}, expires_in: {}s }}",
            self.display_name(),
            self.expires_at - SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        )
    }
}
