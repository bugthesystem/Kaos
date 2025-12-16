# KaosNet

A high-performance, Nakama-like game server framework built in Rust.

## Features

- **Real-time Sessions**: Client connection management with heartbeat tracking
- **Rooms/Matches**: Tick-based multiplayer coordination
- **Lua Scripting**: Sandboxed server-side logic
- **Storage**: Key-value and document storage with queries
- **Leaderboards**: Score tracking with rankings
- **Matchmaking**: Skill-based matching with parties
- **Social**: Friends, groups, and presence
- **Notifications**: Real-time and persistent push notifications
- **Console UI**: React-based admin dashboard

## Protocol: KaosTalk

KaosNet uses the KaosTalk protocol for real-time communication over UDP.

```
┌─────────────────────────────────────────────────┐
│ KaosTalk Message (variable length)              │
├────────────┬────────────┬───────────────────────┤
│ op: u8     │ len: u16   │ payload: [u8; len]    │
└────────────┴────────────┴───────────────────────┘
```

## Quick Start

```rust
use kaosnet::{Server, ServerBuilder, RoomConfig};

let server = ServerBuilder::new()
    .bind("0.0.0.0:7350")
    .tick_rate(60)
    .lua_scripts("scripts")
    .build()?;

server.run()?;
```

## Services

### Storage

Persistent key-value and document storage with MongoDB-style queries.

```rust
use kaosnet::{Storage, Query, QueryOp};

let storage = Storage::new();

// Store an object
storage.set(
    "user123",
    "inventory",
    "sword",
    serde_json::json!({"damage": 50, "level": 10}),
    None,
)?;

// Query objects
let query = Query::new().filter("level", QueryOp::Gte(serde_json::json!(5)));
let items = storage.query("user123", "inventory", &query)?;
```

**Query Operators:**
- `Eq`, `Ne`: Equality/inequality
- `Gt`, `Gte`, `Lt`, `Lte`: Comparisons
- `In`: Value in list
- `Contains`: String contains
- `Exists`: Field exists
- `And`, `Or`, `Not`: Logical operators

### Leaderboards

Score tracking with multiple sort orders and operators.

```rust
use kaosnet::{Leaderboards, LeaderboardConfig, SortOrder, ScoreOperator};

let lb = Leaderboards::new();

// Create a leaderboard
lb.create(LeaderboardConfig {
    id: "highscores".into(),
    name: "High Scores".into(),
    sort_order: SortOrder::Descending,
    operator: ScoreOperator::Best,
    ..Default::default()
})?;

// Submit scores
lb.submit("highscores", "user1", "Alice", 1000, None)?;

// Get rankings
let top10 = lb.get_top("highscores", 10)?;
let around_me = lb.get_around("highscores", "user1", 5)?;
```

**Score Operators:**
- `Best`: Keep the best score
- `Latest`: Always use latest score
- `Sum`: Accumulate scores
- `Increment`: Add to existing score

**Sort Orders:**
- `Descending`: Higher is better (default)
- `Ascending`: Lower is better (speedruns, golf)

### Matchmaking

Skill-based matchmaking with party support.

```rust
use kaosnet::{Matchmaker, MatchmakerConfig, MatchmakerTicket};

let mm = Matchmaker::new();

// Create a queue
mm.create_queue(MatchmakerConfig {
    queue: "ranked".into(),
    min_players: 2,
    max_players: 10,
    initial_skill_range: 100.0,
    skill_range_expansion: 10.0,
    max_skill_range: 500.0,
    max_wait_secs: 60,
    ..Default::default()
})?;

// Add player to queue
let ticket = mm.add(MatchmakerTicket {
    queue: "ranked".into(),
    user_ids: vec!["user1".into()],
    skill: 1500.0,
    ..Default::default()
})?;

// Process matches
let matches = mm.process_queue("ranked");
for m in matches {
    println!("Match found! Players: {:?}", m.players);
}
```

**Features:**
- Skill-based matching (MMR/ELO)
- Party/group support
- Custom match properties
- Skill range expansion over time

### Social

Friends, groups, and presence tracking.

```rust
use kaosnet::{Social, PresenceStatus};

let social = Social::new();

// Friend requests
social.add_friend("alice", "bob", "Bob")?;
social.accept_friend("bob", "alice")?;

// Get friends
let friends = social.get_friends("alice");

// Presence
social.update_presence("alice", "Alice", PresenceStatus::InGame, None, Some("match123".into()));
let online = social.get_online_friends("bob");

// Groups
let group = social.create_group("alice", "Alice", "Cool Clan".into(), "A cool clan".into(), true)?;
social.join_group(&group.id, "bob", "Bob")?;
```

**Friend States:**
- `Pending`: Friend request sent
- `Accepted`: Mutual friends
- `Blocked`: User blocked

**Presence Status:**
- `Online`, `Away`, `Busy`, `InGame`, `Offline`

### Notifications

Real-time and persistent push notifications.

```rust
use kaosnet::{Notifications, Notification, NotificationCode};

let notifs = Notifications::new();

// Register real-time callback
notifs.register_callback("user1", Box::new(|n| {
    println!("New notification: {}", n.subject);
}));

// Send notification
let n = Notification::new("user1", NotificationCode::Reward, "Reward!", "You earned 100 coins")
    .with_data(serde_json::json!({"coins": 100}));
notifs.send(n)?;

// Helper methods
notifs.notify_friend_request("bob", "alice", "Alice")?;
notifs.notify_match_found("user1", "match123", "ranked")?;
notifs.notify_reward("user1", "coins", 100, "Daily login bonus")?;

// List notifications
let list = notifs.list("user1", 20, 0, false);
let unread = notifs.unread_count("user1");
```

**Notification Types:**
- Friend requests/accepts
- Match found/starting/ended
- Group invites
- Rewards and achievements
- System announcements
- Custom notifications

## Lua Scripting

Server-side logic via sandboxed Lua runtime.

```lua
-- Register RPC
kaos.register_rpc("purchase_item", function(ctx, payload)
    local user_id = ctx.user_id
    local item_id = payload.item_id

    -- Validate and process
    return { success = true, item = item_id }
end)

-- Match handler
local M = {}

function M.match_init(ctx, params)
    return { state = {}, tick_rate = 10, label = "my_game" }
end

function M.match_loop(ctx, state, tick, messages)
    -- Game logic runs 10 times per second
    return state
end

return M
```

## Console UI

React-based admin dashboard for server management.

```bash
cd console-ui
npm install
npm run dev
```

Features:
- Dashboard with server stats
- Session management (list, kick)
- Room management (list, terminate)
- Lua script viewer
- Account management
- API key management

## Configuration

```rust
let server = ServerBuilder::new()
    .bind("0.0.0.0:7350")
    .console_bind("0.0.0.0:7351")  // Admin console
    .tick_rate(60)                  // Server tick rate
    .lua_scripts("scripts")         // Lua scripts directory
    .max_sessions(10000)            // Max concurrent sessions
    .heartbeat_timeout_secs(30)     // Session timeout
    .build()?;
```

## Performance

Targets:
- 10K+ concurrent sessions per node
- 1000+ active rooms per node
- Sub-millisecond message routing
- 60 tick/sec match loop support

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                      KaosNet Server                       │
├──────────────────────────────────────────────────────────┤
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐    │
│  │ Sessions │ │  Rooms   │ │ Lua Pool │ │ Services │    │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘    │
│       │            │            │            │           │
│  ┌────▼────────────▼────────────▼────────────▼─────┐    │
│  │                Message Router                    │    │
│  └─────────────────────┬───────────────────────────┘    │
│                        │                                 │
│  ┌─────────────────────▼───────────────────────────┐    │
│  │              kaos-rudp Transport                 │    │
│  │   (reliable UDP, congestion control, NAK/ACK)    │    │
│  └──────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

## Comparison with Nakama

| Feature | KaosNet | Nakama |
|---------|---------|--------|
| Transport | UDP (kaos-rudp) | WebSocket/TCP |
| Language | Rust | Go |
| Scripting | Lua | Lua/JS/TS |
| Storage | In-memory/SQLite/PG | CockroachDB |
| Matchmaking | Yes | Yes |
| Leaderboards | Yes | Yes |
| Friends | Yes | Yes |
| Groups | Yes | Yes |
| Notifications | Yes | Yes |
| Console | Yes | Yes |

## Building

```bash
# Build library
cargo build --release

# Build with Lua support
cargo build --release --features lua

# Build with console
cargo build --release --features console

# Run tests
cargo test

# Build console UI
cd console-ui && npm run build
```

## License

See LICENSE file.
