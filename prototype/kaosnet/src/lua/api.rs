//! Lua API - kaos.* functions exposed to scripts.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use mlua::{Function, Lua, Result as LuaResult, Table, UserData, Value};
use uuid::Uuid;

use super::HookRegistry;
use crate::leaderboard::{LeaderboardConfig, Leaderboards, ResetSchedule, ScoreOperator, SortOrder};
use crate::social::{PresenceStatus, Social};
use crate::storage::Storage;

/// Services available to Lua scripts
#[derive(Clone)]
pub struct LuaServices {
    pub storage: Arc<Storage>,
    pub leaderboards: Arc<Leaderboards>,
    pub social: Arc<Social>,
}

impl LuaServices {
    pub fn new(
        storage: Arc<Storage>,
        leaderboards: Arc<Leaderboards>,
        social: Arc<Social>,
    ) -> Self {
        Self {
            storage,
            leaderboards,
            social,
        }
    }
}

/// Wrapper for services to be stored in Lua app data
struct ServicesHandle(Arc<LuaServices>);

impl UserData for ServicesHandle {}

/// Lua API provider
pub struct LuaApi {
    services: Option<Arc<LuaServices>>,
}

impl LuaApi {
    pub fn new() -> Self {
        Self { services: None }
    }

    /// Create API with services
    pub fn with_services(services: Arc<LuaServices>) -> Self {
        Self {
            services: Some(services),
        }
    }

    /// Register all kaos.* functions
    pub fn register(&self, lua: &Lua, _hooks: Arc<HookRegistry>) -> LuaResult<()> {
        let globals = lua.globals();

        // Create kaos table
        let kaos = lua.create_table()?;

        // Store services handle if available
        if let Some(ref services) = self.services {
            lua.set_app_data(ServicesHandle(services.clone()));
        }

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
        let uuid_fn = lua.create_function(|_, ()| Ok(Uuid::new_v4().to_string()))?;
        kaos.set("uuid", uuid_fn)?;

        // kaos.logger_info(msg)
        let logger_info = lua.create_function(|_, msg: String| {
            #[cfg(feature = "telemetry")]
            tracing::info!("[lua] {}", msg);
            #[cfg(not(feature = "telemetry"))]
            eprintln!("[INFO] [lua] {}", msg);
            Ok(())
        })?;
        kaos.set("logger_info", logger_info)?;

        // kaos.logger_warn(msg)
        let logger_warn = lua.create_function(|_, msg: String| {
            #[cfg(feature = "telemetry")]
            tracing::warn!("[lua] {}", msg);
            #[cfg(not(feature = "telemetry"))]
            eprintln!("[WARN] [lua] {}", msg);
            Ok(())
        })?;
        kaos.set("logger_warn", logger_warn)?;

        // kaos.logger_error(msg)
        let logger_error = lua.create_function(|_, msg: String| {
            #[cfg(feature = "telemetry")]
            tracing::error!("[lua] {}", msg);
            #[cfg(not(feature = "telemetry"))]
            eprintln!("[ERROR] [lua] {}", msg);
            Ok(())
        })?;
        kaos.set("logger_error", logger_error)?;

        // Register Storage API
        self.register_storage_api(lua, &kaos)?;

        // Register Leaderboard API
        self.register_leaderboard_api(lua, &kaos)?;

        // Register Social API
        self.register_social_api(lua, &kaos)?;

        globals.set("kaos", kaos)?;

        // Create json table for encoding/decoding
        self.register_json(lua)?;

        Ok(())
    }

    /// Register storage_* functions
    fn register_storage_api(&self, lua: &Lua, kaos: &Table) -> LuaResult<()> {
        // kaos.storage_read(user_id, collection, key) -> table or nil
        let storage_read =
            lua.create_function(|lua, (user_id, collection, key): (String, String, String)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                match services.0.storage.get(&user_id, &collection, &key) {
                    Ok(Some(obj)) => {
                        let result = lua.create_table()?;
                        result.set("user_id", obj.user_id.as_str())?;
                        result.set("collection", obj.collection.as_str())?;
                        result.set("key", obj.key.as_str())?;
                        result.set("value", json_to_lua_value(lua, &obj.value)?)?;
                        result.set("version", obj.version)?;
                        Ok(Value::Table(result))
                    }
                    Ok(None) => Ok(Value::Nil),
                    Err(e) => Err(mlua::Error::external(e)),
                }
            })?;
        kaos.set("storage_read", storage_read)?;

        // kaos.storage_write(user_id, collection, key, value) -> table
        let storage_write = lua.create_function(
            |lua, (user_id, collection, key, value): (String, String, String, Value)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                let json_value = lua_to_serde(&value)?;

                match services.0.storage.set(&user_id, &collection, &key, json_value) {
                    Ok(obj) => {
                        let result = lua.create_table()?;
                        result.set("user_id", obj.user_id.as_str())?;
                        result.set("collection", obj.collection.as_str())?;
                        result.set("key", obj.key.as_str())?;
                        result.set("version", obj.version)?;
                        Ok(result)
                    }
                    Err(e) => Err(mlua::Error::external(e)),
                }
            },
        )?;
        kaos.set("storage_write", storage_write)?;

        // kaos.storage_delete(user_id, collection, key) -> boolean
        let storage_delete =
            lua.create_function(|lua, (user_id, collection, key): (String, String, String)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                match services.0.storage.delete(&user_id, &collection, &key) {
                    Ok(deleted) => Ok(deleted),
                    Err(e) => Err(mlua::Error::external(e)),
                }
            })?;
        kaos.set("storage_delete", storage_delete)?;

        // kaos.storage_list(user_id, collection, limit) -> array of objects
        let storage_list =
            lua.create_function(|lua, (user_id, collection, limit): (String, String, usize)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                match services.0.storage.list(&user_id, &collection, limit, None) {
                    Ok((objects, _cursor)) => {
                        let result = lua.create_table()?;
                        for (i, obj) in objects.iter().enumerate() {
                            let item = lua.create_table()?;
                            item.set("user_id", obj.user_id.as_str())?;
                            item.set("collection", obj.collection.as_str())?;
                            item.set("key", obj.key.as_str())?;
                            item.set("value", json_to_lua_value(lua, &obj.value)?)?;
                            item.set("version", obj.version)?;
                            result.set(i + 1, item)?;
                        }
                        Ok(result)
                    }
                    Err(e) => Err(mlua::Error::external(e)),
                }
            })?;
        kaos.set("storage_list", storage_list)?;

        Ok(())
    }

    /// Register leaderboard_* functions
    fn register_leaderboard_api(&self, lua: &Lua, kaos: &Table) -> LuaResult<()> {
        // kaos.leaderboard_create(id, name, sort_order, operator) -> boolean
        // sort_order: "descending" or "ascending"
        // operator: "best", "latest", "sum", "increment"
        let leaderboard_create = lua.create_function(
            |lua, (id, name, sort_order, operator): (String, String, String, String)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                let sort = match sort_order.as_str() {
                    "ascending" => SortOrder::Ascending,
                    _ => SortOrder::Descending,
                };

                let op = match operator.as_str() {
                    "latest" => ScoreOperator::Latest,
                    "sum" => ScoreOperator::Sum,
                    "increment" => ScoreOperator::Increment,
                    _ => ScoreOperator::Best,
                };

                let config = LeaderboardConfig {
                    id,
                    name,
                    sort_order: sort,
                    operator: op,
                    reset_schedule: ResetSchedule::Never,
                    max_entries: 10000,
                    metadata_schema: None,
                };

                match services.0.leaderboards.create(config) {
                    Ok(()) => Ok(true),
                    Err(e) => Err(mlua::Error::external(e)),
                }
            },
        )?;
        kaos.set("leaderboard_create", leaderboard_create)?;

        // kaos.leaderboard_submit(id, user_id, username, score, metadata) -> table
        let leaderboard_submit = lua.create_function(
            |lua, (id, user_id, username, score, metadata): (String, String, String, i64, Value)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                let meta = if matches!(metadata, Value::Nil) {
                    None
                } else {
                    Some(lua_to_serde(&metadata)?)
                };

                match services
                    .0
                    .leaderboards
                    .submit(&id, &user_id, &username, score, meta)
                {
                    Ok(record) => {
                        let result = lua.create_table()?;
                        result.set("user_id", record.user_id.as_str())?;
                        result.set("username", record.username.as_str())?;
                        result.set("score", record.score)?;
                        result.set("num_submissions", record.num_submissions)?;
                        Ok(result)
                    }
                    Err(e) => Err(mlua::Error::external(e)),
                }
            },
        )?;
        kaos.set("leaderboard_submit", leaderboard_submit)?;

        // kaos.leaderboard_list(id, limit) -> array of records
        let leaderboard_list = lua.create_function(|lua, (id, limit): (String, usize)| {
            let services = lua
                .app_data_ref::<ServicesHandle>()
                .ok_or_else(|| mlua::Error::external("services not initialized"))?;

            match services.0.leaderboards.get_top(&id, limit) {
                Ok(records) => {
                    let result = lua.create_table()?;
                    for (i, record) in records.iter().enumerate() {
                        let item = lua.create_table()?;
                        item.set("user_id", record.user_id.as_str())?;
                        item.set("username", record.username.as_str())?;
                        item.set("score", record.score)?;
                        item.set("rank", record.rank.unwrap_or(0))?;
                        item.set("num_submissions", record.num_submissions)?;
                        result.set(i + 1, item)?;
                    }
                    Ok(result)
                }
                Err(e) => Err(mlua::Error::external(e)),
            }
        })?;
        kaos.set("leaderboard_list", leaderboard_list)?;

        // kaos.leaderboard_around(id, user_id, count) -> array of records
        let leaderboard_around =
            lua.create_function(|lua, (id, user_id, count): (String, String, usize)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                match services.0.leaderboards.get_around(&id, &user_id, count) {
                    Ok(records) => {
                        let result = lua.create_table()?;
                        for (i, record) in records.iter().enumerate() {
                            let item = lua.create_table()?;
                            item.set("user_id", record.user_id.as_str())?;
                            item.set("username", record.username.as_str())?;
                            item.set("score", record.score)?;
                            item.set("rank", record.rank.unwrap_or(0))?;
                            result.set(i + 1, item)?;
                        }
                        Ok(result)
                    }
                    Err(e) => Err(mlua::Error::external(e)),
                }
            })?;
        kaos.set("leaderboard_around", leaderboard_around)?;

        // kaos.leaderboard_record(id, user_id) -> table or nil
        let leaderboard_record =
            lua.create_function(|lua, (id, user_id): (String, String)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                match services.0.leaderboards.get_record(&id, &user_id) {
                    Ok(Some(record)) => {
                        let result = lua.create_table()?;
                        result.set("user_id", record.user_id.as_str())?;
                        result.set("username", record.username.as_str())?;
                        result.set("score", record.score)?;
                        result.set("rank", record.rank.unwrap_or(0))?;
                        result.set("num_submissions", record.num_submissions)?;
                        Ok(Value::Table(result))
                    }
                    Ok(None) => Ok(Value::Nil),
                    Err(e) => Err(mlua::Error::external(e)),
                }
            })?;
        kaos.set("leaderboard_record", leaderboard_record)?;

        Ok(())
    }

    /// Register social_* functions (friends, groups, presence)
    fn register_social_api(&self, lua: &Lua, kaos: &Table) -> LuaResult<()> {
        // kaos.friends_add(user_id, friend_id, friend_username) -> table
        let friends_add = lua.create_function(
            |lua, (user_id, friend_id, friend_username): (String, String, String)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                match services
                    .0
                    .social
                    .add_friend(&user_id, &friend_id, &friend_username)
                {
                    Ok(friend) => {
                        let result = lua.create_table()?;
                        result.set("user_id", friend.user_id.as_str())?;
                        result.set("username", friend.username.as_str())?;
                        result.set(
                            "state",
                            match friend.state {
                                crate::social::FriendState::Pending => "pending",
                                crate::social::FriendState::Accepted => "accepted",
                                crate::social::FriendState::Blocked => "blocked",
                            },
                        )?;
                        Ok(result)
                    }
                    Err(e) => Err(mlua::Error::external(e)),
                }
            },
        )?;
        kaos.set("friends_add", friends_add)?;

        // kaos.friends_list(user_id) -> array of friends
        let friends_list = lua.create_function(|lua, user_id: String| {
            let services = lua
                .app_data_ref::<ServicesHandle>()
                .ok_or_else(|| mlua::Error::external("services not initialized"))?;

            let friends = services.0.social.get_friends(&user_id);
            let result = lua.create_table()?;
            for (i, friend) in friends.iter().enumerate() {
                let item = lua.create_table()?;
                item.set("user_id", friend.user_id.as_str())?;
                item.set("username", friend.username.as_str())?;
                item.set(
                    "state",
                    match friend.state {
                        crate::social::FriendState::Pending => "pending",
                        crate::social::FriendState::Accepted => "accepted",
                        crate::social::FriendState::Blocked => "blocked",
                    },
                )?;
                result.set(i + 1, item)?;
            }
            Ok(result)
        })?;
        kaos.set("friends_list", friends_list)?;

        // kaos.friends_remove(user_id, friend_id) -> boolean
        let friends_remove = lua.create_function(|lua, (user_id, friend_id): (String, String)| {
            let services = lua
                .app_data_ref::<ServicesHandle>()
                .ok_or_else(|| mlua::Error::external("services not initialized"))?;

            match services.0.social.remove_friend(&user_id, &friend_id) {
                Ok(()) => Ok(true),
                Err(e) => Err(mlua::Error::external(e)),
            }
        })?;
        kaos.set("friends_remove", friends_remove)?;

        // kaos.group_create(creator_id, creator_username, name, description, open) -> table
        let group_create = lua.create_function(
            |lua,
             (creator_id, creator_username, name, description, open): (
                String,
                String,
                String,
                String,
                bool,
            )| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                match services.0.social.create_group(
                    &creator_id,
                    &creator_username,
                    name,
                    description,
                    open,
                ) {
                    Ok(group) => {
                        let result = lua.create_table()?;
                        result.set("id", group.id.as_str())?;
                        result.set("name", group.name.as_str())?;
                        result.set("description", group.description.as_str())?;
                        result.set("open", group.open)?;
                        Ok(result)
                    }
                    Err(e) => Err(mlua::Error::external(e)),
                }
            },
        )?;
        kaos.set("group_create", group_create)?;

        // kaos.group_join(group_id, user_id, username) -> boolean
        let group_join =
            lua.create_function(|lua, (group_id, user_id, username): (String, String, String)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                match services.0.social.join_group(&group_id, &user_id, &username) {
                    Ok(()) => Ok(true),
                    Err(e) => Err(mlua::Error::external(e)),
                }
            })?;
        kaos.set("group_join", group_join)?;

        // kaos.group_leave(group_id, user_id) -> boolean
        let group_leave = lua.create_function(|lua, (group_id, user_id): (String, String)| {
            let services = lua
                .app_data_ref::<ServicesHandle>()
                .ok_or_else(|| mlua::Error::external("services not initialized"))?;

            match services.0.social.leave_group(&group_id, &user_id) {
                Ok(()) => Ok(true),
                Err(e) => Err(mlua::Error::external(e)),
            }
        })?;
        kaos.set("group_leave", group_leave)?;

        // kaos.group_members(group_id) -> array of members
        let group_members = lua.create_function(|lua, group_id: String| {
            let services = lua
                .app_data_ref::<ServicesHandle>()
                .ok_or_else(|| mlua::Error::external("services not initialized"))?;

            match services.0.social.get_group_members(&group_id) {
                Ok(members) => {
                    let result = lua.create_table()?;
                    for (i, member) in members.iter().enumerate() {
                        let item = lua.create_table()?;
                        item.set("user_id", member.user_id.as_str())?;
                        item.set("username", member.username.as_str())?;
                        item.set(
                            "role",
                            match member.role {
                                crate::social::GroupRole::Member => "member",
                                crate::social::GroupRole::Moderator => "moderator",
                                crate::social::GroupRole::Admin => "admin",
                                crate::social::GroupRole::Owner => "owner",
                            },
                        )?;
                        result.set(i + 1, item)?;
                    }
                    Ok(result)
                }
                Err(e) => Err(mlua::Error::external(e)),
            }
        })?;
        kaos.set("group_members", group_members)?;

        // kaos.presence_update(user_id, username, status, message) -> nil
        // status: "online", "away", "busy", "in_game", "offline"
        let presence_update = lua.create_function(
            |lua, (user_id, username, status, message): (String, String, String, Option<String>)| {
                let services = lua
                    .app_data_ref::<ServicesHandle>()
                    .ok_or_else(|| mlua::Error::external("services not initialized"))?;

                let presence_status = match status.as_str() {
                    "online" => PresenceStatus::Online,
                    "away" => PresenceStatus::Away,
                    "busy" => PresenceStatus::Busy,
                    "in_game" => PresenceStatus::InGame,
                    _ => PresenceStatus::Offline,
                };

                services
                    .0
                    .social
                    .update_presence(&user_id, &username, presence_status, message, None);
                Ok(())
            },
        )?;
        kaos.set("presence_update", presence_update)?;

        // kaos.presence_get(user_id) -> table or nil
        let presence_get = lua.create_function(|lua, user_id: String| {
            let services = lua
                .app_data_ref::<ServicesHandle>()
                .ok_or_else(|| mlua::Error::external("services not initialized"))?;

            match services.0.social.get_presence(&user_id) {
                Some(presence) => {
                    let result = lua.create_table()?;
                    result.set("user_id", presence.user_id.as_str())?;
                    result.set("username", presence.username.as_str())?;
                    result.set(
                        "status",
                        match presence.status {
                            PresenceStatus::Online => "online",
                            PresenceStatus::Away => "away",
                            PresenceStatus::Busy => "busy",
                            PresenceStatus::InGame => "in_game",
                            PresenceStatus::Offline => "offline",
                        },
                    )?;
                    if let Some(msg) = &presence.status_message {
                        result.set("status_message", msg.as_str())?;
                    }
                    Ok(Value::Table(result))
                }
                None => Ok(Value::Nil),
            }
        })?;
        kaos.set("presence_get", presence_get)?;

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
            let value: serde_json::Value =
                serde_json::from_str(&s).map_err(|e| mlua::Error::external(e))?;
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
pub fn lua_to_serde(value: &Value) -> LuaResult<serde_json::Value> {
    match value {
        Value::Nil => Ok(serde_json::Value::Null),
        Value::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
        Value::Number(n) => serde_json::Number::from_f64(*n)
            .map(serde_json::Value::Number)
            .ok_or_else(|| mlua::Error::external("invalid number")),
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
pub fn json_to_lua_value(lua: &Lua, value: &serde_json::Value) -> LuaResult<Value> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_services() -> Arc<LuaServices> {
        Arc::new(LuaServices::new(
            Arc::new(Storage::default()),
            Arc::new(Leaderboards::new()),
            Arc::new(Social::new()),
        ))
    }

    #[test]
    fn test_lua_storage_read_write() {
        let services = create_test_services();
        let api = LuaApi::with_services(services);
        let lua = Lua::new();
        let hooks = Arc::new(HookRegistry::new());

        api.register(&lua, hooks).unwrap();

        // Test storage write and read
        let result: bool = lua
            .load(
                r#"
                local obj = kaos.storage_write("user1", "saves", "slot1", {level = 5, gold = 100})
                return obj.version == 1
            "#,
            )
            .eval()
            .unwrap();
        assert!(result);

        // Test storage read
        let result: i64 = lua
            .load(
                r#"
                local obj = kaos.storage_read("user1", "saves", "slot1")
                return obj.value.level
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(result, 5);

        // Test storage read non-existent
        let result: bool = lua
            .load(
                r#"
                local obj = kaos.storage_read("user1", "saves", "nonexistent")
                return obj == nil
            "#,
            )
            .eval()
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_lua_storage_list() {
        let services = create_test_services();
        let api = LuaApi::with_services(services);
        let lua = Lua::new();
        let hooks = Arc::new(HookRegistry::new());

        api.register(&lua, hooks).unwrap();

        let count: i64 = lua
            .load(
                r#"
                kaos.storage_write("user1", "items", "item1", {name = "Sword"})
                kaos.storage_write("user1", "items", "item2", {name = "Shield"})
                kaos.storage_write("user1", "items", "item3", {name = "Potion"})

                local items = kaos.storage_list("user1", "items", 10)
                return #items
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_lua_storage_delete() {
        let services = create_test_services();
        let api = LuaApi::with_services(services);
        let lua = Lua::new();
        let hooks = Arc::new(HookRegistry::new());

        api.register(&lua, hooks).unwrap();

        let result: bool = lua
            .load(
                r#"
                kaos.storage_write("user1", "temp", "key1", {data = "test"})
                local deleted = kaos.storage_delete("user1", "temp", "key1")
                local obj = kaos.storage_read("user1", "temp", "key1")
                return deleted and obj == nil
            "#,
            )
            .eval()
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_lua_leaderboard_submit() {
        let services = create_test_services();
        let api = LuaApi::with_services(services);
        let lua = Lua::new();
        let hooks = Arc::new(HookRegistry::new());

        api.register(&lua, hooks).unwrap();

        let score: i64 = lua
            .load(
                r#"
                kaos.leaderboard_create("highscores", "High Scores", "descending", "best")
                local record = kaos.leaderboard_submit("highscores", "user1", "Alice", 100, nil)
                return record.score
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(score, 100);
    }

    #[test]
    fn test_lua_leaderboard_list() {
        let services = create_test_services();
        let api = LuaApi::with_services(services);
        let lua = Lua::new();
        let hooks = Arc::new(HookRegistry::new());

        api.register(&lua, hooks).unwrap();

        let top_user: String = lua
            .load(
                r#"
                kaos.leaderboard_create("scores", "Scores", "descending", "best")
                kaos.leaderboard_submit("scores", "user1", "Alice", 100, nil)
                kaos.leaderboard_submit("scores", "user2", "Bob", 200, nil)
                kaos.leaderboard_submit("scores", "user3", "Charlie", 150, nil)

                local top = kaos.leaderboard_list("scores", 10)
                return top[1].username
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(top_user, "Bob"); // Highest score
    }

    #[test]
    fn test_lua_friends_api() {
        let services = create_test_services();
        let api = LuaApi::with_services(services);
        let lua = Lua::new();
        let hooks = Arc::new(HookRegistry::new());

        api.register(&lua, hooks).unwrap();

        let state: String = lua
            .load(
                r#"
                local friend = kaos.friends_add("alice", "bob", "Bob")
                return friend.state
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(state, "pending");
    }

    #[test]
    fn test_lua_groups_api() {
        let services = create_test_services();
        let api = LuaApi::with_services(services);
        let lua = Lua::new();
        let hooks = Arc::new(HookRegistry::new());

        api.register(&lua, hooks).unwrap();

        let member_count: i64 = lua
            .load(
                r#"
                local group = kaos.group_create("alice", "Alice", "Cool Group", "A cool group", true)
                kaos.group_join(group.id, "bob", "Bob")
                local members = kaos.group_members(group.id)
                return #members
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(member_count, 2); // Alice (creator) + Bob
    }

    #[test]
    fn test_lua_presence_api() {
        let services = create_test_services();
        let api = LuaApi::with_services(services);
        let lua = Lua::new();
        let hooks = Arc::new(HookRegistry::new());

        api.register(&lua, hooks).unwrap();

        let status: String = lua
            .load(
                r#"
                kaos.presence_update("alice", "Alice", "online", "Playing game")
                local presence = kaos.presence_get("alice")
                return presence.status
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(status, "online");
    }
}
