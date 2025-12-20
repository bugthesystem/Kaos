//! Service wrappers with hook and metrics integration.
//!
//! These wrappers execute before/after hooks around service operations,
//! allowing Lua scripts and native hooks to intercept and modify requests.
//!
//! When the `metrics` feature is enabled, operations automatically record
//! metrics like storage read/write counts, leaderboard submissions, etc.

use std::sync::Arc;

use crate::hooks::{HookContext, HookError, HookExecutor, HookOperation, HookRegistry};
use crate::leaderboard::{LeaderboardError, LeaderboardRecord, Leaderboards};
use crate::matchmaker::{
    Matchmaker, MatchmakerConfig, MatchmakerError, MatchmakerMatch, MatchmakerTicket, QueueStats,
};
use crate::notifications::Notifications;
use crate::social::{Friend, Group, GroupMember, Social, SocialError};
use crate::storage::{Storage, StorageError, StorageObject};

#[cfg(feature = "metrics")]
use crate::metrics::Metrics;

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

    #[error("matchmaker error: {0}")]
    Matchmaker(#[from] MatchmakerError),
}

pub type Result<T> = std::result::Result<T, ServiceError>;

/// Storage service with hook and metrics integration.
pub struct HookedStorage {
    storage: Arc<Storage>,
    hooks: HookExecutor,
    #[cfg(feature = "metrics")]
    metrics: Option<Arc<Metrics>>,
}

impl HookedStorage {
    pub fn new(storage: Arc<Storage>, hook_registry: Arc<HookRegistry>) -> Self {
        Self {
            storage,
            hooks: HookExecutor::new(hook_registry),
            #[cfg(feature = "metrics")]
            metrics: None,
        }
    }

    /// Enable metrics recording.
    #[cfg(feature = "metrics")]
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    #[cfg(feature = "metrics")]
    fn record_metric(&self, operation: &str) {
        if let Some(ref m) = self.metrics {
            m.record_storage_operation(operation);
        }
    }

    #[cfg(not(feature = "metrics"))]
    fn record_metric(&self, _operation: &str) {}

    /// Read a storage object with hooks.
    pub fn get(
        &self,
        ctx: &HookContext,
        user_id: &str,
        collection: &str,
        key: &str,
    ) -> Result<Option<StorageObject>> {
        // Record metric
        self.record_metric("get");

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
        // Record metric
        self.record_metric("set");

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
        // Record metric
        self.record_metric("delete");

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

/// Leaderboards service with hook and metrics integration.
pub struct HookedLeaderboards {
    leaderboards: Arc<Leaderboards>,
    hooks: HookExecutor,
    #[cfg(feature = "metrics")]
    metrics: Option<Arc<Metrics>>,
}

impl HookedLeaderboards {
    pub fn new(leaderboards: Arc<Leaderboards>, hook_registry: Arc<HookRegistry>) -> Self {
        Self {
            leaderboards,
            hooks: HookExecutor::new(hook_registry),
            #[cfg(feature = "metrics")]
            metrics: None,
        }
    }

    /// Enable metrics recording.
    #[cfg(feature = "metrics")]
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
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
        // Record metric
        #[cfg(feature = "metrics")]
        if let Some(ref m) = self.metrics {
            m.leaderboard_submissions_total.inc();
        }

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

/// Social service with hook, metrics, and notification integration.
pub struct HookedSocial {
    social: Arc<Social>,
    hooks: HookExecutor,
    notifications: Option<Arc<Notifications>>,
    #[cfg(feature = "metrics")]
    #[allow(dead_code)]
    metrics: Option<Arc<Metrics>>,
}

impl HookedSocial {
    pub fn new(social: Arc<Social>, hook_registry: Arc<HookRegistry>) -> Self {
        Self {
            social,
            hooks: HookExecutor::new(hook_registry),
            notifications: None,
            #[cfg(feature = "metrics")]
            metrics: None,
        }
    }

    /// Enable notifications for social events.
    pub fn with_notifications(mut self, notifications: Arc<Notifications>) -> Self {
        self.notifications = Some(notifications);
        self
    }

    /// Enable metrics recording.
    #[cfg(feature = "metrics")]
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Add a friend with hooks.
    ///
    /// Sends a FriendRequest notification to the target user.
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

        // Send friend request notification to the target
        if let Some(ref notifications) = self.notifications {
            // Get sender's username from context or use user_id
            let sender_username = ctx.username.as_deref().unwrap_or(user_id);
            let _ = notifications.notify_friend_request(friend_id, user_id, sender_username);
        }

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

/// Matchmaker service with hook, metrics, and notification integration.
pub struct HookedMatchmaker {
    matchmaker: Arc<Matchmaker>,
    hooks: HookExecutor,
    notifications: Option<Arc<Notifications>>,
    #[cfg(feature = "metrics")]
    metrics: Option<Arc<Metrics>>,
}

impl HookedMatchmaker {
    pub fn new(matchmaker: Arc<Matchmaker>, hook_registry: Arc<HookRegistry>) -> Self {
        Self {
            matchmaker,
            hooks: HookExecutor::new(hook_registry),
            notifications: None,
            #[cfg(feature = "metrics")]
            metrics: None,
        }
    }

    /// Enable notifications for matchmaker events.
    pub fn with_notifications(mut self, notifications: Arc<Notifications>) -> Self {
        self.notifications = Some(notifications);
        self
    }

    /// Enable metrics recording.
    #[cfg(feature = "metrics")]
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    #[cfg(feature = "metrics")]
    fn update_queue_metrics(&self) {
        if let Some(ref m) = self.metrics {
            let queues = self.matchmaker.list_queues();
            let total_players: usize = queues.iter().map(|q| q.players).sum();
            m.matchmaker_queue_size.set(total_players as i64);
        }
    }

    #[cfg(not(feature = "metrics"))]
    fn update_queue_metrics(&self) {}

    /// Register a queue configuration.
    pub fn register_queue(&self, config: MatchmakerConfig) {
        self.matchmaker.register_queue(config);
    }

    /// Add a ticket to matchmaking with hooks.
    pub fn add(&self, ctx: &HookContext, ticket: MatchmakerTicket) -> Result<String> {
        // Build payload for hooks
        let payload = serde_json::json!({
            "ticket_id": ticket.id,
            "queue": ticket.queue,
            "players": ticket.players.iter().map(|p| serde_json::json!({
                "user_id": p.user_id,
                "session_id": p.session_id,
                "username": p.username,
                "skill": p.skill,
            })).collect::<Vec<_>>(),
            "properties": ticket.properties,
        });

        // Execute before hooks (can reject)
        let _payload = self.hooks.before(HookOperation::MatchmakerAdd, ctx, payload)?;

        // Execute actual operation
        let ticket_id = self.matchmaker.add(ticket)?;

        // Update metrics
        self.update_queue_metrics();

        // Execute after hooks
        let result_json = serde_json::json!({
            "ticket_id": ticket_id,
        });
        self.hooks.after(HookOperation::MatchmakerAdd, ctx, &_payload, &result_json);

        Ok(ticket_id)
    }

    /// Remove a ticket from matchmaking with hooks.
    pub fn remove(&self, ctx: &HookContext, ticket_id: &str) -> Result<MatchmakerTicket> {
        // Build payload for hooks
        let payload = serde_json::json!({
            "ticket_id": ticket_id,
        });

        // Execute before hooks (can reject)
        let payload = self.hooks.before(HookOperation::MatchmakerRemove, ctx, payload)?;

        let ticket_id = payload["ticket_id"].as_str().unwrap_or(ticket_id);

        // Execute actual operation
        let ticket = self.matchmaker.remove(ticket_id)?;

        // Update metrics
        self.update_queue_metrics();

        // Execute after hooks
        let result_json = serde_json::json!({
            "ticket_id": ticket.id,
            "queue": ticket.queue,
        });
        self.hooks.after(HookOperation::MatchmakerRemove, ctx, &payload, &result_json);

        Ok(ticket)
    }

    /// Remove a player's ticket with hooks.
    pub fn remove_player(&self, ctx: &HookContext, user_id: &str) -> Result<MatchmakerTicket> {
        let payload = serde_json::json!({
            "user_id": user_id,
        });

        let payload = self.hooks.before(HookOperation::MatchmakerRemove, ctx, payload)?;

        let user_id = payload["user_id"].as_str().unwrap_or(user_id);

        let ticket = self.matchmaker.remove_player(user_id)?;

        self.update_queue_metrics();

        let result_json = serde_json::json!({
            "ticket_id": ticket.id,
            "queue": ticket.queue,
        });
        self.hooks.after(HookOperation::MatchmakerRemove, ctx, &payload, &result_json);

        Ok(ticket)
    }

    /// Check if a player is queued.
    pub fn is_queued(&self, user_id: &str) -> bool {
        self.matchmaker.is_queued(user_id)
    }

    /// Get a player's ticket.
    pub fn get_ticket(&self, user_id: &str) -> Option<MatchmakerTicket> {
        self.matchmaker.get_ticket(user_id)
    }

    /// Process matchmaking for all queues.
    /// Executes hooks and sends notifications for each match.
    pub fn tick(&self, ctx: &HookContext) -> Vec<MatchmakerMatch> {
        let matches = self.matchmaker.tick();

        // Record metrics
        #[cfg(feature = "metrics")]
        if let Some(ref m) = self.metrics {
            for _ in &matches {
                m.matchmaker_matches_total.inc();
            }
        }

        // Execute hooks and send notifications for each match
        for mm_match in &matches {
            let payload = serde_json::json!({
                "match_id": mm_match.id,
                "queue": mm_match.queue,
                "players": mm_match.players.iter().map(|p| serde_json::json!({
                    "user_id": p.user_id,
                    "session_id": p.session_id,
                    "username": p.username,
                    "skill": p.skill,
                })).collect::<Vec<_>>(),
                "properties": mm_match.properties,
            });

            // Execute after hook for match created
            self.hooks.after(HookOperation::MatchmakerMatch, ctx, &payload, &payload);

            // Send notifications to all players
            if let Some(ref notifications) = self.notifications {
                for player in &mm_match.players {
                    let _ = notifications.notify_match_found(&player.user_id, &mm_match.id, &mm_match.queue);
                }
            }
        }

        // Update queue size metrics after processing
        self.update_queue_metrics();

        matches
    }

    /// Get pending matches (and clear them).
    pub fn drain_matches(&self) -> Vec<MatchmakerMatch> {
        self.matchmaker.drain_matches()
    }

    /// Register a match handler callback.
    pub fn on_match<F>(&self, handler: F)
    where
        F: Fn(&MatchmakerMatch) + Send + Sync + 'static,
    {
        self.matchmaker.on_match(handler);
    }

    /// Get queue stats.
    pub fn stats(&self, queue: &str) -> Option<QueueStats> {
        self.matchmaker.stats(queue)
    }

    /// List all registered queues with their stats.
    pub fn list_queues(&self) -> Vec<QueueStats> {
        self.matchmaker.list_queues()
    }

    /// Get underlying matchmaker (for operations that don't need hooks).
    pub fn inner(&self) -> &Arc<Matchmaker> {
        &self.matchmaker
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

    #[test]
    fn test_hooked_matchmaker_add_ticket() {
        let matchmaker = Arc::new(Matchmaker::new());
        let registry = Arc::new(HookRegistry::new());

        matchmaker.register_queue(MatchmakerConfig {
            queue: "ranked".into(),
            min_players: 2,
            max_players: 2,
            initial_skill_range: 100.0,
            ..Default::default()
        });

        let hooked = HookedMatchmaker::new(matchmaker, registry);

        let ctx = HookContext::default();
        let ticket = crate::matchmaker::create_ticket("ranked", "user1", 1, "Alice", 1500.0);

        let ticket_id = hooked.add(&ctx, ticket).unwrap();
        assert!(!ticket_id.is_empty());
        assert!(hooked.is_queued("user1"));
    }

    #[test]
    fn test_hooked_matchmaker_reject_add() {
        let matchmaker = Arc::new(Matchmaker::new());
        let registry = Arc::new(HookRegistry::new());

        matchmaker.register_queue(MatchmakerConfig {
            queue: "ranked".into(),
            min_players: 2,
            max_players: 2,
            ..Default::default()
        });

        // Block all matchmaker adds
        struct BlockMatchmaking;
        impl BeforeHook for BlockMatchmaking {
            fn execute(
                &self,
                _ctx: &HookContext,
                _payload: serde_json::Value,
            ) -> crate::hooks::Result<BeforeHookResult> {
                Ok(BeforeHookResult::Reject("Matchmaking disabled".to_string()))
            }
        }

        registry.register_before(HookOperation::MatchmakerAdd, BlockMatchmaking);

        let hooked = HookedMatchmaker::new(matchmaker, registry);

        let ctx = HookContext::default();
        let ticket = crate::matchmaker::create_ticket("ranked", "user1", 1, "Alice", 1500.0);

        let result = hooked.add(&ctx, ticket);
        assert!(matches!(result, Err(ServiceError::Hook(HookError::Rejected(_)))));
    }

    #[test]
    fn test_hooked_matchmaker_tick() {
        let matchmaker = Arc::new(Matchmaker::new());
        let registry = Arc::new(HookRegistry::new());

        matchmaker.register_queue(MatchmakerConfig {
            queue: "ranked".into(),
            min_players: 2,
            max_players: 2,
            initial_skill_range: 100.0,
            ..Default::default()
        });

        let hooked = HookedMatchmaker::new(matchmaker, registry);

        let ctx = HookContext::default();

        // Add two players
        let ticket1 = crate::matchmaker::create_ticket("ranked", "user1", 1, "Alice", 1500.0);
        let ticket2 = crate::matchmaker::create_ticket("ranked", "user2", 2, "Bob", 1550.0);

        hooked.add(&ctx, ticket1).unwrap();
        hooked.add(&ctx, ticket2).unwrap();

        // Process matchmaking
        let matches = hooked.tick(&ctx);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].players.len(), 2);

        // Players should no longer be queued
        assert!(!hooked.is_queued("user1"));
        assert!(!hooked.is_queued("user2"));
    }
}
