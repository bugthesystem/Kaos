//! Lua-based match handler implementation.
//!
//! This module provides a `LuaMatchHandler` that wraps a Lua runtime and delegates
//! match lifecycle methods to Lua functions.

use std::sync::Arc;

use crate::match_handler::{
    MatchContext, MatchDispatcher, MatchError, MatchHandler, MatchInit, MatchMessage,
    MatchPresence, MatchState, Result,
};

use super::{LuaRuntime, MatchLoopMessage, MatchPresenceInfo};

/// A match handler that delegates to Lua scripts.
///
/// The Lua module must define a table with the following functions:
/// - `match_init(ctx, params)` -> { state, tick_rate, label }
/// - `match_join_attempt(ctx, state, presences)` -> state (optional)
/// - `match_leave(ctx, state, presences)` -> state (optional)
/// - `match_loop(ctx, state, tick, messages)` -> state, broadcasts
/// - `match_terminate(ctx, state)` (optional)
pub struct LuaMatchHandler {
    runtime: Arc<LuaRuntime>,
    module: String,
}

impl LuaMatchHandler {
    /// Create a new Lua match handler.
    ///
    /// # Arguments
    /// - `runtime`: The Lua runtime to use
    /// - `module`: The name of the Lua module (as registered with `kaos.register_match`)
    pub fn new(runtime: Arc<LuaRuntime>, module: impl Into<String>) -> Self {
        Self {
            runtime,
            module: module.into(),
        }
    }

    /// Get the module name
    pub fn module(&self) -> &str {
        &self.module
    }

    /// Check if this handler's module exists in the runtime
    pub fn exists(&self) -> bool {
        self.runtime.has_match_handler(&self.module)
    }
}

impl MatchHandler for LuaMatchHandler {
    fn init(&self, ctx: &MatchContext, params: serde_json::Value) -> Result<MatchInit> {
        let params_bytes = serde_json::to_vec(&params)
            .map_err(|e| MatchError::Internal(e.to_string()))?;
        let result = self.runtime.call_match_init(&self.module, &params_bytes)
            .map_err(|e| MatchError::Internal(e.to_string()))?;

        // Parse state as JSON
        let state_json: serde_json::Value = if result.state.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_slice(&result.state).unwrap_or(serde_json::json!({}))
        };

        let mut state = MatchState::new(state_json);
        state.tick_rate = result.tick_rate;
        state.label = ctx.label.clone().or(Some(result.label).filter(|s| !s.is_empty()));

        Ok(MatchInit {
            state,
            tick_rate: result.tick_rate,
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
        // Convert state to bytes
        let state_bytes = serde_json::to_vec(&state.state)
            .map_err(|e| MatchError::Internal(e.to_string()))?;

        // Convert presences
        let presence_infos: Vec<MatchPresenceInfo> = presences
            .iter()
            .map(|p| MatchPresenceInfo {
                user_id: p.user_id.clone(),
                session_id: p.session_id,
                username: p.username.clone(),
            })
            .collect();

        // Call Lua
        let new_state = self.runtime.call_match_join(&self.module, &state_bytes, &presence_infos)
            .map_err(|e| MatchError::Internal(e.to_string()))?;

        // Update state if changed
        if !new_state.is_empty() {
            if let Ok(json) = serde_json::from_slice(&new_state) {
                state.state = json;
            }
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
        let state_bytes = serde_json::to_vec(&state.state)
            .map_err(|e| MatchError::Internal(e.to_string()))?;

        let presence_infos: Vec<MatchPresenceInfo> = presences
            .iter()
            .map(|p| MatchPresenceInfo {
                user_id: p.user_id.clone(),
                session_id: p.session_id,
                username: p.username.clone(),
            })
            .collect();

        let new_state = self.runtime.call_match_leave(&self.module, &state_bytes, &presence_infos)
            .map_err(|e| MatchError::Internal(e.to_string()))?;

        if !new_state.is_empty() {
            if let Ok(json) = serde_json::from_slice(&new_state) {
                state.state = json;
            }
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
        let state_bytes = serde_json::to_vec(&state.state)
            .map_err(|e| MatchError::Internal(e.to_string()))?;

        // Convert messages
        let lua_messages: Vec<MatchLoopMessage> = messages
            .iter()
            .map(|m| MatchLoopMessage {
                sender: m.sender.session_id,
                op: m.op_code as u8,
                data: m.data.clone(),
            })
            .collect();

        let result = self.runtime.call_match_loop(
            &self.module,
            &state_bytes,
            state.tick,
            &lua_messages,
        ).map_err(|e| MatchError::Internal(e.to_string()))?;

        // Update state
        if !result.state.is_empty() {
            if let Ok(json) = serde_json::from_slice(&result.state) {
                state.state = json;
            }
        }

        // TODO: Handle broadcasts via dispatcher when we implement network sending

        Ok(())
    }

    fn terminate(&self, _ctx: &MatchContext, state: &MatchState) -> Result<()> {
        let state_bytes = serde_json::to_vec(&state.state)
            .map_err(|e| MatchError::Internal(e.to_string()))?;
        self.runtime.call_match_terminate(&self.module, &state_bytes)
            .map_err(|e| MatchError::Internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua::LuaConfig;

    #[test]
    fn test_lua_match_handler_creation() {
        let runtime = Arc::new(LuaRuntime::new(LuaConfig::default()).unwrap());
        let handler = LuaMatchHandler::new(runtime.clone(), "test_game");

        assert_eq!(handler.module(), "test_game");
        assert!(!handler.exists()); // No script loaded
    }

    #[test]
    fn test_lua_match_handler_with_script() {
        let runtime = Arc::new(LuaRuntime::new(LuaConfig::default()).unwrap());

        // Register a simple match handler via Lua
        runtime.exec(|lua| {
            lua.load(r#"
                kaos.register_match("pong", {
                    match_init = function(ctx, params)
                        return {
                            state = '{"score": 0}',
                            tick_rate = 30,
                            label = "pong-match"
                        }
                    end,
                    match_loop = function(ctx, state, tick, messages)
                        return state, {}
                    end
                })
            "#).exec()?;
            Ok(())
        }).unwrap();

        let handler = LuaMatchHandler::new(runtime.clone(), "pong");
        assert!(handler.exists());
    }
}
