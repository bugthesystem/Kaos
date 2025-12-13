# KaosNet Design

Game server built on kaos-rudp. Inspired by Nakama's architecture.

## Core Concepts

### Sessions
Client connection lifecycle. Each session:
- Unique 64-bit ID
- Authentication state (anonymous, authenticated)
- Metadata (user_id, username, vars)
- Heartbeat tracking

### Peers
Network endpoint abstraction. Wraps kaos-rudp transport.
- Send/receive with session context
- Automatic reconnection handling
- Latency tracking

### Rooms (Matches)
Multiplayer coordination unit:
- Room ID (UUID)
- State (custom Lua table or bytes)
- Tick rate (configurable)
- Max players
- Authoritative loop

### Messages
Wire protocol:
```
┌─────────────────────────────────────────────────┐
│ KaosNet Message (variable length)               │
├────────────┬────────────┬───────────────────────┤
│ op: u8     │ len: u16   │ payload: [u8; len]    │
└────────────┴────────────┴───────────────────────┘
```

Operations:
- 0x01: SessionStart
- 0x02: SessionEnd
- 0x03: Heartbeat
- 0x10: RoomCreate
- 0x11: RoomJoin
- 0x12: RoomLeave
- 0x13: RoomData
- 0x14: RoomState
- 0x20: Rpc
- 0x21: RpcResponse

## Lua Runtime

Sandboxed execution via mlua. No filesystem/network access from scripts.

### Hook Registration
```lua
kaos.register_rpc("my_rpc", function(ctx, payload)
    return { result = "ok" }
end)

kaos.register_before("room_join", function(ctx, msg)
    -- validate or modify
    return msg
end)

kaos.register_after("room_join", function(ctx, msg)
    -- side effects
end)
```

### Match Handler
```lua
local M = {}

function M.match_init(ctx, params)
    return { state = {}, tick_rate = 10, label = "my_match" }
end

function M.match_join(ctx, state, presences)
    return state
end

function M.match_leave(ctx, state, presences)
    return state
end

function M.match_loop(ctx, state, tick, messages)
    -- game logic
    return state
end

function M.match_terminate(ctx, state)
    return nil
end

return M
```

### Exposed APIs
```lua
kaos.session_get(session_id)
kaos.session_kick(session_id)
kaos.room_create(module, params)
kaos.room_get(room_id)
kaos.room_list(query)
kaos.room_send(room_id, op, data)
kaos.broadcast(room_id, sender, data, presences)
kaos.time_now()
kaos.uuid()
kaos.json_encode(table)
kaos.json_decode(string)
kaos.logger_info(message)
kaos.logger_warn(message)
kaos.logger_error(message)
```

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                      KaosNet Server                      │
├──────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │   Sessions  │  │    Rooms    │  │  Lua Pool   │      │
│  │   HashMap   │  │   HashMap   │  │  (mlua VMs) │      │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘      │
│         │                │                │              │
│  ┌──────▼────────────────▼────────────────▼──────┐      │
│  │              Message Router                    │      │
│  │  (dispatch by opcode, call hooks/handlers)     │      │
│  └──────────────────────┬────────────────────────┘      │
│                         │                                │
│  ┌──────────────────────▼────────────────────────┐      │
│  │              Peer Manager                      │      │
│  │  (accept connections, manage transports)       │      │
│  └──────────────────────┬────────────────────────┘      │
│                         │                                │
│  ┌──────────────────────▼────────────────────────┐      │
│  │              kaos-rudp Transport               │      │
│  │  (reliable UDP, congestion control, NAK/ACK)   │      │
│  └───────────────────────────────────────────────┘      │
└──────────────────────────────────────────────────────────┘
```

## Module Layout

```
kaosnet/
├── src/
│   ├── lib.rs          # Public API, re-exports
│   ├── error.rs        # Error types
│   ├── protocol.rs     # Wire format, opcodes
│   ├── session.rs      # Session management
│   ├── peer.rs         # Network peer abstraction
│   ├── room.rs         # Match/room coordination
│   ├── server.rs       # Main server loop
│   └── lua/
│       ├── mod.rs      # Lua runtime pool
│       ├── api.rs      # kaos.* functions
│       └── hooks.rs    # Hook registration
├── examples/
│   ├── echo_server.rs  # Simple echo
│   └── game_server.rs  # With Lua scripting
├── Cargo.toml
└── DESIGN.md
```

## Performance Targets

- 10K concurrent sessions per node
- 1000 active rooms per node
- Sub-millisecond message routing
- 60 tick/sec match loop support

## Dependencies

- `kaos-rudp`: Transport layer
- `mlua`: Lua 5.4 runtime (luajit feature optional)
- `dashmap`: Concurrent hash maps
- `uuid`: Room/session IDs
- `serde`: Serialization
- `thiserror`: Error handling
