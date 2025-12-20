# KaosNet vs Nakama: Comprehensive Comparison & Improvement Plan

## Executive Summary

After extensive analysis of Nakama's documentation, API design, console features, and developer experience, this document outlines gaps in KaosNet and provides a concrete improvement plan focused on **developer/player experience** without bloat.

---

## Part 1: Feature Comparison

### Authentication

| Feature | Nakama | KaosNet | Gap |
|---------|--------|---------|-----|
| Device Auth | 1-line: `client.authenticateDevice(deviceId)` | ✅ `client.authenticateDevice(deviceId)` | DONE |
| Email/Password | 1-line: `client.authenticateEmail(email, pass)` | ✅ `client.authenticateEmail(email, pass)` | DONE |
| Social (Google, Apple, Facebook, etc.) | Built-in for 7+ providers | Not implemented | MEDIUM |
| Custom Auth | Hook-based integration | ✅ `client.authenticateCustom(id)` | DONE |
| Account Linking | Link multiple auth methods to one account | Not implemented | HIGH |
| JWT Session | Auto-managed with refresh | ✅ Auto-managed | DONE |

**Nakama Code (3 lines):**
```javascript
const client = new nakamajs.Client("defaultkey", "127.0.0.1", 7350);
const session = await client.authenticateDevice(deviceId, true, "username");
const socket = client.createSocket();
await socket.connect(session);
```

**KaosNet Current (Web Example - 30+ lines of raw WebSocket handling)**

---

### Matchmaking

| Feature | Nakama | KaosNet | Gap |
|---------|--------|---------|-----|
| Query Syntax | Rich: `+properties.region:europe +properties.rank:>=5` | ✅ `POST /api/matchmaker/add` with query | DONE |
| String Properties | `{ region: "europe" }` | ✅ `string_properties` in request | DONE |
| Numeric Properties | `{ rank: 8 }` with range queries | ✅ `numeric_properties` in request | DONE |
| Min/Max Count | Flexible party sizes | ✅ `min_count`, `max_count` in request | DONE |
| Match Found Callback | `socket.onmatchmakermatched` | Manual polling | MEDIUM |
| Server-side Hooks | Before/After hooks for validation | Exists but undocumented | LOW |

**Nakama Matchmaker (Simple):**
```javascript
const ticket = await socket.addMatchmaker(
  "+properties.region:europe +properties.rank:>=5",
  2, 4,  // min, max players
  { region: "europe" },  // string props
  { rank: 8 }  // numeric props
);

socket.onmatchmakermatched = (matched) => {
  console.log("Match found!", matched.match_id);
};
```

---

### Real-time Multiplayer

| Feature | Nakama | KaosNet | Gap |
|---------|--------|---------|-----|
| Relayed Matches | Simple message forwarding | Supported | OK |
| Authoritative Matches | Server validates all state | Lua-based (exists) | OK |
| Match State | Typed state management | Raw bytes | MEDIUM |
| Presence System | Who's online/in-match | Basic | MEDIUM |
| rUDP Support | Yes | Yes | OK |

---

### Console/Dashboard

| Feature | Nakama | KaosNet | Gap |
|---------|--------|---------|-----|
| **Dashboard** | Real-time metrics, latency, bandwidth | Basic stats only | MEDIUM |
| **Players Page** | Full CRUD, export data, ban/unban, wallet, friends | ✅ Real API with pagination | DONE |
| **Storage Explorer** | Browse all, filter, create, CSV/JSON import | Basic (just fixed) | MEDIUM |
| **Leaderboards** | View records, search, manage | Read-only | MEDIUM |
| **Tournaments** | Create, manage, view participants | Stub page | HIGH |
| **Chat Moderation** | View/delete messages by channel | Stub page | MEDIUM |
| **Matches** | Live match viewer, state inspection | Basic rooms view | MEDIUM |
| **API Explorer** | Test any endpoint, tabbed views, session vars | ✅ Implemented | DONE |
| **Role-Based Access** | Admin/Developer/Maintainer/ViewOnly | Admin only | LOW |
| **Data Import/Export** | CSV, JSON upload, full data reset | Not implemented | MEDIUM |
| **Groups/Social** | Manage groups, members, invites | Stub page | MEDIUM |
| **Notifications** | In-app notifications management | Stub page | LOW |

---

### Client SDKs

| Platform | Nakama | KaosNet | Gap |
|----------|--------|---------|-----|
| JavaScript/TypeScript | Official SDK | ✅ `kaosnet-js` | DONE |
| Rust | Community SDK | ✅ `kaosnet-rs` (WS + RUDP) | DONE |
| Unity (C#) | Official SDK | Via Rust FFI | MEDIUM |
| Godot (GDScript) | Official SDK + High-level Multiplayer API | Via Rust FFI | MEDIUM |
| Unreal (C++) | Official SDK | Via Rust FFI | MEDIUM |
| Flutter/Dart | Official SDK | None | LOW |
| Swift/iOS | Official SDK | None | LOW |

---

## Part 2: Root Cause Analysis

### Why Our Example Looks Complicated

1. **No Client SDK** - Developers write raw WebSocket code
2. **No abstraction layer** - Every game reinvents connection, auth, state sync
3. **Protocol is implicit** - JSON messages with undocumented structure
4. **No TypeScript types** - No autocomplete, no safety

**Current kaos.io Web Client Pain Points:**
- 775 lines of JavaScript for a simple game
- Manual JSON parsing/serialization
- Hand-rolled interpolation/prediction (good, but should be SDK-provided)
- No connection state machine
- No reconnection logic beyond basic retry

---

## Part 3: Improvement Plan (Prioritized)

### Phase 1: Client SDK Foundation (CRITICAL)

**Goal:** Make KaosNet as easy to use as Nakama

#### 1.1 Create `kaosnet-js` SDK

```typescript
// Target API (like Nakama)
import { KaosClient } from 'kaosnet-js';

const client = new KaosClient('localhost', 7350);

// Authentication (multiple methods)
const session = await client.authenticateDevice(deviceId);
// OR
const session = await client.authenticateEmail(email, password);

// Real-time connection
const socket = client.createSocket();
await socket.connect(session);

// Matchmaking
const ticket = await socket.addMatchmaker({
  query: '+mode:ranked +region:us',
  minPlayers: 2,
  maxPlayers: 4,
  properties: { mode: 'ranked', region: 'us' },
  numericProperties: { skill: 1500 }
});

socket.onMatchFound = (match) => {
  await socket.joinMatch(match.matchId);
};

// Match state (with built-in interpolation option)
socket.onMatchState = (state) => {
  // Typed state based on your game
};

await socket.sendMatchState(opcode, data);
```

**Deliverables:**
- `kaosnet-js` npm package
- TypeScript types for all APIs
- Built-in reconnection
- Optional client-side prediction helpers
- WebSocket + rUDP (via WebRTC DataChannel) support

#### 1.2 Protocol Documentation

Create clear protocol spec:
```
/docs/PROTOCOL.md
- Authentication flow
- Message formats (JSON schema)
- OpCodes for match state
- Error codes
```

---

### Phase 2: Console Dashboard (HIGH)

**Goal:** Production-ready admin console

#### 2.1 Fix Stub Pages

Current stub pages that need real backend integration:
- `Players.tsx` - Uses sample data, needs real `/api/players` endpoint
- `Chat.tsx` - Stub
- `Social.tsx` - Stub
- `Tournaments.tsx` - Stub
- `Notifications.tsx` - Stub
- `Matchmaker.tsx` - Partially working

#### 2.2 Add API Explorer

Like Nakama's API Explorer:
- List all available endpoints
- Test requests with pre-filled bodies
- View responses
- Support session variables

#### 2.3 Player Management

Full player CRUD:
- Search by ID, username, email
- View/edit player data
- Ban/unban with reason
- View linked devices
- View payment history
- Export player data (GDPR)
- Delete player (with confirmation)

#### 2.4 Match Inspector

Live match viewing:
- See all active matches
- Inspect match state (decoded, not raw bytes)
- View connected players
- Terminate matches

---

### Phase 3: API Simplification (HIGH)

#### 3.1 Expose Matchmaker Properties

Current: Backend-only matchmaking
Target: Client-driven with properties

```rust
// Add to console API
POST /api/matchmaker/add
{
  "query": "+region:us +skill:>=1000",
  "min_count": 2,
  "max_count": 4,
  "string_properties": { "region": "us" },
  "numeric_properties": { "skill": 1200 }
}
```

#### 3.2 Simplify Authentication Endpoints

Current scattered auth:
```
POST /api/auth/login (console only)
WebSocket auth (game only)
```

Target unified auth:
```
POST /api/auth/device    -> { token, session }
POST /api/auth/email     -> { token, session }
POST /api/auth/custom    -> { token, session }
POST /api/auth/refresh   -> { token }
```

#### 3.3 Add Account Linking

```
POST /api/account/link/device
POST /api/account/link/email
POST /api/account/unlink/device
```

---

### Phase 4: Documentation (MEDIUM)

#### 4.1 Quick Start Guide

```markdown
# KaosNet Quick Start

## 1. Install
npm install kaosnet-js

## 2. Connect
const client = new KaosClient('localhost', 7350);
const session = await client.authenticateDevice(getDeviceId());
const socket = client.createSocket();
await socket.connect(session);

## 3. Join Match
const match = await socket.createMatch();
// OR
await socket.joinMatch(matchId);

## 4. Send/Receive State
socket.sendState(1, { x: 100, y: 200 });
socket.onState = (opcode, data, sender) => { ... };
```

#### 4.2 API Reference

Auto-generated from:
- Rust handler code
- TypeScript SDK types

---

### Phase 5: Game Engine SDKs (MEDIUM-LONG TERM)

#### 5.1 Unity SDK (`kaosnet-unity`)
- C# bindings
- Unity-specific transport (could use Unity's WebSocket)
- Example projects

#### 5.2 Godot SDK (`kaosnet-godot`)
- GDScript bindings
- Integration with Godot's High-level Multiplayer API
- Signal-based events (Godot-idiomatic)

---

## Part 4: What NOT to Add (Avoiding Bloat)

| Feature | Nakama Has | We Skip | Reason |
|---------|-----------|---------|--------|
| CockroachDB support | Yes | No | Postgres is enough for most |
| gRPC API | Yes | No | REST + WebSocket sufficient |
| Push Notifications | Yes | No | Use external service (Firebase) |
| Purchase Validation | Yes | No | Use platform SDKs directly |
| Full Social Graph | Yes | Minimal | Friends list is enough |
| Parties | Yes | No | Use matchmaker groups |

---

## Part 5: Success Metrics

After implementing this plan:

1. **Developer Time to Hello World:** < 5 minutes (currently ~30 min)
2. **Lines of Code for Basic Game:** < 50 (currently 200+)
3. **Console Completeness:** 100% functional pages (currently ~50%)
4. **Documentation Coverage:** Full API + Quick Start + Examples

---

## Part 6: Implementation Progress

### Completed ✅

| Phase | Item | Status | Location |
|-------|------|--------|----------|
| 1.1 | kaosnet-js SDK | ✅ Done | `/prototype/kaosnet-js/` |
| 1.1 | kaosnet-rs SDK (Rust) | ✅ Done | `/prototype/kaosnet-rs/` |
| 1.1 | kaosnet-rs reconnection | ✅ Done | Exponential backoff reconnection in `socket.rs` |
| 1.1 | TypeScript types | ✅ Done | `/prototype/kaosnet-js/src/types.ts` |
| 1.1 | Device/Email auth | ✅ Done | `POST /api/auth/device`, `/email`, `/custom`, `/refresh` |
| 1.1 | Browser bundle (IIFE) | ✅ Done | `/prototype/kaosnet-js/dist/index.global.js` |
| 1.1 | SDK example integration | ✅ Done | `/prototype/examples/kaos_io/web/index-sdk.html` |
| 1.2 | Protocol documentation | ✅ Done | `/prototype/docs/PROTOCOL.md` (KaosTalk) |
| 2.1 | Players page (real data) | ✅ Done | Uses `client_auth` accounts |
| 2.1 | Chat page (real data) | ✅ Done | Uses `list_channels_with_counts` + send messages + create channels |
| 2.1 | Social page (real data) | ✅ Done | Groups + Friends listing + delete groups |
| 2.1 | Tournaments page (real data) | ✅ Done | Tournament listing |
| 2.2 | API Explorer | ✅ Done | `/prototype/console-ui/src/pages/ApiExplorer.tsx` |
| 2.3 | Player Management UI | ✅ Done | Server-side pagination, disabled/banned fix |
| 3.1 | Matchmaker properties | ✅ Done | `POST /api/matchmaker/add` with string/numeric props |
| 2.4 | Example game session wiring | ✅ Done | kaos_io + kaos_asteroids register sessions with KaosNet |

### In Progress 🔄

| Phase | Item | Status |
|-------|------|--------|
| 5.1 | Unity SDK (via Rust FFI) | Planned |
| 5.2 | Godot SDK (via Rust FFI) | Planned |

### SDK Usage

#### JavaScript/TypeScript

```typescript
// Node.js / Bundler
import { KaosClient } from 'kaosnet-js';

const client = new KaosClient('localhost', 7350);
const session = await client.authenticateDevice('device-id');
const socket = client.createSocket();
await socket.connect(session);
```

```html
<!-- Browser (IIFE global) -->
<script src="kaosnet-js/dist/index.global.js"></script>
<script>
  const { KaosClient } = KaosNet;
  const client = new KaosClient('localhost', 7350);
  // ... same API
</script>
```

#### Rust

```rust
use kaosnet_rs::{KaosClient, RudpClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // HTTP Client + Auth
    let client = KaosClient::new("localhost", 7350);
    let session = client.authenticate_device("my-device-id").await?;

    // WebSocket for real-time (chat, matchmaker, presence)
    let socket = client.create_socket();
    socket.connect(&session).await?;

    // Matchmaker with properties
    let ticket = client.add_matchmaker(&session, "ranked")
        .string_property("region", "us")
        .numeric_property("skill", 1500.0)
        .min_count(2)
        .max_count(4)
        .send()
        .await?;

    // Handle matchmaker events
    socket.on_matchmaker_matched(|matched| {
        println!("Match found: {}", matched.match_id);
    }).await;

    Ok(())
}

// RUDP for low-latency game data (60 Hz game loop)
fn game_loop() -> std::io::Result<()> {
    let server = "127.0.0.1:7351".parse().unwrap();
    let mut rudp = RudpClient::connect(server)?;

    loop {
        // Send player state
        rudp.send(1, b"x:100,y:200,rot:45")?;

        // Receive other players
        rudp.receive(|op_code, data| {
            // Handle game state
        });

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
```

### Remaining Work

- Quick Start guide + examples
- Unity/Godot SDK via Rust FFI bindings
- Account linking endpoints

### Recently Completed

- **Matchmaker notifications** - Match found notifications sent to all players when matches are created
- **PostgreSQL storage** - Default storage backend for both example games (kaos_io, kaos_asteroids)
- **Metrics integration** - HookedStorage, HookedLeaderboards, HookedSocial all record metrics
- **Rust SDK reconnection** - `ReconnectConfig` with exponential backoff, auto-reconnect
- **Console Chat page** - Send messages, create channels, view messages
- **Console Social page** - Groups CRUD (create, list, delete), Friends listing
- **Example games wired to KaosNet** - kaos_io and kaos_asteroids register sessions with SessionRegistry
- **WsTransport peer_addr** - Server connections now track peer address for session registration
- Console: Tournaments page wired to real backend

---

## Appendix A: Nakama Console Pages Reference

From Nakama's console (for reference when building):

1. **Dashboard** - Server health, metrics graphs, node status
2. **Accounts** - User search, profile view/edit, ban management
3. **Groups** - Group listing, member management
4. **Storage** - Collection browser, object CRUD
5. **Leaderboards** - Leaderboard list, record viewing
6. **Chat** - Channel browser, message moderation
7. **Matches** - Active match list, state inspection
8. **API Explorer** - Endpoint testing tool
9. **Runtime** - Module listing, function status
10. **Settings** - Console user management, data tools

---

## Appendix B: Authentication Flow Comparison

**Nakama (Simple):**
```
Client                    Server
  |                         |
  |-- authenticateDevice -->|
  |<------ session ---------|
  |                         |
  |-- socket.connect ------>|
  |<--- connected ----------|
```

**KaosNet Current (Complex):**
```
Client                    Server
  |                         |
  |-- WebSocket connect --->|
  |<--- raw connection -----|
  |                         |
  |-- JSON: {name, ...} --->|
  |<--- JSON: {players} ----|
  |-- manually track state -|
  |-- manually handle auth -|
```

**KaosNet Target (With SDK):**
```
Client                    Server
  |                         |
  |-- authenticateDevice -->|  (HTTP)
  |<------ session ---------|
  |                         |
  |-- socket.connect ------>|  (WS)
  |<--- authenticated ------|
  |                         |
  |-- joinMatch(id) ------->|
  |<--- match state --------|
```

---

## Sources

- [Nakama GitHub](https://github.com/heroiclabs/nakama)
- [Nakama Documentation](https://heroiclabs.com/docs/)
- [Nakama Console Guide](https://heroiclabs.com/docs/nakama/getting-started/console/)
- [Nakama Client Libraries](https://heroiclabs.com/docs/nakama/client-libraries/)
- [Nakama Matchmaker](https://heroiclabs.com/docs/nakama/concepts/multiplayer/matchmaker/)
- [Nakama Authentication](https://heroiclabs.com/docs/nakama/concepts/authentication/)
