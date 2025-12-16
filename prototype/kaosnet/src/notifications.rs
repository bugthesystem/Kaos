//! Notifications system for KaosNet.
//!
//! Provides real-time and persistent notifications for game events.
//!
//! ## Features
//!
//! - Real-time push to connected clients
//! - Persistent storage for offline users
//! - Multiple notification types
//! - Read/unread tracking
//! - Expiration (TTL)
//! - Batch operations

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Notification code/type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(i32)]
pub enum NotificationCode {
    /// Friend request received.
    FriendRequest = 1,
    /// Friend request accepted.
    FriendAccepted = 2,
    /// Match found.
    MatchFound = 10,
    /// Match starting soon.
    MatchStarting = 11,
    /// Match ended.
    MatchEnded = 12,
    /// Group invite received.
    GroupInvite = 20,
    /// Group join request.
    GroupJoinRequest = 21,
    /// Reward received.
    Reward = 30,
    /// Achievement unlocked.
    Achievement = 31,
    /// Leaderboard rank changed.
    LeaderboardRank = 32,
    /// Chat message.
    ChatMessage = 40,
    /// System announcement.
    SystemAnnouncement = 100,
    /// Custom game notification.
    Custom = 200,
}

impl From<i32> for NotificationCode {
    fn from(code: i32) -> Self {
        match code {
            1 => Self::FriendRequest,
            2 => Self::FriendAccepted,
            10 => Self::MatchFound,
            11 => Self::MatchStarting,
            12 => Self::MatchEnded,
            20 => Self::GroupInvite,
            21 => Self::GroupJoinRequest,
            30 => Self::Reward,
            31 => Self::Achievement,
            32 => Self::LeaderboardRank,
            40 => Self::ChatMessage,
            100 => Self::SystemAnnouncement,
            _ => Self::Custom,
        }
    }
}

/// A notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Unique notification ID.
    pub id: String,
    /// Recipient user ID.
    pub user_id: String,
    /// Notification code/type.
    pub code: NotificationCode,
    /// Subject/title.
    pub subject: String,
    /// Content/body.
    pub content: String,
    /// Sender user ID (optional, for user-originated notifications).
    pub sender_id: Option<String>,
    /// Custom data payload.
    pub data: Option<serde_json::Value>,
    /// Whether notification has been read.
    pub read: bool,
    /// Whether notification is persistent (survives reconnect).
    pub persistent: bool,
    /// When notification was created.
    pub created_at: i64,
    /// When notification expires (0 = never).
    pub expires_at: i64,
}

impl Notification {
    /// Create a new notification.
    pub fn new(user_id: impl Into<String>, code: NotificationCode, subject: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            code,
            subject: subject.into(),
            content: content.into(),
            sender_id: None,
            data: None,
            read: false,
            persistent: true,
            created_at: now_millis(),
            expires_at: 0,
        }
    }

    /// Set sender.
    pub fn with_sender(mut self, sender_id: impl Into<String>) -> Self {
        self.sender_id = Some(sender_id.into());
        self
    }

    /// Set custom data.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Set as non-persistent (ephemeral).
    pub fn ephemeral(mut self) -> Self {
        self.persistent = false;
        self
    }

    /// Set expiration time.
    pub fn expires_in(mut self, duration_secs: u64) -> Self {
        self.expires_at = now_millis() + (duration_secs as i64 * 1000);
        self
    }

    /// Check if notification is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at > 0 && now_millis() > self.expires_at
    }
}

/// Callback for real-time notification delivery.
pub type NotificationCallback = Box<dyn Fn(&Notification) + Send + Sync>;

/// Notifications service.
pub struct Notifications {
    /// Stored notifications: user_id -> notifications (newest first).
    storage: DashMap<String, VecDeque<Notification>>,
    /// Maximum notifications per user.
    max_per_user: usize,
    /// Real-time delivery callbacks (for connected users).
    callbacks: DashMap<String, NotificationCallback>,
}

impl Notifications {
    /// Create a new notifications service.
    pub fn new() -> Self {
        Self {
            storage: DashMap::new(),
            max_per_user: 100,
            callbacks: DashMap::new(),
        }
    }

    /// Create with custom max notifications per user.
    pub fn with_max(max_per_user: usize) -> Self {
        Self {
            storage: DashMap::new(),
            max_per_user,
            callbacks: DashMap::new(),
        }
    }

    /// Register a callback for real-time delivery.
    pub fn register_callback(&self, user_id: impl Into<String>, callback: NotificationCallback) {
        self.callbacks.insert(user_id.into(), callback);
    }

    /// Unregister callback (when user disconnects).
    pub fn unregister_callback(&self, user_id: &str) {
        self.callbacks.remove(user_id);
    }

    /// Send a notification.
    pub fn send(&self, notification: Notification) -> Result<(), NotificationError> {
        let user_id = notification.user_id.clone();

        // Try real-time delivery first
        if let Some(callback) = self.callbacks.get(&user_id) {
            callback(&notification);
        }

        // Store if persistent
        if notification.persistent {
            let mut queue = self.storage.entry(user_id).or_insert_with(VecDeque::new);
            queue.push_front(notification);

            // Trim to max size
            while queue.len() > self.max_per_user {
                queue.pop_back();
            }
        }

        Ok(())
    }

    /// Send notification to multiple users.
    pub fn send_many(&self, user_ids: &[String], mut notification: Notification) -> Result<usize, NotificationError> {
        let mut sent = 0;
        for user_id in user_ids {
            notification.id = Uuid::new_v4().to_string();
            notification.user_id = user_id.clone();
            self.send(notification.clone())?;
            sent += 1;
        }
        Ok(sent)
    }

    /// Broadcast to all users with registered callbacks.
    pub fn broadcast(&self, notification: Notification) -> usize {
        let mut sent = 0;
        for entry in self.callbacks.iter() {
            let mut n = notification.clone();
            n.id = Uuid::new_v4().to_string();
            n.user_id = entry.key().clone();

            entry.value()(&n);

            if n.persistent {
                let mut queue = self.storage.entry(entry.key().clone()).or_insert_with(VecDeque::new);
                queue.push_front(n);
                while queue.len() > self.max_per_user {
                    queue.pop_back();
                }
            }
            sent += 1;
        }
        sent
    }

    /// List notifications for a user.
    pub fn list(&self, user_id: &str, limit: usize, offset: usize, unread_only: bool) -> Vec<Notification> {
        self.storage
            .get(user_id)
            .map(|queue| {
                queue
                    .iter()
                    .filter(|n| !n.is_expired())
                    .filter(|n| !unread_only || !n.read)
                    .skip(offset)
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get unread count.
    pub fn unread_count(&self, user_id: &str) -> usize {
        self.storage
            .get(user_id)
            .map(|queue| {
                queue
                    .iter()
                    .filter(|n| !n.is_expired() && !n.read)
                    .count()
            })
            .unwrap_or(0)
    }

    /// Mark notification as read.
    pub fn mark_read(&self, user_id: &str, notification_id: &str) -> bool {
        if let Some(mut queue) = self.storage.get_mut(user_id) {
            for n in queue.iter_mut() {
                if n.id == notification_id {
                    n.read = true;
                    return true;
                }
            }
        }
        false
    }

    /// Mark all notifications as read.
    pub fn mark_all_read(&self, user_id: &str) -> usize {
        if let Some(mut queue) = self.storage.get_mut(user_id) {
            let mut count = 0;
            for n in queue.iter_mut() {
                if !n.read {
                    n.read = true;
                    count += 1;
                }
            }
            return count;
        }
        0
    }

    /// Delete a notification.
    pub fn delete(&self, user_id: &str, notification_id: &str) -> bool {
        if let Some(mut queue) = self.storage.get_mut(user_id) {
            let len_before = queue.len();
            queue.retain(|n| n.id != notification_id);
            return queue.len() < len_before;
        }
        false
    }

    /// Delete all notifications for a user.
    pub fn delete_all(&self, user_id: &str) -> usize {
        if let Some((_, queue)) = self.storage.remove(user_id) {
            return queue.len();
        }
        0
    }

    /// Clean up expired notifications (call periodically).
    pub fn cleanup_expired(&self) {
        for mut entry in self.storage.iter_mut() {
            entry.value_mut().retain(|n| !n.is_expired());
        }
    }

    /// Get total notification count for a user.
    pub fn total_count(&self, user_id: &str) -> usize {
        self.storage
            .get(user_id)
            .map(|q| q.len())
            .unwrap_or(0)
    }
}

impl Default for Notifications {
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

/// Notification errors.
#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("notification not found: {0}")]
    NotFound(String),

    #[error("user not found: {0}")]
    UserNotFound(String),
}

// Helper functions for common notification types
impl Notifications {
    /// Send friend request notification.
    pub fn notify_friend_request(&self, to_user_id: &str, from_user_id: &str, from_username: &str) -> Result<(), NotificationError> {
        let notification = Notification::new(
            to_user_id,
            NotificationCode::FriendRequest,
            "Friend Request",
            format!("{} sent you a friend request", from_username),
        )
        .with_sender(from_user_id)
        .with_data(serde_json::json!({
            "from_user_id": from_user_id,
            "from_username": from_username
        }));

        self.send(notification)
    }

    /// Send match found notification.
    pub fn notify_match_found(&self, user_id: &str, match_id: &str, queue: &str) -> Result<(), NotificationError> {
        let notification = Notification::new(
            user_id,
            NotificationCode::MatchFound,
            "Match Found!",
            format!("A match has been found in {}", queue),
        )
        .with_data(serde_json::json!({
            "match_id": match_id,
            "queue": queue
        }))
        .expires_in(300); // 5 minute expiry

        self.send(notification)
    }

    /// Send reward notification.
    pub fn notify_reward(&self, user_id: &str, reward_type: &str, amount: i64, description: &str) -> Result<(), NotificationError> {
        let notification = Notification::new(
            user_id,
            NotificationCode::Reward,
            "Reward Received",
            description,
        )
        .with_data(serde_json::json!({
            "reward_type": reward_type,
            "amount": amount
        }));

        self.send(notification)
    }

    /// Send achievement notification.
    pub fn notify_achievement(&self, user_id: &str, achievement_id: &str, achievement_name: &str) -> Result<(), NotificationError> {
        let notification = Notification::new(
            user_id,
            NotificationCode::Achievement,
            "Achievement Unlocked!",
            format!("You've earned: {}", achievement_name),
        )
        .with_data(serde_json::json!({
            "achievement_id": achievement_id,
            "achievement_name": achievement_name
        }));

        self.send(notification)
    }

    /// Send system announcement to all.
    pub fn announce(&self, subject: &str, content: &str) -> usize {
        let notification = Notification::new(
            "", // Will be set per-user
            NotificationCode::SystemAnnouncement,
            subject,
            content,
        );

        self.broadcast(notification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_send_and_list() {
        let notifs = Notifications::new();

        let n = Notification::new("user1", NotificationCode::Custom, "Test", "Hello");
        notifs.send(n).unwrap();

        let list = notifs.list("user1", 10, 0, false);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].subject, "Test");
    }

    #[test]
    fn test_read_tracking() {
        let notifs = Notifications::new();

        let n = Notification::new("user1", NotificationCode::Custom, "Test", "Hello");
        let id = n.id.clone();
        notifs.send(n).unwrap();

        assert_eq!(notifs.unread_count("user1"), 1);

        notifs.mark_read("user1", &id);
        assert_eq!(notifs.unread_count("user1"), 0);
    }

    #[test]
    fn test_real_time_callback() {
        let notifs = Notifications::new();
        let received = Arc::new(AtomicUsize::new(0));

        let r = received.clone();
        notifs.register_callback("user1", Box::new(move |_n| {
            r.fetch_add(1, Ordering::SeqCst);
        }));

        let n = Notification::new("user1", NotificationCode::Custom, "Test", "Hello");
        notifs.send(n).unwrap();

        assert_eq!(received.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_ephemeral_notification() {
        let notifs = Notifications::new();

        // Ephemeral should not be stored
        let n = Notification::new("user1", NotificationCode::Custom, "Test", "Hello").ephemeral();
        notifs.send(n).unwrap();

        let list = notifs.list("user1", 10, 0, false);
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_max_notifications() {
        let notifs = Notifications::with_max(5);

        for i in 0..10 {
            let n = Notification::new("user1", NotificationCode::Custom, format!("Test {}", i), "Hello");
            notifs.send(n).unwrap();
        }

        let list = notifs.list("user1", 100, 0, false);
        assert_eq!(list.len(), 5);
        // Should have newest (9, 8, 7, 6, 5)
        assert!(list[0].subject.contains("9"));
    }

    #[test]
    fn test_friend_request_notification() {
        let notifs = Notifications::new();

        notifs.notify_friend_request("user1", "user2", "Alice").unwrap();

        let list = notifs.list("user1", 10, 0, false);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].code, NotificationCode::FriendRequest);
        assert!(list[0].sender_id.as_ref().unwrap() == "user2");
    }
}
