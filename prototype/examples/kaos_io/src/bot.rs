//! Kaos.io RUDP Bot Client
//!
//! An AI-controlled bot that connects via RUDP transport to demonstrate
//! KaosNet's dual-transport capability (WebSocket + RUDP).
//!
//! The server auto-authenticates RUDP clients on first packet, so the bot
//! just needs to send movement commands and receive game state.
//!
//! Usage:
//!   cargo run -p kaos-io --bin kaos-io-bot [bot_name] [server_addr]

use kaos_rudp::{RudpTransport, Transport};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

const WORLD_WIDTH: f32 = 2000.0;
const WORLD_HEIGHT: f32 = 2000.0;

// ==================== Protocol Messages ====================

/// Client -> Server message (movement input)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MoveInput {
    target_x: f32,
    target_y: f32,
}

/// Server -> Client game state (matches Lua broadcast format)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameState {
    players: Option<Vec<PlayerState>>,
    food: Option<Vec<FoodState>>,
    leaderboard: Option<Vec<LeaderboardEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlayerState {
    id: u64,
    name: String,
    x: f32,
    y: f32,
    #[serde(default)]
    mass: f32,
    #[serde(default)]
    radius: f32,
    color: String,
    score: i32,
    #[serde(default)]
    alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FoodState {
    id: i32,
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LeaderboardEntry {
    name: String,
    score: i32,
}

// ==================== Bot ====================

struct Bot {
    name: String,
    transport: RudpTransport,

    // Position (updated from server state)
    my_x: f32,
    my_y: f32,
    my_mass: f32,
    my_score: i32,

    // AI target
    target_x: f32,
    target_y: f32,

    // AI state
    last_direction_change: Instant,
    wander_interval: Duration,

    // Stats
    messages_sent: u64,
    messages_received: u64,
    bytes_received: u64,
    last_stats_print: Instant,
    connected: bool,
}

impl Bot {
    fn new(name: &str, server_addr: SocketAddr) -> std::io::Result<Self> {
        let mut rng = rand::thread_rng();

        // Bind to random local port
        let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let transport = RudpTransport::new(local_addr, server_addr, 256)?;

        Ok(Self {
            name: name.to_string(),
            transport,
            my_x: rng.gen_range(100.0..WORLD_WIDTH - 100.0),
            my_y: rng.gen_range(100.0..WORLD_HEIGHT - 100.0),
            my_mass: 400.0,
            my_score: 0,
            target_x: WORLD_WIDTH / 2.0,
            target_y: WORLD_HEIGHT / 2.0,
            last_direction_change: Instant::now(),
            wander_interval: Duration::from_millis(rng.gen_range(500..2000)),
            messages_sent: 0,
            messages_received: 0,
            bytes_received: 0,
            last_stats_print: Instant::now(),
            connected: false,
        })
    }

    fn send_movement(&mut self) {
        // Send as JSON matching what the Lua handler expects
        let input = MoveInput {
            target_x: self.target_x,
            target_y: self.target_y,
        };
        if let Ok(json) = serde_json::to_vec(&input) {
            // Ignore send errors (congestion window full is normal during high traffic)
            if self.transport.send(&json).is_ok() {
                self.messages_sent += 1;
            }
        }
    }

    fn update_ai(&mut self) {
        let mut rng = rand::thread_rng();

        // Change direction periodically (wander behavior)
        if self.last_direction_change.elapsed() >= self.wander_interval {
            // Pick a new random target
            self.target_x = rng.gen_range(100.0..WORLD_WIDTH - 100.0);
            self.target_y = rng.gen_range(100.0..WORLD_HEIGHT - 100.0);
            self.last_direction_change = Instant::now();
            self.wander_interval = Duration::from_millis(rng.gen_range(800..3000));
        }
    }

    fn process_messages(&mut self) {
        // Use kaos-rudp Transport trait for reliable delivery
        self.transport.receive(|data| {
            self.bytes_received += data.len() as u64;
            self.messages_received += 1;

            // Parse game state
            if let Ok(state) = serde_json::from_slice::<GameState>(data) {
                if !self.connected {
                    let player_count = state.players.as_ref().map(|p| p.len()).unwrap_or(0);
                    println!("[{}] Connected! Receiving game state via RUDP ({} players)", self.name, player_count);
                    self.connected = true;
                }

                // Update our position from server state
                if let Some(players) = &state.players {
                    for player in players {
                        if player.name.starts_with("RudpPlayer") || player.name == self.name {
                            self.my_x = player.x;
                            self.my_y = player.y;
                            self.my_mass = player.radius; // Server sends radius, not mass
                            self.my_score = player.score;
                            break;
                        }
                    }
                }
            }
        });
    }

    fn print_stats(&mut self) {
        if self.last_stats_print.elapsed() >= Duration::from_secs(5) {
            let status = if self.connected { "connected" } else { "connecting" };
            println!(
                "[{}] {} | sent={} recv={} ({} bytes) | pos=({:.0},{:.0}) mass={:.0} score={}",
                self.name, status,
                self.messages_sent, self.messages_received, self.bytes_received,
                self.my_x, self.my_y, self.my_mass, self.my_score
            );
            self.last_stats_print = Instant::now();
        }
    }

    fn run(&mut self) {
        println!("[{}] Starting RUDP bot", self.name);
        println!("[{}] Local:  {:?}", self.name, self.transport.socket().local_addr());
        println!("[{}] Server: {:?}", self.name, self.transport.remote_addr());
        println!("[{}] Sending initial packet to register with server...", self.name);

        // Send initial packet to trigger server-side client creation
        self.send_movement();

        let tick_duration = Duration::from_millis(50); // 20 Hz
        let mut last_move = Instant::now();

        loop {
            let tick_start = Instant::now();

            // Process incoming messages
            self.process_messages();

            // Update AI behavior
            self.update_ai();

            // Send movement updates at 10 Hz
            if last_move.elapsed() >= Duration::from_millis(100) {
                self.send_movement();
                last_move = Instant::now();
            }

            // Print periodic stats
            self.print_stats();

            // Maintain tick rate
            let elapsed = tick_start.elapsed();
            if elapsed < tick_duration {
                std::thread::sleep(tick_duration - elapsed);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let bot_name = args.get(1)
        .map(|s| s.as_str())
        .unwrap_or("RudpBot");

    let server_addr: SocketAddr = args.get(2)
        .map(|s| s.as_str())
        .unwrap_or("127.0.0.1:7354")
        .parse()?;

    println!(r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║   RUDP Bot - KaosNet Dual Transport Demo                      ║
    ║   Connects via RUDP while web clients use WebSocket           ║
    ╚═══════════════════════════════════════════════════════════════╝
    "#);

    let mut bot = Bot::new(bot_name, server_addr)?;
    bot.run();

    Ok(())
}
