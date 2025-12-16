//! Lua runtime for server-side scripting.

mod api;
mod hooks;

pub use api::LuaApi;
pub use hooks::{HookRegistry, HookType};

use std::path::Path;
use std::sync::Arc;

use mlua::{Function, Lua, MultiValue, Result as LuaResult, Table, Value};
use parking_lot::Mutex;

use crate::error::{KaosNetError, Result};

/// Lua VM pool configuration
#[derive(Debug, Clone)]
pub struct LuaConfig {
    pub pool_size: usize,
    pub script_path: String,
}

impl Default for LuaConfig {
    fn default() -> Self {
        Self {
            pool_size: 4,
            script_path: "scripts".to_string(),
        }
    }
}

/// Lua runtime context passed to scripts
#[derive(Debug, Clone)]
pub struct LuaContext {
    pub session_id: u64,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub room_id: Option<String>,
}

impl LuaContext {
    pub fn new(session_id: u64) -> Self {
        Self {
            session_id,
            user_id: None,
            username: None,
            room_id: None,
        }
    }
}

/// Pooled Lua VM
struct LuaVm {
    lua: Lua,
    hooks: Arc<HookRegistry>,
}

impl LuaVm {
    fn new(hooks: Arc<HookRegistry>) -> Result<Self> {
        let lua = Lua::new();

        // Sandbox: disable dangerous functions
        lua.globals().set("os", Value::Nil)?;
        lua.globals().set("io", Value::Nil)?;
        lua.globals().set("loadfile", Value::Nil)?;
        lua.globals().set("dofile", Value::Nil)?;

        Ok(Self { lua, hooks })
    }

    fn setup_api(&self, api: &LuaApi) -> Result<()> {
        Ok(api.register(&self.lua, self.hooks.clone())?)
    }

    fn load_script(&self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| KaosNetError::Io(e))?;
        self.lua.load(&content).exec()?;
        Ok(())
    }
}

/// Lua runtime pool
pub struct LuaRuntime {
    vms: Vec<Mutex<LuaVm>>,
    hooks: Arc<HookRegistry>,
    /// API reference kept for future hot-reload support
    _api: Arc<LuaApi>,
}

impl LuaRuntime {
    pub fn new(config: LuaConfig) -> Result<Self> {
        let hooks = Arc::new(HookRegistry::new());
        let api = Arc::new(LuaApi::new());

        let mut vms = Vec::with_capacity(config.pool_size);
        for _ in 0..config.pool_size {
            let vm = LuaVm::new(hooks.clone())?;
            vm.setup_api(&api)?;
            vms.push(Mutex::new(vm));
        }

        let runtime = Self {
            vms,
            hooks,
            _api: api,
        };

        // Load scripts from path
        let script_path = Path::new(&config.script_path);
        if script_path.exists() {
            runtime.load_scripts(script_path)?;
        }

        Ok(runtime)
    }

    /// Load all Lua scripts from a directory
    pub fn load_scripts(&self, dir: &Path) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir).map_err(|e| KaosNetError::Io(e))? {
            let entry = entry.map_err(|e| KaosNetError::Io(e))?;
            let path = entry.path();
            if path.extension().map(|e| e == "lua").unwrap_or(false) {
                self.load_script(&path)?;
            }
        }
        Ok(())
    }

    /// Load a single script into all VMs
    pub fn load_script(&self, path: &Path) -> Result<()> {
        for vm in &self.vms {
            vm.lock().load_script(path)?;
        }
        Ok(())
    }

    /// Execute code on first available VM
    pub fn exec<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Lua) -> LuaResult<R>,
    {
        // Simple round-robin (could use better scheduling)
        for vm in &self.vms {
            if let Some(guard) = vm.try_lock() {
                return f(&guard.lua).map_err(|e| e.into());
            }
        }
        // All busy, wait for first
        let guard = self.vms[0].lock();
        f(&guard.lua).map_err(|e| e.into())
    }

    /// Get hooks registry
    pub fn hooks(&self) -> &HookRegistry {
        &self.hooks
    }

    /// Call an RPC function
    pub fn call_rpc(&self, name: &str, ctx: &LuaContext, payload: &[u8]) -> Result<Vec<u8>> {
        self.exec(|lua| {
            let globals = lua.globals();
            let kaos: Table = globals.get("kaos")?;
            let rpc_registry: Table = kaos.get("_rpc")?;

            let func: Function = rpc_registry.get(name)?;
            let ctx_table = lua.create_table()?;
            ctx_table.set("session_id", ctx.session_id)?;
            if let Some(ref uid) = ctx.user_id {
                ctx_table.set("user_id", uid.as_str())?;
            }
            if let Some(ref uname) = ctx.username {
                ctx_table.set("username", uname.as_str())?;
            }
            if let Some(ref rid) = ctx.room_id {
                ctx_table.set("room_id", rid.as_str())?;
            }

            let payload_str = lua.create_string(payload)?;
            let result: Value = func.call((ctx_table, payload_str))?;

            match result {
                Value::String(s) => Ok(s.as_bytes().to_vec()),
                Value::Table(t) => {
                    let json: Table = globals.get("json")?;
                    let encode: Function = json.get("encode")?;
                    let s: mlua::String = encode.call(t)?;
                    Ok(s.as_bytes().to_vec())
                }
                Value::Nil => Ok(Vec::new()),
                _ => Ok(Vec::new()),
            }
        })
    }

    /// Call match handler function
    pub fn call_match_init(&self, module: &str, params: &[u8]) -> Result<MatchInitResult> {
        self.exec(|lua| {
            let globals = lua.globals();
            let handlers: Table = globals.get("_match_handlers")?;
            let handler: Table = handlers.get(module)?;
            let init: Function = handler.get("match_init")?;

            let ctx = lua.create_table()?;
            let params_str = lua.create_string(params)?;

            let result: Table = init.call((ctx, params_str))?;

            Ok(MatchInitResult {
                state: result.get::<Vec<u8>>("state").unwrap_or_default(),
                tick_rate: result.get::<u32>("tick_rate").unwrap_or(10),
                label: result.get::<String>("label").unwrap_or_default(),
            })
        })
    }

    /// Call match loop
    pub fn call_match_loop(
        &self,
        module: &str,
        state: &[u8],
        tick: u64,
        messages: &[MatchLoopMessage],
    ) -> Result<MatchLoopResult> {
        self.exec(|lua| {
            let globals = lua.globals();
            let handlers: Table = globals.get("_match_handlers")?;
            let handler: Table = handlers.get(module)?;
            let loop_fn: Function = handler.get("match_loop")?;

            let ctx = lua.create_table()?;
            let state_str = lua.create_string(state)?;

            let msgs = lua.create_table()?;
            for (i, m) in messages.iter().enumerate() {
                let msg = lua.create_table()?;
                msg.set("sender", m.sender)?;
                msg.set("op", m.op)?;
                msg.set("data", lua.create_string(&m.data)?)?;
                msgs.set(i + 1, msg)?;
            }

            let result: MultiValue = loop_fn.call((ctx, state_str, tick, msgs))?;
            let mut iter = result.into_iter();

            let new_state: Vec<u8> = match iter.next() {
                Some(Value::String(s)) => s.as_bytes().to_vec(),
                _ => state.to_vec(),
            };

            let broadcasts: Vec<MatchBroadcast> = match iter.next() {
                Some(Value::Table(t)) => {
                    let mut bs = Vec::new();
                    for pair in t.pairs::<i64, Table>() {
                        if let Ok((_, b)) = pair {
                            bs.push(MatchBroadcast {
                                data: b.get::<Vec<u8>>("data").unwrap_or_default(),
                            });
                        }
                    }
                    bs
                }
                _ => Vec::new(),
            };

            Ok(MatchLoopResult {
                state: new_state,
                broadcasts,
            })
        })
    }
}

/// Match init result from Lua
#[derive(Debug, Clone)]
pub struct MatchInitResult {
    pub state: Vec<u8>,
    pub tick_rate: u32,
    pub label: String,
}

/// Message for match loop
#[derive(Debug, Clone)]
pub struct MatchLoopMessage {
    pub sender: u64,
    pub op: u8,
    pub data: Vec<u8>,
}

/// Match loop result from Lua
#[derive(Debug, Clone)]
pub struct MatchLoopResult {
    pub state: Vec<u8>,
    pub broadcasts: Vec<MatchBroadcast>,
}

/// Broadcast from match loop
#[derive(Debug, Clone)]
pub struct MatchBroadcast {
    pub data: Vec<u8>,
}
