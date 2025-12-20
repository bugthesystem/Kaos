//! Social features for KaosNet.
//!
//! Provides friends, groups, and presence tracking.
//!
//! ## Features
//!
//! - Friend requests and management
//! - Friend presence (online/offline/in-game)
//! - Groups/clans with roles
//! - Blocking users
//! - User status messages

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Friend relationship state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FriendState {
    /// Friend request sent, waiting for response.
    Pending,
    /// Mutual friends.
    Accepted,
    /// User has been blocked.
    Blocked,
}

/// A friend relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    /// Friend's user ID.
    pub user_id: String,
    /// Friend's username.
    pub username: String,
    /// Relationship state.
    pub state: FriendState,
    /// When relationship was created.
    pub created_at: i64,
    /// When relationship was last updated.
    pub updated_at: i64,
}

/// User presence status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PresenceStatus {
    Online,
    Away,
    Busy,
    InGame,
    #[default]
    Offline,
}


/// User presence information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presence {
    /// User ID.
    pub user_id: String,
    /// Username.
    pub username: String,
    /// Current status.
    pub status: PresenceStatus,
    /// Custom status message.
    pub status_message: Option<String>,
    /// Current game/room ID if in-game.
    pub game_id: Option<String>,
    /// When presence was last updated.
    pub last_seen: i64,
}

/// A group/clan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    /// Unique group ID.
    pub id: String,
    /// Group name.
    pub name: String,
    /// Group description.
    pub description: String,
    /// Creator user ID.
    pub creator_id: String,
    /// Whether group is open to join.
    pub open: bool,
    /// Maximum members.
    pub max_members: usize,
    /// Group metadata.
    pub metadata: Option<serde_json::Value>,
    /// When group was created.
    pub created_at: i64,
    /// When group was last updated.
    pub updated_at: i64,
}

/// Group member role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum GroupRole {
    #[default]
    Member = 0,
    Moderator = 1,
    Admin = 2,
    Owner = 3,
}


/// A group member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub user_id: String,
    pub username: String,
    pub role: GroupRole,
    pub joined_at: i64,
}

/// Social service.
pub struct Social {
    /// Friends: user_id -> friend_user_id -> Friend.
    friends: DashMap<String, DashMap<String, Friend>>,
    /// Presence: user_id -> Presence.
    presence: DashMap<String, Presence>,
    /// Groups: group_id -> Group.
    groups: DashMap<String, Group>,
    /// Group members: group_id -> user_id -> GroupMember.
    group_members: DashMap<String, DashMap<String, GroupMember>>,
    /// User's groups: user_id -> set of group_ids.
    user_groups: DashMap<String, HashSet<String>>,
}

impl Social {
    pub fn new() -> Self {
        Self {
            friends: DashMap::new(),
            presence: DashMap::new(),
            groups: DashMap::new(),
            group_members: DashMap::new(),
            user_groups: DashMap::new(),
        }
    }

    // ==================== Friends ====================

    /// Send a friend request.
    pub fn add_friend(&self, user_id: &str, friend_id: &str, friend_username: &str) -> Result<Friend, SocialError> {
        if user_id == friend_id {
            return Err(SocialError::InvalidOperation("cannot friend yourself".into()));
        }

        // Check if blocked (can't friend someone who blocked you)
        if self.is_blocked(user_id, friend_id) {
            return Err(SocialError::Blocked);
        }

        let user_friends = self.friends.entry(user_id.to_string()).or_default();
        let now = now_millis();

        // Check for existing relationship
        if let Some(existing) = user_friends.get(friend_id) {
            match existing.state {
                FriendState::Accepted => return Err(SocialError::AlreadyFriends),
                FriendState::Blocked => return Err(SocialError::Blocked),
                FriendState::Pending => {} // Will update
            }
        }

        // Check if they already sent us a request - auto accept
        let friend_has_pending = self.friends
            .get(friend_id)
            .map(|f| {
                f.get(user_id)
                    .map(|r| r.state == FriendState::Pending)
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        let state = if friend_has_pending {
            // Accept mutual request
            if let Some(their_friends) = self.friends.get(friend_id) {
                if let Some(mut their_request) = their_friends.get_mut(user_id) {
                    their_request.state = FriendState::Accepted;
                    their_request.updated_at = now;
                }
            }
            FriendState::Accepted
        } else {
            FriendState::Pending
        };

        let friend = Friend {
            user_id: friend_id.to_string(),
            username: friend_username.to_string(),
            state,
            created_at: now,
            updated_at: now,
        };

        user_friends.insert(friend_id.to_string(), friend.clone());
        Ok(friend)
    }

    /// Accept a friend request.
    pub fn accept_friend(&self, user_id: &str, friend_id: &str) -> Result<(), SocialError> {
        // Get their pending request to us
        let their_friends = self.friends.get(friend_id)
            .ok_or(SocialError::NoPendingRequest)?;

        let mut their_request = their_friends.get_mut(user_id)
            .ok_or(SocialError::NoPendingRequest)?;

        if their_request.state != FriendState::Pending {
            return Err(SocialError::NoPendingRequest);
        }

        let now = now_millis();
        their_request.state = FriendState::Accepted;
        their_request.updated_at = now;

        // Add reciprocal relationship
        let user_friends = self.friends.entry(user_id.to_string()).or_default();

        // Get friend's username from the request
        let friend_username = self.get_username(friend_id).unwrap_or_default();

        user_friends.insert(friend_id.to_string(), Friend {
            user_id: friend_id.to_string(),
            username: friend_username,
            state: FriendState::Accepted,
            created_at: now,
            updated_at: now,
        });

        Ok(())
    }

    /// Remove a friend.
    pub fn remove_friend(&self, user_id: &str, friend_id: &str) -> Result<(), SocialError> {
        // Remove from both sides
        if let Some(user_friends) = self.friends.get(user_id) {
            user_friends.remove(friend_id);
        }
        if let Some(friend_friends) = self.friends.get(friend_id) {
            friend_friends.remove(user_id);
        }
        Ok(())
    }

    /// Block a user.
    pub fn block_user(&self, user_id: &str, blocked_id: &str) -> Result<(), SocialError> {
        let user_friends = self.friends.entry(user_id.to_string()).or_default();
        let now = now_millis();

        let blocked_username = self.get_username(blocked_id).unwrap_or_default();

        user_friends.insert(blocked_id.to_string(), Friend {
            user_id: blocked_id.to_string(),
            username: blocked_username,
            state: FriendState::Blocked,
            created_at: now,
            updated_at: now,
        });

        // Remove from their friends too
        if let Some(their_friends) = self.friends.get(blocked_id) {
            their_friends.remove(user_id);
        }

        Ok(())
    }

    /// Unblock a user.
    pub fn unblock_user(&self, user_id: &str, blocked_id: &str) -> Result<(), SocialError> {
        if let Some(user_friends) = self.friends.get(user_id) {
            if let Some(entry) = user_friends.get(blocked_id) {
                if entry.state == FriendState::Blocked {
                    user_friends.remove(blocked_id);
                }
            }
        }
        Ok(())
    }

    /// Check if a user is blocked.
    pub fn is_blocked(&self, user_id: &str, by_user_id: &str) -> bool {
        self.friends
            .get(by_user_id)
            .map(|f| {
                f.get(user_id)
                    .map(|r| r.state == FriendState::Blocked)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Get all friends for a user.
    pub fn get_friends(&self, user_id: &str) -> Vec<Friend> {
        self.friends
            .get(user_id)
            .map(|f| f.iter().filter(|r| r.state == FriendState::Accepted).map(|r| r.clone()).collect())
            .unwrap_or_default()
    }

    /// Get pending friend requests.
    pub fn get_pending_requests(&self, user_id: &str) -> Vec<Friend> {
        // Find requests FROM others TO this user
        let mut pending = Vec::new();
        for entry in self.friends.iter() {
            if let Some(request) = entry.value().get(user_id) {
                if request.state == FriendState::Pending {
                    pending.push(Friend {
                        user_id: entry.key().clone(),
                        username: self.get_username(entry.key()).unwrap_or_default(),
                        state: FriendState::Pending,
                        created_at: request.created_at,
                        updated_at: request.updated_at,
                    });
                }
            }
        }
        pending
    }

    // ==================== Presence ====================

    /// Update user presence.
    pub fn update_presence(&self, user_id: &str, username: &str, status: PresenceStatus, status_message: Option<String>, game_id: Option<String>) {
        let presence = Presence {
            user_id: user_id.to_string(),
            username: username.to_string(),
            status,
            status_message,
            game_id,
            last_seen: now_millis(),
        };
        self.presence.insert(user_id.to_string(), presence);
    }

    /// Set user offline.
    pub fn set_offline(&self, user_id: &str) {
        if let Some(mut presence) = self.presence.get_mut(user_id) {
            presence.status = PresenceStatus::Offline;
            presence.game_id = None;
            presence.last_seen = now_millis();
        }
    }

    /// Get user presence.
    pub fn get_presence(&self, user_id: &str) -> Option<Presence> {
        self.presence.get(user_id).map(|p| p.clone())
    }

    /// Get presence for multiple users (for friends list).
    pub fn get_presences(&self, user_ids: &[String]) -> Vec<Presence> {
        user_ids.iter()
            .filter_map(|id| self.presence.get(id).map(|p| p.clone()))
            .collect()
    }

    /// Get online friends.
    pub fn get_online_friends(&self, user_id: &str) -> Vec<Presence> {
        self.get_friends(user_id)
            .iter()
            .filter_map(|f| {
                self.presence.get(&f.user_id)
                    .filter(|p| p.status != PresenceStatus::Offline)
                    .map(|p| p.clone())
            })
            .collect()
    }

    // ==================== Groups ====================

    /// Create a group.
    pub fn create_group(&self, creator_id: &str, creator_username: &str, name: String, description: String, open: bool) -> Result<Group, SocialError> {
        let now = now_millis();
        let group = Group {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            creator_id: creator_id.to_string(),
            open,
            max_members: 100,
            metadata: None,
            created_at: now,
            updated_at: now,
        };

        let group_id = group.id.clone();
        self.groups.insert(group_id.clone(), group.clone());

        // Add creator as owner
        let members = DashMap::new();
        members.insert(creator_id.to_string(), GroupMember {
            user_id: creator_id.to_string(),
            username: creator_username.to_string(),
            role: GroupRole::Owner,
            joined_at: now,
        });
        self.group_members.insert(group_id.clone(), members);

        // Track user's groups
        self.user_groups.entry(creator_id.to_string())
            .or_default()
            .insert(group_id);

        Ok(group)
    }

    /// Join a group.
    pub fn join_group(&self, group_id: &str, user_id: &str, username: &str) -> Result<(), SocialError> {
        let group = self.groups.get(group_id)
            .ok_or_else(|| SocialError::GroupNotFound(group_id.to_string()))?;

        if !group.open {
            return Err(SocialError::GroupNotOpen);
        }

        let members = self.group_members.get(group_id)
            .ok_or_else(|| SocialError::GroupNotFound(group_id.to_string()))?;

        if members.len() >= group.max_members {
            return Err(SocialError::GroupFull);
        }

        if members.contains_key(user_id) {
            return Err(SocialError::AlreadyInGroup);
        }

        members.insert(user_id.to_string(), GroupMember {
            user_id: user_id.to_string(),
            username: username.to_string(),
            role: GroupRole::Member,
            joined_at: now_millis(),
        });

        self.user_groups.entry(user_id.to_string())
            .or_default()
            .insert(group_id.to_string());

        Ok(())
    }

    /// Leave a group.
    pub fn leave_group(&self, group_id: &str, user_id: &str) -> Result<(), SocialError> {
        if let Some(members) = self.group_members.get(group_id) {
            // Check if user is owner - owners can't leave, must delete or transfer
            if let Some(member) = members.get(user_id) {
                if member.role == GroupRole::Owner {
                    return Err(SocialError::OwnerCannotLeave);
                }
            }
            members.remove(user_id);
        }

        if let Some(mut user_groups) = self.user_groups.get_mut(user_id) {
            user_groups.remove(group_id);
        }

        Ok(())
    }

    /// Get group members.
    pub fn get_group_members(&self, group_id: &str) -> Result<Vec<GroupMember>, SocialError> {
        let members = self.group_members.get(group_id)
            .ok_or_else(|| SocialError::GroupNotFound(group_id.to_string()))?;

        Ok(members.iter().map(|m| m.clone()).collect())
    }

    /// Get groups a user is in.
    pub fn get_user_groups(&self, user_id: &str) -> Vec<Group> {
        self.user_groups.get(user_id)
            .map(|group_ids| {
                group_ids.iter()
                    .filter_map(|id| self.groups.get(id).map(|g| g.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search groups.
    pub fn search_groups(&self, query: &str, limit: usize) -> Vec<Group> {
        let query_lower = query.to_lowercase();
        self.groups.iter()
            .filter(|g| g.name.to_lowercase().contains(&query_lower))
            .take(limit)
            .map(|g| g.clone())
            .collect()
    }

    /// Delete a group.
    pub fn delete_group(&self, group_id: &str) -> Result<(), SocialError> {
        // Remove the group from the main groups map
        let removed = self.groups.remove(group_id);

        if removed.is_none() {
            return Err(SocialError::GroupNotFound(group_id.to_string()));
        }

        // Remove all members from this group
        self.group_members.remove(group_id);

        // Remove group from all users' group lists
        for mut user_groups in self.user_groups.iter_mut() {
            user_groups.remove(group_id);
        }

        Ok(())
    }

    /// Get a group by ID.
    pub fn get_group(&self, group_id: &str) -> Option<Group> {
        self.groups.get(group_id).map(|g| g.clone())
    }

    /// List all groups.
    pub fn list_groups(&self) -> Vec<Group> {
        self.groups.iter().map(|g| g.clone()).collect()
    }

    // ==================== Helpers ====================

    fn get_username(&self, user_id: &str) -> Option<String> {
        self.presence.get(user_id).map(|p| p.username.clone())
    }
}

impl Default for Social {
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

/// Social system errors.
#[derive(Debug, thiserror::Error)]
pub enum SocialError {
    #[error("already friends")]
    AlreadyFriends,

    #[error("user is blocked")]
    Blocked,

    #[error("no pending friend request")]
    NoPendingRequest,

    #[error("invalid operation: {0}")]
    InvalidOperation(String),

    #[error("group not found: {0}")]
    GroupNotFound(String),

    #[error("group is not open")]
    GroupNotOpen,

    #[error("group is full")]
    GroupFull,

    #[error("already in group")]
    AlreadyInGroup,

    #[error("owner cannot leave group")]
    OwnerCannotLeave,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_friend_request_flow() {
        let social = Social::new();

        // Alice sends request to Bob
        social.add_friend("alice", "bob", "Bob").unwrap();

        // Bob should see pending request
        let pending = social.get_pending_requests("bob");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].user_id, "alice");

        // Bob accepts
        social.accept_friend("bob", "alice").unwrap();

        // Both should be friends
        let alice_friends = social.get_friends("alice");
        let bob_friends = social.get_friends("bob");
        assert_eq!(alice_friends.len(), 1);
        assert_eq!(bob_friends.len(), 1);
    }

    #[test]
    fn test_mutual_friend_request() {
        let social = Social::new();

        // Both send requests at same time
        social.add_friend("alice", "bob", "Bob").unwrap();
        let friend = social.add_friend("bob", "alice", "Alice").unwrap();

        // Should auto-accept
        assert_eq!(friend.state, FriendState::Accepted);
    }

    #[test]
    fn test_blocking() {
        let social = Social::new();

        // Alice blocks Bob
        social.block_user("alice", "bob").unwrap();

        // Bob can't friend Alice
        let result = social.add_friend("bob", "alice", "Alice");
        assert!(matches!(result, Err(SocialError::Blocked)));
    }

    #[test]
    fn test_group_lifecycle() {
        let social = Social::new();

        // Create group
        let group = social.create_group("alice", "Alice", "Cool Group".into(), "A cool group".into(), true).unwrap();

        // Bob joins
        social.join_group(&group.id, "bob", "Bob").unwrap();

        // Check members
        let members = social.get_group_members(&group.id).unwrap();
        assert_eq!(members.len(), 2);

        // Bob leaves
        social.leave_group(&group.id, "bob").unwrap();
        let members = social.get_group_members(&group.id).unwrap();
        assert_eq!(members.len(), 1);
    }
}
