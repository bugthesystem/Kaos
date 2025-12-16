//! Authoritative Match Handler System
//!
//! Server-validated game state for competitive multiplayer games.
//!
//! ## Overview
//!
//! The match handler system provides authoritative server-side game logic:
//! - Server owns and validates all game state
//! - Clients send inputs/actions, server processes them
//! - Server broadcasts validated state updates
//!
//! ## Example (Rust)
//!
//! ```rust,ignore
//! use kaosnet::match_handler::{MatchHandler, MatchContext, MatchState};
//!
//! struct TicTacToeHandler;
//!
//! impl MatchHandler for TicTacToeHandler {
//!     fn init(&self, ctx: &MatchContext, params: serde_json::Value) -> MatchState {
//!         MatchState::new(serde_json::json!({
//!             "board": [[null; 3]; 3],
//!             "current_player": 0,
//!         }))
//!     }
//!
//!     fn join(&self, ctx: &MatchContext, state: &mut MatchState, presences: Vec<Presence>) {
//!         // Handle player joining
//!     }
//!
//!     fn tick(&self, ctx: &MatchContext, state: &mut MatchState, messages: Vec<MatchMessage>) {
//!         // Process player inputs and update state
//!     }
//! }
//! ```
//!
//! ## Example (Lua)
//!
//! ```lua
//! function match_init(ctx, params)
//!     return {
//!         state = { board = {}, current_player = 1 },
//!         tick_rate = 1, -- 1 tick per second for turn-based
//!         label = "tic-tac-toe"
//!     }
//! end
//!
//! function match_join(ctx, state, presences)
//!     for _, p in ipairs(presences) do
//!         print("Player joined: " .. p.username)
//!     end
//!     return state
//! end
//!
//! function match_tick(ctx, state, tick, messages)
//!     for _, msg in ipairs(messages) do
//!         -- Process player move
//!     end
//!     return state
//! end
//! ```

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;

/// Match handler errors.
#[derive(Debug, Error)]
pub enum MatchError {
    #[error("match not found: {0}")]
    NotFound(String),

    #[error("match full: {current}/{max} players")]
    MatchFull { current: usize, max: usize },

    #[error("player already in match")]
    AlreadyJoined,

    #[error("player not in match")]
    NotInMatch,

    #[error("match already started")]
    AlreadyStarted,

    #[error("match ended")]
    MatchEnded,

    #[error("invalid state: {0}")]
    InvalidState(String),

    #[error("handler error: {0}")]
    HandlerError(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, MatchError>;

/// Presence information for a player in a match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPresence {
    pub user_id: String,
    pub session_id: u64,
    pub username: String,
    pub node: Option<String>,
}

/// Message from a client in a match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchMessage {
    /// Sender presence
    pub sender: MatchPresence,
    /// OpCode for message type
    pub op_code: i64,
    /// Message data
    pub data: Vec<u8>,
    /// Reliable delivery
    pub reliable: bool,
    /// Timestamp when received
    pub received_at: u64,
}

/// Context passed to match handler callbacks.
#[derive(Debug, Clone)]
pub struct MatchContext {
    /// Match ID
    pub match_id: String,
    /// Node this match is running on
    pub node: String,
    /// Match creation time
    pub created_at: Instant,
    /// Match label (for filtering)
    pub label: Option<String>,
    /// Custom context vars
    pub vars: HashMap<String, String>,
}

/// Match state container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchState {
    /// The game state (JSON-serializable)
    pub state: serde_json::Value,
    /// Current tick number
    pub tick: u64,
    /// Players currently in the match
    pub presences: Vec<MatchPresence>,
    /// Optional match label
    pub label: Option<String>,
    /// Tick rate (ticks per second)
    pub tick_rate: u32,
    /// Max players allowed
    pub max_size: usize,
    /// Whether match is empty should be terminated
    pub empty_timeout_secs: Option<u32>,
}

impl MatchState {
    pub fn new(initial_state: serde_json::Value) -> Self {
        Self {
            state: initial_state,
            tick: 0,
            presences: Vec::new(),
            label: None,
            tick_rate: 10,
            max_size: 16,
            empty_timeout_secs: Some(30),
        }
    }

    pub fn with_tick_rate(mut self, rate: u32) -> Self {
        self.tick_rate = rate;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_max_size(mut self, max: usize) -> Self {
        self.max_size = max;
        self
    }
}

/// Result from match initialization.
#[derive(Debug, Clone)]
pub struct MatchInit {
    pub state: MatchState,
    pub tick_rate: u32,
    pub label: Option<String>,
}

/// Dispatcher for sending messages to clients.
pub trait MatchDispatcher: Send + Sync {
    /// Broadcast to all presences
    fn broadcast(&self, op_code: i64, data: &[u8], reliable: bool);

    /// Send to specific presences
    fn send(&self, presences: &[MatchPresence], op_code: i64, data: &[u8], reliable: bool);

    /// Kick a player from the match
    fn kick(&self, presence: &MatchPresence);
}

/// Match handler trait for implementing game logic.
pub trait MatchHandler: Send + Sync {
    /// Initialize the match with parameters.
    fn init(&self, ctx: &MatchContext, params: serde_json::Value) -> Result<MatchInit>;

    /// Called when players join the match.
    fn join(
        &self,
        ctx: &MatchContext,
        dispatcher: &dyn MatchDispatcher,
        state: &mut MatchState,
        presences: Vec<MatchPresence>,
    ) -> Result<()>;

    /// Called when players leave the match.
    fn leave(
        &self,
        ctx: &MatchContext,
        dispatcher: &dyn MatchDispatcher,
        state: &mut MatchState,
        presences: Vec<MatchPresence>,
    ) -> Result<()>;

    /// Called every tick with pending messages.
    fn tick(
        &self,
        ctx: &MatchContext,
        dispatcher: &dyn MatchDispatcher,
        state: &mut MatchState,
        messages: Vec<MatchMessage>,
    ) -> Result<()>;

    /// Called when the match is being terminated.
    fn terminate(&self, ctx: &MatchContext, state: &MatchState) -> Result<()>;

    /// Called for custom signals (e.g., from RPCs).
    fn signal(
        &self,
        ctx: &MatchContext,
        dispatcher: &dyn MatchDispatcher,
        state: &mut MatchState,
        data: &str,
    ) -> Result<Option<String>> {
        let _ = (ctx, dispatcher, state, data);
        Ok(None)
    }
}

/// Match lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchLifecycle {
    Created,
    Running,
    Stopped,
}

/// Active match instance.
pub struct Match {
    pub id: String,
    pub context: MatchContext,
    pub state: MatchState,
    pub lifecycle: MatchLifecycle,
    pub handler_name: String,
    pub created_at: Instant,
    pub last_activity: Instant,
    pending_messages: Vec<MatchMessage>,
}

impl Match {
    fn new(id: String, context: MatchContext, state: MatchState, handler_name: String) -> Self {
        let now = Instant::now();
        Self {
            id,
            context,
            state,
            lifecycle: MatchLifecycle::Created,
            handler_name,
            created_at: now,
            last_activity: now,
            pending_messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, msg: MatchMessage) {
        self.pending_messages.push(msg);
        self.last_activity = Instant::now();
    }

    pub fn take_messages(&mut self) -> Vec<MatchMessage> {
        std::mem::take(&mut self.pending_messages)
    }

    pub fn is_empty(&self) -> bool {
        self.state.presences.is_empty()
    }

    pub fn player_count(&self) -> usize {
        self.state.presences.len()
    }
}

/// Null dispatcher (for testing).
pub struct NullDispatcher;

impl MatchDispatcher for NullDispatcher {
    fn broadcast(&self, _op_code: i64, _data: &[u8], _reliable: bool) {}
    fn send(&self, _presences: &[MatchPresence], _op_code: i64, _data: &[u8], _reliable: bool) {}
    fn kick(&self, _presence: &MatchPresence) {}
}

/// Registry for match handlers.
pub struct MatchHandlerRegistry {
    handlers: DashMap<String, Arc<dyn MatchHandler>>,
}

impl MatchHandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }

    /// Register a match handler.
    pub fn register(&self, name: impl Into<String>, handler: impl MatchHandler + 'static) {
        self.handlers.insert(name.into(), Arc::new(handler));
    }

    /// Get a handler by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn MatchHandler>> {
        self.handlers.get(name).map(|r| r.clone())
    }

    /// List registered handler names.
    pub fn list(&self) -> Vec<String> {
        self.handlers.iter().map(|r| r.key().clone()).collect()
    }
}

impl Default for MatchHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for active matches.
pub struct MatchRegistry {
    matches: DashMap<String, Match>,
    handlers: Arc<MatchHandlerRegistry>,
    id_counter: AtomicU64,
}

impl MatchRegistry {
    pub fn new(handlers: Arc<MatchHandlerRegistry>) -> Self {
        Self {
            matches: DashMap::new(),
            handlers,
            id_counter: AtomicU64::new(1),
        }
    }

    /// Create a new match.
    pub fn create(
        &self,
        handler_name: &str,
        params: serde_json::Value,
        label: Option<String>,
    ) -> Result<String> {
        let handler = self.handlers.get(handler_name)
            .ok_or_else(|| MatchError::NotFound(format!("Handler not found: {}", handler_name)))?;

        let id = format!("match-{}", self.id_counter.fetch_add(1, Ordering::Relaxed));

        let context = MatchContext {
            match_id: id.clone(),
            node: "local".to_string(),
            created_at: Instant::now(),
            label: label.clone(),
            vars: HashMap::new(),
        };

        let init = handler.init(&context, params)?;
        let mut state = init.state;
        state.tick_rate = init.tick_rate;
        state.label = init.label.or(label);

        let m = Match::new(id.clone(), context, state, handler_name.to_string());
        self.matches.insert(id.clone(), m);

        Ok(id)
    }

    /// Get a match by ID.
    pub fn get(&self, match_id: &str) -> Option<dashmap::mapref::one::Ref<'_, String, Match>> {
        self.matches.get(match_id)
    }

    /// Get mutable reference to a match.
    pub fn get_mut(&self, match_id: &str) -> Option<dashmap::mapref::one::RefMut<'_, String, Match>> {
        self.matches.get_mut(match_id)
    }

    /// Join a match.
    pub fn join(&self, match_id: &str, presence: MatchPresence) -> Result<()> {
        let mut m = self.matches.get_mut(match_id)
            .ok_or_else(|| MatchError::NotFound(match_id.to_string()))?;

        if m.lifecycle == MatchLifecycle::Stopped {
            return Err(MatchError::MatchEnded);
        }

        if m.state.presences.iter().any(|p| p.session_id == presence.session_id) {
            return Err(MatchError::AlreadyJoined);
        }

        if m.state.presences.len() >= m.state.max_size {
            return Err(MatchError::MatchFull {
                current: m.state.presences.len(),
                max: m.state.max_size,
            });
        }

        let handler = self.handlers.get(&m.handler_name)
            .ok_or_else(|| MatchError::Internal("Handler not found".to_string()))?;

        m.state.presences.push(presence.clone());
        m.lifecycle = MatchLifecycle::Running;

        // Clone context to avoid borrow issues
        let ctx = m.context.clone();
        let dispatcher = NullDispatcher;
        handler.join(&ctx, &dispatcher, &mut m.state, vec![presence])?;

        Ok(())
    }

    /// Leave a match.
    pub fn leave(&self, match_id: &str, session_id: u64) -> Result<()> {
        let mut m = self.matches.get_mut(match_id)
            .ok_or_else(|| MatchError::NotFound(match_id.to_string()))?;

        let idx = m.state.presences.iter()
            .position(|p| p.session_id == session_id)
            .ok_or(MatchError::NotInMatch)?;

        let presence = m.state.presences.remove(idx);

        let handler = self.handlers.get(&m.handler_name)
            .ok_or_else(|| MatchError::Internal("Handler not found".to_string()))?;

        // Clone context to avoid borrow issues
        let ctx = m.context.clone();
        let dispatcher = NullDispatcher;
        handler.leave(&ctx, &dispatcher, &mut m.state, vec![presence])?;

        Ok(())
    }

    /// Send a message to a match.
    pub fn send_message(&self, match_id: &str, msg: MatchMessage) -> Result<()> {
        let mut m = self.matches.get_mut(match_id)
            .ok_or_else(|| MatchError::NotFound(match_id.to_string()))?;

        if m.lifecycle == MatchLifecycle::Stopped {
            return Err(MatchError::MatchEnded);
        }

        m.add_message(msg);
        Ok(())
    }

    /// Tick a match (call from game loop).
    pub fn tick(&self, match_id: &str) -> Result<()> {
        let mut m = self.matches.get_mut(match_id)
            .ok_or_else(|| MatchError::NotFound(match_id.to_string()))?;

        if m.lifecycle != MatchLifecycle::Running {
            return Ok(());
        }

        let messages = m.take_messages();
        m.state.tick += 1;

        let handler = self.handlers.get(&m.handler_name)
            .ok_or_else(|| MatchError::Internal("Handler not found".to_string()))?;

        // Clone context to avoid borrow issues
        let ctx = m.context.clone();
        let dispatcher = NullDispatcher;
        handler.tick(&ctx, &dispatcher, &mut m.state, messages)?;

        Ok(())
    }

    /// Stop a match.
    pub fn stop(&self, match_id: &str) -> Result<()> {
        let mut m = self.matches.get_mut(match_id)
            .ok_or_else(|| MatchError::NotFound(match_id.to_string()))?;

        if m.lifecycle == MatchLifecycle::Stopped {
            return Ok(());
        }

        let handler = self.handlers.get(&m.handler_name)
            .ok_or_else(|| MatchError::Internal("Handler not found".to_string()))?;

        handler.terminate(&m.context, &m.state)?;
        m.lifecycle = MatchLifecycle::Stopped;

        Ok(())
    }

    /// Remove a match from the registry.
    pub fn remove(&self, match_id: &str) -> Option<Match> {
        self.matches.remove(match_id).map(|(_, m)| m)
    }

    /// List all matches.
    pub fn list(&self) -> Vec<(String, MatchLifecycle, usize, Option<String>)> {
        self.matches.iter()
            .map(|r| {
                let m = r.value();
                (m.id.clone(), m.lifecycle, m.player_count(), m.state.label.clone())
            })
            .collect()
    }

    /// List matches by label.
    pub fn list_by_label(&self, label: &str) -> Vec<String> {
        self.matches.iter()
            .filter(|r| r.value().state.label.as_deref() == Some(label))
            .map(|r| r.key().clone())
            .collect()
    }

    /// Count active matches.
    pub fn count(&self) -> usize {
        self.matches.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;

    impl MatchHandler for TestHandler {
        fn init(&self, ctx: &MatchContext, _params: serde_json::Value) -> Result<MatchInit> {
            Ok(MatchInit {
                state: MatchState::new(serde_json::json!({"score": 0})),
                tick_rate: 10,
                // Use label from context (passed via create) - don't override
                label: ctx.label.clone(),
            })
        }

        fn join(
            &self,
            _ctx: &MatchContext,
            _dispatcher: &dyn MatchDispatcher,
            state: &mut MatchState,
            presences: Vec<MatchPresence>,
        ) -> Result<()> {
            // Add players to state
            if let Some(obj) = state.state.as_object_mut() {
                let count = presences.len() as i64;
                let current = obj.get("players").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("players".to_string(), serde_json::json!(current + count));
            }
            Ok(())
        }

        fn leave(
            &self,
            _ctx: &MatchContext,
            _dispatcher: &dyn MatchDispatcher,
            state: &mut MatchState,
            presences: Vec<MatchPresence>,
        ) -> Result<()> {
            if let Some(obj) = state.state.as_object_mut() {
                let count = presences.len() as i64;
                let current = obj.get("players").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("players".to_string(), serde_json::json!((current - count).max(0)));
            }
            Ok(())
        }

        fn tick(
            &self,
            _ctx: &MatchContext,
            _dispatcher: &dyn MatchDispatcher,
            state: &mut MatchState,
            messages: Vec<MatchMessage>,
        ) -> Result<()> {
            // Process messages - increment score for each
            if let Some(obj) = state.state.as_object_mut() {
                let score = obj.get("score").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("score".to_string(), serde_json::json!(score + messages.len() as i64));
            }
            Ok(())
        }

        fn terminate(&self, _ctx: &MatchContext, _state: &MatchState) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_create_match() {
        let handlers = Arc::new(MatchHandlerRegistry::new());
        handlers.register("test", TestHandler);

        let registry = MatchRegistry::new(handlers);
        let match_id = registry.create("test", serde_json::json!({}), None).unwrap();

        assert!(registry.get(&match_id).is_some());
    }

    #[test]
    fn test_join_leave() {
        let handlers = Arc::new(MatchHandlerRegistry::new());
        handlers.register("test", TestHandler);

        let registry = MatchRegistry::new(handlers);
        let match_id = registry.create("test", serde_json::json!({}), None).unwrap();

        let presence = MatchPresence {
            user_id: "user-1".to_string(),
            session_id: 1,
            username: "Player1".to_string(),
            node: None,
        };

        registry.join(&match_id, presence).unwrap();

        {
            let m = registry.get(&match_id).unwrap();
            assert_eq!(m.player_count(), 1);
        }

        registry.leave(&match_id, 1).unwrap();

        {
            let m = registry.get(&match_id).unwrap();
            assert_eq!(m.player_count(), 0);
        }
    }

    #[test]
    fn test_tick_with_messages() {
        let handlers = Arc::new(MatchHandlerRegistry::new());
        handlers.register("test", TestHandler);

        let registry = MatchRegistry::new(handlers);
        let match_id = registry.create("test", serde_json::json!({}), None).unwrap();

        let presence = MatchPresence {
            user_id: "user-1".to_string(),
            session_id: 1,
            username: "Player1".to_string(),
            node: None,
        };

        registry.join(&match_id, presence.clone()).unwrap();

        // Send messages
        for _ in 0..3 {
            registry.send_message(&match_id, MatchMessage {
                sender: presence.clone(),
                op_code: 1,
                data: vec![1, 2, 3],
                reliable: true,
                received_at: 0,
            }).unwrap();
        }

        // Tick
        registry.tick(&match_id).unwrap();

        {
            let m = registry.get(&match_id).unwrap();
            assert_eq!(m.state.tick, 1);
            assert_eq!(m.state.state["score"], 3);
        }
    }

    #[test]
    fn test_list_by_label() {
        let handlers = Arc::new(MatchHandlerRegistry::new());
        handlers.register("test", TestHandler);

        let registry = MatchRegistry::new(handlers);

        registry.create("test", serde_json::json!({}), Some("game-a".to_string())).unwrap();
        registry.create("test", serde_json::json!({}), Some("game-a".to_string())).unwrap();
        registry.create("test", serde_json::json!({}), Some("game-b".to_string())).unwrap();

        let game_a = registry.list_by_label("game-a");
        assert_eq!(game_a.len(), 2);

        let game_b = registry.list_by_label("game-b");
        assert_eq!(game_b.len(), 1);
    }
}
