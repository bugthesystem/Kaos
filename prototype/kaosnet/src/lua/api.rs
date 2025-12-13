//! Lua API - kaos.* functions exposed to scripts.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use mlua::{Function, Lua, Result as LuaResult, Table, Value};
use uuid::Uuid;

use super::HookRegistry;

/// Lua API provider
pub struct LuaApi;

impl LuaApi {
    pub fn new() -> Self {
        Self
    }

    /// Register all kaos.* functions
    pub fn register(&self, lua: &Lua, _hooks: Arc<HookRegistry>) -> LuaResult<()> {
        let globals = lua.globals();

        // Create kaos table
        let kaos = lua.create_table()?;

        // RPC registry
        let rpc_registry = lua.create_table()?;
        kaos.set("_rpc", rpc_registry)?;

        // Before hooks registry
        let before_hooks = lua.create_table()?;
        kaos.set("_before", before_hooks)?;

        // After hooks registry
        let after_hooks = lua.create_table()?;
        kaos.set("_after", after_hooks)?;

        // Match handlers registry
        let match_handlers = lua.create_table()?;
        globals.set("_match_handlers", match_handlers)?;

        // kaos.register_rpc(name, func)
        let register_rpc = lua.create_function(|lua, (name, func): (String, Function)| {
            let globals = lua.globals();
            let kaos: Table = globals.get("kaos")?;
            let registry: Table = kaos.get("_rpc")?;
            registry.set(name, func)?;
            Ok(())
        })?;
        kaos.set("register_rpc", register_rpc)?;

        // kaos.register_before(event, func)
        let register_before = lua.create_function(|lua, (event, func): (String, Function)| {
            let globals = lua.globals();
            let kaos: Table = globals.get("kaos")?;
            let registry: Table = kaos.get("_before")?;
            registry.set(event, func)?;
            Ok(())
        })?;
        kaos.set("register_before", register_before)?;

        // kaos.register_after(event, func)
        let register_after = lua.create_function(|lua, (event, func): (String, Function)| {
            let globals = lua.globals();
            let kaos: Table = globals.get("kaos")?;
            let registry: Table = kaos.get("_after")?;
            registry.set(event, func)?;
            Ok(())
        })?;
        kaos.set("register_after", register_after)?;

        // kaos.register_match(name, handler_table)
        let register_match = lua.create_function(|lua, (name, handler): (String, Table)| {
            let globals = lua.globals();
            let handlers: Table = globals.get("_match_handlers")?;
            handlers.set(name, handler)?;
            Ok(())
        })?;
        kaos.set("register_match", register_match)?;

        // kaos.time_now() -> milliseconds since epoch
        let time_now = lua.create_function(|_, ()| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            Ok(now)
        })?;
        kaos.set("time_now", time_now)?;

        // kaos.uuid() -> UUID string
        let uuid_fn = lua.create_function(|_, ()| {
            Ok(Uuid::new_v4().to_string())
        })?;
        kaos.set("uuid", uuid_fn)?;

        // kaos.logger_info(msg)
        let logger_info = lua.create_function(|_, msg: String| {
            #[cfg(feature = "tracing")]
            tracing::info!("[lua] {}", msg);
            #[cfg(not(feature = "tracing"))]
            eprintln!("[INFO] [lua] {}", msg);
            Ok(())
        })?;
        kaos.set("logger_info", logger_info)?;

        // kaos.logger_warn(msg)
        let logger_warn = lua.create_function(|_, msg: String| {
            #[cfg(feature = "tracing")]
            tracing::warn!("[lua] {}", msg);
            #[cfg(not(feature = "tracing"))]
            eprintln!("[WARN] [lua] {}", msg);
            Ok(())
        })?;
        kaos.set("logger_warn", logger_warn)?;

        // kaos.logger_error(msg)
        let logger_error = lua.create_function(|_, msg: String| {
            #[cfg(feature = "tracing")]
            tracing::error!("[lua] {}", msg);
            #[cfg(not(feature = "tracing"))]
            eprintln!("[ERROR] [lua] {}", msg);
            Ok(())
        })?;
        kaos.set("logger_error", logger_error)?;

        globals.set("kaos", kaos)?;

        // Create json table for encoding/decoding
        self.register_json(lua)?;

        Ok(())
    }

    fn register_json(&self, lua: &Lua) -> LuaResult<()> {
        let globals = lua.globals();
        let json = lua.create_table()?;

        // json.encode(table) -> string
        let encode = lua.create_function(|_lua, value: Value| {
            let json_str = lua_value_to_json(&value)?;
            Ok(json_str)
        })?;
        json.set("encode", encode)?;

        // json.decode(string) -> table
        let decode = lua.create_function(|lua, s: String| {
            let value: serde_json::Value = serde_json::from_str(&s)
                .map_err(|e| mlua::Error::external(e))?;
            json_to_lua_value(lua, &value)
        })?;
        json.set("decode", decode)?;

        globals.set("json", json)?;
        Ok(())
    }
}

impl Default for LuaApi {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert Lua value to JSON string
fn lua_value_to_json(value: &Value) -> LuaResult<String> {
    let json = lua_to_serde(value)?;
    serde_json::to_string(&json).map_err(|e| mlua::Error::external(e))
}

/// Convert Lua value to serde_json::Value
fn lua_to_serde(value: &Value) -> LuaResult<serde_json::Value> {
    match value {
        Value::Nil => Ok(serde_json::Value::Null),
        Value::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
        Value::Number(n) => {
            serde_json::Number::from_f64(*n)
                .map(serde_json::Value::Number)
                .ok_or_else(|| mlua::Error::external("invalid number"))
        }
        Value::String(s) => Ok(serde_json::Value::String(s.to_str()?.to_string())),
        Value::Table(t) => {
            // Check if array or object
            let mut is_array = true;
            let mut max_index = 0i64;
            for pair in t.clone().pairs::<Value, Value>() {
                let (k, _) = pair?;
                match k {
                    Value::Integer(i) if i > 0 => {
                        max_index = max_index.max(i);
                    }
                    _ => {
                        is_array = false;
                        break;
                    }
                }
            }

            if is_array && max_index > 0 {
                let mut arr = Vec::with_capacity(max_index as usize);
                for i in 1..=max_index {
                    let v: Value = t.get(i)?;
                    arr.push(lua_to_serde(&v)?);
                }
                Ok(serde_json::Value::Array(arr))
            } else {
                let mut map = serde_json::Map::new();
                for pair in t.clone().pairs::<String, Value>() {
                    let (k, v) = pair?;
                    map.insert(k, lua_to_serde(&v)?);
                }
                Ok(serde_json::Value::Object(map))
            }
        }
        _ => Ok(serde_json::Value::Null),
    }
}

/// Convert serde_json::Value to Lua value
fn json_to_lua_value(lua: &Lua, value: &serde_json::Value) -> LuaResult<Value> {
    match value {
        serde_json::Value::Null => Ok(Value::Nil),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Number(f))
            } else {
                Ok(Value::Nil)
            }
        }
        serde_json::Value::String(s) => Ok(Value::String(lua.create_string(s)?)),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua_value(lua, v)?)?;
            }
            Ok(Value::Table(table))
        }
        serde_json::Value::Object(map) => {
            let table = lua.create_table()?;
            for (k, v) in map {
                table.set(k.as_str(), json_to_lua_value(lua, v)?)?;
            }
            Ok(Value::Table(table))
        }
    }
}
