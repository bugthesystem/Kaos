# KaosTalk Protocol Specification

This document describes the KaosTalk wire protocol and API endpoints for KaosNet game servers.

## Overview

KaosTalk is the real-time communication protocol for KaosNet. It supports two primary channels:

1. **HTTP REST API** - Authentication, storage, leaderboards, admin operations
2. **WebSocket** - Real-time game state, match communication, RPC

## Authentication

### Device Authentication (Anonymous)

Authenticates a client using a unique device identifier. Creates a new account if one doesn't exist.

```
POST /api/auth/device
Content-Type: application/json

{
  "device_id": "unique-device-id",
  "create": true,
  "username": "PlayerName"
}
```

**Response:**
```json
{
  "session": {
    "token": "eyJ0eXAiOiJKV1Q...",
    "refresh_token": "eyJ0eXAiOiJKV1Q...",
    "user_id": "a1b2c3d4-e5f6-...",
    "username": "PlayerName",
    "expires_at": 1734567890,
    "created_at": 1734564290
  },
  "new_account": true
}
```

### Email Authentication

Authenticates with email and password. Creates account if `create` is true.

```
POST /api/auth/email
Content-Type: application/json

{
  "email": "player@example.com",
  "password": "securepassword",
  "create": true,
  "username": "PlayerName"
}
```

### Custom Authentication

For external auth systems (Steam, custom backends, etc.).

```
POST /api/auth/custom
Content-Type: application/json

{
  "id": "steam:12345678",
  "create": true,
  "username": "SteamPlayer",
  "vars": {
    "platform": "steam",
    "region": "us"
  }
}
```

### Token Refresh

Refreshes an expired access token using the refresh token.

```
POST /api/auth/refresh
Content-Type: application/json

{
  "refresh_token": "eyJ0eXAiOiJKV1Q..."
}
```

## Error Codes

| Code | Description |
|------|-------------|
| `ACCOUNT_NOT_FOUND` | Account does not exist |
| `INVALID_CREDENTIALS` | Wrong password or invalid token |
| `ACCOUNT_EXISTS` | Account already exists |
| `DEVICE_ALREADY_LINKED` | Device linked to another account |
| `EMAIL_ALREADY_REGISTERED` | Email already in use |
| `INVALID_TOKEN` | Token is malformed |
| `TOKEN_EXPIRED` | Token has expired |
| `ACCOUNT_DISABLED` | Account is banned/disabled |
| `WEAK_PASSWORD` | Password doesn't meet requirements |
| `INVALID_EMAIL` | Email format is invalid |

## KaosTalk WebSocket Protocol

### Connection

Connect to the KaosTalk WebSocket endpoint with your session token:

```
ws://localhost:7351?token=eyJ0eXAiOiJKV1Q...
```

### KaosTalk Binary Wire Format

KaosTalk messages use a 4-byte header followed by a variable-length payload:

```
+--------+--------+--------+--------+----------------+
|  op    | flags  |    length (LE)   |    payload...  |
+--------+--------+--------+--------+----------------+
  1 byte   1 byte      2 bytes          N bytes
```

- **op**: Operation code (see OpCodes below)
- **flags**: Reserved for future use
- **length**: Little-endian 16-bit payload length
- **payload**: Variable-length message data

### KaosTalk OpCodes

| OpCode | Name | Description |
|--------|------|-------------|
| 0x01 | SessionStart | Initiate session handshake |
| 0x02 | SessionEnd | Close session |
| 0x03 | Heartbeat | Keep-alive ping/pong |
| 0x04 | SessionAck | Server acknowledges session |
| 0x10 | RoomCreate | Create a new match/room |
| 0x11 | RoomJoin | Join an existing match |
| 0x12 | RoomLeave | Leave current match |
| 0x13 | RoomData | Broadcast data to match |
| 0x14 | RoomState | Server sends match state |
| 0x15 | RoomList | List available rooms |
| 0x20 | Rpc | Remote procedure call |
| 0x21 | RpcResponse | RPC response |
| 0x30 | MatchmakerAdd | Join matchmaking queue |
| 0x31 | MatchmakerRemove | Leave matchmaking queue |
| 0x32 | MatchmakerMatched | Match found notification |
| 0xFF | Error | Error message |

### Message Payloads

#### RoomJoin (0x11)

```
+--------+-------------------+-----------------+
| length |    room_id        |   metadata...   |
+--------+-------------------+-----------------+
  1 byte   N bytes (UTF-8)      variable
```

#### RoomData (0x13)

```
+--------+---------------+----------+-------------+
| length |   room_id     | sender   |   data...   |
+--------+---------------+----------+-------------+
  1 byte   N bytes (UTF-8)  8 bytes    variable
                           (LE u64)
```

#### RPC (0x20)

```
+----------+--------+--------------+--------------+
|  rpc_id  | length |   method     |   data...    |
+----------+--------+--------------+--------------+
  4 bytes    1 byte   N bytes (UTF-8)   variable
  (LE u32)
```

### JSON Message Format (Legacy/Simple Games)

For simpler games, you can use JSON messages instead of binary:

**Client Input:**
```json
{
  "seq": 42,
  "target_x": 1234.5,
  "target_y": 5678.9,
  "name": "Player1"
}
```

**Server State Broadcast:**
```json
{
  "p": [
    {
      "id": 1,
      "name": "Player1",
      "x": 1234,
      "y": 5678,
      "r": 50,
      "color": "#FF6B6B",
      "score": 12345
    }
  ],
  "f": [
    { "id": 1, "x": 100, "y": 200, "r": 5, "c": 0 }
  ],
  "lb": [
    { "rank": 1, "name": "TopPlayer", "score": 99999 }
  ],
  "ww": 10000,
  "wh": 10000,
  "t": 42
}
```

## REST API Endpoints

### Storage

```
GET  /api/storage?collection=<col>&user_id=<uid>
POST /api/storage
DELETE /api/storage/:user_id/:collection/:key
```

### Leaderboards

```
GET  /api/leaderboards/:id/records
POST /api/leaderboards/:id/records
```

### Sessions (Admin)

```
GET  /api/sessions
GET  /api/sessions/:id
POST /api/sessions/:id/kick
```

### Rooms (Admin)

```
GET  /api/rooms
GET  /api/rooms/:id
GET  /api/rooms/:id/state
POST /api/rooms/:id/terminate
```

## Using with kaosnet-js SDK

The SDK abstracts all protocol details:

```typescript
import { KaosClient } from 'kaosnet-js';

// Create client
const client = new KaosClient('localhost', 7350);

// Authenticate
const session = await client.authenticateDevice('unique-device-id');

// Connect WebSocket
const socket = client.createSocket();
await socket.connect(session);

// Join matchmaking
const ticket = await socket.addMatchmaker({
  query: '+mode:ranked +region:us',
  minCount: 2,
  maxCount: 4,
  stringProperties: { mode: 'ranked', region: 'us' },
  numericProperties: { skill: 1500 },
});

// Handle match found
socket.onMatchmakerMatched = (match) => {
  socket.joinMatch(match.matchId);
};

// Send/receive game state
socket.onMatchState = (state) => {
  console.log('Game state:', state.data);
};

socket.sendMatchState(1, { x: 100, y: 200 });
```

## Session Lifecycle

```
1. Client connects (WebSocket or HTTP)
2. Client authenticates (device/email/custom)
3. Server returns session tokens (access + refresh)
4. Client connects WebSocket with access token
5. Server creates Session in SessionRegistry
6. Client joins match (via matchmaker or direct)
7. Match state broadcasts at configured tick rate
8. On disconnect:
   - Session removed from registry
   - Match presence removed
   - Cleanup hooks called
```

## Security Considerations

1. **JWT Tokens**: Access tokens expire after 1 hour (configurable)
2. **Refresh Tokens**: Valid for 7 days (configurable)
3. **Password Requirements**: Minimum 8 characters
4. **Rate Limiting**: Applied to all endpoints
5. **CORS**: Configured for web clients

## Version

KaosTalk Protocol Version: 1.0
