//! Shared types for console API.

use crate::console::auth::Role;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Console user account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: Role,
    pub created_at: i64,
    pub last_login: Option<i64>,
    pub disabled: bool,
}

/// API key for service authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub key_prefix: String,
    pub scopes: ApiKeyScope,
    pub created_by: Uuid,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub last_used: Option<i64>,
    pub request_count: u64,
    pub disabled: bool,
}

impl ApiKey {
    /// Check if key is expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            now > expires_at
        } else {
            false
        }
    }
}

/// API key permission scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ApiKeyScope(u32);

impl ApiKeyScope {
    pub const READ_STATUS: Self = Self(1 << 0);
    pub const READ_SESSIONS: Self = Self(1 << 1);
    pub const READ_ROOMS: Self = Self(1 << 2);
    pub const KICK_SESSIONS: Self = Self(1 << 3);
    pub const TERMINATE_ROOMS: Self = Self(1 << 4);
    pub const EXECUTE_RPC: Self = Self(1 << 5);
    pub const READ_CONFIG: Self = Self(1 << 6);
    pub const READ_LUA: Self = Self(1 << 7);

    pub const READ_ALL: Self = Self(
        Self::READ_STATUS.0
            | Self::READ_SESSIONS.0
            | Self::READ_ROOMS.0
            | Self::READ_CONFIG.0
            | Self::READ_LUA.0,
    );

    pub const FULL: Self = Self(0xFFFF_FFFF);

    /// Create empty scope.
    pub fn empty() -> Self {
        Self(0)
    }

    /// Check if scope contains another.
    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Add scope.
    pub fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check permission.
    pub fn has_permission(&self, permission: crate::console::auth::Permission) -> bool {
        use crate::console::auth::Permission;
        match permission {
            Permission::ViewStatus => self.contains(Self::READ_STATUS),
            Permission::ViewSessions => self.contains(Self::READ_SESSIONS),
            Permission::ViewRooms => self.contains(Self::READ_ROOMS),
            Permission::ViewLua => self.contains(Self::READ_LUA),
            Permission::ViewConfig => self.contains(Self::READ_CONFIG),
            Permission::KickSession => self.contains(Self::KICK_SESSIONS),
            Permission::TerminateRoom => self.contains(Self::TERMINATE_ROOMS),
            Permission::ExecuteRpc => self.contains(Self::EXECUTE_RPC),
            Permission::ManageAccounts => false, // API keys can never manage accounts
            Permission::ManageApiKeys => false,  // API keys can never manage API keys
            // Service management - API keys don't have these permissions
            Permission::ManagePlayers => false,
            Permission::ManageLeaderboards => false,
            Permission::ManageTournaments => false,
            Permission::ManageStorage => false,
            Permission::ManageSocial => false,
            Permission::ManageChat => false,
            Permission::ManageMatchmaker => false,
            Permission::ManageNotifications => false,
        }
    }

    /// Get scope names.
    pub fn to_vec(&self) -> Vec<&'static str> {
        let mut scopes = Vec::new();
        if self.contains(Self::READ_STATUS) {
            scopes.push("read:status");
        }
        if self.contains(Self::READ_SESSIONS) {
            scopes.push("read:sessions");
        }
        if self.contains(Self::READ_ROOMS) {
            scopes.push("read:rooms");
        }
        if self.contains(Self::READ_CONFIG) {
            scopes.push("read:config");
        }
        if self.contains(Self::READ_LUA) {
            scopes.push("read:lua");
        }
        if self.contains(Self::KICK_SESSIONS) {
            scopes.push("write:kick");
        }
        if self.contains(Self::TERMINATE_ROOMS) {
            scopes.push("write:terminate");
        }
        if self.contains(Self::EXECUTE_RPC) {
            scopes.push("write:rpc");
        }
        scopes
    }

    /// Parse from scope names.
    pub fn from_vec(names: &[&str]) -> Self {
        let mut scope = Self::empty();
        for name in names {
            match *name {
                "read:status" => scope = scope.with(Self::READ_STATUS),
                "read:sessions" => scope = scope.with(Self::READ_SESSIONS),
                "read:rooms" => scope = scope.with(Self::READ_ROOMS),
                "read:config" => scope = scope.with(Self::READ_CONFIG),
                "read:lua" => scope = scope.with(Self::READ_LUA),
                "write:kick" => scope = scope.with(Self::KICK_SESSIONS),
                "write:terminate" => scope = scope.with(Self::TERMINATE_ROOMS),
                "write:rpc" => scope = scope.with(Self::EXECUTE_RPC),
                "read:all" => scope = scope.with(Self::READ_ALL),
                "full" | "*" => scope = Self::FULL,
                _ => {}
            }
        }
        scope
    }
}

// API request/response types

/// Login request.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: i64,
    pub user: AccountInfo,
}

/// Account info (public).
#[derive(Debug, Serialize)]
pub struct AccountInfo {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

impl From<&Account> for AccountInfo {
    fn from(account: &Account) -> Self {
        Self {
            id: account.id,
            username: account.username.clone(),
            role: account.role.as_str().to_string(),
        }
    }
}

/// Create account request.
#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub username: String,
    pub password: String,
    pub role: String,
}

/// Update account request.
#[derive(Debug, Deserialize)]
pub struct UpdateAccountRequest {
    pub username: Option<String>,
    pub role: Option<String>,
    pub disabled: Option<bool>,
}

/// Change password request.
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub password: String,
}

/// Create API key request.
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<u32>,
}

/// Create API key response.
#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub key: String, // Only shown once!
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<i64>,
}

/// API key info (public).
#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub last_used: Option<i64>,
    pub request_count: u64,
    pub disabled: bool,
}

impl From<&ApiKey> for ApiKeyInfo {
    fn from(key: &ApiKey) -> Self {
        Self {
            id: key.id,
            name: key.name.clone(),
            key_prefix: key.key_prefix.clone(),
            scopes: key.scopes.to_vec().into_iter().map(String::from).collect(),
            created_at: key.created_at,
            expires_at: key.expires_at,
            last_used: key.last_used,
            request_count: key.request_count,
            disabled: key.disabled,
        }
    }
}

/// Server status response.
#[derive(Debug, Serialize)]
pub struct ServerStatus {
    pub version: String,
    pub uptime_secs: u64,
    pub sessions: SessionStats,
    pub rooms: RoomStats,
}

/// Session statistics.
#[derive(Debug, Serialize)]
pub struct SessionStats {
    pub total: u32,
    pub connecting: u32,
    pub connected: u32,
    pub authenticated: u32,
}

/// Room statistics.
#[derive(Debug, Serialize)]
pub struct RoomStats {
    pub total: u32,
    pub players: u32,
}

/// Session info.
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub id: u64,
    pub address: String,
    pub state: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub room_id: Option<String>,
    pub connected_at: i64,
    pub last_heartbeat: i64,
}

/// Room info.
#[derive(Debug, Serialize)]
pub struct RoomInfo {
    pub id: String,
    pub label: Option<String>,
    pub module: Option<String>,
    pub state: String,
    pub tick_rate: u32,
    pub player_count: u32,
    pub max_players: u32,
    pub created_at: i64,
}

/// Player info for room details.
#[derive(Debug, Serialize)]
pub struct RoomPlayerInfo {
    pub session_id: u64,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub address: String,
}

/// Paginated list response.
#[derive(Debug, Serialize)]
pub struct PaginatedList<T> {
    pub items: Vec<T>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

/// Metrics data for Console UI.
#[derive(Debug, Serialize)]
pub struct MetricsData {
    pub uptime_seconds: i64,
    pub sessions_active: i64,
    pub sessions_total: i64,
    pub sessions_by_state: std::collections::HashMap<String, i64>,
    pub rooms_active: i64,
    pub rooms_total: i64,
    pub websocket_connections: i64,
    pub bytes_received_total: i64,
    pub bytes_sent_total: i64,
    pub udp_packets_received_total: i64,
    pub udp_packets_sent_total: i64,
    pub chat_messages_total: i64,
    pub leaderboard_submissions_total: i64,
    pub matchmaker_queue_size: i64,
    pub matchmaker_matches_total: i64,
    pub notifications_total: i64,
}
