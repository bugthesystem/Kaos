# KaosNet vs Nakama - Deep Analysis & Plan

## Feature Comparison Matrix

| Feature | Nakama | KaosNet | Status | Priority |
|---------|--------|---------|--------|----------|
| **Authentication** |
| Device Auth | Yes | Yes | DONE | - |
| Email/Password | Yes | Yes | DONE | - |
| Social Providers (Google, FB, Apple) | Yes | No | MISSING | MEDIUM |
| Steam/Console Auth | Yes | No | MISSING | LOW |
| Custom Auth | Yes | Yes | DONE | - |
| Account Linking | Yes | Yes | DONE | - |
| JWT Sessions | Yes | Yes | DONE | - |
| Session Variables | Yes | No | MISSING | MEDIUM |
| **User Accounts** |
| User Profiles | Yes | Yes | DONE | - |
| Username/Display Names | Yes | Yes | DONE | - |
| Avatar URLs | Yes | Yes | DONE | - |
| Metadata | Yes | Yes | DONE | - |
| **Social** |
| Friends System | Yes | Yes | DONE | - |
| Friend States (request/mutual/blocked) | Yes | Yes | DONE | - |
| Groups/Clans | Yes | Yes | DONE | - |
| Group Roles (admin/member) | Yes | Yes | DONE | - |
| Presence/Status | Yes | Yes | DONE | - |
| **Real-time Chat** |
| Channel Chat | Yes | Yes | DONE | - |
| Direct Messages | Yes | Yes | DONE | - |
| Group Chat | Yes | Yes | DONE | - |
| Persistent Messages | Yes | Partial | PARTIAL | MEDIUM |
| Chat History | Yes | Partial | PARTIAL | MEDIUM |
| **Notifications** |
| Real-time Notifications | Yes | Yes | DONE | - |
| Persistent Notifications | Yes | Yes | DONE | - |
| Push Notifications (FCM/APNS) | Yes | No | MISSING | MEDIUM |
| **Leaderboards** |
| Create/Delete | Yes | Yes | DONE | - |
| Score Submission | Yes | Yes | DONE | - |
| Sort Orders (asc/desc) | Yes | Yes | DONE | - |
| Operators (set/best/incr/decr) | Yes | Yes | DONE | - |
| Get Top N | Yes | Yes | DONE | - |
| Get Around User | Yes | Yes | DONE | - |
| Reset Schedules | Yes | Partial | PARTIAL | MEDIUM |
| **Tournaments** |
| Create/Schedule | Yes | Yes | DONE | - |
| Join/Leave | Yes | Yes | DONE | - |
| Score Submission | Yes | Yes | DONE | - |
| Brackets | Yes | Yes | DONE | - |
| Leagues (linked tournaments) | Yes | No | MISSING | LOW |
| **Matchmaking** |
| Skill-based Matching | Yes | Yes | DONE | - |
| Query Syntax | Yes | Partial | PARTIAL | MEDIUM |
| Parties | Yes | Yes | DONE | - |
| Custom Properties | Yes | Yes | DONE | - |
| **Multiplayer** |
| Client-Relayed | Yes | No | MISSING | MEDIUM |
| Authoritative | Yes | Partial (rooms) | PARTIAL | HIGH |
| Match Listing | Yes | Partial | PARTIAL | MEDIUM |
| Match Labels | Yes | Partial | PARTIAL | MEDIUM |
| Session-based | Yes | Yes | DONE | - |
| Turn-based | Yes | Partial | PARTIAL | MEDIUM |
| **Storage** |
| Collections | Yes | Yes | DONE | - |
| Key-Value Store | Yes | Yes | DONE | - |
| JSON Documents | Yes | Yes | DONE | - |
| Access Permissions | Yes | Partial | PARTIAL | HIGH |
| Search/Query | Yes | Yes | DONE | - |
| Memory + Postgres | Yes | Yes | DONE | - |
| **Server Framework** |
| TypeScript Runtime | Yes | No | MISSING | LOW |
| Go Runtime | Yes | No (Rust native) | N/A | - |
| Lua Runtime | Yes | Yes | DONE | - |
| Hooks (Before/After) | Yes | Yes | DONE | - |
| RPC Functions | Yes | Yes | DONE | - |
| Match Handler API | Yes | Partial | PARTIAL | HIGH |
| **Console/Admin** |
| Dashboard | Yes | Yes | DONE | - |
| User Management | Yes | Yes | DONE | - |
| Storage Browser | Yes | Yes | DONE | - |
| Leaderboard Admin | Yes | Yes | DONE | - |
| Match Monitor | Yes | Partial | PARTIAL | MEDIUM |
| API Explorer | Yes | No | MISSING | MEDIUM |
| Data Export | Yes | No | MISSING | LOW |
| Role-based Access | Yes | Yes | DONE | - |
| **Transports** |
| WebSocket | Yes | Yes | DONE | - |
| gRPC | Yes | No | MISSING | LOW |
| HTTP/REST | Yes | Yes | DONE | - |
| rUDP | Yes | Yes | DONE | - |
| **Infrastructure** |
| Cluster Mode | Yes | No | MISSING | HIGH |
| Metrics/Prometheus | Yes | No | MISSING | MEDIUM |
| Health Checks | Yes | Partial | PARTIAL | MEDIUM |
| **Monetization** |
| Virtual Wallet | Yes | No | MISSING | LOW |
| IAP Validation (Apple/Google) | Yes | No | MISSING | LOW |

---

## Current Status Summary

### What KaosNet Does Well (Parity or Better)
- **Transports**: WebSocket + RUDP (Nakama-like), plus custom ring-buffer RUDP
- **Core Services**: Sessions, Rooms, Chat, Leaderboards, Matchmaker, Social, Notifications, Tournaments, Storage
- **Lua Scripting**: Storage + Leaderboard APIs exposed
- **Console UI**: All 16 pages, modern React design, RBAC
- **Rate Limiting**: Token bucket + per-operation (Nakama doesn't expose this)
- **Demo Games**: 2 working examples (WebSocket + RUDP)

### Critical Gaps (Must Fix for Nakama Parity)

1. **Client Authentication System** - Nakama's biggest feature
   - Device ID auth (anonymous users)
   - Email/password for players
   - Custom auth hook
   - Account linking

2. **Server Hooks System**
   - Before/After hooks for all APIs
   - Allows intercepting and modifying requests
   - Essential for custom game logic

3. **Authoritative Multiplayer**
   - Match Handler API (init, join, leave, loop, terminate)
   - Server-validated game state
   - Currently rooms are client-relayed only

4. **Storage Permissions**
   - Owner read/write
   - Public read
   - Server-only access

5. **Cluster Mode**
   - Horizontal scaling
   - Node discovery
   - State synchronization

---

## Updated Plan - Priority Order

### Phase 1: Client Authentication (HIGH PRIORITY)
**Goal**: Allow game clients to authenticate without console credentials

```
1. Add client auth endpoints:
   POST /v2/account/authenticate/device
   POST /v2/account/authenticate/email
   POST /v2/account/authenticate/custom

2. User accounts table:
   - user_id (UUID)
   - username
   - display_name
   - avatar_url
   - metadata (JSON)
   - devices[] (linked device IDs)
   - email (optional)
   - password_hash (optional)
   - custom_id (optional)
   - created_at, updated_at

3. Session tokens for clients (separate from console JWT)
   - Short-lived access tokens
   - Refresh token flow
   - Session variables

4. Account linking:
   - Link device to email
   - Link multiple devices
```

### Phase 2: Server Hooks System (HIGH PRIORITY)
**Goal**: Allow Lua/custom code to intercept API calls

```
1. Hook registration in Lua:
   kaos.register_before("authenticate", function(ctx, payload)
     -- Validate, modify, or reject
     return payload
   end)

   kaos.register_after("leaderboard_submit", function(ctx, payload, result)
     -- Post-processing, analytics, etc.
   end)

2. Hook types:
   - Before hooks (can modify/reject)
   - After hooks (read-only, side effects)

3. Hookable APIs:
   - Authentication
   - Leaderboard operations
   - Storage operations
   - Matchmaker operations
   - Social operations
   - RPC calls
```

### Phase 3: Authoritative Multiplayer (HIGH PRIORITY)
**Goal**: Server-validated game state for competitive games

```
1. Match Handler interface:
   trait MatchHandler {
     fn init(ctx, params) -> State
     fn join(ctx, state, presences) -> State
     fn leave(ctx, state, presences) -> State
     fn loop(ctx, state, tick, messages) -> State
     fn terminate(ctx, state)
   }

2. Lua match handlers:
   function match_init(ctx, params)
   function match_join(ctx, state, presences)
   function match_loop(ctx, state, tick, messages)
   -- etc.

3. Match creation:
   - Create from matchmaker result
   - Create from RPC
   - Match labels for filtering
```

### Phase 4: Storage Permissions (MEDIUM PRIORITY)
**Goal**: Proper access control for game data

```
1. Permission levels:
   - NO_READ, NO_WRITE (0, 0) - Server only
   - OWNER_READ, NO_WRITE (1, 0) - Player profile
   - OWNER_READ, OWNER_WRITE (1, 1) - Player inventory
   - PUBLIC_READ, OWNER_WRITE (2, 1) - Public profile
   - PUBLIC_READ, PUBLIC_WRITE (2, 2) - Shared data

2. Enforce on all storage operations
3. Console can override (admin access)
```

### Phase 5: Metrics & Observability (MEDIUM PRIORITY)
**Goal**: Production-ready monitoring

```
1. Prometheus metrics endpoint
   - Request latency histograms
   - Active sessions gauge
   - Match counts
   - Storage operations
   - Rate limit hits

2. Health check endpoint
   - Database connectivity
   - Service status
   - Memory/CPU usage

3. Structured logging (JSON)
```

### Phase 6: Enhanced Console Features (MEDIUM PRIORITY)

```
1. API Explorer - test endpoints interactively
2. Match monitor - real-time match viewer
3. Data export (CSV/JSON)
4. Player data export (GDPR compliance)
5. Bulk operations
```

### Phase 7: Cluster Mode (LOW PRIORITY - Complex)
**Goal**: Horizontal scaling

```
1. Node discovery (etcd/consul or custom)
2. State synchronization
3. Match migration
4. Session affinity
```

### Phase 8: Nice-to-Have Features (LOW PRIORITY)

```
1. Social auth providers (Google, Facebook, Apple)
2. Push notifications (FCM/APNS)
3. Virtual wallet/economy
4. IAP validation
5. gRPC transport
6. TypeScript runtime (alongside Lua)
```

---

## Immediate Next Steps

1. **Design client auth schema** - Database tables, token format
2. **Implement device auth** - Simplest auth method first
3. **Add before/after hooks to Lua** - Framework for extensibility
4. **Create match handler trait** - Foundation for authoritative MP

---

## Architecture Comparison

```
NAKAMA                              KAOSNET
======                              =======
Go + TypeScript + Lua               Rust + Lua
CockroachDB (default)               PostgreSQL + Memory
gRPC + WebSocket + HTTP             WebSocket + RUDP + HTTP
Built-in clustering                 Single-node (planned)
Enterprise: Satori LiveOps          N/A

Pros:                               Pros:
- Mature, battle-tested             - Rust performance
- Large community                   - Custom RUDP transport
- Console/Mobile SDKs               - Ring buffer backbone
- Enterprise support                - Lower resource usage
                                   - Single binary deployment

Cons:                               Cons:
- Resource heavy                    - Missing client auth
- Complex deployment                - No hooks system yet
- Expensive enterprise              - Single-node only
```

---

## Sources

- [Nakama Official Documentation](https://heroiclabs.com/docs/)
- [Nakama GitHub](https://github.com/heroiclabs/nakama)
- [Nakama Authentication](https://heroiclabs.com/docs/nakama/concepts/authentication/)
- [Nakama Storage](https://heroiclabs.com/docs/nakama/concepts/storage/)
- [Nakama Multiplayer](https://heroiclabs.com/docs/nakama/concepts/multiplayer/)
- [Nakama Console](https://heroiclabs.com/docs/nakama/getting-started/console/)
- [Nakama Server Framework](https://heroiclabs.com/docs/nakama/server-framework/)
