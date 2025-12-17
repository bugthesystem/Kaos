# KaosNet vs Nakama - Status & Remaining Work

**Last Updated**: 2024-12-16
**Test Count**: 85 passing
**Overall Parity**: ~95% features exist, ~85% production-ready

---

## Completed Work (Validated Dec 16, 2024)

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

### Phase 2: Hooked Services ✅ COMPLETE

All hooked service wrappers implemented in `kaosnet/src/services.rs`:

| Service | Before Hooks | After Hooks | Metrics | Notifications |
|---------|--------------|-------------|---------|---------------|
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

### Phase 4: PostgreSQL Backend ✅ COMPLETE

`PostgresBackend` + `PostgresSyncBackend` in `kaosnet/src/storage/postgres.rs`:

| Feature | Status |
|---------|--------|
| CRUD operations | ✅ Done |
| Connection pooling (sqlx) | ✅ Done |
| Schema migrations | ✅ Done |
| `AsyncStorageBackend` trait | ✅ Done |
| **Sync wrapper (`PostgresSyncBackend`)** | ✅ Done |

Usage:
```rust
use kaosnet::{Storage, PostgresSyncBackend};

let backend = PostgresSyncBackend::connect("postgres://localhost/kaosnet")?;
backend.migrate()?;
let storage = Storage::with_backend(Arc::new(backend));
```

---

### Phase 5: Metrics Integration ✅ COMPLETE

Services now record metrics when `metrics` feature is enabled:

| Service | Metrics Recorded |
|---------|------------------|
| `HookedStorage` | `storage_operations_total{op="get/set/delete"}` |
| `HookedLeaderboards` | `leaderboard_submissions_total` |

---

### Phase 6: Console Players API ✅ COMPLETE

Players API queries real storage:
- `GET /api/players` - Lists from `storage_objects` with `collection="players"`
- Seed data now includes player profiles in correct format

---

### Phase 7: Notifications Integration ✅ COMPLETE

`HookedSocial` now sends notifications:

| Event | Notification Type |
|-------|------------------|
| Friend request | `FriendRequest` |

Usage:
```rust
let hooked_social = HookedSocial::new(social, hooks)
    .with_notifications(notifications);
```

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
| Device Auth | Yes | Yes | ✅ Done |
| Email/Password | Yes | Yes | ✅ Done |
| Custom Auth | Yes | Yes | ✅ Done |
| JWT + Refresh | Yes | Yes | ✅ Done |
| **Sessions** |
| Session Management | Yes | Yes | ✅ Done |
| Heartbeat | Yes | Yes | ✅ Done |
| **Rooms/Matches** |
| Room CRUD | Yes | Yes | ✅ Done |
| Match Handler Trait | Yes | Yes | ✅ Done |
| Lua Match Handler | Yes | Yes | ✅ Done |
| **Social** |
| Friends | Yes | Yes | ✅ Done |
| Groups | Yes | Yes | ✅ Done |
| Presence | Yes | Yes | ✅ Done |
| **Lua API** |
| Storage API | Yes | Yes | ✅ Done |
| Leaderboard API | Yes | Yes | ✅ Done |
| Social API | Yes | Yes | ✅ Done |
| **Hooks** |
| Hook Registry | Yes | Yes | ✅ Done |
| Before/After Hooks | Yes | Yes | ✅ Done |
| Hooked Services | Yes | Yes | ✅ Done |
| **Storage** |
| In-Memory | Yes | Yes | ✅ Done |
| PostgreSQL | Yes | Yes | ✅ Done |
| **Notifications** |
| Module | Yes | Yes | ✅ Done |
| Event Integration | Yes | Yes | ✅ Done |
| **Infrastructure** |
| Metrics Module | Yes | Yes | ✅ Done |
| Metrics Integration | Yes | Yes | ✅ Done |
| Clustering | Yes | No | ❌ Not started |

---

## Test Commands

```bash
# Run all tests (85 passing)
cargo test -p kaosnet

# With features
cargo test -p kaosnet --features postgres
cargo test -p kaosnet --features metrics

# Specific modules
cargo test -p kaosnet lua::
cargo test -p kaosnet services::
cargo test -p kaosnet hooks::
cargo test -p kaosnet match_handler::
```

---

## References

- [Nakama Server Framework](https://heroiclabs.com/docs/nakama/server-framework/)
- [Nakama Lua Runtime](https://heroiclabs.com/docs/nakama/server-framework/lua-runtime/)
- [Nakama Match Handler](https://heroiclabs.com/docs/nakama/concepts/multiplayer/authoritative/)
