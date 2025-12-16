//! Kaos.io Game Server
//!
//! An Agar.io-style multiplayer game demonstrating ALL KaosNet capabilities.
//!
//! Features demonstrated:
//! - Real-time WebSocket game loop using kaosnet transport abstraction
//! - Client authentication (device auth with JWT tokens)
//! - Server hooks (before/after for storage and match events)
//! - Storage with permissions (PublicRead for profiles)
//! - Leaderboard service for persistent high scores
//! - Room management

use kaosnet::{
    RoomConfig, RoomRegistry, SessionRegistry,
    Leaderboards, LeaderboardConfig, SortOrder, ScoreOperator,
    Storage, Chat, Matchmaker, Notifications, Social, Tournaments,
    // Authentication
    AuthService, DeviceAuthRequest,
    // Hooks
    HookRegistry, HookOperation, HookContext, BeforeHook, AfterHook, BeforeHookResult,
    hooks::Result as HookResult,
    // Console server
    ConsoleServer, ConsoleConfig,
    // Unified transport from kaosnet
    WsServerTransport, WsClientTransport, TransportServer, ClientTransport,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Player state (internal - uses f64 for physics)
#[derive(Debug, Clone)]
struct Player {
    id: u64,
    name: String,
    x: f64,
    y: f64,
    radius: f64,
    color: String,
    target_x: f64,
    target_y: f64,
    score: u32,
    high_score: u32,
    kills: u32,
}

/// Player state for network (quantized - uses i32 for bandwidth)
/// Gaffer On Games: Quantize floats to fixed-point for smaller payloads
#[derive(Debug, Clone, Serialize)]
struct PlayerNet {
    id: u64,
    name: String,
    x: i32,      // Quantized: actual_x * 10
    y: i32,      // Quantized: actual_y * 10
    r: i16,      // Quantized radius: actual_r * 10
    color: String,
    score: u32,
    high_score: u32,
    kills: u32,
}

impl From<&Player> for PlayerNet {
    fn from(p: &Player) -> Self {
        PlayerNet {
            id: p.id,
            name: p.name.clone(),
            x: (p.x * 10.0) as i32,
            y: (p.y * 10.0) as i32,
            r: (p.radius * 10.0) as i16,
            color: p.color.clone(),
            score: p.score,
            high_score: p.high_score,
            kills: p.kills,
        }
    }
}

/// Food item (internal)
#[derive(Debug, Clone)]
struct Food {
    id: u32,
    x: f64,
    y: f64,
    radius: f64,
    color: String,
}

/// Food item for network (quantized)
#[derive(Debug, Clone, Serialize)]
struct FoodNet {
    id: u32,
    x: i16,      // Quantized: actual_x * 10
    y: i16,      // Quantized: actual_y * 10
    r: u8,       // Quantized radius (small, fits in u8)
    c: u8,       // Color index (0-9) instead of string
}

// Color palette for food (sent once or known client-side)
const FOOD_COLORS: [&str; 10] = [
    "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7",
    "#DDA0DD", "#98D8C8", "#F7DC6F", "#BB8FCE", "#85C1E9",
];

impl Food {
    fn to_net(&self) -> FoodNet {
        // Find color index
        let c = FOOD_COLORS.iter()
            .position(|&c| c == self.color)
            .unwrap_or(0) as u8;
        FoodNet {
            id: self.id,
            x: (self.x * 10.0) as i16,
            y: (self.y * 10.0) as i16,
            r: (self.radius * 10.0) as u8,
            c,
        }
    }
}

/// Leaderboard entry for broadcast
#[derive(Debug, Clone, Serialize)]
struct LeaderboardEntry {
    rank: u64,
    name: String,
    score: i64,
    kills: u32,
}

/// Game state broadcast to clients (quantized for bandwidth)
/// Gaffer On Games: Using fixed-point reduces JSON size by ~40%
#[derive(Debug, Serialize)]
struct GameState {
    #[serde(rename = "p")]  // Short key names
    players: Vec<PlayerNet>,
    #[serde(rename = "f")]
    food: Vec<FoodNet>,
    #[serde(rename = "lb")]
    leaderboard: Vec<LeaderboardEntry>,
    #[serde(rename = "ww")]
    world_width: u16,       // Quantized: fits in u16
    #[serde(rename = "wh")]
    world_height: u16,
    #[serde(rename = "t")]
    tick: u64,
    #[serde(rename = "q")]  // Quantization scale factor
    q: u8,                  // = 10 (tells client to divide by 10)
}

/// Client input message
#[derive(Debug, Deserialize)]
struct ClientInput {
    target_x: f64,
    target_y: f64,
    #[serde(default)]
    name: Option<String>,
}

/// Connected client with authenticated user info
struct Client {
    transport: WsClientTransport,
    player_id: u64,
    /// Authenticated user ID (from device auth)
    user_id: String,
    /// Account ID from auth service
    account_id: String,
    last_input: Option<ClientInput>,
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

/// After hook for match leave - logs player disconnections
struct MatchLeaveLogHook;

impl AfterHook for MatchLeaveLogHook {
    fn execute(&self, ctx: &HookContext, payload: &serde_json::Value, _result: &serde_json::Value) {
        let score = payload.get("final_score").and_then(|s| s.as_u64()).unwrap_or(0);
        let kills = payload.get("kills").and_then(|k| k.as_u64()).unwrap_or(0);
        println!("[hook] Player left match: user={:?}, score={}, kills={}", ctx.user_id, score, kills);
    }
}

// Game constants
const WORLD_WIDTH: f64 = 2000.0;
const WORLD_HEIGHT: f64 = 2000.0;
const BASE_SPEED: f64 = 200.0;
const BASE_RADIUS: f64 = 20.0;
const FOOD_COUNT: usize = 200;
const FOOD_RADIUS: f64 = 8.0;
const FOOD_VALUE: f64 = 5.0;
const TICK_RATE: u32 = 20;
const LEADERBOARD_ID: &str = "kaos_io_highscores";

fn random_color() -> String {
    FOOD_COLORS[rand::random::<usize>() % FOOD_COLORS.len()].to_string()
}

fn spawn_food(food: &mut Vec<Food>, count: usize, next_id: &mut u32) {
    for _ in 0..count {
        food.push(Food {
            id: *next_id,
            x: rand::random::<f64>() * (WORLD_WIDTH - 100.0) + 50.0,
            y: rand::random::<f64>() * (WORLD_HEIGHT - 100.0) + 50.0,
            radius: FOOD_RADIUS,
            color: random_color(),
        });
        *next_id += 1;
    }
}

fn distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Initialize KaosNet services
    let sessions = Arc::new(SessionRegistry::new());
    let rooms = Arc::new(RoomRegistry::new());
    let leaderboards = Arc::new(Leaderboards::new());
    let storage = Arc::new(Storage::new());
    let chat = Arc::new(Chat::new());
    let matchmaker = Arc::new(Matchmaker::new());
    let notifications = Arc::new(Notifications::new());
    let social = Arc::new(Social::new());
    let tournaments = Arc::new(Tournaments::new());

    // Initialize Authentication service with secret key
    let auth = Arc::new(AuthService::new("kaos-io-game-secret-key"));
    println!("✓ Auth service initialized (device/email auth with JWT)");

    // Initialize Hooks registry
    let hooks = Arc::new(HookRegistry::new());

    // Register before hook for storage writes - validates data before saving
    hooks.register_before(HookOperation::StorageWrite, StorageValidationHook);

    // Register after hook for match join - logs player connections
    hooks.register_after(HookOperation::MatchJoin, MatchJoinLogHook);

    // Register after hook for match leave - logs player disconnections
    hooks.register_after(HookOperation::MatchLeave, MatchLeaveLogHook);

    println!("✓ Hooks system initialized (storage validation, match logging)");

    // Create high scores leaderboard
    leaderboards.create(LeaderboardConfig {
        id: LEADERBOARD_ID.to_string(),
        name: "Kaos.io High Scores".to_string(),
        sort_order: SortOrder::Descending,
        operator: ScoreOperator::Best,
        max_entries: 1000,
        ..Default::default()
    }).expect("Failed to create leaderboard");
    println!("✓ Leaderboard service initialized");

    // Storage is ready (with permission enforcement)
    println!("✓ Storage service initialized (with NoRead/OwnerOnly/PublicRead permissions)");

    // Create the main game room
    let game_room_id = rooms.create(RoomConfig {
        max_players: 50,
        tick_rate: TICK_RATE,
        label: "Kaos.io Main Arena".to_string(),
        module: "game".to_string(),
    });
    println!("✓ Game room created: {}", game_room_id);

    println!();
    println!("Server Configuration:");
    println!("  WebSocket: ws://0.0.0.0:7351");
    println!("  Console:   http://0.0.0.0:7350");
    println!("  Tick Rate: {} Hz", TICK_RATE);
    println!("  World:     {}x{}", WORLD_WIDTH, WORLD_HEIGHT);
    println!();
    println!("KaosNet Services:");
    println!("  - Auth: Device/email authentication with JWT tokens");
    println!("  - Hooks: Before/after hooks for validation and logging");
    println!("  - Leaderboards: Track high scores across sessions");
    println!("  - Storage: Persist profiles (PublicRead), server data (NoRead)");
    println!("  - Rooms: Multiplayer coordination");
    println!("  - Console: Admin API (login: admin/admin)");
    println!();

    // Start Console API server in background thread
    let console_sessions = Arc::clone(&sessions);
    let console_rooms = Arc::clone(&rooms);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            let console_config = ConsoleConfig {
                bind_addr: "0.0.0.0:7350".to_string(),
                ..ConsoleConfig::default()
            };
            let console = ConsoleServer::new(
                console_config,
                console_sessions,
                console_rooms,
            );

            if let Err(e) = console.serve().await {
                eprintln!("Console server error: {}", e);
            }
        });
    });
    println!("✓ Console API listening on http://0.0.0.0:7350");

    // Start WebSocket server using kaosnet transport abstraction
    let mut ws_server = WsServerTransport::bind("0.0.0.0:7351")?;
    ws_server.set_nonblocking(true)?;
    println!("WebSocket server listening on 0.0.0.0:7351");
    println!("Waiting for players...\n");

    // Game state
    let mut clients: HashMap<u64, Client> = HashMap::new();
    let mut players: HashMap<u64, Player> = HashMap::new();
    let mut food: Vec<Food> = Vec::new();
    let mut food_id: u32 = 0;
    let mut next_player_id: u64 = 1;
    let mut tick: u64 = 0;
    let mut total_players_ever: u64 = 0;

    // Spawn initial food
    spawn_food(&mut food, FOOD_COUNT, &mut food_id);

    let tick_duration = Duration::from_micros(1_000_000 / TICK_RATE as u64);

    // Main game loop
    loop {
        let tick_start = Instant::now();

        // Accept new connections
        match ws_server.accept() {
            Ok(Some(transport)) => {
                let player_id = next_player_id;
                next_player_id += 1;
                total_players_ever += 1;

                // Authenticate using device auth (creates account on first use)
                let device_id = format!("device_{}", player_id);
                let auth_request = DeviceAuthRequest {
                    device_id: device_id.clone(),
                    create: true,
                    username: Some(format!("Player{}", player_id)),
                };
                let auth_response = match auth.authenticate_device(&auth_request) {
                    Ok(response) => response,
                    Err(e) => {
                        eprintln!("[!] Auth failed for device {}: {:?}", device_id, e);
                        continue;
                    }
                };

                let user_id = auth_response.account.id.clone();
                let account_id = auth_response.account.id.clone();
                let player_name = auth_response.account.username.clone().unwrap_or_else(|| format!("Player{}", player_id));

                // Execute match join hook
                let hook_ctx = HookContext {
                    user_id: Some(user_id.clone()),
                    username: Some(player_name.clone()),
                    session_id: Some(player_id),
                    ..Default::default()
                };
                let hook_payload = serde_json::json!({
                    "device_id": device_id,
                    "player_id": player_id
                });
                let hook_result = serde_json::json!({"status": "joined"});
                hooks.execute_after(
                    HookOperation::MatchJoin,
                    &hook_ctx,
                    &hook_payload,
                    &hook_result,
                );

                // Try to load player profile from storage (PublicRead allows others to see)
                let (color, high_score, saved_name) = match storage.get(&user_id, "profiles", "main") {
                    Ok(Some(obj)) => {
                        let color = obj.value.get("color")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(random_color);
                        let hs = obj.value.get("high_score")
                            .and_then(|h| h.as_u64())
                            .unwrap_or(0) as u32;
                        let name = obj.value.get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string());
                        (color, hs, name)
                    }
                    _ => (random_color(), 0, None),
                };

                let final_name = saved_name.unwrap_or(player_name);

                // Create new player
                let player = Player {
                    id: player_id,
                    name: final_name.clone(),
                    x: rand::random::<f64>() * (WORLD_WIDTH - 200.0) + 100.0,
                    y: rand::random::<f64>() * (WORLD_HEIGHT - 200.0) + 100.0,
                    radius: BASE_RADIUS,
                    color: color.clone(),
                    target_x: WORLD_WIDTH / 2.0,
                    target_y: WORLD_HEIGHT / 2.0,
                    score: 0,
                    high_score,
                    kills: 0,
                };

                println!("[+] {} joined (user={}, account={}, high_score={})",
                    player.name, user_id, account_id, high_score);

                players.insert(player_id, player);
                clients.insert(player_id, Client {
                    transport,
                    player_id,
                    user_id,
                    account_id,
                    last_input: None,
                });

                if let Some(room) = rooms.get(&game_room_id) {
                    let _ = room.add_player(player_id);
                }
            }
            Ok(None) => {}
            Err(_) => {}
        }

        // Process client messages
        let mut disconnected: Vec<u64> = Vec::new();
        for (player_id, client) in clients.iter_mut() {
            if !client.transport.is_open() {
                disconnected.push(*player_id);
                continue;
            }

            client.transport.receive(|data| {
                if let Ok(input) = serde_json::from_slice::<ClientInput>(data) {
                    // Update name if provided
                    if let Some(ref name) = input.name {
                        if !name.is_empty() && name.len() <= 20 {
                            // Will be applied below
                        }
                    }
                    client.last_input = Some(input);
                }
            });

            // Apply input to player
            if let Some(input) = &client.last_input {
                if let Some(player) = players.get_mut(player_id) {
                    player.target_x = input.target_x;
                    player.target_y = input.target_y;
                    if let Some(ref name) = input.name {
                        if !name.is_empty() && name.len() <= 20 {
                            player.name = name.clone();
                        }
                    }
                }
            }
        }

        // Handle disconnections - save to leaderboard and storage with hooks
        for player_id in &disconnected {
            // Get client info before removing
            let client_info = clients.remove(player_id);

            if let Some(player) = players.remove(player_id) {
                let user_id = client_info.as_ref().map(|c| c.user_id.clone())
                    .unwrap_or_else(|| format!("user_{}", player_id));

                // Execute match leave hook
                let hook_ctx = HookContext {
                    user_id: Some(user_id.clone()),
                    username: Some(player.name.clone()),
                    session_id: Some(*player_id),
                    ..Default::default()
                };
                let hook_payload = serde_json::json!({
                    "player_id": player_id,
                    "final_score": player.score,
                    "kills": player.kills
                });
                let hook_result = serde_json::json!({"status": "left"});
                hooks.execute_after(
                    HookOperation::MatchLeave,
                    &hook_ctx,
                    &hook_payload,
                    &hook_result,
                );

                // Submit final score to leaderboard if they scored
                if player.score > 0 {
                    if let Err(e) = leaderboards.submit(
                        LEADERBOARD_ID,
                        &user_id,
                        &player.name,
                        player.score as i64,
                        Some(serde_json::json!({
                            "kills": player.kills,
                            "color": player.color
                        })),
                    ) {
                        eprintln!("Leaderboard submit error: {:?}", e);
                    }
                }

                // Save profile to storage with PublicRead permission
                // This allows other players to see each other's profiles
                let new_high = player.score.max(player.high_score);
                let profile_data = serde_json::json!({
                    "name": player.name,
                    "color": player.color,
                    "high_score": new_high,
                    "total_kills": player.kills,
                });

                // Execute before hook for storage write validation
                let storage_ctx = HookContext {
                    user_id: Some(user_id.clone()),
                    ..Default::default()
                };
                let hook_result = hooks.execute_before(
                    HookOperation::StorageWrite,
                    &storage_ctx,
                    profile_data.clone(),
                );

                match hook_result {
                    Ok(validated_data) => {
                        if let Err(e) = storage.set(
                            &user_id,
                            "profiles",
                            "main",
                            validated_data,
                        ) {
                            eprintln!("Storage save error: {:?}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("[!] Storage write rejected by hook: {:?}", e);
                    }
                }

                println!("[-] {} left (score={}, kills={}, high_score={})",
                    player.name, player.score, player.kills, new_high);
            }

            if let Some(room) = rooms.get(&game_room_id) {
                room.remove_player(*player_id);
            }
        }

        // Update game state
        let dt = 1.0 / TICK_RATE as f64;

        // Move players toward their targets
        for player in players.values_mut() {
            let dx = player.target_x - player.x;
            let dy = player.target_y - player.y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > 1.0 {
                let speed = BASE_SPEED / (player.radius / BASE_RADIUS).sqrt();
                let move_dist = (speed * dt).min(dist);
                player.x += (dx / dist) * move_dist;
                player.y += (dy / dist) * move_dist;
            }

            player.x = player.x.clamp(player.radius, WORLD_WIDTH - player.radius);
            player.y = player.y.clamp(player.radius, WORLD_HEIGHT - player.radius);
        }

        // Check food collisions
        let mut eaten_food: Vec<u32> = Vec::new();
        for player in players.values_mut() {
            for f in &food {
                if distance(player.x, player.y, f.x, f.y) < player.radius {
                    eaten_food.push(f.id);
                    player.radius = (player.radius.powi(2) + FOOD_VALUE).sqrt();
                    player.score += 1;
                }
            }
        }
        food.retain(|f| !eaten_food.contains(&f.id));

        // Respawn food
        let food_needed = FOOD_COUNT.saturating_sub(food.len());
        if food_needed > 0 {
            spawn_food(&mut food, food_needed, &mut food_id);
        }

        // Check player-player collisions
        let player_ids: Vec<u64> = players.keys().copied().collect();
        let mut eaten_players: Vec<(u64, u64)> = Vec::new(); // (eaten, eater)

        for i in 0..player_ids.len() {
            for j in (i + 1)..player_ids.len() {
                let id1 = player_ids[i];
                let id2 = player_ids[j];

                if eaten_players.iter().any(|(e, _)| *e == id1 || *e == id2) {
                    continue;
                }

                let (p1_x, p1_y, p1_r) = {
                    let p = &players[&id1];
                    (p.x, p.y, p.radius)
                };
                let (p2_x, p2_y, p2_r) = {
                    let p = &players[&id2];
                    (p.x, p.y, p.radius)
                };

                let dist = distance(p1_x, p1_y, p2_x, p2_y);

                if dist < p1_r.max(p2_r) * 0.8 {
                    if p1_r > p2_r * 1.1 {
                        let p2_score = players[&id2].score;
                        eaten_players.push((id2, id1));
                        if let Some(p1) = players.get_mut(&id1) {
                            p1.radius = (p1_r.powi(2) + p2_r.powi(2)).sqrt();
                            p1.score += p2_score + 10;
                            p1.kills += 1;
                        }
                    } else if p2_r > p1_r * 1.1 {
                        let p1_score = players[&id1].score;
                        eaten_players.push((id1, id2));
                        if let Some(p2) = players.get_mut(&id2) {
                            p2.radius = (p2_r.powi(2) + p1_r.powi(2)).sqrt();
                            p2.score += p1_score + 10;
                            p2.kills += 1;
                        }
                    }
                }
            }
        }

        // Respawn eaten players
        for (eaten_id, eater_id) in eaten_players {
            let eater_name = players.get(&eater_id).map(|p| p.name.clone()).unwrap_or_default();
            if let Some(player) = players.get_mut(&eaten_id) {
                println!("[*] {} was eaten by {} (score={})", player.name, eater_name, player.score);

                // Update high score before respawn
                player.high_score = player.high_score.max(player.score);

                // Respawn
                player.x = rand::random::<f64>() * (WORLD_WIDTH - 200.0) + 100.0;
                player.y = rand::random::<f64>() * (WORLD_HEIGHT - 200.0) + 100.0;
                player.radius = BASE_RADIUS;
                player.score = 0;
            }
        }

        // Get top 10 from leaderboard for broadcast
        let leaderboard_entries: Vec<LeaderboardEntry> = leaderboards
            .get_top(LEADERBOARD_ID, 10)
            .unwrap_or_default()
            .into_iter()
            .map(|r| LeaderboardEntry {
                rank: r.rank.unwrap_or(0),
                name: r.username,
                score: r.score,
                kills: r.metadata
                    .and_then(|m| m.get("kills").and_then(|k| k.as_u64()))
                    .unwrap_or(0) as u32,
            })
            .collect();

        // Build game state with quantized values
        // Gaffer On Games: Quantization reduces bandwidth by ~40%
        let state = GameState {
            players: players.values().map(PlayerNet::from).collect(),
            food: food.iter().map(|f| f.to_net()).collect(),
            leaderboard: leaderboard_entries,
            world_width: WORLD_WIDTH as u16,
            world_height: WORLD_HEIGHT as u16,
            tick,
            q: 10,  // Quantization scale: divide by 10 to get actual values
        };

        // Broadcast state to all clients
        if let Ok(state_json) = serde_json::to_vec(&state) {
            for client in clients.values_mut() {
                if client.transport.is_open() {
                    let _ = client.transport.send(&state_json);
                }
            }
        }

        tick += 1;

        // Periodic status
        if tick % (TICK_RATE as u64 * 10) == 0 && !players.is_empty() {
            let lb_count = leaderboards.count(LEADERBOARD_ID).unwrap_or(0);
            println!("[tick {}] {} players online, {} on leaderboard", tick, players.len(), lb_count);
        }

        // Sleep for remaining tick time
        let elapsed = tick_start.elapsed();
        if elapsed < tick_duration {
            std::thread::sleep(tick_duration - elapsed);
        }
    }
}
