//! Kaos Asteroids Server
//!
//! A multiplayer Asteroids game demonstrating kaos-rudp transport.
//!
//! Features:
//! - **Lua scripting** - ALL game logic in Lua (scripts/game.lua)
//! - 60Hz game loop using kaos_rudp::RudpServer (MessageRingBuffer per client)
//! - Player ships with thrust, rotation, shooting
//! - Asteroids that split when destroyed
//! - Leaderboard integration for high scores (accessible from Lua)

// Use kaos_rudp directly for proper per-client reliability
use kaos_rudp::RudpServer;
use kaosnet::{
    Leaderboards, LeaderboardConfig, SortOrder, ScoreOperator, ResetSchedule,
    Storage, SessionRegistry, RoomRegistry, RoomConfig,
    Chat, Matchmaker, Notifications, Social, Tournaments,
    ConsoleServer, ConsoleConfig,
    // Authentication
    AuthService, DeviceAuthRequest,
    // Hooks
    HookRegistry, HookOperation, HookContext, BeforeHook, AfterHook, BeforeHookResult,
    hooks::Result as HookResult,
    // Hooked Services (with metrics integration)
    HookedStorage, HookedLeaderboards, HookedSocial,
    // Metrics
    Metrics,
    // Lua runtime and match handler
    LuaConfig, LuaRuntime, LuaMatchHandler, LuaServices,
    // Match system
    MatchHandlerRegistry, MatchRegistry, MatchPresence, MatchMessage,
};
use std::net::SocketAddr;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Game constants (also in Lua, kept for server reference)
const TICK_RATE: u64 = 60;
const TICK_DURATION: Duration = Duration::from_micros(1_000_000 / TICK_RATE);
const LEADERBOARD_ID: &str = "asteroids_highscore";
const LUA_MODULE_NAME: &str = "kaos_asteroids";

// Protocol messages (for RUDP client communication)
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
    GameState { state: serde_json::Value },  // State comes from Lua as JSON
}

// Client connection tracking (game state is managed by Lua)
#[allow(dead_code)]
struct Client {
    addr: SocketAddr,
    session_id: u64, // Reserved for session management
    user_id: String,
    username: String,
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

fn main() -> std::io::Result<()> {
    println!("\n  ╔════════════════════════════════════════════════════╗");
    println!("    ║    KAOS ASTEROIDS - RUDP Multiplayer Demo          ║");
    println!("    ║    Powered by KaosNet Services                     ║");
    println!("    ╚════════════════════════════════════════════════════╝\n");

    // Initialize Prometheus metrics
    let metrics = Arc::new(Metrics::new());
    println!("✓ Metrics initialized");

    // Initialize all KaosNet services
    let sessions = Arc::new(SessionRegistry::new());
    let rooms = Arc::new(RoomRegistry::new());
    let leaderboards = Arc::new(Leaderboards::new());

    // Storage: Use PostgreSQL if DATABASE_URL is set, otherwise in-memory
    let storage = if let Ok(db_url) = std::env::var("DATABASE_URL") {
        println!("Connecting to PostgreSQL: {}...", db_url.split('@').last().unwrap_or("***"));
        #[cfg(feature = "postgres")]
        {
            use kaosnet::storage::PostgresSyncBackend;
            match PostgresSyncBackend::connect(&db_url) {
                Ok(backend) => {
                    if let Err(e) = backend.migrate() {
                        eprintln!("Warning: Migration failed: {}", e);
                    }
                    println!("✓ PostgreSQL storage connected");
                    Arc::new(Storage::with_backend(Arc::new(backend)))
                }
                Err(e) => {
                    eprintln!("Warning: PostgreSQL connection failed: {}, using in-memory", e);
                    Arc::new(Storage::new())
                }
            }
        }
        #[cfg(not(feature = "postgres"))]
        {
            eprintln!("Warning: DATABASE_URL set but postgres feature not enabled, using in-memory");
            Arc::new(Storage::new())
        }
    } else {
        println!("✓ In-memory storage (set DATABASE_URL for PostgreSQL)");
        Arc::new(Storage::new())
    };

    let chat = Arc::new(Chat::new());
    let matchmaker = Arc::new(Matchmaker::new());
    let notifications = Arc::new(Notifications::new());
    let social = Arc::new(Social::new());
    let tournaments = Arc::new(Tournaments::new());

    // Register matchmaker callback to send notifications when matches are found
    {
        let notifs = Arc::clone(&notifications);
        matchmaker.on_match(move |mm_match| {
            for player in &mm_match.players {
                let _ = notifs.notify_match_found(&player.user_id, &mm_match.id, &mm_match.queue);
            }
        });
    }

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

    // Create Hooked Services with metrics integration
    let hooked_storage = Arc::new(
        HookedStorage::new(Arc::clone(&storage), Arc::clone(&hooks))
            .with_metrics(Arc::clone(&metrics))
    );
    let hooked_leaderboards = Arc::new(
        HookedLeaderboards::new(Arc::clone(&leaderboards), Arc::clone(&hooks))
            .with_metrics(Arc::clone(&metrics))
    );
    let hooked_social = Arc::new(
        HookedSocial::new(Arc::clone(&social), Arc::clone(&hooks))
            .with_notifications(Arc::clone(&notifications))
            .with_metrics(Arc::clone(&metrics))
    );
    println!("✓ Hooked services initialized (storage, leaderboards, social with metrics)");

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

    // ==================== Lua Runtime Setup ====================
    // Create LuaServices with HOOKED services for Lua script access
    // This ensures Lua scripts get metrics, hooks, and notifications!
    let lua_services = Arc::new(LuaServices::with_hooked_services(
        Arc::clone(&hooked_storage),
        Arc::clone(&hooked_leaderboards),
        Arc::clone(&hooked_social),
    ));

    // Create Lua runtime with services (use env var in Docker, fallback to cargo dir)
    let script_path = std::env::var("KAOS_LUA_SCRIPTS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts"));
    let lua_config = LuaConfig {
        pool_size: 2,
        script_path: script_path.to_string_lossy().to_string(),
    };
    let lua_runtime = LuaRuntime::with_services(lua_config, Arc::clone(&lua_services))
        .expect("Failed to create Lua runtime");
    let lua_runtime = Arc::new(lua_runtime);
    println!("✓ Lua runtime initialized (scripts loaded from {:?})", script_path);

    // Create Lua match handler
    let lua_handler = LuaMatchHandler::new(Arc::clone(&lua_runtime), LUA_MODULE_NAME);

    // Register handler and create match registry
    let match_handlers = Arc::new(MatchHandlerRegistry::new());
    match_handlers.register(LUA_MODULE_NAME, lua_handler);
    let match_registry = Arc::new(MatchRegistry::new(match_handlers));

    // Create the main game match (Lua handles all game logic)
    let game_match_id = match_registry
        .create(LUA_MODULE_NAME, serde_json::json!({}), None)
        .expect("Failed to create game match");
    println!("✓ Lua game match created: {} (module={})", game_match_id, LUA_MODULE_NAME);

    // Create the main game room
    let game_room_id = rooms.create(RoomConfig {
        max_players: 20,
        tick_rate: TICK_RATE as u32,
        label: "Asteroids Arena".to_string(),
        module: "asteroids".to_string(),
    });
    println!("✓ Game room created: {}", game_room_id);

    // Start Console API server in background thread with all services + metrics
    let console_sessions = Arc::clone(&sessions);
    let console_rooms = Arc::clone(&rooms);
    let console_storage = Arc::clone(&storage);
    let console_leaderboards = Arc::clone(&leaderboards);
    let console_tournaments = Arc::clone(&tournaments);
    let console_social = Arc::clone(&social);
    let console_chat = Arc::clone(&chat);
    let console_matchmaker = Arc::clone(&matchmaker);
    let console_notifications = Arc::clone(&notifications);
    let console_metrics = Arc::clone(&metrics);

    // Get bind addresses from env (for Docker compatibility)
    let console_bind = std::env::var("KAOS_CONSOLE_BIND")
        .unwrap_or_else(|_| "127.0.0.1:7350".to_string());
    let rudp_bind = std::env::var("KAOS_RUDP_BIND")
        .unwrap_or_else(|_| "0.0.0.0:7354".to_string());

    let console_bind_clone = console_bind.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            let mut config = ConsoleConfig::default();
            config.bind_addr = console_bind_clone;

            // Use builder to wire all services to console (including metrics)
            let console = ConsoleServer::builder(config, console_sessions, console_rooms)
                .storage(console_storage)
                .leaderboards(console_leaderboards)
                .tournaments(console_tournaments)
                .social(console_social)
                .chat(console_chat)
                .matchmaker(console_matchmaker)
                .notifications(console_notifications)
                .metrics(console_metrics)
                .build();

            if let Err(e) = console.serve().await {
                eprintln!("Console server error: {}", e);
            }
        });
    });
    println!("✓ Console API listening on http://{} (all services + metrics wired)", console_bind);

    // Start RUDP server using kaos_rudp directly (with MessageRingBuffer per client)
    let mut server = RudpServer::bind(&rudp_bind, 256)?;
    println!("✓ RUDP server listening on {}", rudp_bind);

    println!();
    println!("Server Configuration:");
    println!("  RUDP Game: udp://{}", rudp_bind);
    println!("  Console:   http://{}", console_bind);
    println!("  Tick Rate: {}Hz", TICK_RATE);
    println!("  Lua Script: scripts/game.lua");
    println!();
    println!("KaosNet Features Demonstrated:");
    println!("  - Lua Scripting: ALL game logic in scripts/game.lua");
    println!("  - RUDP Transport: Reliable UDP with per-client sequencing");
    println!("  - Leaderboards: Track high scores (accessible from Lua)");
    println!("  - Storage: Persist player profiles");
    println!("  - Match Handler: Lua-based match lifecycle");
    println!("  - Console: Admin API (login: admin/admin)");
    println!();
    println!("Waiting for players... (use asteroids-client to connect)\n");

    // Client tracking (game state is managed by Lua)
    let mut clients: HashMap<u64, Client> = HashMap::new();
    let mut addr_to_session: HashMap<SocketAddr, u64> = HashMap::new();
    let mut next_session_id: u64 = 1;
    let mut tick: u64 = 0;

    // Pending join messages (addr -> name)
    let mut pending_joins: HashMap<SocketAddr, String> = HashMap::new();

    let mut last_tick = Instant::now();
    let mut last_status = Instant::now();

    loop {
        let now = Instant::now();

        // Poll server (receive packets, manage clients)
        server.poll();

        // ==================== Accept New Connections (auto-join on first packet) ====================
        while let Some(addr) = server.accept() {
            // Skip if already in clients map
            if clients.contains_key(&addr_to_session.get(&addr).copied().unwrap_or(0)) {
                continue;
            }

            let session_id = next_session_id;
            next_session_id += 1;

            // Create device ID from RUDP address (auto-auth like kaos-io)
            let device_id = format!("rudp_{}", addr);
            let name = format!("AstroPlayer{}", session_id);
            let auth_request = DeviceAuthRequest {
                device_id: device_id.clone(),
                create: true,
                username: Some(name.clone()),
            };

            let (user_id, username) = match auth.authenticate_device(&auth_request) {
                Ok(auth_response) => {
                    let uid = auth_response.account.id.clone();
                    let uname = auth_response.account.username.clone().unwrap_or(name.clone());
                    println!("[auth] Device authenticated: {} -> {}", device_id, uid);
                    (uid, uname)
                }
                Err(e) => {
                    println!("[auth] Auth failed, using fallback: {}", e);
                    let fallback = format!("ast_{}", session_id);
                    (fallback, name.clone())
                }
            };

            // Execute match join hook
            let hook_ctx = HookContext {
                user_id: Some(user_id.clone()),
                username: Some(username.clone()),
                session_id: Some(session_id),
                client_ip: Some(addr.to_string()),
                vars: std::collections::HashMap::new(),
            };
            let _ = hooks.execute_after(
                HookOperation::MatchJoin,
                &hook_ctx,
                &serde_json::json!({ "player_name": username }),
                &serde_json::json!({}),
            );

            // Join the Lua match - this triggers match_join in Lua
            let presence = MatchPresence {
                user_id: user_id.clone(),
                session_id,
                username: username.clone(),
                node: None,
            };
            if let Err(e) = match_registry.join(&game_match_id, presence) {
                eprintln!("[!] Failed to join match: {:?}", e);
                continue;
            }

            println!("[+] {} joined (session={}, user={}) from {}", username, session_id, user_id, addr);

            // Add player to game room
            if let Some(room) = rooms.get(&game_room_id) {
                let _ = room.add_player(session_id);
            }

            // Register session with KaosNet sessions service
            let registered_id = sessions.create(addr);
            if let Some(mut session) = sessions.get_mut(registered_id) {
                session.set_authenticated(user_id.clone(), Some(username.clone()));
                session.join_room(game_room_id.clone());
            }

            addr_to_session.insert(addr, session_id);
            clients.insert(session_id, Client {
                addr,
                session_id,
                user_id,
                username,
            });
        }

        // ==================== Process Client Messages ====================
        let mut disconnected = Vec::new();
        for (session_id, client) in &clients {
            // Check if client still connected
            if !server.has_client(&client.addr) {
                disconnected.push(*session_id);
                continue;
            }

            let session_id_copy = *session_id;
            let user_id = client.user_id.clone();
            let username = client.username.clone();
            let match_registry_ref = &match_registry;
            let game_match_id_ref = &game_match_id;

            server.receive(&client.addr, |data| {
                if let Ok(msg) = serde_json::from_slice::<ClientMessage>(data) {
                    match msg {
                        ClientMessage::Input { thrust, left, right, fire } => {
                            // Forward input to Lua match as MatchMessage
                            let presence = MatchPresence {
                                user_id: user_id.clone(),
                                session_id: session_id_copy,
                                username: username.clone(),
                                node: None,
                            };

                            let input_data = serde_json::json!({
                                "thrust": thrust,
                                "left": left,
                                "right": right,
                                "fire": fire
                            });

                            let message = MatchMessage {
                                sender: presence,
                                op_code: 1,
                                data: serde_json::to_vec(&input_data).unwrap_or_default(),
                                reliable: true,
                                received_at: tick,
                            };

                            if let Err(e) = match_registry_ref.send_message(game_match_id_ref, message) {
                                eprintln!("[!] Failed to send message to match: {:?}", e);
                            }
                        }
                        ClientMessage::Leave => {
                            // Will be handled in disconnected
                        }
                        _ => {}
                    }
                }
            });
        }

        // ==================== Handle Disconnections ====================
        for session_id in disconnected {
            if let Some(client) = clients.remove(&session_id) {
                // Clean up session from KaosNet sessions service
                if let Some(sid) = sessions.get_by_addr(&client.addr) {
                    sessions.remove(sid);
                }

                addr_to_session.remove(&client.addr);
                server.disconnect(&client.addr);

                // Execute match leave hook
                let leave_ctx = HookContext {
                    user_id: Some(client.user_id.clone()),
                    username: Some(client.username.clone()),
                    session_id: Some(session_id),
                    client_ip: Some(client.addr.to_string()),
                    vars: std::collections::HashMap::new(),
                };
                let _ = hooks.execute_after(
                    HookOperation::MatchLeave,
                    &leave_ctx,
                    &serde_json::json!({ "session_id": session_id }),
                    &serde_json::json!({}),
                );

                // Leave the Lua match - triggers match_leave in Lua (handles leaderboard)
                if let Err(e) = match_registry.leave(&game_match_id, session_id) {
                    eprintln!("[!] Failed to leave match: {:?}", e);
                }

                println!("[-] {} left (session={})", client.username, session_id);

                // Remove from game room
                if let Some(room) = rooms.get(&game_room_id) {
                    room.remove_player(session_id);
                }
            }
        }

        // ==================== Game Tick ====================
        if now.duration_since(last_tick) >= TICK_DURATION {
            tick += 1;

            // Tick the Lua match (ALL game logic in Lua)
            if let Err(e) = match_registry.tick(&game_match_id) {
                eprintln!("[!] Match tick error: {:?}", e);
            }

            // Get state from Lua and broadcast to clients
            if let Some(match_instance) = match_registry.get(&game_match_id) {
                let broadcast_data = match_instance.state.state.get("broadcast").cloned()
                    .unwrap_or_else(|| match_instance.state.state.clone());

                let msg = ServerMessage::GameState { state: broadcast_data };
                let data = serde_json::to_vec(&msg).unwrap_or_default();

                for client in clients.values() {
                    let _ = server.send(&client.addr, &data);
                }
            }

            last_tick = now;
        }

        // Status update
        if now.duration_since(last_status) >= Duration::from_secs(10) {
            if !clients.is_empty() {
                println!("[i] {} players connected", clients.len());
            }
            last_status = now;
        }

        // Sleep to not burn CPU
        std::thread::sleep(Duration::from_micros(100));
    }
}
