# KaosNet vs Nakama - Status & Remaining Work

**Last Updated**: 2024-12-17
**Test Count**: 139 passing
**Overall Parity**: ~98% features exist, ~98% actually wired

---

## 🔧 FIXES APPLIED (Dec 17-18, 2024)

| Issue | Fix | Status |
|-------|-----|--------|
| `with_metrics()` never called | Wired in kaos-io server.rs | ✅ Fixed |
| `with_notifications()` never called | Wired in kaos-io server.rs | ✅ Fixed |
| No `/metrics` endpoint in console | Added to console/server.rs | ✅ Fixed |
| ClientTransportType missing impl | Added is_open/receive/send methods | ✅ Fixed |
| kaos-rudp path in Cargo.toml | Fixed relative path | ✅ Fixed |
| PostgresBackend has 0 tests | Added 6 tests in postgres.rs | ✅ Fixed |
| **LuaServices used RAW services** | Now uses `with_hooked_services()` | ✅ Fixed |
| **game.lua leaderboard bypassed hooks** | Lua API now routes through hooked services | ✅ Fixed |
| **kaos-asteroids missing metrics** | Added metrics + hooked services + PostgreSQL support | ✅ Fixed |
| **Examples only in-memory storage** | Both examples now support PostgreSQL via DATABASE_URL | ✅ Fixed |

---

## Completed Work

### Phase 1: Lua API ✅ COMPLETE

All 18 Lua functions implemented and tested in `kaosnet/src/lua/api.rs`:

| Function | Status | Tests |
|----------|--------|-------|
| `storage_read/write/delete/list` | ✅ Done | 3 tests |
| `leaderboard_create/submit/list/around/record` | ✅ Done | 2 tests |
| `friends_add/list/remove` | ✅ Done | 1 test |
| `group_create/join/leave/members` | ✅ Done | 1 test |
| `presence_update/get` | ✅ Done | 1 test |

---

### Phase 2: Hooked Services ✅ COMPLETE (NOW WIRED)

All hooked service wrappers implemented and wired in `kaos-io/src/server.rs`:

| Service | Before Hooks | After Hooks | Metrics Wired | Notifications Wired |
|---------|--------------|-------------|---------------|---------------------|
| `HookedStorage` | ✅ | ✅ | ✅ | - |
| `HookedLeaderboards` | ✅ | ✅ | ✅ | - |
| `HookedSocial` | ✅ | ✅ | ✅ | ✅ |

---

### Phase 3: Lua Match Handler ✅ COMPLETE

`LuaMatchHandler` implemented in `kaosnet/src/lua/match_adapter.rs`:

| Callback | Status | Tests |
|----------|--------|-------|
| `match_init` | ✅ Done | test_lua_match_init |
| `match_join` | ✅ Done | test_lua_match_join_leave |
| `match_leave` | ✅ Done | test_lua_match_join_leave |
| `match_loop` | ✅ Done | test_lua_match_tick |
| `match_terminate` | ✅ Done | - |
| MatchRegistry integration | ✅ Done | test_lua_match_with_registry |

---

### Phase 4: PostgreSQL Backend ✅ COMPLETE (NOW TESTED)

`PostgresBackend` + `PostgresSyncBackend` in `kaosnet/src/storage/postgres.rs`:

| Feature | Status | Tests |
|---------|--------|-------|
| CRUD operations | ✅ Done | test_postgres_crud |
| Version conflicts | ✅ Done | test_postgres_version_conflict |
| List with pagination | ✅ Done | test_postgres_list |
| Query/Count | ✅ Done | test_postgres_query |
| Batch operations | ✅ Done | test_postgres_batch_operations |
| **Sync wrapper** | ✅ Done | test_postgres_sync_wrapper |

Tests skip gracefully if `POSTGRES_TEST_URL` env var is not set.

---

### Phase 5: Metrics Integration ✅ COMPLETE (NOW WIRED)

Services now have metrics wired when `metrics` feature is enabled:

```rust
// kaos-io/src/server.rs - NOW WIRED
let hooked_storage = Arc::new(
    HookedStorage::new(Arc::clone(&storage), Arc::clone(&hooks))
        .with_metrics(Arc::clone(&metrics))
);
let hooked_leaderboards = Arc::new(
    HookedLeaderboards::new(Arc::clone(&leaderboards), Arc::clone(&hooks))
        .with_metrics(Arc::clone(&metrics))
);
```

| Service | Metrics Recorded |
|---------|------------------|
| `HookedStorage` | `storage_operations_total{op="get/set/delete"}` |
| `HookedLeaderboards` | `leaderboard_submissions_total` |

---

### Phase 6: Console Players API ✅ COMPLETE

Players API queries real storage:
- `GET /api/players` - Lists from `storage_objects` with `collection="players"`
- `POST /api/players/:id/ban` - Bans player
- `POST /api/players/:id/unban` - Unbans player
- `DELETE /api/players/:id` - Deletes player

---

### Phase 7: Notifications Integration ✅ COMPLETE (NOW WIRED)

`HookedSocial` now has notifications wired:

```rust
// kaos-io/src/server.rs - NOW WIRED
let hooked_social = Arc::new(
    HookedSocial::new(Arc::clone(&social), Arc::clone(&hooks))
        .with_notifications(Arc::clone(&notifications))
        .with_metrics(Arc::clone(&metrics))
);
```

| Event | Notification Type |
|-------|------------------|
| Friend request | `FriendRequest` |

---

### Phase 8: Console Metrics Endpoint ✅ COMPLETE (ADDED)

Console server now has `/metrics` endpoint for Prometheus scraping:

```rust
// GET /metrics (no auth required)
// Returns Prometheus text format metrics
```

Both console (port 7350) and kaos-io game server (port 9090) expose metrics.

---

## Remaining Work

### P3: Cluster Mode (Major Feature - Not Started)

Research needed:
1. Node discovery and registration
2. Distributed session registry
3. Distributed room registry
4. Cross-node message routing
5. Leader election for global state

---

## Verified Feature Matrix

| Feature | Nakama | KaosNet | Status |
|---------|--------|---------|--------|
| **Auth** |
| Device Auth | Yes | Yes | ✅ Tested |
| Email/Password | Yes | Yes | ✅ Tested |
| Custom Auth | Yes | Yes | ✅ Tested |
| JWT + Refresh | Yes | Yes | ✅ Tested |
| **Sessions** |
| Session Management | Yes | Yes | ✅ Tested |
| Heartbeat | Yes | Yes | ✅ Tested |
| **Rooms/Matches** |
| Room CRUD | Yes | Yes | ✅ Tested |
| Match Handler Trait | Yes | Yes | ✅ Tested |
| Lua Match Handler | Yes | Yes | ✅ Tested |
| **Social** |
| Friends | Yes | Yes | ✅ Tested |
| Groups | Yes | Yes | ✅ Tested |
| Presence | Yes | Yes | ✅ Tested |
| **Lua API** |
| Storage API | Yes | Yes | ✅ Tested |
| Leaderboard API | Yes | Yes | ✅ Tested |
| Social API | Yes | Yes | ✅ Tested |
| **Hooks** |
| Hook Registry | Yes | Yes | ✅ Tested |
| Before/After Hooks | Yes | Yes | ✅ Tested |
| Hooked Services | Yes | Yes | ✅ Wired |
| **Storage** |
| In-Memory | Yes | Yes | ✅ Tested |
| PostgreSQL | Yes | Yes | ✅ 6 tests |
| **Notifications** |
| Module | Yes | Yes | ✅ Tested |
| Event Integration | Yes | Yes | ✅ Wired |
| **Infrastructure** |
| Metrics Module | Yes | Yes | ✅ Tested |
| Metrics Integration | Yes | Yes | ✅ Wired |
| Console /metrics | Yes | Yes | ✅ Added |
| Clustering | Yes | No | ❌ Not started |

---

## Test Commands

```bash
# Run all tests (139 passing)
cargo test -p kaosnet --all-features

# Specific modules
cargo test -p kaosnet lua::
cargo test -p kaosnet services::
cargo test -p kaosnet hooks::
cargo test -p kaosnet match_handler::

# Build kaos-io demo
cargo build -p kaos-io
```

---

## Running Services

| Port | Service |
|------|---------|
| 7351 | WebSocket (game server - web clients) |
| 7354 | RUDP (game server - native clients/bots) |
| 7350 | Console (admin API + /metrics) |
| 9090 | Metrics (game server Prometheus) |
| 8082 | Web client (static files) |

---

## References

- [Nakama Server Framework](https://heroiclabs.com/docs/nakama/server-framework/)
- [Nakama Lua Runtime](https://heroiclabs.com/docs/nakama/server-framework/lua-runtime/)
- [Nakama Match Handler](https://heroiclabs.com/docs/nakama/concepts/multiplayer/authoritative/)
