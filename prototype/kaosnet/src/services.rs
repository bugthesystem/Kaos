//! Service wrappers with hook integration.
//!
//! These wrappers execute before/after hooks around service operations,
//! allowing Lua scripts and native hooks to intercept and modify requests.

use std::sync::Arc;

use crate::hooks::{HookContext, HookError, HookExecutor, HookOperation, HookRegistry};
use crate::leaderboard::{LeaderboardError, LeaderboardRecord, Leaderboards};
use crate::social::{Friend, Group, GroupMember, Social, SocialError};
use crate::storage::{Storage, StorageError, StorageObject};

/// Error type for hooked service operations.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("hook error: {0}")]
    Hook(#[from] HookError),

    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("leaderboard error: {0}")]
    Leaderboard(#[from] LeaderboardError),

    #[error("social error: {0}")]
    Social(#[from] SocialError),
}

pub type Result<T> = std::result::Result<T, ServiceError>;

/// Storage service with hook integration.
pub struct HookedStorage {
    storage: Arc<Storage>,
    hooks: HookExecutor,
}

impl HookedStorage {
    pub fn new(storage: Arc<Storage>, hook_registry: Arc<HookRegistry>) -> Self {
        Self {
            storage,
            hooks: HookExecutor::new(hook_registry),
        }
    }

    /// Read a storage object with hooks.
    pub fn get(
        &self,
        ctx: &HookContext,
        user_id: &str,
        collection: &str,
        key: &str,
    ) -> Result<Option<StorageObject>> {
        // Build payload for hooks
        let payload = serde_json::json!({
            "user_id": user_id,
            "collection": collection,
            "key": key,
        });

        // Execute before hooks (can reject)
        let payload = self
            .hooks
            .before(HookOperation::StorageRead, ctx, payload)?;

        // Extract (possibly modified) values from payload
        let user_id = payload["user_id"].as_str().unwrap_or(user_id);
        let collection = payload["collection"].as_str().unwrap_or(collection);
        let key = payload["key"].as_str().unwrap_or(key);

        // Execute actual operation
        let result = self.storage.get(user_id, collection, key)?;

        // Execute after hooks
        let result_json = match &result {
            Some(obj) => serde_json::json!({
                "found": true,
                "user_id": obj.user_id,
                "collection": obj.collection,
                "key": obj.key,
                "value": obj.value,
                "version": obj.version,
            }),
            None => serde_json::json!({ "found": false }),
        };
        self.hooks
            .after(HookOperation::StorageRead, ctx, &payload, &result_json);

        Ok(result)
    }

    /// Write a storage object with hooks.
    pub fn set(
        &self,
        ctx: &HookContext,
        user_id: &str,
        collection: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<StorageObject> {
        // Build payload for hooks
        let payload = serde_json::json!({
            "user_id": user_id,
            "collection": collection,
            "key": key,
            "value": value,
        });

        // Execute before hooks (can modify/reject)
        let payload = self
            .hooks
            .before(HookOperation::StorageWrite, ctx, payload)?;

        // Extract (possibly modified) values from payload
        let user_id = payload["user_id"].as_str().unwrap_or(user_id);
        let collection = payload["collection"].as_str().unwrap_or(collection);
        let key = payload["key"].as_str().unwrap_or(key);
        let value = payload.get("value").cloned().unwrap_or(value);

        // Execute actual operation
        let result = self.storage.set(user_id, collection, key, value.clone())?;

        // Execute after hooks
        let result_json = serde_json::json!({
            "user_id": result.user_id,
            "collection": result.collection,
            "key": result.key,
            "version": result.version,
        });
        self.hooks
            .after(HookOperation::StorageWrite, ctx, &payload, &result_json);

        Ok(result)
    }

    /// Delete a storage object with hooks.
    pub fn delete(
        &self,
        ctx: &HookContext,
        user_id: &str,
        collection: &str,
        key: &str,
    ) -> Result<bool> {
        // Build payload for hooks
        let payload = serde_json::json!({
            "user_id": user_id,
            "collection": collection,
            "key": key,
        });

        // Execute before hooks (can reject)
        let payload = self
            .hooks
            .before(HookOperation::StorageDelete, ctx, payload)?;

        // Extract values
        let user_id = payload["user_id"].as_str().unwrap_or(user_id);
        let collection = payload["collection"].as_str().unwrap_or(collection);
        let key = payload["key"].as_str().unwrap_or(key);

        // Execute actual operation
        let deleted = self.storage.delete(user_id, collection, key)?;

        // Execute after hooks
        let result_json = serde_json::json!({ "deleted": deleted });
        self.hooks
            .after(HookOperation::StorageDelete, ctx, &payload, &result_json);

        Ok(deleted)
    }

    /// List storage objects (no hooks, just convenience wrapper).
    pub fn list(
        &self,
        user_id: &str,
        collection: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<(Vec<StorageObject>, Option<String>)> {
        Ok(self.storage.list(user_id, collection, limit, cursor)?)
    }

    /// Get underlying storage (for operations that don't need hooks).
    pub fn inner(&self) -> &Arc<Storage> {
        &self.storage
    }
}

/// Leaderboards service with hook integration.
pub struct HookedLeaderboards {
    leaderboards: Arc<Leaderboards>,
    hooks: HookExecutor,
}

impl HookedLeaderboards {
    pub fn new(leaderboards: Arc<Leaderboards>, hook_registry: Arc<HookRegistry>) -> Self {
        Self {
            leaderboards,
            hooks: HookExecutor::new(hook_registry),
        }
    }

    /// Submit a score with hooks.
    pub fn submit(
        &self,
        ctx: &HookContext,
        leaderboard_id: &str,
        user_id: &str,
        username: &str,
        score: i64,
        metadata: Option<serde_json::Value>,
    ) -> Result<LeaderboardRecord> {
        // Build payload for hooks
        let payload = serde_json::json!({
            "leaderboard_id": leaderboard_id,
            "user_id": user_id,
            "username": username,
            "score": score,
            "metadata": metadata,
        });

        // Execute before hooks (can modify score/reject)
        let payload = self
            .hooks
            .before(HookOperation::LeaderboardSubmit, ctx, payload)?;

        // Extract (possibly modified) values
        let leaderboard_id = payload["leaderboard_id"]
            .as_str()
            .unwrap_or(leaderboard_id);
        let user_id = payload["user_id"].as_str().unwrap_or(user_id);
        let username = payload["username"].as_str().unwrap_or(username);
        let score = payload["score"].as_i64().unwrap_or(score);
        let metadata = payload.get("metadata").and_then(|v| {
            if v.is_null() {
                None
            } else {
                Some(v.clone())
            }
        });

        // Execute actual operation
        let result = self
            .leaderboards
            .submit(leaderboard_id, user_id, username, score, metadata)?;

        // Execute after hooks
        let result_json = serde_json::json!({
            "user_id": result.user_id,
            "username": result.username,
            "score": result.score,
            "rank": result.rank,
            "num_submissions": result.num_submissions,
        });
        self.hooks
            .after(HookOperation::LeaderboardSubmit, ctx, &payload, &result_json);

        Ok(result)
    }

    /// Get top scores with hooks.
    pub fn get_top(
        &self,
        ctx: &HookContext,
        leaderboard_id: &str,
        limit: usize,
    ) -> Result<Vec<LeaderboardRecord>> {
        // Build payload for hooks
        let payload = serde_json::json!({
            "leaderboard_id": leaderboard_id,
            "limit": limit,
        });

        // Execute before hooks
        let payload = self
            .hooks
            .before(HookOperation::LeaderboardRead, ctx, payload)?;

        let leaderboard_id = payload["leaderboard_id"]
            .as_str()
            .unwrap_or(leaderboard_id);
        let limit = payload["limit"].as_u64().unwrap_or(limit as u64) as usize;

        // Execute actual operation
        let result = self.leaderboards.get_top(leaderboard_id, limit)?;

        // Execute after hooks
        let result_json = serde_json::json!({
            "count": result.len(),
            "records": result.iter().map(|r| serde_json::json!({
                "user_id": r.user_id,
                "username": r.username,
                "score": r.score,
                "rank": r.rank,
            })).collect::<Vec<_>>(),
        });
        self.hooks
            .after(HookOperation::LeaderboardRead, ctx, &payload, &result_json);

        Ok(result)
    }

    /// Get underlying leaderboards (for operations that don't need hooks).
    pub fn inner(&self) -> &Arc<Leaderboards> {
        &self.leaderboards
    }
}

/// Social service with hook integration.
pub struct HookedSocial {
    social: Arc<Social>,
    hooks: HookExecutor,
}

impl HookedSocial {
    pub fn new(social: Arc<Social>, hook_registry: Arc<HookRegistry>) -> Self {
        Self {
            social,
            hooks: HookExecutor::new(hook_registry),
        }
    }

    /// Add a friend with hooks.
    pub fn add_friend(
        &self,
        ctx: &HookContext,
        user_id: &str,
        friend_id: &str,
        friend_username: &str,
    ) -> Result<Friend> {
        let payload = serde_json::json!({
            "user_id": user_id,
            "friend_id": friend_id,
            "friend_username": friend_username,
        });

        let payload = self.hooks.before(HookOperation::FriendAdd, ctx, payload)?;

        let user_id = payload["user_id"].as_str().unwrap_or(user_id);
        let friend_id = payload["friend_id"].as_str().unwrap_or(friend_id);
        let friend_username = payload["friend_username"]
            .as_str()
            .unwrap_or(friend_username);

        let result = self.social.add_friend(user_id, friend_id, friend_username)?;

        let result_json = serde_json::json!({
            "friend_id": result.user_id,
            "username": result.username,
            "state": format!("{:?}", result.state),
        });
        self.hooks
            .after(HookOperation::FriendAdd, ctx, &payload, &result_json);

        Ok(result)
    }

    /// Remove a friend with hooks.
    pub fn remove_friend(&self, ctx: &HookContext, user_id: &str, friend_id: &str) -> Result<()> {
        let payload = serde_json::json!({
            "user_id": user_id,
            "friend_id": friend_id,
        });

        let payload = self
            .hooks
            .before(HookOperation::FriendRemove, ctx, payload)?;

        let user_id = payload["user_id"].as_str().unwrap_or(user_id);
        let friend_id = payload["friend_id"].as_str().unwrap_or(friend_id);

        self.social.remove_friend(user_id, friend_id)?;

        let result_json = serde_json::json!({ "removed": true });
        self.hooks
            .after(HookOperation::FriendRemove, ctx, &payload, &result_json);

        Ok(())
    }

    /// Block a user with hooks.
    pub fn block_user(&self, ctx: &HookContext, user_id: &str, blocked_id: &str) -> Result<()> {
        let payload = serde_json::json!({
            "user_id": user_id,
            "blocked_id": blocked_id,
        });

        let payload = self
            .hooks
            .before(HookOperation::FriendBlock, ctx, payload)?;

        let user_id = payload["user_id"].as_str().unwrap_or(user_id);
        let blocked_id = payload["blocked_id"].as_str().unwrap_or(blocked_id);

        self.social.block_user(user_id, blocked_id)?;

        let result_json = serde_json::json!({ "blocked": true });
        self.hooks
            .after(HookOperation::FriendBlock, ctx, &payload, &result_json);

        Ok(())
    }

    /// Create a group with hooks.
    pub fn create_group(
        &self,
        ctx: &HookContext,
        creator_id: &str,
        creator_username: &str,
        name: String,
        description: String,
        open: bool,
    ) -> Result<Group> {
        let payload = serde_json::json!({
            "creator_id": creator_id,
            "creator_username": creator_username,
            "name": name,
            "description": description,
            "open": open,
        });

        let payload = self
            .hooks
            .before(HookOperation::GroupCreate, ctx, payload)?;

        let creator_id = payload["creator_id"].as_str().unwrap_or(creator_id);
        let creator_username = payload["creator_username"]
            .as_str()
            .unwrap_or(creator_username);
        let name = payload["name"]
            .as_str()
            .map(String::from)
            .unwrap_or(name);
        let description = payload["description"]
            .as_str()
            .map(String::from)
            .unwrap_or(description);
        let open = payload["open"].as_bool().unwrap_or(open);

        let result = self
            .social
            .create_group(creator_id, creator_username, name, description, open)?;

        let result_json = serde_json::json!({
            "group_id": result.id,
            "name": result.name,
        });
        self.hooks
            .after(HookOperation::GroupCreate, ctx, &payload, &result_json);

        Ok(result)
    }

    /// Join a group with hooks.
    pub fn join_group(
        &self,
        ctx: &HookContext,
        group_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<()> {
        let payload = serde_json::json!({
            "group_id": group_id,
            "user_id": user_id,
            "username": username,
        });

        let payload = self.hooks.before(HookOperation::GroupJoin, ctx, payload)?;

        let group_id = payload["group_id"].as_str().unwrap_or(group_id);
        let user_id = payload["user_id"].as_str().unwrap_or(user_id);
        let username = payload["username"].as_str().unwrap_or(username);

        self.social.join_group(group_id, user_id, username)?;

        let result_json = serde_json::json!({ "joined": true });
        self.hooks
            .after(HookOperation::GroupJoin, ctx, &payload, &result_json);

        Ok(())
    }

    /// Leave a group with hooks.
    pub fn leave_group(&self, ctx: &HookContext, group_id: &str, user_id: &str) -> Result<()> {
        let payload = serde_json::json!({
            "group_id": group_id,
            "user_id": user_id,
        });

        let payload = self.hooks.before(HookOperation::GroupLeave, ctx, payload)?;

        let group_id = payload["group_id"].as_str().unwrap_or(group_id);
        let user_id = payload["user_id"].as_str().unwrap_or(user_id);

        self.social.leave_group(group_id, user_id)?;

        let result_json = serde_json::json!({ "left": true });
        self.hooks
            .after(HookOperation::GroupLeave, ctx, &payload, &result_json);

        Ok(())
    }

    /// Get friends list (no hooks).
    pub fn get_friends(&self, user_id: &str) -> Vec<Friend> {
        self.social.get_friends(user_id)
    }

    /// Get group members (no hooks).
    pub fn get_group_members(&self, group_id: &str) -> std::result::Result<Vec<GroupMember>, SocialError> {
        self.social.get_group_members(group_id)
    }

    /// Get underlying social service.
    pub fn inner(&self) -> &Arc<Social> {
        &self.social
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::BeforeHook;
    use crate::hooks::BeforeHookResult;

    struct RejectWriteHook {
        blocked_collection: String,
    }

    impl BeforeHook for RejectWriteHook {
        fn execute(
            &self,
            _ctx: &HookContext,
            payload: serde_json::Value,
        ) -> crate::hooks::Result<BeforeHookResult> {
            if payload["collection"].as_str() == Some(&self.blocked_collection) {
                return Ok(BeforeHookResult::Reject(format!(
                    "Collection '{}' is read-only",
                    self.blocked_collection
                )));
            }
            Ok(BeforeHookResult::Continue(payload))
        }
    }

    struct ModifyScoreHook {
        multiplier: i64,
    }

    impl BeforeHook for ModifyScoreHook {
        fn execute(
            &self,
            _ctx: &HookContext,
            mut payload: serde_json::Value,
        ) -> crate::hooks::Result<BeforeHookResult> {
            if let Some(score) = payload["score"].as_i64() {
                payload["score"] = serde_json::json!(score * self.multiplier);
            }
            Ok(BeforeHookResult::Continue(payload))
        }
    }

    struct AfterHookCounter {
        count: std::sync::atomic::AtomicU32,
    }

    impl crate::hooks::AfterHook for AfterHookCounter {
        fn execute(
            &self,
            _ctx: &HookContext,
            _payload: &serde_json::Value,
            _result: &serde_json::Value,
        ) {
            self.count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    #[test]
    fn test_hooked_storage_write() {
        let storage = Arc::new(Storage::default());
        let registry = Arc::new(HookRegistry::new());
        let hooked = HookedStorage::new(storage, registry);

        let ctx = HookContext::default();
        let result = hooked
            .set(
                &ctx,
                "user1",
                "saves",
                "slot1",
                serde_json::json!({"level": 5}),
            )
            .unwrap();

        assert_eq!(result.version, 1);
    }

    #[test]
    fn test_hooked_storage_reject() {
        let storage = Arc::new(Storage::default());
        let registry = Arc::new(HookRegistry::new());

        registry.register_before(
            HookOperation::StorageWrite,
            RejectWriteHook {
                blocked_collection: "readonly".to_string(),
            },
        );

        let hooked = HookedStorage::new(storage, registry);

        let ctx = HookContext::default();
        let result = hooked.set(
            &ctx,
            "user1",
            "readonly",
            "key1",
            serde_json::json!({"data": "test"}),
        );

        assert!(matches!(result, Err(ServiceError::Hook(HookError::Rejected(_)))));
    }

    #[test]
    fn test_hooked_storage_read_after_hook() {
        let storage = Arc::new(Storage::default());
        let registry = Arc::new(HookRegistry::new());

        // Pre-populate data
        storage
            .set("user1", "data", "key1", serde_json::json!({"value": 42}))
            .unwrap();

        let hooked = HookedStorage::new(storage, registry);

        let ctx = HookContext::default();
        let result = hooked.get(&ctx, "user1", "data", "key1").unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().value["value"], 42);
    }

    #[test]
    fn test_hooked_leaderboard_modify_score() {
        let leaderboards = Arc::new(Leaderboards::new());
        let registry = Arc::new(HookRegistry::new());

        // Register hook that doubles scores
        registry.register_before(
            HookOperation::LeaderboardSubmit,
            ModifyScoreHook { multiplier: 2 },
        );

        leaderboards
            .create(crate::leaderboard::LeaderboardConfig {
                id: "scores".to_string(),
                name: "Scores".to_string(),
                sort_order: crate::leaderboard::SortOrder::Descending,
                operator: crate::leaderboard::ScoreOperator::Best,
                reset_schedule: crate::leaderboard::ResetSchedule::Never,
                max_entries: 1000,
                metadata_schema: None,
            })
            .unwrap();

        let hooked = HookedLeaderboards::new(leaderboards, registry);

        let ctx = HookContext::default();
        let record = hooked
            .submit(&ctx, "scores", "user1", "Alice", 100, None)
            .unwrap();

        // Score should be doubled by hook
        assert_eq!(record.score, 200);
    }

    #[test]
    fn test_hooked_social_add_friend() {
        let social = Arc::new(Social::new());
        let registry = Arc::new(HookRegistry::new());
        let hooked = HookedSocial::new(social, registry);

        let ctx = HookContext::default();
        let friend = hooked
            .add_friend(&ctx, "alice", "bob", "Bob")
            .unwrap();

        assert_eq!(friend.user_id, "bob");
        assert_eq!(friend.username, "Bob");
    }

    #[test]
    fn test_hooked_social_reject_friend() {
        let social = Arc::new(Social::new());
        let registry = Arc::new(HookRegistry::new());

        // Block all friend requests
        struct BlockAllFriends;
        impl BeforeHook for BlockAllFriends {
            fn execute(
                &self,
                _ctx: &HookContext,
                _payload: serde_json::Value,
            ) -> crate::hooks::Result<BeforeHookResult> {
                Ok(BeforeHookResult::Reject("Friend requests disabled".to_string()))
            }
        }

        registry.register_before(HookOperation::FriendAdd, BlockAllFriends);

        let hooked = HookedSocial::new(social, registry);

        let ctx = HookContext::default();
        let result = hooked.add_friend(&ctx, "alice", "bob", "Bob");

        assert!(matches!(result, Err(ServiceError::Hook(HookError::Rejected(_)))));
    }
}
