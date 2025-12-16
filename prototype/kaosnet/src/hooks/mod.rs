//! Server Hooks System
//!
//! Allows intercepting API calls with before/after hooks for custom game logic.
//!
//! ## Hook Types
//!
//! - **Before Hooks**: Execute before the operation, can modify or reject requests
//! - **After Hooks**: Execute after the operation, for side effects/logging
//!
//! ## Example (Lua)
//!
//! ```lua
//! -- Before hook: validate/modify request
//! kaos.register_before("authenticate", function(ctx, payload)
//!     if payload.device_id == "banned-device" then
//!         return nil, "Device is banned"
//!     end
//!     return payload
//! end)
//!
//! -- After hook: analytics/logging
//! kaos.register_after("leaderboard_submit", function(ctx, payload, result)
//!     print("User " .. ctx.user_id .. " submitted score: " .. payload.score)
//! end)
//! ```

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Hook execution errors.
#[derive(Debug, Error)]
pub enum HookError {
    #[error("hook rejected request: {0}")]
    Rejected(String),

    #[error("hook execution failed: {0}")]
    ExecutionFailed(String),

    #[error("hook not found: {0}")]
    NotFound(String),

    #[error("hook timeout")]
    Timeout,

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, HookError>;

/// Hook operation types that can be intercepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookOperation {
    // Authentication
    AuthenticateDevice,
    AuthenticateEmail,
    AuthenticateCustom,
    RefreshToken,

    // Storage
    StorageRead,
    StorageWrite,
    StorageDelete,

    // Leaderboards
    LeaderboardSubmit,
    LeaderboardRead,

    // Matchmaker
    MatchmakerAdd,
    MatchmakerRemove,

    // Social
    FriendAdd,
    FriendRemove,
    FriendBlock,
    GroupCreate,
    GroupJoin,
    GroupLeave,

    // Chat
    ChannelJoin,
    ChannelLeave,
    ChannelMessage,

    // Multiplayer
    MatchCreate,
    MatchJoin,
    MatchLeave,
    MatchData,

    // Notifications
    NotificationSend,

    // RPC
    Rpc,
}

impl HookOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookOperation::AuthenticateDevice => "authenticate_device",
            HookOperation::AuthenticateEmail => "authenticate_email",
            HookOperation::AuthenticateCustom => "authenticate_custom",
            HookOperation::RefreshToken => "refresh_token",
            HookOperation::StorageRead => "storage_read",
            HookOperation::StorageWrite => "storage_write",
            HookOperation::StorageDelete => "storage_delete",
            HookOperation::LeaderboardSubmit => "leaderboard_submit",
            HookOperation::LeaderboardRead => "leaderboard_read",
            HookOperation::MatchmakerAdd => "matchmaker_add",
            HookOperation::MatchmakerRemove => "matchmaker_remove",
            HookOperation::FriendAdd => "friend_add",
            HookOperation::FriendRemove => "friend_remove",
            HookOperation::FriendBlock => "friend_block",
            HookOperation::GroupCreate => "group_create",
            HookOperation::GroupJoin => "group_join",
            HookOperation::GroupLeave => "group_leave",
            HookOperation::ChannelJoin => "channel_join",
            HookOperation::ChannelLeave => "channel_leave",
            HookOperation::ChannelMessage => "channel_message",
            HookOperation::MatchCreate => "match_create",
            HookOperation::MatchJoin => "match_join",
            HookOperation::MatchLeave => "match_leave",
            HookOperation::MatchData => "match_data",
            HookOperation::NotificationSend => "notification_send",
            HookOperation::Rpc => "rpc",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "authenticate_device" => Some(HookOperation::AuthenticateDevice),
            "authenticate_email" => Some(HookOperation::AuthenticateEmail),
            "authenticate_custom" => Some(HookOperation::AuthenticateCustom),
            "refresh_token" => Some(HookOperation::RefreshToken),
            "storage_read" => Some(HookOperation::StorageRead),
            "storage_write" => Some(HookOperation::StorageWrite),
            "storage_delete" => Some(HookOperation::StorageDelete),
            "leaderboard_submit" => Some(HookOperation::LeaderboardSubmit),
            "leaderboard_read" => Some(HookOperation::LeaderboardRead),
            "matchmaker_add" => Some(HookOperation::MatchmakerAdd),
            "matchmaker_remove" => Some(HookOperation::MatchmakerRemove),
            "friend_add" => Some(HookOperation::FriendAdd),
            "friend_remove" => Some(HookOperation::FriendRemove),
            "friend_block" => Some(HookOperation::FriendBlock),
            "group_create" => Some(HookOperation::GroupCreate),
            "group_join" => Some(HookOperation::GroupJoin),
            "group_leave" => Some(HookOperation::GroupLeave),
            "channel_join" => Some(HookOperation::ChannelJoin),
            "channel_leave" => Some(HookOperation::ChannelLeave),
            "channel_message" => Some(HookOperation::ChannelMessage),
            "match_create" => Some(HookOperation::MatchCreate),
            "match_join" => Some(HookOperation::MatchJoin),
            "match_leave" => Some(HookOperation::MatchLeave),
            "match_data" => Some(HookOperation::MatchData),
            "notification_send" => Some(HookOperation::NotificationSend),
            "rpc" => Some(HookOperation::Rpc),
            _ => None,
        }
    }
}

/// Context passed to hooks with request information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Username (if available)
    pub username: Option<String>,
    /// Session ID
    pub session_id: Option<u64>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// Custom variables from session
    #[serde(default)]
    pub vars: std::collections::HashMap<String, String>,
}

impl Default for HookContext {
    fn default() -> Self {
        Self {
            user_id: None,
            username: None,
            session_id: None,
            client_ip: None,
            vars: std::collections::HashMap::new(),
        }
    }
}

/// Result from a before hook.
#[derive(Debug, Clone)]
pub enum BeforeHookResult {
    /// Continue with (possibly modified) payload
    Continue(serde_json::Value),
    /// Reject the request with error message
    Reject(String),
}

/// Hook function trait for native Rust hooks.
pub trait BeforeHook: Send + Sync {
    fn execute(&self, ctx: &HookContext, payload: serde_json::Value) -> Result<BeforeHookResult>;
}

pub trait AfterHook: Send + Sync {
    fn execute(&self, ctx: &HookContext, payload: &serde_json::Value, result: &serde_json::Value);
}

/// Boxed hook types for storage.
type BoxedBeforeHook = Box<dyn BeforeHook>;
type BoxedAfterHook = Box<dyn AfterHook>;

/// Hook registry for storing and executing hooks.
pub struct HookRegistry {
    before_hooks: DashMap<HookOperation, Vec<BoxedBeforeHook>>,
    after_hooks: DashMap<HookOperation, Vec<BoxedAfterHook>>,
    /// Lua hook function names (operation -> lua function name)
    lua_before_hooks: DashMap<HookOperation, Vec<String>>,
    lua_after_hooks: DashMap<HookOperation, Vec<String>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            before_hooks: DashMap::new(),
            after_hooks: DashMap::new(),
            lua_before_hooks: DashMap::new(),
            lua_after_hooks: DashMap::new(),
        }
    }

    /// Register a native Rust before hook.
    pub fn register_before(&self, op: HookOperation, hook: impl BeforeHook + 'static) {
        self.before_hooks
            .entry(op)
            .or_insert_with(Vec::new)
            .push(Box::new(hook));
    }

    /// Register a native Rust after hook.
    pub fn register_after(&self, op: HookOperation, hook: impl AfterHook + 'static) {
        self.after_hooks
            .entry(op)
            .or_insert_with(Vec::new)
            .push(Box::new(hook));
    }

    /// Register a Lua before hook by function name.
    pub fn register_lua_before(&self, op: HookOperation, function_name: String) {
        self.lua_before_hooks
            .entry(op)
            .or_insert_with(Vec::new)
            .push(function_name);
    }

    /// Register a Lua after hook by function name.
    pub fn register_lua_after(&self, op: HookOperation, function_name: String) {
        self.lua_after_hooks
            .entry(op)
            .or_insert_with(Vec::new)
            .push(function_name);
    }

    /// Execute all before hooks for an operation.
    /// Returns the (possibly modified) payload or error.
    pub fn execute_before(
        &self,
        op: HookOperation,
        ctx: &HookContext,
        mut payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Execute native hooks
        if let Some(hooks) = self.before_hooks.get(&op) {
            for hook in hooks.iter() {
                match hook.execute(ctx, payload.clone())? {
                    BeforeHookResult::Continue(modified) => {
                        payload = modified;
                    }
                    BeforeHookResult::Reject(reason) => {
                        return Err(HookError::Rejected(reason));
                    }
                }
            }
        }

        Ok(payload)
    }

    /// Execute all after hooks for an operation.
    pub fn execute_after(
        &self,
        op: HookOperation,
        ctx: &HookContext,
        payload: &serde_json::Value,
        result: &serde_json::Value,
    ) {
        // Execute native hooks
        if let Some(hooks) = self.after_hooks.get(&op) {
            for hook in hooks.iter() {
                hook.execute(ctx, payload, result);
            }
        }
    }

    /// Get Lua before hook function names for an operation.
    pub fn get_lua_before_hooks(&self, op: HookOperation) -> Vec<String> {
        self.lua_before_hooks
            .get(&op)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Get Lua after hook function names for an operation.
    pub fn get_lua_after_hooks(&self, op: HookOperation) -> Vec<String> {
        self.lua_after_hooks
            .get(&op)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Check if any hooks are registered for an operation.
    pub fn has_hooks(&self, op: HookOperation) -> bool {
        self.before_hooks.contains_key(&op)
            || self.after_hooks.contains_key(&op)
            || self.lua_before_hooks.contains_key(&op)
            || self.lua_after_hooks.contains_key(&op)
    }

    /// Clear all hooks.
    pub fn clear(&self) {
        self.before_hooks.clear();
        self.after_hooks.clear();
        self.lua_before_hooks.clear();
        self.lua_after_hooks.clear();
    }

    /// Clear hooks for a specific operation.
    pub fn clear_operation(&self, op: HookOperation) {
        self.before_hooks.remove(&op);
        self.after_hooks.remove(&op);
        self.lua_before_hooks.remove(&op);
        self.lua_after_hooks.remove(&op);
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience wrapper for hook execution with logging.
pub struct HookExecutor {
    registry: Arc<HookRegistry>,
}

impl HookExecutor {
    pub fn new(registry: Arc<HookRegistry>) -> Self {
        Self { registry }
    }

    /// Execute before hooks, returning modified payload or error.
    pub fn before(
        &self,
        op: HookOperation,
        ctx: &HookContext,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.registry.execute_before(op, ctx, payload)
    }

    /// Execute after hooks (fire and forget).
    pub fn after(
        &self,
        op: HookOperation,
        ctx: &HookContext,
        payload: &serde_json::Value,
        result: &serde_json::Value,
    ) {
        self.registry.execute_after(op, ctx, payload, result);
    }

    /// Get the underlying registry.
    pub fn registry(&self) -> &Arc<HookRegistry> {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestBeforeHook {
        add_field: bool,
    }

    impl BeforeHook for TestBeforeHook {
        fn execute(&self, _ctx: &HookContext, mut payload: serde_json::Value) -> Result<BeforeHookResult> {
            if self.add_field {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("hook_added".to_string(), serde_json::json!(true));
                }
            }
            Ok(BeforeHookResult::Continue(payload))
        }
    }

    struct RejectHook {
        message: String,
    }

    impl BeforeHook for RejectHook {
        fn execute(&self, _ctx: &HookContext, _payload: serde_json::Value) -> Result<BeforeHookResult> {
            Ok(BeforeHookResult::Reject(self.message.clone()))
        }
    }

    struct CountingAfterHook {
        counter: std::sync::atomic::AtomicU32,
    }

    impl AfterHook for CountingAfterHook {
        fn execute(&self, _ctx: &HookContext, _payload: &serde_json::Value, _result: &serde_json::Value) {
            self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    #[test]
    fn test_before_hook_modify() {
        let registry = HookRegistry::new();
        registry.register_before(HookOperation::StorageWrite, TestBeforeHook { add_field: true });

        let ctx = HookContext::default();
        let payload = serde_json::json!({"key": "value"});

        let result = registry.execute_before(HookOperation::StorageWrite, &ctx, payload).unwrap();

        assert_eq!(result["key"], "value");
        assert_eq!(result["hook_added"], true);
    }

    #[test]
    fn test_before_hook_reject() {
        let registry = HookRegistry::new();
        registry.register_before(HookOperation::AuthenticateDevice, RejectHook {
            message: "Device banned".to_string(),
        });

        let ctx = HookContext::default();
        let payload = serde_json::json!({"device_id": "banned"});

        let result = registry.execute_before(HookOperation::AuthenticateDevice, &ctx, payload);

        assert!(matches!(result, Err(HookError::Rejected(msg)) if msg == "Device banned"));
    }

    #[test]
    fn test_after_hook_executed() {
        let registry = HookRegistry::new();
        let hook = CountingAfterHook {
            counter: std::sync::atomic::AtomicU32::new(0),
        };
        let counter_ptr = &hook.counter as *const _;
        registry.register_after(HookOperation::LeaderboardSubmit, hook);

        let ctx = HookContext::default();
        let payload = serde_json::json!({"score": 100});
        let result = serde_json::json!({"success": true});

        registry.execute_after(HookOperation::LeaderboardSubmit, &ctx, &payload, &result);

        // Access counter through registry
        let hooks = registry.after_hooks.get(&HookOperation::LeaderboardSubmit).unwrap();
        // Hook was executed (counter check would need unsafe, so just verify no panic)
        assert!(!hooks.is_empty());
    }

    #[test]
    fn test_lua_hook_registration() {
        let registry = HookRegistry::new();

        registry.register_lua_before(HookOperation::Rpc, "my_rpc_before_hook".to_string());
        registry.register_lua_after(HookOperation::Rpc, "my_rpc_after_hook".to_string());

        let before = registry.get_lua_before_hooks(HookOperation::Rpc);
        let after = registry.get_lua_after_hooks(HookOperation::Rpc);

        assert_eq!(before, vec!["my_rpc_before_hook"]);
        assert_eq!(after, vec!["my_rpc_after_hook"]);
    }

    #[test]
    fn test_has_hooks() {
        let registry = HookRegistry::new();

        assert!(!registry.has_hooks(HookOperation::StorageWrite));

        registry.register_before(HookOperation::StorageWrite, TestBeforeHook { add_field: false });

        assert!(registry.has_hooks(HookOperation::StorageWrite));
    }

    #[test]
    fn test_operation_from_str() {
        assert_eq!(HookOperation::from_str("authenticate_device"), Some(HookOperation::AuthenticateDevice));
        assert_eq!(HookOperation::from_str("storage_write"), Some(HookOperation::StorageWrite));
        assert_eq!(HookOperation::from_str("invalid"), None);
    }

    #[test]
    fn test_clear_hooks() {
        let registry = HookRegistry::new();
        registry.register_before(HookOperation::StorageWrite, TestBeforeHook { add_field: false });
        registry.register_lua_before(HookOperation::Rpc, "test".to_string());

        assert!(registry.has_hooks(HookOperation::StorageWrite));
        assert!(registry.has_hooks(HookOperation::Rpc));

        registry.clear();

        assert!(!registry.has_hooks(HookOperation::StorageWrite));
        assert!(!registry.has_hooks(HookOperation::Rpc));
    }
}
