//! Type definitions for KaosNet client.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Authentication
// ============================================================================

/// Authentication response from server.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthResponse {
    pub session: SessionData,
    /// Whether this is a newly created account.
    #[serde(default)]
    pub new_account: bool,
}

/// Session data from authentication.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionData {
    pub token: String,
    pub refresh_token: Option<String>,
    pub user_id: String,
    pub username: Option<String>,
    pub expires_at: i64,
    /// Time when the session was created (Unix timestamp).
    #[serde(default)]
    pub created_at: i64,
}

/// User account information.
#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    pub id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    pub devices: Vec<DeviceInfo>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
    pub disabled: bool,
}

/// Device linked to an account.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub linked_at: i64,
}

/// Response from link/unlink operations.
#[derive(Debug, Clone, Deserialize)]
pub struct LinkResponse {
    pub success: bool,
}

// ============================================================================
// Storage
// ============================================================================

/// A storage object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageObject<T = serde_json::Value> {
    pub collection: String,
    pub key: String,
    pub user_id: String,
    pub value: T,
    pub version: String,
    pub permission_read: i32,
    pub permission_write: i32,
    pub create_time: i64,
    pub update_time: i64,
}

/// Request to write a storage object.
#[derive(Debug, Clone, Serialize)]
pub struct StorageWriteRequest<T = serde_json::Value> {
    pub collection: String,
    pub key: String,
    pub value: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_read: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_write: Option<i32>,
}

/// Request to read storage objects.
#[derive(Debug, Clone, Serialize)]
pub struct StorageReadRequest {
    pub collection: String,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Request to delete storage objects.
#[derive(Debug, Clone, Serialize)]
pub struct StorageDeleteRequest {
    pub collection: String,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

// ============================================================================
// Leaderboards
// ============================================================================

/// A leaderboard record.
#[derive(Debug, Clone, Deserialize)]
pub struct LeaderboardRecord {
    pub leaderboard_id: String,
    pub owner_id: String,
    pub username: Option<String>,
    pub score: i64,
    pub subscore: i64,
    pub num_score: i32,
    pub max_num_score: i32,
    pub metadata: Option<serde_json::Value>,
    pub create_time: i64,
    pub update_time: i64,
    pub expiry_time: Option<i64>,
    pub rank: i64,
}

/// List of leaderboard records.
#[derive(Debug, Clone, Deserialize)]
pub struct LeaderboardRecordList {
    pub records: Vec<LeaderboardRecord>,
    pub owner_records: Option<Vec<LeaderboardRecord>>,
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
}

// ============================================================================
// Matchmaker
// ============================================================================

/// Matchmaker ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakerTicket {
    pub id: String,
    pub queue: String,
    pub players: Vec<MatchmakerPlayer>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: i64,
}

/// Player in a matchmaker ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakerPlayer {
    pub user_id: String,
    pub username: String,
    pub skill: f64,
}

/// Matchmaker add response.
#[derive(Debug, Clone, Deserialize)]
pub struct MatchmakerAddResponse {
    pub ticket: MatchmakerTicket,
}

/// Queue statistics.
#[derive(Debug, Clone, Deserialize)]
pub struct QueueStats {
    pub queue: String,
    pub tickets: usize,
    pub players: usize,
    pub longest_wait_secs: u64,
}

// ============================================================================
// Real-time Messages
// ============================================================================

/// Channel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Room,
    Group,
    DirectMessage,
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub channel_id: String,
    pub sender_id: String,
    pub sender_username: String,
    pub content: String,
    pub code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A match (game room).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub match_id: String,
    pub authoritative: bool,
    pub label: Option<String>,
    pub size: i32,
    pub tick_rate: i32,
    pub handler_name: Option<String>,
}

/// Match data sent between players.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchData {
    pub match_id: String,
    pub presence: UserPresence,
    pub op_code: i64,
    pub data: Vec<u8>,
}

/// User presence information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    pub user_id: String,
    pub session_id: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

// ============================================================================
// Notifications
// ============================================================================

/// A notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub subject: String,
    pub content: serde_json::Value,
    pub code: i32,
    pub sender_id: String,
    pub create_time: i64,
    pub persistent: bool,
}

// ============================================================================
// Social
// ============================================================================

/// A friend relationship.
#[derive(Debug, Clone, Deserialize)]
pub struct Friend {
    pub user_id: String,
    pub username: String,
    pub state: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A group.
#[derive(Debug, Clone, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: String,
    pub creator_id: String,
    pub open: bool,
    pub member_count: i32,
    pub max_members: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A group member.
#[derive(Debug, Clone, Deserialize)]
pub struct GroupMember {
    pub user_id: String,
    pub username: String,
    pub state: i32,
    pub joined_at: i64,
}
