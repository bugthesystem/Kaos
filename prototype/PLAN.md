# KaosNet vs Nakama - Brutal Honest Assessment & Execution Plan

**Last Updated**: 2024-12-16
**Test Count**: 67 passing
**Overall Parity**: 70% features exist, 45% production-ready

---

## Reality Check: What's TRUE vs What Was Claimed

### VERIFIED WORKING (67 tests prove it)

| Module | Lines | Tests | Verdict |
|--------|-------|-------|---------|
| Auth (device/email/custom) | 704 | 8 | **SOLID** |
| Sessions | 237 | 2 | **SOLID** |
| Rooms | 344 | 3 | **SOLID** |
| Leaderboards | 492 | 4 | **SOLID** |
| Tournaments | 676 | 3 | **SOLID** |
| Chat | 579 | 5 | **SOLID** |
| Social (friends/groups) | 623 | 4 | **SOLID** |
| Storage | 500+ | 2 | **SOLID** |
| Matchmaker | 556 | 4 | **SOLID** |
| Hooks (definitions) | 528 | 8 | **SOLID** |
| Rate Limiting | 200+ | 3 | **SOLID** |

### CRITICAL ISSUES (The Painful Truth)

| Issue | Severity | Description |
|-------|----------|-------------|
| **Lua API is a stub** | P0 | Only ~10 functions. Missing: storage, leaderboard, social APIs |
| **Services are islands** | P0 | Chat, Social, Storage don't talk to each other |
| **Match handlers not wired** | P0 | Trait exists, not connected to Room system |
| **Hooks not invoked** | P0 | 26 operations defined, nothing calls them |
| **In-memory only** | P1 | No PostgreSQL backend despite claims |
| **Notifications dead** | P1 | Module exists, not integrated |
| **Console Players page** | P2 | Uses sample data |

---

## Corrected Feature Matrix

| Feature | Nakama | KaosNet | Real Status |
|---------|--------|---------|-------------|
| **Auth** |
| Device Auth | Yes | Yes | **DONE** |
| Email/Password | Yes | Yes | **DONE** |
| Custom Auth | Yes | Yes | **DONE** |
| Account Linking | Yes | Yes | **DONE** |
| JWT + Refresh | Yes | Yes | **DONE** |
| Social Providers | Yes | No | MISSING |
| **Sessions** |
| Session Management | Yes | Yes | **DONE** |
| Heartbeat | Yes | Yes | **DONE** |
| Session Variables | Yes | Yes | **DONE** (via DashMap) |
| **Rooms/Matches** |
| Room Creation | Yes | Yes | **DONE** |
| Join/Leave | Yes | Yes | **DONE** |
| Player Limits | Yes | Yes | **DONE** |
| Match Handler Trait | Yes | Yes | **DONE** |
| **Match Handler Invocation** | Yes | No | **NOT WIRED** |
| **Lua match_loop** | Yes | Partial | **STUB** |
| **Social** |
| Friends | Yes | Yes | **DONE** |
| Groups | Yes | Yes | **DONE** |
| Presence | Yes | Yes | **DONE** |
| Blocking | Yes | Yes | **DONE** |
| **Chat** |
| Room Chat | Yes | Yes | **DONE** |
| DMs | Yes | Yes | **DONE** |
| History | Yes | Yes | **DONE** |
| **Leaderboards** |
| All features | Yes | Yes | **DONE** |
| **Tournaments** |
| All features | Yes | Yes | **DONE** |
| **Matchmaker** |
| Skill-based | Yes | Yes | **DONE** |
| Parties | Yes | Yes | **DONE** |
| **Storage** |
| Key-Value | Yes | Yes | **DONE** |
| Collections | Yes | Yes | **DONE** |
| Queries | Yes | Yes | **DONE** |
| **PostgreSQL** | Yes | No | **MEMORY ONLY** |
| **Lua Scripting** |
| register_rpc | Yes | Yes | **DONE** |
| register_before/after | Yes | Yes | **DONE** |
| register_match | Yes | Yes | **DONE** |
| **storage_read/write** | Yes | No | **MISSING** |
| **leaderboard_submit** | Yes | No | **MISSING** |
| **social_* APIs** | Yes | No | **MISSING** |
| **Hooks** |
| Hook Definitions | Yes | Yes | **DONE** (26 ops) |
| **Hook Invocation** | Yes | No | **NOT WIRED** |
| **Console** |
| Dashboard | Yes | Yes | **DONE** |
| 15/16 Pages | Yes | Yes | **DONE** |
| Players Page | Yes | No | **SAMPLE DATA** |
| **Infrastructure** |
| Metrics Module | Yes | Yes | **DONE** |
| **Metrics Wired** | Yes | No | **NOT INTEGRATED** |
| Clustering | Yes | No | MISSING |
| **Wallet/Economy** | Yes | No | MISSING |

---

## Execution Plan: Fix It Step by Step

### PHASE 1: Wire the Lua API (P0 - BLOCKING)

**Goal**: Make Lua scripts actually useful

#### Step 1.1: Add Storage API to Lua
```lua
-- Target API:
kaos.storage_read(user_id, collection, key)
kaos.storage_write(user_id, collection, key, value)
kaos.storage_delete(user_id, collection, key)
kaos.storage_list(user_id, collection, limit)
```

**Files to modify**:
- `kaosnet/src/lua/api.rs` - Add storage functions
- `kaosnet/src/lua/mod.rs` - Wire Storage service

**Tests to add**:
- `test_lua_storage_read_write`
- `test_lua_storage_list`

#### Step 1.2: Add Leaderboard API to Lua
```lua
kaos.leaderboard_create(id, name, sort_order, operator)
kaos.leaderboard_submit(id, user_id, username, score, metadata)
kaos.leaderboard_list(id, limit)
kaos.leaderboard_around(id, user_id, count)
```

**Tests to add**:
- `test_lua_leaderboard_submit`
- `test_lua_leaderboard_list`

#### Step 1.3: Add Social API to Lua
```lua
kaos.friends_add(user_id, friend_id, friend_username)
kaos.friends_list(user_id)
kaos.friends_remove(user_id, friend_id)
kaos.group_create(creator_id, name, description, open)
kaos.group_join(group_id, user_id, username)
kaos.presence_update(user_id, status, message)
```

**Tests to add**:
- `test_lua_friends_api`
- `test_lua_groups_api`

---

### PHASE 2: Wire Hooks to Operations (P0 - BLOCKING)

**Goal**: Before/after hooks actually get called

#### Step 2.1: Create HookExecutor integration
```rust
// In each service operation, wrap with hooks:
pub fn storage_write(&self, hooks: &HookExecutor, ...) {
    let ctx = HookContext::new(user_id, ...);

    // Before hook (can reject/modify)
    let payload = hooks.run_before(HookOperation::StorageWrite, ctx, payload)?;

    // Actual operation
    let result = self.backend.set(...)?;

    // After hook (side effects)
    hooks.run_after(HookOperation::StorageWrite, ctx, &result);

    Ok(result)
}
```

**Files to modify**:
- `kaosnet/src/storage/mod.rs` - Add hook calls
- `kaosnet/src/leaderboard.rs` - Add hook calls
- `kaosnet/src/social.rs` - Add hook calls
- `kaosnet/src/matchmaker.rs` - Add hook calls

**Tests to add**:
- `test_storage_before_hook_reject`
- `test_storage_after_hook_called`
- `test_leaderboard_hooks`

---

### PHASE 3: Wire Match Handlers to Rooms (P0 - BLOCKING)

**Goal**: Lua match handlers actually run on room ticks

#### Step 3.1: Connect MatchHandler to Room
```rust
// Room should have optional handler reference
pub struct Room {
    // ... existing fields
    pub handler: Option<Arc<dyn MatchHandler>>,
}

// Room tick should invoke handler
pub fn tick(&mut self, messages: Vec<MatchMessage>) {
    if let Some(handler) = &self.handler {
        let (new_state, broadcasts) = handler.tick(&self.state, self.tick, &messages);
        self.state = new_state;
        // Send broadcasts to players
    }
    self.tick += 1;
}
```

#### Step 3.2: Wire Lua match handlers
```rust
// LuaMatchHandler implements MatchHandler trait
// Calls Lua functions: match_init, match_join, match_leave, match_loop, match_terminate
```

**Tests to add**:
- `test_room_with_lua_handler`
- `test_match_loop_invocation`
- `test_match_broadcast`

---

### PHASE 4: Add PostgreSQL Backend (P1)

**Goal**: Persistent storage

#### Step 4.1: Implement PostgresBackend
```rust
pub struct PostgresBackend {
    pool: PgPool,
}

impl StorageBackend for PostgresBackend {
    // Implement all trait methods with SQL
}
```

**Dependencies to add**:
- `sqlx = { version = "0.7", features = ["postgres", "runtime-tokio"] }`

**Tests to add**:
- `test_postgres_storage_crud` (integration test)

---

### PHASE 5: Wire Notifications (P1)

**Goal**: Events trigger notifications

#### Step 5.1: Add NotificationTrigger
```rust
// When friend request received -> notification
// When tournament starts -> notification
// When match found -> notification
```

---

### PHASE 6: Console Players Page (P2)

**Goal**: Show real session data

#### Step 6.1: Connect to SessionRegistry
- Expose `/v2/console/sessions` endpoint
- Wire to actual SessionRegistry::list()

---

## Test Execution Order

Run after each step to verify:

```bash
# Quick check
cargo test -p kaosnet

# Full check with features
cargo test -p kaosnet --all-features

# Specific module
cargo test -p kaosnet lua::
cargo test -p kaosnet hooks::
cargo test -p kaosnet storage::
```

---

## Success Criteria

### Phase 1 Complete When:
- [ ] `cargo test -p kaosnet lua::` passes with storage/leaderboard/social tests
- [ ] Example Lua script can read/write storage

### Phase 2 Complete When:
- [ ] Before hooks can reject operations
- [ ] After hooks receive operation results
- [ ] `cargo test -p kaosnet hooks::` passes with integration tests

### Phase 3 Complete When:
- [ ] Room with Lua handler ticks and invokes match_loop
- [ ] kaos_asteroids example uses Lua match handler
- [ ] `cargo test -p kaosnet match_handler::` passes

### Phase 4 Complete When:
- [ ] `docker-compose up` starts with PostgreSQL
- [ ] Storage operations persist across restarts
- [ ] Integration test passes

---

## Architecture After Fixes

```
                    ┌─────────────────┐
                    │   Lua Runtime   │
                    │  (full API now) │
                    └────────┬────────┘
                             │ calls
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐  ┌─────────────────┐  ┌─────────────────┐
│    Storage    │  │   Leaderboard   │  │     Social      │
│ + hooks wired │  │  + hooks wired  │  │  + hooks wired  │
└───────────────┘  └─────────────────┘  └─────────────────┘
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │
                    ┌────────┴────────┐
                    │  HookExecutor   │
                    │  (invokes Lua)  │
                    └─────────────────┘
                             │
                    ┌────────┴────────┐
                    │      Room       │
                    │ + MatchHandler  │
                    │   (Lua-backed)  │
                    └─────────────────┘
```

---

## References

- [Nakama Server Framework](https://heroiclabs.com/docs/nakama/server-framework/)
- [Nakama Lua Runtime](https://heroiclabs.com/docs/nakama/server-framework/lua-runtime/)
- [Nakama Match Handler](https://heroiclabs.com/docs/nakama/concepts/multiplayer/authoritative/)
