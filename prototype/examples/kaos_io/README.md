# Kaos.io

An **Agar.io-style multiplayer game** demonstrating KaosNet's real-time capabilities.

```
    ██╗  ██╗ █████╗  ██████╗ ███████╗   ██╗ ██████╗
    ██║ ██╔╝██╔══██╗██╔═══██╗██╔════╝   ██║██╔═══██╗
    █████╔╝ ███████║██║   ██║███████╗   ██║██║   ██║
    ██╔═██╗ ██╔══██║██║   ██║╚════██║   ██║██║   ██║
    ██║  ██╗██║  ██║╚██████╔╝███████║██╗██║╚██████╔╝
    ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝╚═╝ ╚═════╝
```

## Features

- 🎮 **Real-time multiplayer** - Smooth player movement and interactions
- 🎯 **Server-authoritative** - All game logic runs on server (Lua)
- 🌐 **Web client** - Beautiful HTML5 Canvas game
- 💻 **Terminal client** - ASCII-art client for CLI enthusiasts
- 📊 **Live leaderboard** - Real-time score tracking
- 🗺️ **Minimap** - See the whole arena at a glance

## Quick Start

### Running the Server

```bash
cd prototype/examples/kaos_io
cargo run --bin kaos-io-server
```

Then open http://localhost:8080 in your browser to play!

### Terminal Client

```bash
cd examples/kaos_io
cargo run --bin kaos-io-client -- "YourName"
```

**Controls:**
- `WASD` / Arrow keys - Move
- `Space` - Split
- `Q` - Quit

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         KAOS.IO ARCHITECTURE                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌─────────────┐     WebSocket      ┌─────────────────────┐   │
│   │  Web Client │◄──────────────────►│                     │   │
│   │  (Canvas)   │                    │    KaosNet Server   │   │
│   └─────────────┘                    │                     │   │
│                                      │  ┌───────────────┐  │   │
│   ┌─────────────┐     RUDP           │  │  Lua Runtime  │  │   │
│   │ Term Client │◄──────────────────►│  │  (game.lua)   │  │   │
│   │  (ASCII)    │                    │  └───────────────┘  │   │
│   └─────────────┘                    │                     │   │
│                                      │  ┌───────────────┐  │   │
│   ┌─────────────┐     HTTP/2         │  │    Console    │  │   │
│   │ Admin Panel │◄──────────────────►│  │   (kaos-http) │  │   │
│   └─────────────┘                    │  └───────────────┘  │   │
│                                      └─────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Server/Client Time Synchronization

### Best Practices for Multiplayer Games

#### 1. **Server is the Source of Truth**

```lua
-- Server (game.lua)
function match_tick(state_data, tick, messages)
    -- Server tick is authoritative
    -- All game state changes happen here
    current.tick_count = tick

    -- Process inputs, not positions
    for _, msg in ipairs(messages) do
        process_input(current, msg)  -- Input: "move towards (x,y)"
    end                               -- NOT: "set position to (x,y)"
end
```

#### 2. **Client-Side Prediction**

```javascript
// Client predicts movement locally for responsiveness
function update(dt) {
    // Apply input immediately (prediction)
    myPlayer.x += velocity.x * dt;
    myPlayer.y += velocity.y * dt;

    // Store prediction for reconciliation
    pendingInputs.push({
        seq: inputSequence++,
        input: currentInput,
        predictedPos: { x: myPlayer.x, y: myPlayer.y }
    });

    // Send input to server
    send({ type: 'input', seq: inputSequence, input: currentInput });
}
```

#### 3. **Server Reconciliation**

```javascript
// When server state arrives, reconcile with predictions
function onServerState(serverState) {
    // Find our player in server state
    const serverPlayer = serverState.players[myId];

    // Remove acknowledged inputs
    pendingInputs = pendingInputs.filter(i => i.seq > serverState.lastProcessedInput);

    // Reset to server position
    myPlayer.x = serverPlayer.x;
    myPlayer.y = serverPlayer.y;

    // Re-apply unacknowledged inputs
    for (const input of pendingInputs) {
        applyInput(myPlayer, input.input);
    }
}
```

#### 4. **Entity Interpolation**

```javascript
// Interpolate other players between known positions
function interpolateEntities(dt) {
    const renderTime = Date.now() - INTERPOLATION_DELAY; // e.g., 100ms

    for (const entity of otherPlayers) {
        // Find two states to interpolate between
        const [prev, next] = findSurroundingStates(entity.history, renderTime);

        if (prev && next) {
            const t = (renderTime - prev.time) / (next.time - prev.time);
            entity.renderX = lerp(prev.x, next.x, t);
            entity.renderY = lerp(prev.y, next.y, t);
        }
    }
}
```

#### 5. **Clock Synchronization**

```javascript
// Sync client clock with server
async function syncClock() {
    const samples = [];

    for (let i = 0; i < 5; i++) {
        const t0 = Date.now();
        const serverTime = await requestServerTime();
        const t1 = Date.now();

        const rtt = t1 - t0;
        const offset = serverTime - t0 - (rtt / 2);
        samples.push({ rtt, offset });
    }

    // Use median offset (most stable)
    samples.sort((a, b) => a.rtt - b.rtt);
    serverTimeOffset = samples[Math.floor(samples.length / 2)].offset;
}

function getServerTime() {
    return Date.now() + serverTimeOffset;
}
```

### Timing Configuration

| Parameter | Recommended | Description |
|-----------|-------------|-------------|
| Tick Rate | 20-60 Hz | Server update frequency |
| Send Rate | 20-30 Hz | Client→Server input rate |
| Interpolation Delay | 100ms | Smoothing buffer for other entities |
| Max Prediction | 200ms | Maximum client-ahead time |

### Example Flow

```
Time →

Client:  [Input]────────────────[Predict]────────[Reconcile]────►
              \                      ↑                ↑
               \                     │                │
Server:         ──────[Process]─────[Tick]───────[Broadcast]───►
                           │                          │
                           └──────────────────────────┘
                              State sent to all clients
```

## Game Protocol

### Client → Server Messages

```json
{ "type": "move", "x": 150.5, "y": 200.3 }
{ "type": "split" }
{ "type": "eject" }
```

### Server → Client Messages

```json
{
    "type": "state",
    "tick": 1234,
    "players": {
        "player_1": { "id": "player_1", "x": 100, "y": 200, "mass": 500, ... }
    },
    "food": [ { "id": 1, "x": 50, "y": 60, "color": "#FF6B6B" }, ... ],
    "leaderboard": [ { "name": "Pro", "score": 1500 }, ... ]
}

{ "type": "player_eaten", "eater": "player_1", "eaten": "player_2" }
{ "type": "player_joined", "player": { ... } }
{ "type": "player_left", "player_id": "player_3" }
```

## File Structure

```
kaos_io/
├── Cargo.toml           # Rust dependencies
├── README.md            # This file
├── scripts/
│   └── game.lua         # Server-side game logic
├── src/
│   ├── server.rs        # Game server binary
│   └── client.rs        # Terminal client binary
└── web/
    └── index.html       # Web client (standalone)
```

## Extending the Game

### Add a New Power-Up (Lua)

```lua
-- In game.lua
local POWERUPS = {
    speed = { duration = 5, multiplier = 1.5 },
    shield = { duration = 3, invincible = true },
    magnet = { duration = 4, attract_food = true },
}

function spawn_powerup()
    local types = {"speed", "shield", "magnet"}
    return {
        type = types[math.random(#types)],
        x = math.random(100, WORLD_WIDTH - 100),
        y = math.random(100, WORLD_HEIGHT - 100),
    }
end
```

### Add Particle Effects (Client)

```javascript
class ParticleSystem {
    constructor() {
        this.particles = [];
    }

    emit(x, y, color, count = 10) {
        for (let i = 0; i < count; i++) {
            this.particles.push({
                x, y, color,
                vx: (Math.random() - 0.5) * 200,
                vy: (Math.random() - 0.5) * 200,
                life: 1.0
            });
        }
    }

    update(dt) {
        for (const p of this.particles) {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt * 2;
        }
        this.particles = this.particles.filter(p => p.life > 0);
    }
}
```

## Contributing

This is a demo for KaosNet. Feel free to:

- 🐛 Report bugs
- 💡 Suggest features
- 🔧 Submit PRs
- 🎨 Improve the UI
- 🎮 Add game modes

## License

MIT OR Apache-2.0
