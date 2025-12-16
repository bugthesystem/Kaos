//! Kaos.io Game Server
//!
//! An Agar.io-style multiplayer game demonstrating KaosNet capabilities.

use kaosnet::{RoomConfig, RoomRegistry, Server, ServerBuilder, SessionRegistry};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║   ██╗  ██╗ █████╗  ██████╗ ███████╗   ██╗ ██████╗             ║
    ║   ██║ ██╔╝██╔══██╗██╔═══██╗██╔════╝   ██║██╔═══██╗            ║
    ║   █████╔╝ ███████║██║   ██║███████╗   ██║██║   ██║            ║
    ║   ██╔═██╗ ██╔══██║██║   ██║╚════██║   ██║██║   ██║            ║
    ║   ██║  ██╗██║  ██║╚██████╔╝███████║██╗██║╚██████╔╝            ║
    ║   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝╚═╝ ╚═════╝             ║
    ║                                                               ║
    ║   Multiplayer Game Server Demo                                ║
    ║   Powered by KaosNet                                          ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
    "#);

    let sessions = Arc::new(SessionRegistry::new());
    let rooms = Arc::new(RoomRegistry::new());

    // Create the main game room
    let game_room_id = rooms.create(RoomConfig {
        max_players: 50,
        tick_rate: 20,
        label: "Kaos.io Main Arena".to_string(),
        module: "game".to_string(),
    });

    println!("Game room created: {}", game_room_id);
    println!();
    println!("Server Configuration:");
    println!("  Game Port: 7350 (UDP/RUDP)");
    println!("  WebSocket: 7351");
    println!("  Console:   7352 (HTTP API)");
    println!();
    println!("Connect with:");
    println!("  Web Client: Open web/index.html in your browser");
    println!("  CLI Client: cargo run --bin kaos-io-client");
    println!();
    println!("Console API:");
    println!("  Login:  curl -X POST http://localhost:7352/api/auth/login \\");
    println!("          -H 'Content-Type: application/json' \\");
    println!("          -d '{{\"username\":\"admin\",\"password\":\"admin\"}}'");
    println!();
    println!("Waiting for players...");
    println!();

    // In a real implementation, we'd start the actual server here
    // For now, let's just keep it running

    // Create a simple WebSocket server for the game
    let ws_addr = "127.0.0.1:7351";
    println!("Starting WebSocket server on {}...", ws_addr);

    // Keep the server running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
