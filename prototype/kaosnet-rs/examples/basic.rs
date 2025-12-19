//! Basic example demonstrating KaosNet Rust SDK.
//!
//! This example shows how to:
//! 1. Authenticate with the server
//! 2. Use WebSocket for real-time chat
//! 3. Use RUDP for low-latency game data

use kaosnet_rs::{KaosClient, RudpClient, Result};
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Server configuration
    let server_host = "localhost";
    let http_port = 7350;
    let rudp_port = 7351;

    println!("=== KaosNet Rust SDK Example ===\n");

    // ========================================================================
    // Part 1: HTTP Client + Authentication
    // ========================================================================
    println!("1. Authenticating with device ID...");

    let client = KaosClient::new(server_host, http_port);

    // Authenticate (this would connect to a real server)
    // let session = client.authenticate_device("rust-sdk-example-device").await?;
    // println!("   Logged in as: {}", session.user_id);
    println!("   (Would authenticate with server at {}:{})", server_host, http_port);

    // ========================================================================
    // Part 2: WebSocket for Real-time Communication
    // ========================================================================
    println!("\n2. Creating WebSocket connection...");

    let socket = client.create_socket();

    // Set up event handlers
    socket.on_chat_message(|msg| {
        println!("   [Chat] {}: {}", msg.sender_username, msg.content);
    }).await;

    socket.on_match_data(|data| {
        println!("   [Match] Received {} bytes from {}", data.data.len(), data.presence.username);
    }).await;

    socket.on_matchmaker_matched(|matched| {
        println!("   [Matchmaker] Found match! ID: {}", matched.match_id);
    }).await;

    // Connect (would need a valid session)
    // socket.connect(&session).await?;
    println!("   (Would connect WebSocket with session token)");

    // Join a chat room
    // let channel = socket.join_chat("general", 1, true, false).await?;
    // println!("   Joined chat channel: {}", channel.id);

    // Send a message
    // socket.send_chat_message(&channel.id, "Hello from Rust SDK!").await?;

    // ========================================================================
    // Part 3: RUDP for Low-Latency Game Data
    // ========================================================================
    println!("\n3. Demonstrating RUDP transport...");

    // RUDP is synchronous and meant for game loops
    let server_addr: SocketAddr = format!("127.0.0.1:{}", rudp_port).parse().unwrap();

    // In a real scenario:
    // let mut rudp = RudpClient::connect(server_addr)?;
    println!("   (Would connect RUDP to {})", server_addr);

    // Game loop example (not actually running)
    println!("   Example game loop (pseudo-code):");
    println!("   ```");
    println!("   loop {{");
    println!("       // Send player state at 60 Hz");
    println!("       rudp.send(1, b\"x:100,y:200,rot:45\")?;");
    println!("       ");
    println!("       // Receive other players' states");
    println!("       rudp.receive(|op_code, data| {{");
    println!("           // Handle game state update");
    println!("       }});");
    println!("       ");
    println!("       std::thread::sleep(Duration::from_millis(16));");
    println!("   }}");
    println!("   ```");

    // ========================================================================
    // Part 4: Matchmaker Example
    // ========================================================================
    println!("\n4. Matchmaker usage example:");
    println!("   ```");
    println!("   let ticket = client.add_matchmaker(&session, \"ranked\")");
    println!("       .string_property(\"region\", \"us\")");
    println!("       .numeric_property(\"skill\", 1500.0)");
    println!("       .min_count(2)");
    println!("       .max_count(4)");
    println!("       .send()");
    println!("       .await?;");
    println!("   ```");

    // ========================================================================
    // Part 5: Storage Example
    // ========================================================================
    println!("\n5. Storage usage example:");
    println!("   ```");
    println!("   // Write player data");
    println!("   client.write_storage_objects(&session, &[");
    println!("       StorageWriteRequest {{");
    println!("           collection: \"player_data\".into(),");
    println!("           key: \"stats\".into(),");
    println!("           value: json!({{\"level\": 42, \"xp\": 12500}}),");
    println!("           ..Default::default()");
    println!("       }}");
    println!("   ]).await?;");
    println!("   ```");

    println!("\n=== Example Complete ===");
    println!("Note: This example demonstrates API usage without a running server.");

    Ok(())
}
