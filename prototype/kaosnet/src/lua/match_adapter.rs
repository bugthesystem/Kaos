//! Lua Match Handler Adapter
//!
//! Bridges the Lua runtime with the match handler system, allowing
//! game logic to be written in Lua scripts.

use std::sync::Arc;

use mlua::{Function, Table, Value};

use crate::match_handler::{
    MatchContext, MatchDispatcher, MatchError, MatchHandler, MatchInit, MatchMessage,
    MatchPresence, MatchState, Result,
};

use super::{LuaConfig, LuaRuntime, LuaServices};

/// Lua-based match handler that delegates to Lua functions.
pub struct LuaMatchHandler {
    runtime: Arc<LuaRuntime>,
    module_name: String,
}

impl LuaMatchHandler {
    /// Create a new Lua match handler.
    ///
    /// # Arguments
    /// * `runtime` - The Lua runtime to execute scripts in
    /// * `module_name` - Name of the registered match handler module in Lua
    pub fn new(runtime: Arc<LuaRuntime>, module_name: impl Into<String>) -> Self {
        Self {
            runtime,
            module_name: module_name.into(),
        }
    }

    /// Create handler without services (for testing).
    pub fn new_simple(module_name: impl Into<String>) -> Result<Self> {
        let config = LuaConfig {
            pool_size: 1,
            script_path: "scripts".to_string(),
        };
        let runtime = LuaRuntime::new(config).map_err(|e| MatchError::Internal(e.to_string()))?;
        Ok(Self {
            runtime: Arc::new(runtime),
            module_name: module_name.into(),
        })
    }

    /// Create handler with services.
    pub fn with_services(
        module_name: impl Into<String>,
        services: Arc<LuaServices>,
    ) -> Result<Self> {
        let config = LuaConfig {
            pool_size: 1,
            script_path: "scripts".to_string(),
        };
        let runtime = LuaRuntime::with_services(config, services)
            .map_err(|e| MatchError::Internal(e.to_string()))?;
        Ok(Self {
            runtime: Arc::new(runtime),
            module_name: module_name.into(),
        })
    }

    /// Get the Lua runtime.
    pub fn runtime(&self) -> &Arc<LuaRuntime> {
        &self.runtime
    }
}

impl MatchHandler for LuaMatchHandler {
    fn init(&self, ctx: &MatchContext, params: serde_json::Value) -> Result<MatchInit> {
        self.runtime
            .exec(|lua| {
                let globals = lua.globals();
                let handlers: Table = globals.get("_match_handlers").map_err(mlua_to_match_err)?;
                let handler: Table = handlers.get(self.module_name.as_str()).map_err(|_| {
                    mlua::Error::external(format!(
                        "Match handler '{}' not registered",
                        self.module_name
                    ))
                })?;

                let init_fn: Function = handler.get("match_init").map_err(|_| {
                    mlua::Error::external("match_init function not found in handler")
                })?;

                // Create context table
                let ctx_table = lua.create_table()?;
                ctx_table.set("match_id", ctx.match_id.as_str())?;
                ctx_table.set("node", ctx.node.as_str())?;
                if let Some(ref label) = ctx.label {
                    ctx_table.set("label", label.as_str())?;
                }

                // Convert params to Lua
                let params_json =
                    serde_json::to_string(&params).unwrap_or_else(|_| "{}".to_string());
                let json_table: Table = globals.get("json")?;
                let decode_fn: Function = json_table.get("decode")?;
                let params_lua: Value = decode_fn.call(params_json)?;

                // Call init
                let result: Table = init_fn.call((ctx_table, params_lua))?;

                // Extract state
                let state_value: Value = result.get("state").unwrap_or(Value::Nil);
                let state_json = lua_value_to_json(lua, &state_value)?;

                let tick_rate: u32 = result.get("tick_rate").unwrap_or(10);
                let label: Option<String> = result.get("label").ok();
                let max_size: usize = result.get("max_size").unwrap_or(16);

                let mut match_state = MatchState::new(state_json);
                match_state.tick_rate = tick_rate;
                match_state.max_size = max_size;
                match_state.label = label.clone();

                Ok(MatchInit {
                    state: match_state,
                    tick_rate,
                    label,
                })
            })
            .map_err(|e| MatchError::HandlerError(e.to_string()))
    }

    fn join(
        &self,
        ctx: &MatchContext,
        _dispatcher: &dyn MatchDispatcher,
        state: &mut MatchState,
        presences: Vec<MatchPresence>,
    ) -> Result<()> {
        self.runtime
            .exec(|lua| {
                let globals = lua.globals();
                let handlers: Table = globals.get("_match_handlers").map_err(mlua_to_match_err)?;
                let handler: Table = handlers.get(self.module_name.as_str()).map_err(|_| {
                    mlua::Error::external(format!(
                        "Match handler '{}' not registered",
                        self.module_name
                    ))
                })?;

                // Check if match_join exists (optional)
                let join_fn: Option<Function> = handler.get("match_join").ok();
                let join_fn = match join_fn {
                    Some(f) => f,
                    None => return Ok(()), // No join handler, that's fine
                };

                // Create context table
                let ctx_table = lua.create_table()?;
                ctx_table.set("match_id", ctx.match_id.as_str())?;

                // Convert state to Lua
                let state_json = serde_json::to_string(&state.state).unwrap_or_else(|_| "{}".to_string());
                let json_table: Table = globals.get("json")?;
                let decode_fn: Function = json_table.get("decode")?;
                let state_lua: Value = decode_fn.call(state_json)?;

                // Convert presences to Lua table
                let presences_table = lua.create_table()?;
                for (i, p) in presences.iter().enumerate() {
                    let pt = lua.create_table()?;
                    pt.set("user_id", p.user_id.as_str())?;
                    pt.set("session_id", p.session_id)?;
                    pt.set("username", p.username.as_str())?;
                    presences_table.set(i + 1, pt)?;
                }

                // Call join
                let result: Value = join_fn.call((ctx_table, state_lua, presences_table))?;

                // Update state if returned
                if let Value::Table(_) = &result {
                    state.state = lua_value_to_json(lua, &result)?;
                }

                Ok(())
            })
            .map_err(|e| MatchError::HandlerError(e.to_string()))?;

        Ok(())
    }

    fn leave(
        &self,
        ctx: &MatchContext,
        _dispatcher: &dyn MatchDispatcher,
        state: &mut MatchState,
        presences: Vec<MatchPresence>,
    ) -> Result<()> {
        self.runtime
            .exec(|lua| {
                let globals = lua.globals();
                let handlers: Table = globals.get("_match_handlers").map_err(mlua_to_match_err)?;
                let handler: Table = handlers.get(self.module_name.as_str()).map_err(|_| {
                    mlua::Error::external(format!(
                        "Match handler '{}' not registered",
                        self.module_name
                    ))
                })?;

                // Check if match_leave exists (optional)
                let leave_fn: Option<Function> = handler.get("match_leave").ok();
                let leave_fn = match leave_fn {
                    Some(f) => f,
                    None => return Ok(()), // No leave handler
                };

                let ctx_table = lua.create_table()?;
                ctx_table.set("match_id", ctx.match_id.as_str())?;

                let state_json = serde_json::to_string(&state.state).unwrap_or_else(|_| "{}".to_string());
                let json_table: Table = globals.get("json")?;
                let decode_fn: Function = json_table.get("decode")?;
                let state_lua: Value = decode_fn.call(state_json)?;

                let presences_table = lua.create_table()?;
                for (i, p) in presences.iter().enumerate() {
                    let pt = lua.create_table()?;
                    pt.set("user_id", p.user_id.as_str())?;
                    pt.set("session_id", p.session_id)?;
                    pt.set("username", p.username.as_str())?;
                    presences_table.set(i + 1, pt)?;
                }

                let result: Value = leave_fn.call((ctx_table, state_lua, presences_table))?;

                if let Value::Table(_) = &result {
                    state.state = lua_value_to_json(lua, &result)?;
                }

                Ok(())
            })
            .map_err(|e| MatchError::HandlerError(e.to_string()))?;

        Ok(())
    }

    fn tick(
        &self,
        ctx: &MatchContext,
        _dispatcher: &dyn MatchDispatcher,
        state: &mut MatchState,
        messages: Vec<MatchMessage>,
    ) -> Result<()> {
        self.runtime
            .exec(|lua| {
                let globals = lua.globals();
                let handlers: Table = globals.get("_match_handlers").map_err(mlua_to_match_err)?;
                let handler: Table = handlers.get(self.module_name.as_str()).map_err(|_| {
                    mlua::Error::external(format!(
                        "Match handler '{}' not registered",
                        self.module_name
                    ))
                })?;

                let tick_fn: Function = handler.get("match_loop").map_err(|_| {
                    mlua::Error::external("match_loop function not found in handler")
                })?;

                let ctx_table = lua.create_table()?;
                ctx_table.set("match_id", ctx.match_id.as_str())?;

                let state_json = serde_json::to_string(&state.state).unwrap_or_else(|_| "{}".to_string());
                let json_table: Table = globals.get("json")?;
                let decode_fn: Function = json_table.get("decode")?;
                let state_lua: Value = decode_fn.call(state_json)?;

                // Convert messages to Lua
                let messages_table = lua.create_table()?;
                for (i, m) in messages.iter().enumerate() {
                    let mt = lua.create_table()?;
                    mt.set("op_code", m.op_code)?;
                    mt.set("data", lua.create_string(&m.data)?)?;

                    let sender = lua.create_table()?;
                    sender.set("user_id", m.sender.user_id.as_str())?;
                    sender.set("session_id", m.sender.session_id)?;
                    sender.set("username", m.sender.username.as_str())?;
                    mt.set("sender", sender)?;

                    messages_table.set(i + 1, mt)?;
                }

                // Call tick: match_loop(ctx, state, tick, messages)
                let result: Value = tick_fn.call((ctx_table, state_lua, state.tick, messages_table))?;

                // Update state if returned
                if let Value::Table(_) = &result {
                    state.state = lua_value_to_json(lua, &result)?;
                }

                Ok(())
            })
            .map_err(|e| MatchError::HandlerError(e.to_string()))?;

        Ok(())
    }

    fn terminate(&self, ctx: &MatchContext, state: &MatchState) -> Result<()> {
        self.runtime
            .exec(|lua| {
                let globals = lua.globals();
                let handlers: Table = globals.get("_match_handlers").map_err(mlua_to_match_err)?;
                let handler: Table = handlers.get(self.module_name.as_str()).map_err(|_| {
                    mlua::Error::external(format!(
                        "Match handler '{}' not registered",
                        self.module_name
                    ))
                })?;

                // Check if match_terminate exists (optional)
                let terminate_fn: Option<Function> = handler.get("match_terminate").ok();
                let terminate_fn = match terminate_fn {
                    Some(f) => f,
                    None => return Ok(()), // No terminate handler
                };

                let ctx_table = lua.create_table()?;
                ctx_table.set("match_id", ctx.match_id.as_str())?;

                let state_json = serde_json::to_string(&state.state).unwrap_or_else(|_| "{}".to_string());
                let json_table: Table = globals.get("json")?;
                let decode_fn: Function = json_table.get("decode")?;
                let state_lua: Value = decode_fn.call(state_json)?;

                let _: Value = terminate_fn.call((ctx_table, state_lua))?;

                Ok(())
            })
            .map_err(|e| MatchError::HandlerError(e.to_string()))?;

        Ok(())
    }
}

/// Convert mlua error to match error.
fn mlua_to_match_err(e: mlua::Error) -> mlua::Error {
    mlua::Error::external(format!("Lua error: {}", e))
}

/// Convert Lua value to JSON.
fn lua_value_to_json(lua: &mlua::Lua, value: &Value) -> mlua::Result<serde_json::Value> {
    let globals = lua.globals();
    let json_table: Table = globals.get("json")?;
    let encode_fn: Function = json_table.get("encode")?;
    let json_str: String = encode_fn.call(value.clone())?;
    serde_json::from_str(&json_str).map_err(|e| mlua::Error::external(e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::match_handler::{MatchHandlerRegistry, MatchRegistry, NullDispatcher};

    const TEST_HANDLER_LUA: &str = r#"
        kaos.register_match("test_game", {
            match_init = function(ctx, params)
                return {
                    state = { score = 0, player_count = 0 },
                    tick_rate = 10,
                    label = "test-match"
                }
            end,

            match_join = function(ctx, state, presences)
                state.player_count = (state.player_count or 0) + #presences
                return state
            end,

            match_leave = function(ctx, state, presences)
                state.player_count = (state.player_count or 0) - #presences
                if state.player_count < 0 then
                    state.player_count = 0
                end
                return state
            end,

            match_loop = function(ctx, state, tick, messages)
                -- Increment score for each message
                state.score = state.score + #messages
                state.tick = tick
                return state
            end,

            match_terminate = function(ctx, state)
                -- Cleanup
            end
        })
    "#;

    fn create_test_runtime() -> Arc<LuaRuntime> {
        let config = LuaConfig {
            pool_size: 1,
            script_path: "nonexistent".to_string(), // Don't load from disk
        };
        let runtime = LuaRuntime::new(config).unwrap();

        // Load test handler
        runtime
            .exec(|lua| {
                lua.load(TEST_HANDLER_LUA).exec()?;
                Ok(())
            })
            .unwrap();

        Arc::new(runtime)
    }

    #[test]
    fn test_lua_match_init() {
        let runtime = create_test_runtime();
        let handler = LuaMatchHandler::new(runtime, "test_game");

        let ctx = MatchContext {
            match_id: "match-1".to_string(),
            node: "local".to_string(),
            created_at: std::time::Instant::now(),
            label: None,
            vars: std::collections::HashMap::new(),
        };

        let init = handler.init(&ctx, serde_json::json!({})).unwrap();

        assert_eq!(init.tick_rate, 10);
        assert_eq!(init.label, Some("test-match".to_string()));
        assert_eq!(init.state.state["score"], 0);
    }

    #[test]
    fn test_lua_match_join_leave() {
        let runtime = create_test_runtime();
        let handler = LuaMatchHandler::new(runtime, "test_game");

        let ctx = MatchContext {
            match_id: "match-1".to_string(),
            node: "local".to_string(),
            created_at: std::time::Instant::now(),
            label: None,
            vars: std::collections::HashMap::new(),
        };

        let init = handler.init(&ctx, serde_json::json!({})).unwrap();
        let mut state = init.state;

        let presence = MatchPresence {
            user_id: "user-1".to_string(),
            session_id: 1,
            username: "Alice".to_string(),
            node: None,
        };

        handler
            .join(&ctx, &NullDispatcher, &mut state, vec![presence.clone()])
            .unwrap();

        // Check player count was incremented
        assert_eq!(state.state["player_count"], 1);

        handler
            .leave(&ctx, &NullDispatcher, &mut state, vec![presence])
            .unwrap();

        // Check player count was decremented
        assert_eq!(state.state["player_count"], 0);
    }

    #[test]
    fn test_lua_match_tick() {
        let runtime = create_test_runtime();
        let handler = LuaMatchHandler::new(runtime, "test_game");

        let ctx = MatchContext {
            match_id: "match-1".to_string(),
            node: "local".to_string(),
            created_at: std::time::Instant::now(),
            label: None,
            vars: std::collections::HashMap::new(),
        };

        let init = handler.init(&ctx, serde_json::json!({})).unwrap();
        let mut state = init.state;
        state.tick = 5;

        let presence = MatchPresence {
            user_id: "user-1".to_string(),
            session_id: 1,
            username: "Alice".to_string(),
            node: None,
        };

        // Send 3 messages
        let messages: Vec<MatchMessage> = (0..3)
            .map(|i| MatchMessage {
                sender: presence.clone(),
                op_code: i,
                data: vec![1, 2, 3],
                reliable: true,
                received_at: 0,
            })
            .collect();

        handler
            .tick(&ctx, &NullDispatcher, &mut state, messages)
            .unwrap();

        // Score should be incremented by 3
        assert_eq!(state.state["score"], 3);
        assert_eq!(state.state["tick"], 5);
    }

    #[test]
    fn test_lua_match_with_registry() {
        let runtime = create_test_runtime();
        let handler = LuaMatchHandler::new(runtime, "test_game");

        let handlers = Arc::new(MatchHandlerRegistry::new());
        handlers.register("lua_test", handler);

        let registry = MatchRegistry::new(handlers);
        let match_id = registry
            .create("lua_test", serde_json::json!({}), None)
            .unwrap();

        let presence = MatchPresence {
            user_id: "user-1".to_string(),
            session_id: 1,
            username: "Alice".to_string(),
            node: None,
        };

        registry.join(&match_id, presence.clone()).unwrap();

        {
            let m = registry.get(&match_id).unwrap();
            assert_eq!(m.player_count(), 1);
            assert_eq!(m.state.label, Some("test-match".to_string()));
        }

        // Send message and tick
        registry
            .send_message(
                &match_id,
                MatchMessage {
                    sender: presence,
                    op_code: 1,
                    data: vec![1],
                    reliable: true,
                    received_at: 0,
                },
            )
            .unwrap();

        registry.tick(&match_id).unwrap();

        {
            let m = registry.get(&match_id).unwrap();
            assert_eq!(m.state.tick, 1);
            assert_eq!(m.state.state["score"], 1);
        }
    }
}
