//! Kaos Asteroids Server
//!
//! A multiplayer Asteroids game demonstrating kaos-rudp transport.
//!
//! Features:
//! - 60Hz game loop using kaos_rudp::RudpServer (MessageRingBuffer per client)
//! - Player ships with thrust, rotation, shooting
//! - Asteroids that split when destroyed
//! - Leaderboard integration for high scores

// Use kaos_rudp directly for proper per-client reliability
use kaos_rudp::RudpServer;
use kaosnet::{
    Leaderboards, LeaderboardConfig, SortOrder, ScoreOperator, ResetSchedule,
    Storage, RoomRegistry, RoomConfig,
    // Authentication
    AuthService, DeviceAuthRequest,
    // Hooks
    HookRegistry, HookOperation, HookContext, BeforeHook, AfterHook, BeforeHookResult,
    hooks::Result as HookResult,
};
use std::net::SocketAddr;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::time::{Duration, Instant};

// Game constants
const TICK_RATE: u64 = 60;
const TICK_DURATION: Duration = Duration::from_micros(1_000_000 / TICK_RATE);
const WORLD_WIDTH: f64 = 100.0;
const WORLD_HEIGHT: f64 = 50.0;
const SHIP_RADIUS: f64 = 1.5;
const BULLET_RADIUS: f64 = 0.3;
const BULLET_SPEED: f64 = 50.0;
const BULLET_LIFETIME: f64 = 2.0;
const SHIP_THRUST: f64 = 20.0;
const SHIP_ROTATION_SPEED: f64 = 4.0;
const SHIP_DRAG: f64 = 0.98;
const ASTEROID_SPEED: f64 = 8.0;
const INITIAL_ASTEROIDS: usize = 5;
const LEADERBOARD_ID: &str = "asteroids_highscore";

// Protocol messages
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    size: u8, // 3=large, 2=medium, 1=small
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

// Internal game objects
struct Ship {
    id: u64,
    name: String,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    angle: f64,
    score: i64,
    alive: bool,
    respawn_timer: f64,
    fire_cooldown: f64,
}

struct Asteroid {
    id: u64,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    size: u8,
}

struct Bullet {
    owner_id: u64,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    lifetime: f64,
}

struct Client {
    addr: SocketAddr,
    /// Player ID (stored for future match handler)
    _player_id: u64,
    #[allow(dead_code)]
    account_id: String,
    input: PlayerInput,
}

// ==================== Hook Implementations ====================

/// Before hook for storage writes - validates data before saving
struct StorageValidationHook;

impl BeforeHook for StorageValidationHook {
    fn execute(&self, _ctx: &HookContext, payload: serde_json::Value) -> HookResult<BeforeHookResult> {
        // Prevent negative high scores from being saved
        if let Some(score) = payload.get("high_score").and_then(|s| s.as_i64()) {
            if score < 0 {
                return Ok(BeforeHookResult::Reject("Negative scores not allowed".to_string()));
            }
        }
        Ok(BeforeHookResult::Continue(payload))
    }
}

/// After hook for match join - logs player connections
struct MatchJoinLogHook;

impl AfterHook for MatchJoinLogHook {
    fn execute(&self, ctx: &HookContext, _payload: &serde_json::Value, _result: &serde_json::Value) {
        println!("[hook] Player joined match: user={:?}", ctx.user_id);
    }
}

/// After hook for match leave - logs player disconnections and final scores
struct MatchLeaveLogHook;

impl AfterHook for MatchLeaveLogHook {
    fn execute(&self, ctx: &HookContext, payload: &serde_json::Value, _result: &serde_json::Value) {
        let score = payload.get("final_score").and_then(|s| s.as_i64()).unwrap_or(0);
        let asteroids = payload.get("asteroids_destroyed").and_then(|a| a.as_u64()).unwrap_or(0);
        println!("[hook] Player left match: user={:?}, score={}, asteroids_destroyed={}", ctx.user_id, score, asteroids);
    }
}

#[derive(Default)]
struct PlayerInput {
    thrust: bool,
    left: bool,
    right: bool,
    fire: bool,
}

fn main() -> std::io::Result<()> {
    println!("\n  ╔════════════════════════════════════════════════════╗");
    println!("    ║    KAOS ASTEROIDS - RUDP Multiplayer Demo          ║");
    println!("    ║    Powered by KaosNet Services                     ║");
    println!("    ╚════════════════════════════════════════════════════╝\n");

    // Initialize KaosNet services
    let rooms = Arc::new(RoomRegistry::new());
    let leaderboards = Arc::new(Leaderboards::new());
    let storage = Arc::new(Storage::new());

    // Initialize Authentication service
    let auth = Arc::new(AuthService::new("kaos-asteroids-secret-key"));
    println!("✓ Authentication service initialized");

    // Initialize Hooks registry
    let hooks = Arc::new(HookRegistry::new());

    // Register before hook for storage writes - validates data before saving
    hooks.register_before(HookOperation::StorageWrite, StorageValidationHook);

    // Register after hook for match join - logs player connections
    hooks.register_after(HookOperation::MatchJoin, MatchJoinLogHook);

    // Register after hook for match leave - logs player disconnections
    hooks.register_after(HookOperation::MatchLeave, MatchLeaveLogHook);
    println!("✓ Hooks registered (storage validation, match join/leave logging)");

    // Create high scores leaderboard
    leaderboards.create(LeaderboardConfig {
        id: LEADERBOARD_ID.to_string(),
        name: "Asteroids High Scores".to_string(),
        sort_order: SortOrder::Descending,
        operator: ScoreOperator::Best,
        reset_schedule: ResetSchedule::Never,
        max_entries: 100,
        metadata_schema: None,
    }).ok();
    println!("✓ Leaderboard service initialized");
    println!("✓ Storage service initialized");

    // Create the main game room
    let game_room_id = rooms.create(RoomConfig {
        max_players: 20,
        tick_rate: TICK_RATE as u32,
        label: "Asteroids Arena".to_string(),
        module: "asteroids".to_string(),
    });
    println!("✓ Game room created: {}", game_room_id);

    // Start RUDP server using kaos_rudp directly (with MessageRingBuffer per client)
    let mut server = RudpServer::bind("0.0.0.0:7352", 256)?;
    println!("✓ RUDP server listening on 0.0.0.0:7352");

    println!();
    println!("Server Configuration:");
    println!("  RUDP Game: udp://0.0.0.0:7352");
    println!("  Tick Rate: {}Hz", TICK_RATE);
    println!("  World: {}x{}", WORLD_WIDTH, WORLD_HEIGHT);
    println!();
    println!("KaosNet Services:");
    println!("  - Auth: Device authentication with JWT tokens");
    println!("  - Leaderboards: Track high scores across sessions");
    println!("  - Storage: Persist player profiles and stats");
    println!("  - Rooms: Multiplayer coordination");
    println!();
    println!("Waiting for players... (use asteroids-client to connect)\n");

    // Game state
    let mut clients: HashMap<u64, Client> = HashMap::new();
    let mut addr_to_player: HashMap<SocketAddr, u64> = HashMap::new();
    let mut ships: HashMap<u64, Ship> = HashMap::new();
    let mut asteroids: Vec<Asteroid> = Vec::new();
    let mut bullets: Vec<Bullet> = Vec::new();
    let mut next_player_id: u64 = 1;
    let mut next_asteroid_id: u64 = 1;
    let mut tick: u64 = 0;

    // Pending join messages (addr -> name)
    let mut pending_joins: HashMap<SocketAddr, String> = HashMap::new();

    // Spawn initial asteroids
    for _ in 0..INITIAL_ASTEROIDS {
        asteroids.push(spawn_asteroid(&mut next_asteroid_id, 3));
    }

    let mut last_tick = Instant::now();
    let mut last_status = Instant::now();

    loop {
        let now = Instant::now();

        // Poll server (receive packets, manage clients)
        server.poll();

        // Accept new connections
        while let Some(addr) = server.accept() {
            // New client connected, wait for Join message
            pending_joins.insert(addr, format!("Player{}", next_player_id));
        }

        // Process pending joins - receive Join messages
        let pending_addrs: Vec<SocketAddr> = pending_joins.keys().copied().collect();
        for addr in pending_addrs {
            let mut name = pending_joins.get(&addr).cloned().unwrap_or_default();
            let mut got_join = false;

            server.receive(&addr, |data| {
                if let Ok(msg) = serde_json::from_slice::<ClientMessage>(data) {
                    if let ClientMessage::Join { name: n } = msg {
                        name = n;
                        got_join = true;
                    }
                }
            });

            if got_join {
                pending_joins.remove(&addr);
                let player_id = next_player_id;
                next_player_id += 1;

                // Authenticate player using device ID (address as unique identifier)
                let device_id = format!("rudp_{}", addr);
                let auth_request = DeviceAuthRequest {
                    device_id: device_id.clone(),
                    create: true,
                    username: Some(name.clone()),
                };

                let (user_id, account_id, player_name) = match auth.authenticate_device(&auth_request) {
                    Ok(auth_response) => {
                        let uid = auth_response.account.id.clone();
                        let acc_id = auth_response.account.id.clone();
                        let pname = auth_response.account.username.clone().unwrap_or(name.clone());
                        println!("[auth] Device authenticated: {} -> account {}", device_id, acc_id);
                        (uid, acc_id, pname)
                    }
                    Err(e) => {
                        println!("[auth] Auth failed, using fallback: {}", e);
                        let fallback = format!("ast_{}", player_id);
                        (fallback.clone(), fallback, name.clone())
                    }
                };

                // Try to load player profile from storage
                let high_score = match storage.get(&user_id, "profiles", "main") {
                    Ok(Some(obj)) => {
                        obj.value.get("high_score")
                            .and_then(|h| h.as_i64())
                            .unwrap_or(0)
                    }
                    _ => 0,
                };

                println!("[+] {} joined (id={}, account={}, high_score={}) from {}", player_name, player_id, account_id, high_score, addr);

                // Execute match join hook
                let ctx = HookContext {
                    user_id: Some(user_id.clone()),
                    username: Some(player_name.clone()),
                    session_id: Some(player_id),
                    client_ip: Some(addr.to_string()),
                    vars: std::collections::HashMap::new(),
                };
                let _ = hooks.execute_after(
                    HookOperation::MatchJoin,
                    &ctx,
                    &serde_json::json!({ "player_name": player_name, "high_score": high_score }),
                    &serde_json::json!({}),
                );

                // Send welcome
                let welcome = ServerMessage::Welcome { player_id };
                let _ = server.send(&addr, &serde_json::to_vec(&welcome).unwrap());

                // Create ship
                ships.insert(player_id, Ship {
                    id: player_id,
                    name: player_name.clone(),
                    x: WORLD_WIDTH / 2.0 + (player_id as f64 * 5.0) % WORLD_WIDTH,
                    y: WORLD_HEIGHT / 2.0,
                    vx: 0.0,
                    vy: 0.0,
                    angle: 0.0,
                    score: 0,
                    alive: true,
                    respawn_timer: 0.0,
                    fire_cooldown: 0.0,
                });

                // Add player to game room
                if let Some(room) = rooms.get(&game_room_id) {
                    let _ = room.add_player(player_id);
                }

                addr_to_player.insert(addr, player_id);
                clients.insert(player_id, Client {
                    addr,
                    _player_id: player_id,
                    account_id,
                    input: PlayerInput::default(),
                });
            }
        }

        // Process client inputs
        let mut disconnected = Vec::new();
        for (id, client) in &mut clients {
            // Check if client still connected
            if !server.has_client(&client.addr) {
                disconnected.push(*id);
                continue;
            }

            server.receive(&client.addr, |data| {
                if let Ok(msg) = serde_json::from_slice::<ClientMessage>(data) {
                    match msg {
                        ClientMessage::Input { thrust, left, right, fire } => {
                            client.input = PlayerInput { thrust, left, right, fire };
                        }
                        ClientMessage::Leave => {
                            // Will be handled in disconnected
                        }
                        _ => {}
                    }
                }
            });
        }

        // Remove disconnected clients
        for id in disconnected {
            if let Some(client) = clients.remove(&id) {
                addr_to_player.remove(&client.addr);
                server.disconnect(&client.addr);
                if let Some(ship) = ships.remove(&id) {
                    let user_id = client.account_id.clone();
                    println!("[-] {} left (score: {}, account: {})", ship.name, ship.score, user_id);

                    // Execute match leave hook
                    let leave_ctx = HookContext {
                        user_id: Some(user_id.clone()),
                        username: Some(ship.name.clone()),
                        session_id: Some(id),
                        client_ip: Some(client.addr.to_string()),
                        vars: std::collections::HashMap::new(),
                    };
                    let _ = hooks.execute_after(
                        HookOperation::MatchLeave,
                        &leave_ctx,
                        &serde_json::json!({
                            "final_score": ship.score,
                            "asteroids_destroyed": 0,
                            "player_name": ship.name
                        }),
                        &serde_json::json!({}),
                    );

                    // Submit to leaderboard
                    let _ = leaderboards.submit(
                        LEADERBOARD_ID,
                        &user_id,
                        &ship.name,
                        ship.score,
                        Some(serde_json::json!({
                            "deaths": 0
                        })),
                    );

                    // Save profile to storage
                    let _ = storage.set(
                        &user_id,
                        "profiles",
                        "main",
                        serde_json::json!({
                            "name": ship.name,
                            "high_score": ship.score,
                            "last_played": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0),
                        }),
                    );

                    // Remove from game room
                    if let Some(room) = rooms.get(&game_room_id) {
                        room.remove_player(id);
                    }
                }
            }
        }

        // Game tick
        if now.duration_since(last_tick) >= TICK_DURATION {
            let dt = 1.0 / TICK_RATE as f64;
            tick += 1;

            // Update ships
            for (id, ship) in &mut ships {
                if !ship.alive {
                    ship.respawn_timer -= dt;
                    if ship.respawn_timer <= 0.0 {
                        ship.alive = true;
                        ship.x = WORLD_WIDTH / 2.0;
                        ship.y = WORLD_HEIGHT / 2.0;
                        ship.vx = 0.0;
                        ship.vy = 0.0;
                    }
                    continue;
                }

                if let Some(client) = clients.get(id) {
                    // Rotation
                    if client.input.left {
                        ship.angle -= SHIP_ROTATION_SPEED * dt;
                    }
                    if client.input.right {
                        ship.angle += SHIP_ROTATION_SPEED * dt;
                    }

                    // Thrust
                    if client.input.thrust {
                        ship.vx += ship.angle.cos() * SHIP_THRUST * dt;
                        ship.vy += ship.angle.sin() * SHIP_THRUST * dt;
                    }

                    // Fire
                    ship.fire_cooldown -= dt;
                    if client.input.fire && ship.fire_cooldown <= 0.0 {
                        bullets.push(Bullet {
                            owner_id: *id,
                            x: ship.x + ship.angle.cos() * SHIP_RADIUS,
                            y: ship.y + ship.angle.sin() * SHIP_RADIUS,
                            vx: ship.angle.cos() * BULLET_SPEED,
                            vy: ship.angle.sin() * BULLET_SPEED,
                            lifetime: BULLET_LIFETIME,
                        });
                        ship.fire_cooldown = 0.2; // Fire rate limit
                    }
                }

                // Apply drag
                ship.vx *= SHIP_DRAG;
                ship.vy *= SHIP_DRAG;

                // Move
                ship.x += ship.vx * dt;
                ship.y += ship.vy * dt;

                // Wrap around
                ship.x = (ship.x + WORLD_WIDTH) % WORLD_WIDTH;
                ship.y = (ship.y + WORLD_HEIGHT) % WORLD_HEIGHT;
            }

            // Update bullets
            bullets.retain_mut(|bullet| {
                bullet.x += bullet.vx * dt;
                bullet.y += bullet.vy * dt;
                bullet.lifetime -= dt;

                // Wrap around
                bullet.x = (bullet.x + WORLD_WIDTH) % WORLD_WIDTH;
                bullet.y = (bullet.y + WORLD_HEIGHT) % WORLD_HEIGHT;

                bullet.lifetime > 0.0
            });

            // Update asteroids
            for asteroid in &mut asteroids {
                asteroid.x += asteroid.vx * dt;
                asteroid.y += asteroid.vy * dt;

                // Wrap around
                asteroid.x = (asteroid.x + WORLD_WIDTH) % WORLD_WIDTH;
                asteroid.y = (asteroid.y + WORLD_HEIGHT) % WORLD_HEIGHT;
            }

            // Collision: bullets vs asteroids
            let mut new_asteroids = Vec::new();
            let mut bullets_to_remove = Vec::new();
            let mut asteroids_to_remove = Vec::new();

            for (bi, bullet) in bullets.iter().enumerate() {
                for (ai, asteroid) in asteroids.iter().enumerate() {
                    let radius = match asteroid.size {
                        3 => 3.0,
                        2 => 2.0,
                        _ => 1.0,
                    };

                    let dx = bullet.x - asteroid.x;
                    let dy = bullet.y - asteroid.y;
                    if dx * dx + dy * dy < (BULLET_RADIUS + radius).powi(2) {
                        bullets_to_remove.push(bi);
                        asteroids_to_remove.push(ai);

                        // Award points
                        let points = match asteroid.size {
                            3 => 20,
                            2 => 50,
                            _ => 100,
                        };
                        if let Some(ship) = ships.get_mut(&bullet.owner_id) {
                            ship.score += points;
                        }

                        // Split asteroid
                        if asteroid.size > 1 {
                            for _ in 0..2 {
                                let mut new_ast = spawn_asteroid(&mut next_asteroid_id, asteroid.size - 1);
                                new_ast.x = asteroid.x;
                                new_ast.y = asteroid.y;
                                new_asteroids.push(new_ast);
                            }
                        }
                        break;
                    }
                }
            }

            // Remove destroyed
            bullets_to_remove.sort_by(|a, b| b.cmp(a));
            for i in bullets_to_remove {
                if i < bullets.len() {
                    bullets.remove(i);
                }
            }
            asteroids_to_remove.sort_by(|a, b| b.cmp(a));
            asteroids_to_remove.dedup();
            for i in asteroids_to_remove {
                if i < asteroids.len() {
                    asteroids.remove(i);
                }
            }
            asteroids.extend(new_asteroids);

            // Spawn new asteroids if too few
            while asteroids.len() < INITIAL_ASTEROIDS {
                asteroids.push(spawn_asteroid(&mut next_asteroid_id, 3));
            }

            // Collision: ships vs asteroids
            for ship in ships.values_mut() {
                if !ship.alive {
                    continue;
                }

                for asteroid in &asteroids {
                    let radius = match asteroid.size {
                        3 => 3.0,
                        2 => 2.0,
                        _ => 1.0,
                    };

                    let dx = ship.x - asteroid.x;
                    let dy = ship.y - asteroid.y;
                    if dx * dx + dy * dy < (SHIP_RADIUS + radius).powi(2) {
                        ship.alive = false;
                        ship.respawn_timer = 3.0;
                        println!("[!] {} was destroyed (score: {})", ship.name, ship.score);
                        break;
                    }
                }
            }

            // Build game state
            let state = GameState {
                tick,
                ships: ships.values().map(|s| ShipState {
                    id: s.id,
                    name: s.name.clone(),
                    x: s.x,
                    y: s.y,
                    angle: s.angle,
                    score: s.score,
                    alive: s.alive,
                }).collect(),
                asteroids: asteroids.iter().map(|a| AsteroidState {
                    id: a.id,
                    x: a.x,
                    y: a.y,
                    size: a.size,
                }).collect(),
                bullets: bullets.iter().map(|b| BulletState {
                    x: b.x,
                    y: b.y,
                }).collect(),
                leaderboard: leaderboards.get_top(LEADERBOARD_ID, 5)
                    .unwrap_or_default()
                    .iter()
                    .map(|r| LeaderboardEntry {
                        name: r.username.clone(),
                        score: r.score,
                    })
                    .collect(),
            };

            // Broadcast to all clients using kaos_rudp::RudpServer (proper per-client sequences)
            let msg = ServerMessage::GameState { state };
            let data = serde_json::to_vec(&msg).unwrap();
            for client in clients.values() {
                let _ = server.send(&client.addr, &data);
            }

            last_tick = now;
        }

        // Status update
        if now.duration_since(last_status) >= Duration::from_secs(10) {
            if !clients.is_empty() {
                println!("[i] {} players, {} asteroids, {} bullets",
                    clients.len(), asteroids.len(), bullets.len());
            }
            last_status = now;
        }

        // Sleep to not burn CPU
        std::thread::sleep(Duration::from_micros(100));
    }
}

fn spawn_asteroid(next_id: &mut u64, size: u8) -> Asteroid {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let id = *next_id;
    *next_id += 1;

    // Spawn at edges
    let (x, y) = if rng.gen_bool(0.5) {
        (rng.gen_range(0.0..WORLD_WIDTH), if rng.gen_bool(0.5) { 0.0 } else { WORLD_HEIGHT })
    } else {
        (if rng.gen_bool(0.5) { 0.0 } else { WORLD_WIDTH }, rng.gen_range(0.0..WORLD_HEIGHT))
    };

    let angle = rng.gen_range(0.0..2.0 * PI);
    let speed = ASTEROID_SPEED * (4 - size) as f64 / 3.0;

    Asteroid {
        id,
        x,
        y,
        vx: angle.cos() * speed,
        vy: angle.sin() * speed,
        size,
    }
}
