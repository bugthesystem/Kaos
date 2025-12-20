//! Kaos Asteroids RUDP Bot Client
//!
//! An AI-controlled bot that uses the KaosNet Rust SDK:
//! 1. Authenticates via HTTP API using device auth
//! 2. Connects via RUDP transport for low-latency game state
//!
//! Usage:
//!   cargo run -p kaos-asteroids --bin kaos-asteroids-bot [bot_name] [api_host] [api_port] [rudp_port]
//!
//! Docker usage:
//!   The bot connects to kaos-asteroids:7350 for auth and kaos-asteroids:7354 for RUDP

use kaosnet_rs::{KaosClient, RudpClient};
use rand::{Rng, thread_rng};
use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

const WORLD_WIDTH: f32 = 100.0;
const WORLD_HEIGHT: f32 = 50.0;

// ==================== Protocol Messages ====================

/// Client -> Server messages (must match server protocol)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    Join { name: String },
    Input { thrust: bool, left: bool, right: bool, fire: bool },
    Leave,
}

/// Server -> Client messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ServerMessage {
    Welcome { player_id: u64 },
    GameState { state: GameStateData },
}

/// Game state data from server
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameStateData {
    tick: Option<u64>,
    ships: Option<Vec<ShipState>>,
    asteroids: Option<Vec<AsteroidState>>,
    bullets: Option<Vec<BulletState>>,
    leaderboard: Option<Vec<LeaderboardEntry>>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShipState {
    id: u64,
    name: String,
    x: f32,
    y: f32,
    angle: f32,
    score: i32,
    alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AsteroidState {
    id: i32,
    x: f32,
    y: f32,
    size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BulletState {
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
    rudp: RudpClient,
    user_id: String,
    player_id: Option<u64>,

    // Position (updated from server state)
    my_x: f32,
    my_y: f32,
    my_angle: f32,
    my_score: i32,
    my_alive: bool,

    // AI state (current input values)
    thrust: bool,
    left: bool,
    right: bool,
    fire: bool,
    last_decision: Instant,
    decision_interval: Duration,
    target_asteroid: Option<(f32, f32)>,

    // Stats
    messages_sent: u64,
    messages_received: u64,
    bytes_received: u64,
    last_stats_print: Instant,
    joined: bool,
    connected: bool,
}

impl Bot {
    fn new(name: String, rudp: RudpClient, user_id: String) -> Self {
        let mut rng = thread_rng();
        Self {
            name,
            rudp,
            user_id,
            player_id: None,
            my_x: WORLD_WIDTH / 2.0,
            my_y: WORLD_HEIGHT / 2.0,
            my_angle: 0.0,
            my_score: 0,
            my_alive: true,
            thrust: false,
            left: false,
            right: false,
            fire: false,
            last_decision: Instant::now(),
            decision_interval: Duration::from_millis(rng.gen_range(100..300)),
            target_asteroid: None,
            messages_sent: 0,
            messages_received: 0,
            bytes_received: 0,
            last_stats_print: Instant::now(),
            joined: false,
            connected: false,
        }
    }

    fn send_join(&mut self) {
        let join_msg = ClientMessage::Join { name: self.name.clone() };
        if let Ok(data) = serde_json::to_vec(&join_msg) {
            if self.rudp.send(0, &data).is_ok() {
                self.messages_sent += 1;
            }
        }
    }

    fn process_messages(&mut self) {
        let bytes_received = &mut self.bytes_received;
        let messages_received = &mut self.messages_received;
        let joined = &mut self.joined;
        let connected = &mut self.connected;
        let player_id = &mut self.player_id;
        let name = &self.name;
        let my_x = &mut self.my_x;
        let my_y = &mut self.my_y;
        let my_angle = &mut self.my_angle;
        let my_score = &mut self.my_score;
        let my_alive = &mut self.my_alive;
        let target_asteroid = &mut self.target_asteroid;

        self.rudp.receive(|_op_code, data| {
            *bytes_received += data.len() as u64;
            *messages_received += 1;

            // Try to parse server message
            if let Ok(msg) = serde_json::from_slice::<ServerMessage>(data) {
                match msg {
                    ServerMessage::Welcome { player_id: pid } => {
                        println!("[{}] Welcome received! player_id={}", name, pid);
                        *player_id = Some(pid);
                        *joined = true;
                    }
                    ServerMessage::GameState { state } => {
                        if !*connected {
                            let player_count = state.ships.as_ref().map(|s| s.len()).unwrap_or(0);
                            println!("[{}] Connected! Receiving game state via RUDP ({} players)", name, player_count);
                            *connected = true;
                        }

                        // Find ourselves in the ships list
                        if let Some(ships) = &state.ships {
                            for ship in ships {
                                // Match by name or player_id
                                let matches = ship.name == *name
                                    || ship.name.contains(name.as_str())
                                    || player_id.map(|pid| ship.id == pid).unwrap_or(false);
                                if matches {
                                    *my_x = ship.x;
                                    *my_y = ship.y;
                                    *my_angle = ship.angle;
                                    *my_score = ship.score;
                                    *my_alive = ship.alive;
                                    break;
                                }
                            }
                        }

                        // Find nearest asteroid
                        if let Some(asteroids) = &state.asteroids {
                            *target_asteroid = asteroids.iter()
                                .map(|a| {
                                    let dx = a.x - *my_x;
                                    let dy = a.y - *my_y;
                                    let dist = (dx * dx + dy * dy).sqrt();
                                    (dist, a.x, a.y)
                                })
                                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
                                .map(|(_, x, y)| (x, y));
                        }
                    }
                }
            }
        });
    }

    fn update_ai(&mut self) {
        if self.last_decision.elapsed() < self.decision_interval {
            return;
        }
        self.last_decision = Instant::now();

        let mut rng = thread_rng();

        // Reset decision interval with some randomness
        self.decision_interval = Duration::from_millis(rng.gen_range(50..200));

        // If we're dead, do nothing
        if !self.my_alive {
            self.thrust = false;
            self.left = false;
            self.right = false;
            self.fire = false;
            return;
        }

        // AI: Target nearest asteroid
        if let Some((target_x, target_y)) = self.target_asteroid {
            let dx = target_x - self.my_x;
            let dy = target_y - self.my_y;
            let target_angle = dy.atan2(dx);

            // Normalize angles
            let mut angle_diff = target_angle - self.my_angle;
            while angle_diff > std::f32::consts::PI {
                angle_diff -= 2.0 * std::f32::consts::PI;
            }
            while angle_diff < -std::f32::consts::PI {
                angle_diff += 2.0 * std::f32::consts::PI;
            }

            // Rotate towards target
            self.left = angle_diff < -0.1;
            self.right = angle_diff > 0.1;

            // Thrust if facing roughly towards target
            self.thrust = angle_diff.abs() < 0.5;

            // Fire if aimed well
            self.fire = angle_diff.abs() < 0.3;
        } else {
            // No target, wander randomly
            self.thrust = rng.gen_bool(0.5);
            self.left = rng.gen_bool(0.3);
            self.right = rng.gen_bool(0.3);
            self.fire = rng.gen_bool(0.1);
        }
    }

    fn send_input(&mut self) {
        // Only send input once joined
        if !self.joined {
            return;
        }
        let input_msg = ClientMessage::Input {
            thrust: self.thrust,
            left: self.left,
            right: self.right,
            fire: self.fire,
        };
        if let Ok(data) = serde_json::to_vec(&input_msg) {
            if self.rudp.send(0, &data).is_ok() {
                self.messages_sent += 1;
            }
        }
    }

    fn print_stats(&mut self) {
        if self.last_stats_print.elapsed() >= Duration::from_secs(1) {
            let status = if self.connected { "connected" } else { "connecting" };
            let alive = if self.my_alive { "alive" } else { "dead" };
            println!(
                "[{}] {} ({}) | sent={} recv={} ({} bytes) | pos=({:.0},{:.0}) angle={:.1} score={}",
                self.name,
                status,
                alive,
                self.messages_sent,
                self.messages_received,
                self.bytes_received,
                self.my_x,
                self.my_y,
                self.my_angle,
                self.my_score,
            );
            self.last_stats_print = Instant::now();
        }
    }

    fn run(&mut self) {
        println!("[{}] Starting RUDP bot (user_id: {})", self.name, self.user_id);
        println!("[{}] Local:  {:?}", self.name, self.rudp.local_addr());
        println!("[{}] Server: {:?}", self.name, self.rudp.server_addr());
        println!("[{}] Sending Join message to server...", self.name);

        // Send Join message to register with server
        self.send_join();

        // Keep sending join until we get a Welcome response
        let mut join_attempts = 0;
        let mut last_join = Instant::now();

        loop {
            self.process_messages();

            // Re-send join if not yet joined
            if !self.joined && last_join.elapsed() >= Duration::from_millis(500) {
                join_attempts += 1;
                if join_attempts <= 20 {
                    self.send_join();
                    last_join = Instant::now();
                }
            }

            if self.joined {
                self.update_ai();
                self.send_input();
            }

            self.print_stats();

            // ~60 updates per second to match game tick rate
            std::thread::sleep(Duration::from_millis(16));
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let bot_name = args.get(1).cloned().unwrap_or_else(|| "AsteroidsBot".to_string());
    let api_host = args.get(2).cloned().unwrap_or_else(|| "127.0.0.1".to_string());
    let api_port: u16 = args.get(3).and_then(|p| p.parse().ok()).unwrap_or(7350);
    let rudp_port: u16 = args.get(4).and_then(|p| p.parse().ok()).unwrap_or(7354);

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║   KaosNet SDK Bot - Asteroids RUDP Transport Demo             ║");
    println!("║   Uses SDK for auth, then RUDP for low-latency game state     ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Step 1: Authenticate via SDK
    println!("[{}] Authenticating via SDK ({}:{})...", bot_name, api_host, api_port);

    let client = KaosClient::new(&api_host, api_port);

    let device_id = format!("bot_{}", bot_name.to_lowercase().replace(' ', "_"));
    let session = client.authenticate_device(&device_id).await?;

    println!("[{}] Authenticated! user_id={}", bot_name, session.user_id);
    println!("[{}] Token: {}...", bot_name, &session.token[..20]);

    // Step 2: Connect via RUDP
    let rudp_addr_str = format!("{}:{}", api_host, rudp_port);
    println!("[{}] Connecting RUDP to {}...", bot_name, rudp_addr_str);

    let server_addr: SocketAddr = match rudp_addr_str.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => addr,
            None => {
                eprintln!("[{}] Could not resolve RUDP address", bot_name);
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("[{}] Invalid RUDP address: {:?}", bot_name, e);
            std::process::exit(1);
        }
    };

    let rudp = match RudpClient::connect(server_addr) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("[{}] RUDP connection failed: {:?}", bot_name, e);
            std::process::exit(1);
        }
    };

    // Step 3: Run the bot
    let mut bot = Bot::new(bot_name, rudp, session.user_id);
    bot.run();

    Ok(())
}
