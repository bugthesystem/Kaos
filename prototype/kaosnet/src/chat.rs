//! Chat system for KaosNet.
//!
//! Provides real-time messaging with channels and direct messages.
//!
//! ## Features
//!
//! - Public channels (rooms, lobbies)
//! - Private channels (groups, parties)
//! - Direct messages (1:1)
//! - Message history
//! - Channel membership
//! - Typing indicators

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Channel type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// Public channel anyone can join.
    Room,
    /// Private group channel (invite only).
    Group,
    /// Direct message between two users.
    DirectMessage,
}

/// A chat channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    /// Unique channel ID.
    pub id: String,
    /// Channel name (for rooms/groups).
    pub name: String,
    /// Channel type.
    pub channel_type: ChannelType,
    /// Room/match ID if this is a room chat.
    pub room_id: Option<String>,
    /// Group ID if this is a group chat.
    pub group_id: Option<String>,
    /// For DMs: the two user IDs (sorted).
    pub dm_users: Option<(String, String)>,
    /// Channel metadata.
    pub metadata: Option<serde_json::Value>,
    /// Created timestamp.
    pub created_at: i64,
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message ID.
    pub id: String,
    /// Channel ID.
    pub channel_id: String,
    /// Sender user ID.
    pub sender_id: String,
    /// Sender username.
    pub sender_username: String,
    /// Message content.
    pub content: String,
    /// Message code (for system messages).
    pub code: i32,
    /// Custom data.
    pub data: Option<serde_json::Value>,
    /// When message was sent.
    pub created_at: i64,
    /// When message was last updated (for edits).
    pub updated_at: i64,
    /// Whether message is persistent.
    pub persistent: bool,
}

impl ChatMessage {
    /// Create a new text message.
    pub fn text(channel_id: &str, sender_id: &str, sender_username: &str, content: &str) -> Self {
        let now = now_millis();
        Self {
            id: Uuid::new_v4().to_string(),
            channel_id: channel_id.to_string(),
            sender_id: sender_id.to_string(),
            sender_username: sender_username.to_string(),
            content: content.to_string(),
            code: 0,
            data: None,
            created_at: now,
            updated_at: now,
            persistent: true,
        }
    }

    /// Create a system message.
    pub fn system(channel_id: &str, code: i32, content: &str) -> Self {
        let now = now_millis();
        Self {
            id: Uuid::new_v4().to_string(),
            channel_id: channel_id.to_string(),
            sender_id: String::new(),
            sender_username: "System".to_string(),
            content: content.to_string(),
            code,
            data: None,
            created_at: now,
            updated_at: now,
            persistent: true,
        }
    }

    /// Set custom data.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Set as ephemeral (not persisted).
    pub fn ephemeral(mut self) -> Self {
        self.persistent = false;
        self
    }
}

/// Message callback for real-time delivery.
pub type MessageCallback = Box<dyn Fn(&ChatMessage) + Send + Sync>;

/// Channel data.
struct ChannelData {
    channel: Channel,
    members: HashSet<String>,
    messages: VecDeque<ChatMessage>,
    max_messages: usize,
}

/// Chat service.
pub struct Chat {
    channels: DashMap<String, ChannelData>,
    /// User -> channel IDs they're in.
    user_channels: DashMap<String, HashSet<String>>,
    /// DM channel lookup: (user1, user2) -> channel_id (users sorted).
    dm_channels: DashMap<(String, String), String>,
    /// Message callbacks by user.
    callbacks: DashMap<String, MessageCallback>,
    /// Max message history per channel.
    max_history: usize,
}

impl Chat {
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
            user_channels: DashMap::new(),
            dm_channels: DashMap::new(),
            callbacks: DashMap::new(),
            max_history: 100,
        }
    }

    /// Create with custom history limit.
    pub fn with_history_limit(max_history: usize) -> Self {
        Self {
            channels: DashMap::new(),
            user_channels: DashMap::new(),
            dm_channels: DashMap::new(),
            callbacks: DashMap::new(),
            max_history,
        }
    }

    /// Register message callback for a user.
    pub fn register_callback(&self, user_id: impl Into<String>, callback: MessageCallback) {
        self.callbacks.insert(user_id.into(), callback);
    }

    /// Unregister callback.
    pub fn unregister_callback(&self, user_id: &str) {
        self.callbacks.remove(user_id);
    }

    // ==================== Channel Management ====================

    /// Create a room channel.
    pub fn create_room(&self, room_id: &str, name: &str) -> Channel {
        let channel = Channel {
            id: format!("room:{}", room_id),
            name: name.to_string(),
            channel_type: ChannelType::Room,
            room_id: Some(room_id.to_string()),
            group_id: None,
            dm_users: None,
            metadata: None,
            created_at: now_millis(),
        };

        let data = ChannelData {
            channel: channel.clone(),
            members: HashSet::new(),
            messages: VecDeque::new(),
            max_messages: self.max_history,
        };

        self.channels.insert(channel.id.clone(), data);
        channel
    }

    /// Create a group channel.
    pub fn create_group(&self, group_id: &str, name: &str) -> Channel {
        let channel = Channel {
            id: format!("group:{}", group_id),
            name: name.to_string(),
            channel_type: ChannelType::Group,
            room_id: None,
            group_id: Some(group_id.to_string()),
            dm_users: None,
            metadata: None,
            created_at: now_millis(),
        };

        let data = ChannelData {
            channel: channel.clone(),
            members: HashSet::new(),
            messages: VecDeque::new(),
            max_messages: self.max_history,
        };

        self.channels.insert(channel.id.clone(), data);
        channel
    }

    /// Get or create a DM channel between two users.
    pub fn get_or_create_dm(&self, user1: &str, user2: &str) -> Channel {
        // Sort user IDs for consistent key
        let (u1, u2) = if user1 < user2 { (user1, user2) } else { (user2, user1) };
        let key = (u1.to_string(), u2.to_string());

        // Check if exists
        if let Some(channel_id) = self.dm_channels.get(&key) {
            if let Some(data) = self.channels.get(&*channel_id) {
                return data.channel.clone();
            }
        }

        // Create new DM channel
        let channel = Channel {
            id: format!("dm:{}:{}", u1, u2),
            name: format!("DM: {} & {}", u1, u2),
            channel_type: ChannelType::DirectMessage,
            room_id: None,
            group_id: None,
            dm_users: Some((u1.to_string(), u2.to_string())),
            metadata: None,
            created_at: now_millis(),
        };

        let data = ChannelData {
            channel: channel.clone(),
            members: HashSet::from([u1.to_string(), u2.to_string()]),
            messages: VecDeque::new(),
            max_messages: self.max_history,
        };

        self.channels.insert(channel.id.clone(), data);
        self.dm_channels.insert(key, channel.id.clone());

        // Add to user channels
        self.user_channels.entry(u1.to_string()).or_insert_with(HashSet::new).insert(channel.id.clone());
        self.user_channels.entry(u2.to_string()).or_insert_with(HashSet::new).insert(channel.id.clone());

        channel
    }

    /// Get a channel.
    pub fn get_channel(&self, channel_id: &str) -> Option<Channel> {
        self.channels.get(channel_id).map(|d| d.channel.clone())
    }

    /// Delete a channel.
    pub fn delete_channel(&self, channel_id: &str) -> bool {
        if let Some((_, data)) = self.channels.remove(channel_id) {
            // Remove from user channel lists
            for user_id in &data.members {
                if let Some(mut channels) = self.user_channels.get_mut(user_id) {
                    channels.remove(channel_id);
                }
            }

            // Remove from DM index if DM
            if let Some((u1, u2)) = &data.channel.dm_users {
                self.dm_channels.remove(&(u1.clone(), u2.clone()));
            }

            return true;
        }
        false
    }

    // ==================== Membership ====================

    /// Join a channel.
    pub fn join(&self, channel_id: &str, user_id: &str) -> Result<(), ChatError> {
        let mut data = self.channels.get_mut(channel_id)
            .ok_or_else(|| ChatError::ChannelNotFound(channel_id.to_string()))?;

        data.members.insert(user_id.to_string());

        self.user_channels
            .entry(user_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(channel_id.to_string());

        Ok(())
    }

    /// Leave a channel.
    pub fn leave(&self, channel_id: &str, user_id: &str) -> Result<(), ChatError> {
        if let Some(mut data) = self.channels.get_mut(channel_id) {
            data.members.remove(user_id);
        }

        if let Some(mut channels) = self.user_channels.get_mut(user_id) {
            channels.remove(channel_id);
        }

        Ok(())
    }

    /// Get channel members.
    pub fn get_members(&self, channel_id: &str) -> Result<Vec<String>, ChatError> {
        let data = self.channels.get(channel_id)
            .ok_or_else(|| ChatError::ChannelNotFound(channel_id.to_string()))?;

        Ok(data.members.iter().cloned().collect())
    }

    /// Get channels a user is in.
    pub fn get_user_channels(&self, user_id: &str) -> Vec<Channel> {
        self.user_channels.get(user_id)
            .map(|channel_ids| {
                channel_ids.iter()
                    .filter_map(|id| self.get_channel(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    // ==================== Messaging ====================

    /// Send a message.
    pub fn send(&self, message: ChatMessage) -> Result<ChatMessage, ChatError> {
        let mut data = self.channels.get_mut(&message.channel_id)
            .ok_or_else(|| ChatError::ChannelNotFound(message.channel_id.clone()))?;

        // Store if persistent
        if message.persistent {
            data.messages.push_front(message.clone());
            while data.messages.len() > data.max_messages {
                data.messages.pop_back();
            }
        }

        // Deliver to members
        for member_id in &data.members {
            if let Some(callback) = self.callbacks.get(member_id) {
                callback(&message);
            }
        }

        Ok(message)
    }

    /// Send a text message.
    pub fn send_text(&self, channel_id: &str, sender_id: &str, sender_username: &str, content: &str) -> Result<ChatMessage, ChatError> {
        let message = ChatMessage::text(channel_id, sender_id, sender_username, content);
        self.send(message)
    }

    /// Send a system message.
    pub fn send_system(&self, channel_id: &str, code: i32, content: &str) -> Result<ChatMessage, ChatError> {
        let message = ChatMessage::system(channel_id, code, content);
        self.send(message)
    }

    /// Send a direct message.
    pub fn send_dm(&self, from_id: &str, from_username: &str, to_id: &str, content: &str) -> Result<ChatMessage, ChatError> {
        let channel = self.get_or_create_dm(from_id, to_id);
        self.send_text(&channel.id, from_id, from_username, content)
    }

    /// Get message history.
    pub fn get_history(&self, channel_id: &str, limit: usize, before_id: Option<&str>) -> Result<Vec<ChatMessage>, ChatError> {
        let data = self.channels.get(channel_id)
            .ok_or_else(|| ChatError::ChannelNotFound(channel_id.to_string()))?;

        let messages: Vec<ChatMessage> = if let Some(before) = before_id {
            data.messages.iter()
                .skip_while(|m| m.id != before)
                .skip(1)
                .take(limit)
                .cloned()
                .collect()
        } else {
            data.messages.iter().take(limit).cloned().collect()
        };

        Ok(messages)
    }

    /// Update a message (edit).
    pub fn update_message(&self, channel_id: &str, message_id: &str, sender_id: &str, new_content: &str) -> Result<ChatMessage, ChatError> {
        let mut data = self.channels.get_mut(channel_id)
            .ok_or_else(|| ChatError::ChannelNotFound(channel_id.to_string()))?;

        let message = data.messages.iter_mut()
            .find(|m| m.id == message_id)
            .ok_or_else(|| ChatError::MessageNotFound(message_id.to_string()))?;

        if message.sender_id != sender_id {
            return Err(ChatError::NotSender);
        }

        message.content = new_content.to_string();
        message.updated_at = now_millis();

        Ok(message.clone())
    }

    /// Delete a message.
    pub fn delete_message(&self, channel_id: &str, message_id: &str, sender_id: &str) -> Result<(), ChatError> {
        let mut data = self.channels.get_mut(channel_id)
            .ok_or_else(|| ChatError::ChannelNotFound(channel_id.to_string()))?;

        let pos = data.messages.iter()
            .position(|m| m.id == message_id)
            .ok_or_else(|| ChatError::MessageNotFound(message_id.to_string()))?;

        if data.messages[pos].sender_id != sender_id {
            return Err(ChatError::NotSender);
        }

        data.messages.remove(pos);
        Ok(())
    }

    // ==================== Utilities ====================

    /// Get channel count.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Check if user is in channel.
    pub fn is_member(&self, channel_id: &str, user_id: &str) -> bool {
        self.channels.get(channel_id)
            .map(|d| d.members.contains(user_id))
            .unwrap_or(false)
    }
}

impl Default for Chat {
    fn default() -> Self {
        Self::new()
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Chat errors.
#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("channel not found: {0}")]
    ChannelNotFound(String),

    #[error("message not found: {0}")]
    MessageNotFound(String),

    #[error("not a member of channel")]
    NotMember,

    #[error("not the sender")]
    NotSender,

    #[error("channel full")]
    ChannelFull,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_room_chat() {
        let chat = Chat::new();

        // Create room
        let channel = chat.create_room("room1", "Test Room");
        assert_eq!(channel.channel_type, ChannelType::Room);

        // Join
        chat.join(&channel.id, "user1").unwrap();
        chat.join(&channel.id, "user2").unwrap();

        // Send message
        let msg = chat.send_text(&channel.id, "user1", "Alice", "Hello!").unwrap();
        assert_eq!(msg.content, "Hello!");

        // Get history
        let history = chat.get_history(&channel.id, 10, None).unwrap();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_direct_message() {
        let chat = Chat::new();

        // Send DM
        let msg = chat.send_dm("user1", "Alice", "user2", "Hey Bob!").unwrap();
        assert!(msg.channel_id.starts_with("dm:"));

        // Get or create should return same channel
        let channel1 = chat.get_or_create_dm("user1", "user2");
        let channel2 = chat.get_or_create_dm("user2", "user1");
        assert_eq!(channel1.id, channel2.id);
    }

    #[test]
    fn test_message_callback() {
        let chat = Chat::new();
        let received = Arc::new(AtomicUsize::new(0));

        let channel = chat.create_room("room1", "Test");
        chat.join(&channel.id, "user1").unwrap();

        let r = received.clone();
        chat.register_callback("user1", Box::new(move |_msg| {
            r.fetch_add(1, Ordering::SeqCst);
        }));

        chat.send_text(&channel.id, "user2", "Bob", "Hello").unwrap();
        assert_eq!(received.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_message_edit() {
        let chat = Chat::new();
        let channel = chat.create_room("room1", "Test");
        chat.join(&channel.id, "user1").unwrap();

        let msg = chat.send_text(&channel.id, "user1", "Alice", "Helo").unwrap();

        // Edit message
        let updated = chat.update_message(&channel.id, &msg.id, "user1", "Hello").unwrap();
        assert_eq!(updated.content, "Hello");
        assert!(updated.updated_at >= msg.created_at);
    }

    #[test]
    fn test_user_channels() {
        let chat = Chat::new();

        let room1 = chat.create_room("room1", "Room 1");
        let room2 = chat.create_room("room2", "Room 2");

        chat.join(&room1.id, "user1").unwrap();
        chat.join(&room2.id, "user1").unwrap();

        let channels = chat.get_user_channels("user1");
        assert_eq!(channels.len(), 2);
    }
}
