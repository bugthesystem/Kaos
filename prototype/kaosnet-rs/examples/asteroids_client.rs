//! Asteroids Client Example using kaosnet-rs SDK
//!
//! This example shows how an external game developer would use the SDK
//! to connect to a KaosNet RUDP server for a real-time game.
//!
//! Run the asteroids server first:
//!   cargo run --bin asteroids-server -p kaos-asteroids
//!
//! Then run this client:
//!   cargo run --example asteroids_client -p kaosnet-rs

use kaosnet_rs::RudpClient;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

const SERVER_ADDR: &str = "127.0.0.1:7352";

// Protocol messages (must match server)
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ClientMessage {
    Join { name: String },
    Input { thrust: bool, left: bool, right: bool, fire: bool },
    Leave,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ServerMessage {
    Welcome { player_id: u64 },
    GameState { state: GameState },
    PlayerDied { player_id: u64, score: i64, killer: String },
    GameOver { score: i64, rank: Option<usize> },
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct GameState {
    tick: u64,
    ships: Vec<ShipState>,
    asteroids: Vec<AsteroidState>,
    bullets: Vec<BulletState>,
    leaderboard: Vec<LeaderboardEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ShipState {
    id: u64,
    name: String,
    x: f64,
    y: f64,
    angle: f64,
    score: i64,
    alive: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AsteroidState {
    id: u64,
    x: f64,
    y: f64,
    size: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BulletState {
    x: f64,
    y: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LeaderboardEntry {
    name: String,
    score: i64,
}

fn main() -> std::io::Result<()> {
    let name = std::env::args().nth(1).unwrap_or_else(|| {
        format!("Pilot{}", std::process::id() % 1000)
    });

    println!("=== Asteroids Client (kaosnet-rs SDK) ===");
    println!("Connecting to {} as '{}'...", SERVER_ADDR, name);

    let server_addr: SocketAddr = SERVER_ADDR.parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    // Create RUDP client using the SDK
    let mut client = RudpClient::connect(server_addr)?;
    println!("Connected via RUDP!");

    // Send join message
    let join_msg = ClientMessage::Join { name: name.clone() };
    let join_data = serde_json::to_vec(&join_msg)?;
    client.send_raw(&join_data)?;

    // Wait for welcome
    let mut my_id = 0u64;
    for _ in 0..100 {
        client.receive(|_op, data| {
            if let Ok(msg) = serde_json::from_slice::<ServerMessage>(data) {
                if let ServerMessage::Welcome { player_id } = msg {
                    my_id = player_id;
                }
            }
        });
        if my_id != 0 {
            println!("Joined! Player ID: {}", my_id);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if my_id == 0 {
        println!("Failed to connect");
        return Ok(());
    }

    // Simple game loop (demo - would normally have real input handling)
    println!("\nReceiving game state for 3 seconds...\n");
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_secs(3) {
        // Send input (example: random thrust)
        let input_msg = ClientMessage::Input {
            thrust: rand::random::<bool>(),
            left: false,
            right: rand::random::<bool>(),
            fire: rand::random::<u8>() > 250,
        };
        let input_data = serde_json::to_vec(&input_msg)?;
        let _ = client.send_raw(&input_data);

        // Receive game state
        client.receive(|_op, data| {
            if let Ok(msg) = serde_json::from_slice::<ServerMessage>(data) {
                match msg {
                    ServerMessage::GameState { state } => {
                        println!(
                            "Tick {} | Ships: {} | Asteroids: {} | Bullets: {}",
                            state.tick,
                            state.ships.len(),
                            state.asteroids.len(),
                            state.bullets.len()
                        );
                        if let Some(my_ship) = state.ships.iter().find(|s| s.id == my_id) {
                            println!(
                                "  My ship: ({:.1}, {:.1}) angle={:.2} score={} alive={}",
                                my_ship.x, my_ship.y, my_ship.angle, my_ship.score, my_ship.alive
                            );
                        }
                    }
                    ServerMessage::PlayerDied { player_id, score, killer } => {
                        if player_id == my_id {
                            println!("  I died! Score: {} Killed by: {}", score, killer);
                        }
                    }
                    _ => {}
                }
            }
        });

        std::thread::sleep(Duration::from_millis(16)); // 60 FPS
    }

    // Leave
    let leave_msg = ClientMessage::Leave;
    let leave_data = serde_json::to_vec(&leave_msg)?;
    let _ = client.send_raw(&leave_data);

    println!("\nDisconnected. Thanks for playing!");
    Ok(())
}

// Simple random bool for demo (would use proper rand in real game)
mod rand {
    use std::time::{SystemTime, UNIX_EPOCH};
    static mut SEED: u64 = 0;

    pub fn random<T: Random>() -> T {
        T::random()
    }

    pub trait Random {
        fn random() -> Self;
    }

    impl Random for bool {
        fn random() -> bool {
            unsafe {
                SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
                if SEED == 0 {
                    SEED = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
                }
                (SEED >> 16) & 1 == 1
            }
        }
    }

    impl Random for u8 {
        fn random() -> u8 {
            unsafe {
                SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
                if SEED == 0 {
                    SEED = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
                }
                (SEED >> 16) as u8
            }
        }
    }
}
