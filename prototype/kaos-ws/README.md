# kaos-ws

WebSocket transport for Kaos messaging, implementing the shared `Transport` trait.

## Features

- **RFC 6455 compliant** - Full WebSocket protocol via tungstenite
- **Binary frames** - Optimized for game message payloads
- **Non-blocking** - Designed for game server tick loops
- **Transport trait** - Interchangeable with RUDP transport

## Quick Start

### Server

```rust
use kaos_ws::{WsServer, Transport};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = WsServer::bind("127.0.0.1:9000")?;

    loop {
        // Accept new connections
        if let Some(mut client) = server.accept()? {
            // Handle client in game loop
            client.receive(|data| {
                println!("Received: {} bytes", data.len());
            });

            client.send(b"welcome")?;
        }
    }
}
```

### Client

```rust
use kaos_ws::{WsTransport, Transport};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = WsTransport::connect("ws://127.0.0.1:9000")?;

    // Send message
    client.send(b"hello server")?;

    // Receive in game loop
    loop {
        client.receive(|data| {
            println!("Got: {:?}", data);
        });
    }
}
```

## Transport Trait

The `Transport` trait provides a unified interface for network communication:

```rust
pub trait Transport {
    /// Send data, returns bytes sent
    fn send(&mut self, data: &[u8]) -> std::io::Result<usize>;

    /// Receive into callback, returns count received
    fn receive<F: FnMut(&[u8])>(&mut self, handler: F) -> usize;

    /// Flush pending operations
    fn flush(&mut self) -> std::io::Result<()>;

    /// Check if connection is open
    fn is_open(&self) -> bool;

    /// Close the connection
    fn close(&mut self) -> std::io::Result<()>;
}
```

This allows swapping between WebSocket and RUDP transports:

```rust
fn handle_client<T: Transport>(transport: &mut T) {
    transport.receive(|data| {
        // Process game message
    });
    transport.send(b"response")?;
}
```

## Non-Blocking Mode

For game loops, use non-blocking mode:

```rust
server.set_nonblocking(true)?;

// Game loop
loop {
    // Accept doesn't block
    if let Some(client) = server.accept()? {
        clients.push(client);
    }

    // Process existing clients
    for client in &mut clients {
        client.receive(|data| {
            // Handle message
        });
    }

    // Game tick
    std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
}
```

## Binary Data

WebSocket frames are sent as binary for optimal game payload handling:

```rust
// Send binary game state
let state = bincode::serialize(&game_state)?;
client.send(&state)?;

// Receive and deserialize
client.receive(|data| {
    let update: GameUpdate = bincode::deserialize(data).unwrap();
    apply_update(update);
});
```

## Integration with KaosNet

This crate provides the WebSocket transport layer for KaosNet servers:

```rust
// In kaosnet server
use kaos_ws::{WsServer, WsTransport};

let ws_server = WsServer::bind("0.0.0.0:7351")?;

// Use alongside RUDP for browser clients
```

## Error Handling

```rust
use kaos_ws::{WsError, Result};

match client.send(data) {
    Ok(bytes) => println!("Sent {} bytes", bytes),
    Err(WsError::ConnectionClosed) => cleanup_client(),
    Err(WsError::Io(e)) => handle_io_error(e),
    Err(e) => eprintln!("WebSocket error: {}", e),
}
```

## License

MIT License
