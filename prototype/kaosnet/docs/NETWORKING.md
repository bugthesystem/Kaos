# KaosNet Networking Guide

This guide covers networking best practices for real-time multiplayer games, based on [Gaffer On Games](https://gafferongames.com/) articles.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           GAME CODE                                      │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────────────┐  │
│  │ Game Logic      │  │ Prediction       │  │ Interpolation          │  │
│  │ (your code)     │  │ (your code)      │  │ (your code)            │  │
│  └─────────────────┘  └──────────────────┘  └────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          KAOSNET LIBRARY                                 │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────────────┐  │
│  │ Transport       │  │ Quantization     │  │ Snapshot Buffer        │  │
│  │ (WebSocket/UDP) │  │ (utilities)      │  │ (utilities)            │  │
│  └─────────────────┘  └──────────────────┘  └────────────────────────┘  │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────────────┐  │
│  │ Sessions        │  │ Rooms/Matches    │  │ Serialization          │  │
│  └─────────────────┘  └──────────────────┘  └────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

## What KaosNet Provides vs What Games Implement

### KaosNet Provides (Library Level)

| Feature | Description | Location |
|---------|-------------|----------|
| **Transport** | WebSocket and RUDP connections | `kaos-ws`, `kaos-rudp` |
| **Sessions** | Player connection management | `kaosnet/session.rs` |
| **Rooms** | Match coordination, tick-based updates | `kaosnet/room.rs` |
| **Serialization** | JSON/binary encoding | serde integration |
| **Quantization Utils** | Helper traits for fixed-point conversion | `kaosnet/netcode/` |
| **Snapshot Buffer** | Ring buffer for state history | `kaosnet/netcode/` |

### Games Implement (Application Level)

| Feature | Description | Why Game-Specific |
|---------|-------------|-------------------|
| **Client-Side Prediction** | Local player moves immediately | Requires game physics knowledge |
| **Server Reconciliation** | Replay inputs after server update | Depends on game state shape |
| **Entity Interpolation** | Smooth movement between updates | Different per entity type |
| **Input Handling** | Keyboard/mouse → actions | Game-specific controls |

## Networking Techniques

### 1. Client-Side Prediction

**What:** Player sees immediate response to input, even before server confirms.

**Why:** Hides network latency (50-200ms) for responsive gameplay.

**Implementation Pattern:**

```javascript
// CLIENT: Apply input locally AND send to server
function onInput(input) {
    // 1. Apply locally for immediate response
    applyInput(localPlayer, input);

    // 2. Store for reconciliation
    pendingInputs.push({ seq: inputSeq++, ...input });

    // 3. Send to server
    socket.send({ seq: inputSeq, ...input });
}

// CLIENT: When server state arrives
function onServerState(state) {
    // 1. Update to server position
    localPlayer = state.players.find(p => p.id === myId);

    // 2. Remove acknowledged inputs
    pendingInputs = pendingInputs.filter(i => i.seq > state.lastInputSeq);

    // 3. Re-apply unacknowledged inputs
    for (const input of pendingInputs) {
        applyInput(localPlayer, input);
    }
}
```

**KaosNet Support:** Games implement this themselves because:
- Physics varies (2D, 3D, cell-based, continuous)
- State shape varies (position only, velocity, acceleration)
- Some games don't need prediction (turn-based, slow-paced)

---

### 2. Entity Interpolation

**What:** Render other players smoothly between server updates.

**Why:** Server sends 20-60 updates/sec, but rendering at 60+ FPS.

**Implementation Pattern:**

```javascript
// CLIENT: Buffer incoming states
const stateBuffer = [];
const INTERP_DELAY = 100; // ms

function onServerState(state) {
    stateBuffer.push({
        timestamp: performance.now(),
        state: state
    });
    // Keep last N snapshots
    while (stateBuffer.length > 20) stateBuffer.shift();
}

// CLIENT: Render with interpolation
function render() {
    const renderTime = performance.now() - INTERP_DELAY;

    // Find surrounding snapshots
    const [older, newer] = findSnapshots(renderTime);
    const t = (renderTime - older.timestamp) / (newer.timestamp - older.timestamp);

    for (const entity of newer.state.entities) {
        const oldEntity = older.state.entities.find(e => e.id === entity.id);
        if (oldEntity) {
            // Lerp position
            const x = oldEntity.x + (entity.x - oldEntity.x) * t;
            const y = oldEntity.y + (entity.y - oldEntity.y) * t;
            drawEntity(entity, x, y);
        }
    }
}
```

**KaosNet Support:** KaosNet provides `SnapshotBuffer` utility:

```rust
use kaosnet::netcode::SnapshotBuffer;

let mut buffer = SnapshotBuffer::new(20); // 20 snapshots
buffer.push(timestamp, state);

// Get interpolated state
if let Some((older, newer, t)) = buffer.get_interpolation_pair(render_time) {
    // Use t (0.0 to 1.0) to lerp between older and newer
}
```

---

### 3. Quantization (Bandwidth Optimization)

**What:** Convert floats to scaled integers for smaller payloads.

**Why:** `1234.5678` (8+ chars) → `12345` (5 chars) = 40% smaller JSON.

**Implementation Pattern:**

```rust
// SERVER: Quantize before sending
struct PlayerNet {
    x: i32,  // actual_x * SCALE
    y: i32,  // actual_y * SCALE
    r: i16,  // radius * SCALE
}

const SCALE: f64 = 10.0; // 0.1 precision

impl From<&Player> for PlayerNet {
    fn from(p: &Player) -> Self {
        PlayerNet {
            x: (p.x * SCALE) as i32,
            y: (p.y * SCALE) as i32,
            r: (p.radius * SCALE) as i16,
        }
    }
}
```

```javascript
// CLIENT: Dequantize after receiving
function dequantize(raw) {
    const scale = raw.q; // Server sends scale factor
    return {
        players: raw.p.map(p => ({
            x: p.x / scale,
            y: p.y / scale,
            radius: p.r / scale,
            ...p
        }))
    };
}
```

**KaosNet Support:** Provides `Quantize` trait:

```rust
use kaosnet::netcode::{Quantize, Quantized};

#[derive(Quantize)]
#[quantize(scale = 10)]
struct Position {
    x: f64,
    y: f64,
}

// Automatically generates:
// - PositionNet { x: i32, y: i32 }
// - impl From<&Position> for PositionNet
// - impl From<PositionNet> for Position
```

---

### 4. Delta Compression (Advanced)

**What:** Send only changed data since last acknowledged state.

**Why:** Static worlds have mostly unchanged state each tick.

**When to Use:**
- Large worlds with many static entities
- Low entity churn (few spawns/despawns)
- Higher latency tolerance (100ms+)

**When NOT to Use:**
- Small, dynamic games (agar.io style)
- Everything moves every frame
- Simple state that's small anyway

**Implementation Pattern:**

```rust
// SERVER: Track per-client baseline
struct ClientState {
    transport: Transport,
    last_acked_tick: u64,
    last_sent_state: Option<GameState>,
}

fn broadcast(clients: &mut [ClientState], current: &GameState) {
    for client in clients {
        let delta = match &client.last_sent_state {
            Some(baseline) => compute_delta(baseline, current),
            None => current.clone(), // Full state for new clients
        };
        client.transport.send(&delta);
        client.last_sent_state = Some(current.clone());
    }
}
```

**KaosNet Support:** Not built-in (game-specific), but provides building blocks:
- `DiffableState` trait for computing deltas
- `StateHash` for detecting changes

---

## Choosing the Right Approach

| Game Type | Prediction | Interpolation | Quantization | Delta |
|-----------|------------|---------------|--------------|-------|
| **FPS (Shooter)** | Yes | Yes | Yes | Maybe |
| **Racing** | Yes | Yes | Yes | No |
| **Agar.io/Battle Royale** | Yes | Yes | Yes | No |
| **RTS** | No | Yes | Yes | Yes |
| **Turn-based** | No | No | Maybe | No |
| **MMO** | Yes (local) | Yes | Yes | Yes |

## Example: Kaos.io Implementation

Kaos.io demonstrates all techniques:

### Server (`server.rs`)

```rust
// Quantized network types
struct PlayerNet {
    id: u64,
    name: String,
    x: i32,      // × 10
    y: i32,      // × 10
    r: i16,      // × 10
    color: String,
    score: u32,
}

struct FoodNet {
    id: u32,
    x: i16,      // × 10
    y: i16,      // × 10
    r: u8,       // × 10
    c: u8,       // Color index (0-9)
}

// Short JSON keys for bandwidth
#[derive(Serialize)]
struct GameState {
    #[serde(rename = "p")]
    players: Vec<PlayerNet>,
    #[serde(rename = "f")]
    food: Vec<FoodNet>,
    #[serde(rename = "q")]
    q: u8,  // Scale factor (10)
}
```

### Client (`index.html`)

```javascript
// Constants
const INTERPOLATION_DELAY = 100;
const PREDICTION_ENABLED = true;

// State
const stateBuffer = [];
let predictedPlayer = null;
let pendingInputs = [];

// On server message
ws.onmessage = (event) => {
    const raw = JSON.parse(event.data);
    const state = dequantizeState(raw);

    bufferServerState(state);      // For interpolation
    reconcileWithServer(state);     // For prediction
};

// Render loop
function render() {
    interpolateEntities(performance.now());

    for (const player of state.players) {
        const rendered = getRenderedPlayer(player);
        drawPlayer(rendered.x, rendered.y, rendered.radius);
    }
}
```

## Network Stats HUD

Display these metrics to debug networking:

```javascript
// In HUD
document.getElementById('netStats').textContent =
    `${updateRate.toFixed(0)}Hz | ` +     // Server update rate
    `${(bytesReceived/1024).toFixed(1)}KB | ` +  // Total received
    `P:${pendingInputs.length} ` +        // Unacked inputs
    `B:${stateBuffer.length}`;            // Buffered snapshots
```

## Best Practices

1. **Start Simple**: Get basic multiplayer working, then add prediction/interpolation
2. **Measure First**: Profile bandwidth before optimizing
3. **Test with Lag**: Use browser dev tools to simulate 200ms latency
4. **Server Authority**: Never trust client physics for competitive games
5. **Graceful Degradation**: Fall back to raw state if interpolation fails

## Further Reading

- [Gaffer On Games: Networked Physics](https://gafferongames.com/post/introduction_to_networked_physics/)
- [Gaffer On Games: State Synchronization](https://gafferongames.com/post/state_synchronization/)
- [Valve: Source Multiplayer Networking](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking)
- [Gabriel Gambetta: Fast-Paced Multiplayer](https://www.gabrielgambetta.com/client-server-game-architecture.html)
