# KaosNet Game Server

A high-performance multiplayer game server framework in Rust.

## Quick Start

```bash
# Run Kaos.io (Agar.io-style, WebSocket)
docker compose up
# Or: cargo run -p kaos-io --bin kaos-io-server
# Then open http://localhost:8080

# Run Asteroids (Terminal multiplayer, RUDP)
cargo run -p kaos-asteroids --bin asteroids-server
# In another terminal:
cargo run -p kaos-asteroids --bin asteroids-client YourName
```

## Project Structure

```
Kaos/
├── kaos-rudp/             # Reliable UDP transport (at root)
│   └── src/
│       ├── lib.rs         # RudpTransport (point-to-point)
│       ├── server.rs      # RudpServer (multi-client with MessageRingBuffer)
│       ├── window.rs      # BitmapWindow (ordered delivery)
│       ├── congestion.rs  # CongestionController (AIMD)
│       ├── multicast.rs   # MulticastTransport
│       └── driver.rs      # DriverTransport (zero-syscall)
└── prototype/             # KaosNet game server prototype
    ├── kaosnet/           # Core game server library
    │   ├── src/
    │   │   ├── lib.rs         # Main exports
    │   │   ├── transport.rs   # Unified transport (WebSocket + RUDP)
    │   │   ├── ratelimit.rs   # Rate limiting middleware
    │   │   ├── session.rs     # Client session management
    │   │   ├── room.rs        # Multiplayer rooms/matches
    │   │   ├── chat.rs        # Real-time messaging
    │   │   ├── leaderboard.rs # Score tracking & rankings
    │   │   ├── matchmaker.rs  # Skill-based matchmaking
    │   │   ├── social.rs      # Friends, groups, presence
    │   │   ├── notifications.rs # Push notifications
    │   │   ├── tournament.rs  # Tournament brackets
    │   │   ├── storage/       # Persistent storage (memory + postgres)
    │   │   ├── lua/           # Lua scripting (storage/leaderboard API)
    │   │   └── console/       # Admin API + auth
    │   ├── scripts/           # Database scripts (init.sql, seed.sql)
    │   ├── docker-compose.yml # Standalone deployment
    │   └── Makefile           # Build/test/docker commands
    ├── kaos-ws/           # WebSocket transport
    ├── kaos-http/         # HTTP/2 server framework
    ├── console-ui/        # React admin dashboard
    │   └── src/pages/     # All 16 admin pages
    └── examples/
        ├── kaos_io/       # Agar.io-style demo game (WebSocket)
        │   ├── src/server.rs
        │   ├── web/index.html
        │   └── Dockerfile
        └── kaos_asteroids/ # Asteroids demo game (RUDP)
            └── src/
                ├── server.rs  # 60Hz RUDP server
                └── client.rs  # Terminal-based client
```

## Architecture

### Core Services (all in kaosnet crate)

| Service | File | Description |
|---------|------|-------------|
| Sessions | `session.rs` | Client connections, heartbeat, state machine |
| Rooms | `room.rs` | Multiplayer coordination, tick-based updates |
| Chat | `chat.rs` | Channels, DMs, real-time messaging |
| Leaderboards | `leaderboard.rs` | Scores, rankings, resets |
| Matchmaker | `matchmaker.rs` | Skill-based matching with parties |
| Social | `social.rs` | Friends, groups, presence |
| Notifications | `notifications.rs` | Real-time + persistent notifications |
| Tournaments | `tournament.rs` | Scheduled competitions |
| Storage | `storage/` | Key-value + document storage |
| Lua | `lua/` | Server-side scripting |
| Console | `console/` | Admin API with JWT auth |

### Console UI Pages (React + TypeScript + Tailwind)

All pages in `console-ui/src/pages/`:
- Dashboard, Sessions, Rooms, Lua (core admin)
- Players, Chat, Leaderboards, Matchmaker
- Notifications, Social, Storage, Tournaments
- Accounts, ApiKeys (admin management)
- Auth, Login

### Transports

- **kaos-ws**: WebSocket (used by kaos_io demo) - supports multiple clients via `WsServer::accept()`
- **kaos-rudp**: Reliable UDP with kaos ring buffer backbone
  - `RudpTransport`: Point-to-point with MessageRingBuffer send window
  - `RudpServer`: Multi-client server with per-client MessageRingBuffer + BitmapWindow
  - `MulticastTransport`: UDP multicast with ring buffer
  - `DriverTransport`: Zero-syscall via kaos-ipc shared memory (requires kaos-driver)

### Networking Best Practices

See **[kaosnet/docs/NETWORKING.md](kaosnet/docs/NETWORKING.md)** for the full guide.

**What KaosNet provides:**
- Transport layer (WebSocket, RUDP)
- Session/room management
- Serialization utilities
- Quantization helpers

**What games implement:**
- Client-side prediction (game-specific physics)
- Entity interpolation (per-entity type)
- Server reconciliation (state-specific)

This separation exists because networking techniques depend heavily on game physics and state shape. KaosNet provides building blocks; games compose them.

| Technique | KaosNet | Game | Why |
|-----------|---------|------|-----|
| Transport | Yes | - | Generic |
| Quantization | Helpers | Implement | Game-specific types |
| Prediction | - | Yes | Needs game physics |
| Interpolation | Buffer utils | Yes | Entity-specific |
| Delta compression | - | Optional | State-specific |

## Demo Game (Kaos.io)

Agar.io-style multiplayer demonstrating:

```rust
// server.rs - Key integrations
let leaderboards = Arc::new(Leaderboards::new());
let storage = Arc::new(Storage::new());

// On player join - load profile from storage
let (color, high_score) = storage.get(&user_id, "profiles", "main");

// On player disconnect - save to leaderboard + storage
leaderboards.submit(LEADERBOARD_ID, &user_id, &name, score, metadata);
storage.set(&user_id, "profiles", "main", json!({...}));

// Broadcast includes all-time leaderboard
let state = GameState {
    players, food,
    leaderboard: leaderboards.get_top(LEADERBOARD_ID, 10),
    ...
};
```

## Demo Game (Asteroids)

Terminal-based multiplayer Asteroids demonstrating kaos-rudp:

```bash
# Run server (RUDP on port 7352)
cargo run -p kaos-asteroids --bin asteroids-server

# Run clients (multiple terminals)
cargo run -p kaos-asteroids --bin asteroids-client Pilot1
cargo run -p kaos-asteroids --bin asteroids-client Pilot2
```

Controls: W/Up=Thrust, A/Left=Rotate Left, D/Right=Rotate Right, Space=Fire, Q=Quit

Features:
- 60Hz tick rate with smooth physics
- Multi-client UDP using kaos_rudp::RudpServer (MessageRingBuffer per client)
- Client uses kaos_rudp::RudpTransport for full reliability
- Ships with thrust/rotation/shooting
- Asteroids that split when destroyed
- Leaderboard integration for high scores
- Terminal UI with crossterm

```rust
// Server: kaos_rudp::RudpServer (MessageRingBuffer per client)
use kaos_rudp::RudpServer;

let mut server = RudpServer::bind("0.0.0.0:7352", 256)?;
server.poll();  // Receive packets, auto-accept new clients
while let Some(addr) = server.accept() {
    println!("New client: {}", addr);
}
server.send(&client_addr, &game_state)?;  // Reliable send
server.receive(&client_addr, |data| { /* handle */ });

// Client: kaos_rudp::RudpTransport (point-to-point)
use kaos_rudp::{RudpTransport, Transport};

let mut transport = RudpTransport::new(bind_addr, server_addr, 256)?;
transport.send(&input_msg)?;
transport.receive(|data| { /* handle game state */ });
transport.process_acks();  // Maintains reliability
```

## Running Tests

```bash
# All kaosnet tests (56 tests)
cargo test -p kaosnet

# kaos-rudp tests (23 tests including RudpServer)
cargo test -p kaos-rudp

# Specific crate
cargo test -p kaos-ws
cargo test -p kaos-http
```

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build demo game only
cargo build -p kaos-io --release
```

## Docker

```bash
# Full stack (game + web + console + postgres)
docker compose up

# Just the game server
docker compose up kaos-io web
```

Ports:
- 7351: WebSocket game server
- 8080: Web client
- 3000: Console UI
- 5432: PostgreSQL

## What's Implemented

### Backend (100%)
- [x] Session management with heartbeat
- [x] Room/match system with tick updates
- [x] Chat with channels and DMs
- [x] Leaderboards with sort orders and operators
- [x] Matchmaker with skill ranges and parties
- [x] Social (friends, groups, presence)
- [x] Notifications (real-time + persistent)
- [x] Tournaments with brackets
- [x] Storage (memory + PostgreSQL backends)
- [x] Storage permissions (NoRead/OwnerOnly/PublicRead/PublicReadWrite)
- [x] Lua scripting runtime with storage/leaderboard API
- [x] Lua match handlers (authoritative multiplayer)
- [x] Console API with JWT + API key auth
- [x] RBAC permissions system
- [x] Unified transport abstraction (WebSocket + RUDP)
- [x] Rate limiting (token bucket + per-operation)
- [x] Client authentication (device, email, custom)
- [x] Server hooks (before/after hooks for all operations)

### Console UI (100%)
- [x] All 16 pages created and routed
- [x] Authentication flow
- [x] API client with generic methods

### Demo Game (100%)
- [x] Working WebSocket server (via kaosnet transport)
- [x] 20Hz game loop
- [x] Leaderboard integration
- [x] Storage integration
- [x] Web client with real-time sync
- [x] Docker setup

## What Could Be Added

- [x] Lua API bindings (storage, leaderboards from Lua)
- [x] WebSocket transport for kaosnet core (unified transport abstraction)
- [ ] Cluster mode / horizontal scaling
- [x] Rate limiting middleware (token bucket + per-operation limits)
- [x] Metrics/observability (Prometheus with /metrics endpoint)
- [x] Distributed tracing (OpenTelemetry with OTLP export to Jaeger/Tempo)
- [x] More example games (kaos_asteroids - RUDP demo)
- [x] Multi-client RUDP server with kaos ring buffers (kaos_rudp::RudpServer)

## Key APIs

### Transport (Unified)
```rust
use kaosnet::{WsServerTransport, TransportServer, ClientTransport};

// Create WebSocket server
let mut server = WsServerTransport::bind("0.0.0.0:7351")?;
server.set_nonblocking(true)?;

// Accept connections
if let Some(mut client) = server.accept()? {
    client.send(b"hello")?;
    client.receive(|data| {
        println!("Received: {:?}", data);
    });
}
```

### Leaderboards
```rust
let lb = Leaderboards::new();
lb.create(LeaderboardConfig { id, name, sort_order, operator, ... });
lb.submit(leaderboard_id, user_id, username, score, metadata);
lb.get_top(leaderboard_id, limit);
lb.get_around(leaderboard_id, user_id, count);
```

### Storage
```rust
let storage = Storage::new();
storage.set(user_id, collection, key, json_value);
storage.get(user_id, collection, key);
storage.list(user_id, collection, limit, cursor);
storage.query(collection, query, limit);
```

### Matchmaker
```rust
let mm = Matchmaker::new(config);
mm.add(ticket); // MatchmakerTicket with user_id, query, properties
let matches = mm.process(); // Returns Vec<MatchmakerMatch>
```

### Rate Limiting
```rust
use kaosnet::{RateLimiter, RateLimitConfig, RateLimitPresets, OperationRateLimiter};

// Basic rate limiter (100 req/sec, burst of 20)
let limiter = RateLimiter::new(RateLimitConfig::new(100).with_burst(20));
if limiter.check("client_123") {
    // Request allowed
}

// Per-operation rate limiting
let mut op_limiter = OperationRateLimiter::new(RateLimitPresets::standard());
op_limiter.add_operation("login", RateLimitPresets::strict()); // 5 req/sec
op_limiter.add_operation("game_update", RateLimitPresets::realtime()); // 60 req/sec

if op_limiter.check("client_123", "login") {
    // Login allowed
}
```

### Authentication
```rust
use kaosnet::{AuthService, AuthConfig, DeviceAuthRequest, EmailAuthRequest};

let auth = AuthService::new(AuthConfig::default());

// Device auth (anonymous)
let response = auth.authenticate_device(DeviceAuthRequest {
    device_id: "unique-device-id".into(),
    create: true,
    username: Some("Player1".into()),
})?;

// Email auth
let response = auth.authenticate_email(EmailAuthRequest {
    email: "player@example.com".into(),
    password: "secure-password".into(),
    create: true,
    username: Some("Player1".into()),
})?;

// Use the tokens
let access_token = response.token;
let refresh_token = response.refresh_token;
```

### Server Hooks
```rust
use kaosnet::{HookRegistry, HookOperation, BeforeHook, AfterHook};

let hooks = HookRegistry::new();

// Before hook - modify or reject requests
hooks.register_before(HookOperation::StorageWrite, Box::new(|ctx, payload| {
    // Validate data, modify payload, or reject
    if payload.get("cheating").is_some() {
        return BeforeHookResult::Reject("Cheating detected".into());
    }
    BeforeHookResult::Continue(payload)
}));

// After hook - react to events
hooks.register_after(HookOperation::SessionConnect, Box::new(|ctx, payload| {
    println!("Player {} connected", ctx.user_id);
}));
```

### Match Handler (Authoritative Multiplayer)
```rust
use kaosnet::{MatchHandler, MatchRegistry, LuaMatchHandler};

// Rust-based handler
struct MyGame;
impl MatchHandler for MyGame {
    fn init(&self, ctx: &MatchContext, params: Value) -> Result<MatchInit> { ... }
    fn join(&self, ctx, dispatcher, state, presences) -> Result<()> { ... }
    fn leave(&self, ctx, dispatcher, state, presences) -> Result<()> { ... }
    fn tick(&self, ctx, dispatcher, state, messages) -> Result<()> { ... }
    fn terminate(&self, ctx, state) -> Result<()> { ... }
}

// Or use Lua-based handler
let handler = LuaMatchHandler::new(runtime, "my_game");
```

### Storage Permissions
```rust
use kaosnet::{Storage, ObjectPermission};

let storage = Storage::new();

// Server-side (bypasses permissions)
storage.set("user1", "profiles", "main", json!({"level": 5}))?;
storage.set_with_permission("user1", "internal", "flags",
    json!({"trust": 100}), ObjectPermission::NoRead)?;

// Client-side (enforces permissions)
storage.read_as("user2", "user1", "profiles", "public")?;  // OK if PublicRead
storage.write_as("user1", "user1", "profiles", "main",     // OK if owner
    json!({"level": 6}), ObjectPermission::OwnerOnly)?;
```

### Lua API (Storage & Leaderboards)
```lua
-- Storage
local profile = kaos.storage_get("user123", "profiles", "main")
kaos.storage_set("user123", "profiles", "main", {level = 5, xp = 1200})
kaos.storage_delete("user123", "profiles", "main")
local items = kaos.storage_list("user123", "inventory", 100)

-- Leaderboards
kaos.leaderboard_submit("weekly", "user123", "PlayerOne", 5000, {kills = 42})
local top10 = kaos.leaderboard_get_top("weekly", 10)
local around = kaos.leaderboard_get_around("weekly", "user123", 5)
local record = kaos.leaderboard_get_record("weekly", "user123")
```

### Lua Match Handler
```lua
kaos.register_match("my_game", {
    match_init = function(ctx, params)
        return { state = '{"score": 0}', tick_rate = 30, label = "game-1" }
    end,
    match_join_attempt = function(ctx, state, presences)
        return state  -- Allow join
    end,
    match_leave = function(ctx, state, presences)
        return state
    end,
    match_loop = function(ctx, state, tick, messages)
        -- Process messages, update state
        return state, {}  -- Return state and broadcasts
    end,
    match_terminate = function(ctx, state)
        -- Cleanup
    end
})
```

### Tracing (OpenTelemetry)
```rust
use kaosnet::telemetry::{init_tracing, TracingConfig};

// Console output (development)
init_tracing(TracingConfig::default());

// OTLP export to Jaeger/Tempo (production)
let _guard = init_tracing(TracingConfig::new("kaosnet")
    .with_otlp("http://localhost:4317")
    .with_level("info,kaosnet=debug")
    .with_json());  // JSON output for log aggregation

// Guard ensures traces are flushed on drop
```

## Console Auth

Default admin: `admin` / `admin` (set via `KAOS_ADMIN_PASSWORD`)

Roles: Admin > Developer > Readonly

Permissions: ManageAccounts, ManageApiKeys, ViewSessions, KickSessions, ViewRooms, TerminateRooms, ViewLua, ExecuteLua, ReloadLua

## Deduplicated Utils

Shared functions in `console/utils.rs`:
- `unix_timestamp()` - Current Unix time
- `instant_to_epoch()` - Convert Instant to epoch
- `check_permission()` - Request permission check

## Recent Changes

1. Fixed kaos-io server to use WebSocket and run real game loop
2. Integrated Leaderboard + Storage services into demo
3. Updated web client to connect to real server
4. Created Docker setup for one-shot deployment
5. Deduplicated helper functions across console modules
6. Removed unused parking_lot dependency from kaos-http/kaos-ws
7. Created unified transport abstraction (ClientTransport, TransportServer traits)
8. Added Lua API bindings for storage and leaderboards
9. Added rate limiting middleware (token bucket + per-operation)
10. Created kaos_asteroids example (60Hz multiplayer Asteroids)
11. Consolidated RudpServer into kaos-rudp with full kaos ring buffer support:
    - Per-client MessageRingBuffer (kaos::disruptor) for send windows
    - Per-client BitmapWindow for receive windows with ordered delivery
    - Per-client CongestionController (AIMD algorithm)
    - kaosnet wrapper provides TransportServer interface
12. Added Players API endpoints (list, get, ban, unban, delete)
13. Added ViewPlayers and ManagePlayers permissions to RBAC
14. Both example games (kaos-io, kaos-asteroids) now use full KaosNet services
15. Added Makefile with build/test/docker/seed commands
16. Added docker-compose.yml for standalone KaosNet deployment
17. Added database scripts (init.sql, seed.sql) with sample data
18. Reorganized prototype: kaos-ws, kaos-http, console-ui, examples moved to prototype/ (kaos-rudp stays at root)
19. Added client authentication system:
    - Device auth (anonymous, creates account on first use)
    - Email auth (with password validation)
    - Custom auth (for third-party providers)
    - JWT tokens with refresh support
20. Added server hooks system:
    - BeforeHook: Modify/reject requests before processing
    - AfterHook: React to events after processing
    - Lua hook registration via kaos.register_hook()
    - Thread-safe concurrent execution
21. Added authoritative multiplayer (Match Handler API):
    - MatchHandler trait with init/join/leave/tick/terminate
    - LuaMatchHandler for Lua-based game logic
    - MatchRegistry for managing concurrent matches
    - Full Nakama-compatible match lifecycle
22. Added storage permission enforcement:
    - NoRead: Server-only data (anti-cheat, internal state)
    - OwnerOnly: Only owner can read/write
    - PublicRead: Anyone can read, owner writes
    - PublicReadWrite: Anyone can read/write
    - Permission-enforcing client APIs (read_as, write_as, delete_as)
23. Console UI improvements:
    - Fixed light/dark theme support across all pages
    - Added logo.svg with glowing cyberpunk design
    - Extended JWT token expiry to 7 days (was 1 hour)
24. Added Prometheus metrics observability:
    - Full metrics module with `metrics` feature flag
    - `/metrics` endpoint on console server (no auth, for Prometheus scraping)
    - Metrics: uptime, sessions, rooms, HTTP requests, response codes
    - Histograms for request duration, room players
    - Counters for chat messages, leaderboards, storage, matchmaker
25. Added distributed tracing with OpenTelemetry:
    - Full telemetry module with `tracing` feature flag
    - OTLP export to Jaeger, Tempo, or any OpenTelemetry collector
    - Console output for development (pretty or JSON)
    - `#[instrument]` on key functions (session, room, storage)
    - TracingConfig builder for easy configuration

## KaosNet Standalone (prototype/kaosnet/)

The kaosnet folder has its own Makefile and docker-compose for standalone use:

```bash
cd prototype/kaosnet

# Build & test
make build           # Build kaosnet library
make test            # Run all tests

# Docker
make docker-up       # Start kaosnet + postgres + console
make docker-down     # Stop services
make docker-reset    # Reset database and reseed

# Database
make seed            # Add sample data
make reset           # Drop all data and reseed (fresh start)

# Console API testing
make console-login   # Get auth token
make console-status  # Check server status
make console-players # List players
```

### Sample Data

After running `make seed` or `make docker-up`, you get:
- 10 sample players (ProGamer42, CasualPlayer, SpeedRunner, etc.)
- 3 groups (Elite Gamers, Casual Crew, Speed Run Masters)
- Leaderboard entries for kaos_io_highscores, weekly_scores, asteroids_highscores
- Sample storage objects, friends, notifications, tournaments
