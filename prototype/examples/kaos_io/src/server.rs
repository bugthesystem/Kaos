//! Kaos.io Game Server
//!
//! An Agar.io-style multiplayer game demonstrating ALL KaosNet capabilities.
//!
//! Features demonstrated:
//! - **Lua scripting** - ALL game logic in Lua (scripts/game.lua)
//! - **Dual transport** - WebSocket for web clients, RUDP for native clients
//! - Client authentication (device auth with JWT tokens)
//! - Server hooks (before/after for storage and match events)
//! - Storage with permissions (PublicRead for profiles)
//! - Leaderboard service for persistent high scores (accessible from Lua)
//! - Room management

// RUDP support for native clients (bots, native apps)
use kaos_rudp::RudpServer;
use kaosnet::{
    RoomConfig, RoomRegistry, SessionRegistry,
    Leaderboards, LeaderboardConfig, SortOrder, ScoreOperator,
    Storage, Chat, Matchmaker, Notifications, Social, Tournaments,
    // Authentication
    AuthService, DeviceAuthRequest,
    // Hooks
    HookRegistry, HookOperation, HookContext, BeforeHook, AfterHook, BeforeHookResult,
    hooks::Result as HookResult,
    // Hooked Services (with metrics and notifications integration)
    HookedStorage, HookedLeaderboards, HookedSocial,
    // Console server
    ConsoleServer, ConsoleConfig,
    // Unified transport from kaosnet
    WsServerTransport, WsClientTransport, TransportServer, ClientTransport,
    // Metrics
    Metrics,
    // Lua runtime and match handler
    LuaConfig, LuaRuntime, LuaMatchHandler, LuaServices,
    // Match system
    MatchHandlerRegistry, MatchRegistry, MatchPresence, MatchMessage,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ==================== Network Types ====================
// Game state is entirely managed by Lua - no Rust game state structs needed!

/// Transport type for clients (WebSocket for web, RUDP for native)
enum ClientTransportType {
    WebSocket(WsClientTransport),
    Rudp(SocketAddr), // RUDP clients identified by address
}

impl ClientTransportType {
    fn is_open(&self) -> bool {
        match self {
            ClientTransportType::WebSocket(ws) => ws.is_open(),
            ClientTransportType::Rudp(_) => true, // RUDP uses timeout-based disconnect
        }
    }

    fn is_rudp(&self) -> bool {
        matches!(self, ClientTransportType::Rudp(_))
    }

    fn rudp_addr(&self) -> Option<SocketAddr> {
        match self {
            ClientTransportType::Rudp(addr) => Some(*addr),
            _ => None,
        }
    }

    fn get_addr(&self) -> Option<SocketAddr> {
        match self {
            ClientTransportType::WebSocket(ws) => ws.peer_addr(),
            ClientTransportType::Rudp(addr) => Some(*addr),
        }
    }
}

/// Connected client with transport and identity
struct Client {
    transport: ClientTransportType,
    /// Authenticated user ID (from device auth)
    user_id: String,
    /// Username for display
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

/// After hook for match leave - logs player disconnections
struct MatchLeaveLogHook;

impl AfterHook for MatchLeaveLogHook {
    fn execute(&self, ctx: &HookContext, payload: &serde_json::Value, _result: &serde_json::Value) {
        let score = payload.get("final_score").and_then(|s| s.as_u64()).unwrap_or(0);
        let kills = payload.get("kills").and_then(|k| k.as_u64()).unwrap_or(0);
        println!("[hook] Player left match: user={:?}, score={}, kills={}", ctx.user_id, score, kills);
    }
}

// Game constants (also in Lua, kept for server reference)
const TICK_RATE: u32 = 20;
const LEADERBOARD_ID: &str = "kaos_io_highscores";
const LUA_MODULE_NAME: &str = "kaos_io";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging
    // RUST_LOG=kaos_io=debug,kaosnet=info for verbose output
    use tracing_subscriber::{fmt, EnvFilter};
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with_target(true)
        .compact()
        .init();

    tracing::info!("Kaos.io server starting...");

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

    // Initialize Prometheus metrics
    let metrics = Arc::new(Metrics::new());
    println!("✓ Metrics initialized");

    // Initialize KaosNet services
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

    // Initialize Authentication service with secret key
    let auth = Arc::new(AuthService::new("kaos-io-game-secret-key"));
    println!("✓ Auth service initialized (device/email auth with JWT)");

    // Initialize Hooks registry
    let hooks = Arc::new(HookRegistry::new());

    // Create Hooked Services with metrics and notifications integration
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

    // ==================== Lua Runtime Setup ====================
    // Create LuaServices with HOOKED services for Lua script access
    // This ensures Lua scripts get metrics, hooks, and notifications!
    let lua_services = Arc::new(LuaServices::with_hooked_services(
        Arc::clone(&hooked_storage),
        Arc::clone(&hooked_leaderboards),
        Arc::clone(&hooked_social),
    ));

    // Create Lua runtime with services
    // Allow override via environment variable for Docker deployments
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

    // Create the main game room (for room registry tracking)
    let game_room_id = rooms.create(RoomConfig {
        max_players: 50,
        tick_rate: TICK_RATE,
        label: "Kaos.io Main Arena".to_string(),
        module: LUA_MODULE_NAME.to_string(),
    });
    println!("✓ Game room created: {}", game_room_id);

    println!();
    println!("Server Configuration:");
    println!("  Web Game:  http://0.0.0.0:8082");
    println!("  WebSocket: ws://0.0.0.0:7351 (web clients)");
    println!("  RUDP:      udp://0.0.0.0:7354 (native clients)");
    println!("  Console:   http://0.0.0.0:7350");
    println!("  Metrics:   http://0.0.0.0:9090/metrics");
    println!("  Tick Rate: {} Hz", TICK_RATE);
    println!("  Lua Script: scripts/game.lua");
    println!();
    println!("KaosNet Features Demonstrated:");
    println!("  - Lua Scripting: ALL game logic in scripts/game.lua");
    println!("  - Dual Transport: WebSocket + RUDP for maximum flexibility");
    println!("  - Auth: Device/email authentication with JWT tokens");
    println!("  - Hooks: Before/after hooks for validation and logging");
    println!("  - Leaderboards: Track high scores (accessible from Lua)");
    println!("  - Storage: Persist profiles (accessible from Lua)");
    println!("  - Match Handler: Lua-based match lifecycle");
    println!("  - Console: Admin API (login: admin/admin)");
    println!("  - Metrics: Prometheus endpoint for observability");
    println!();

    // Start Console API server in background thread
    let console_sessions = Arc::clone(&sessions);
    let console_rooms = Arc::clone(&rooms);
    let console_storage = Arc::clone(&storage);
    let console_leaderboards = Arc::clone(&leaderboards);
    let console_social = Arc::clone(&social);
    let console_chat = Arc::clone(&chat);
    let console_matchmaker = Arc::clone(&matchmaker);
    let console_notifications = Arc::clone(&notifications);
    let console_tournaments = Arc::clone(&tournaments);
    let console_metrics = Arc::clone(&metrics);
    let console_script_path = script_path.to_string_lossy().to_string();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            let console_config = ConsoleConfig {
                bind_addr: "0.0.0.0:7350".to_string(),
                ..ConsoleConfig::default()
            };
            let console = ConsoleServer::builder(
                console_config,
                console_sessions,
                console_rooms,
            )
            .metrics(console_metrics)
            .storage(console_storage)
            .leaderboards(console_leaderboards)
            .social(console_social)
            .chat(console_chat)
            .matchmaker(console_matchmaker)
            .notifications(console_notifications)
            .tournaments(console_tournaments)
            .lua_script_path(console_script_path)
            .build();

            if let Err(e) = console.serve().await {
                eprintln!("Console server error: {}", e);
            }
        });
    });
    println!("✓ Console API listening on http://0.0.0.0:7350");

    // Start Metrics HTTP server in background thread (Prometheus scrape endpoint)
    let metrics_clone = Arc::clone(&metrics);
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("0.0.0.0:9090").expect("Failed to bind metrics server");
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);

                // Simple HTTP response with metrics
                let body = metrics_clone.gather();
                let response = format!(
                    "HTTP/1.1 200 OK\r\n\
                     Content-Type: text/plain; version=0.0.4\r\n\
                     Content-Length: {}\r\n\
                     \r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        }
    });
    println!("✓ Metrics endpoint on http://0.0.0.0:9090/metrics");

    // Start static file server for web client (serves index.html and assets)
    std::thread::spawn(|| {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;
        use std::path::Path;

        let listener = TcpListener::bind("0.0.0.0:8082").expect("Failed to bind web server");
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let buf_reader = BufReader::new(&stream);
                let request_line = buf_reader.lines().next().and_then(|l| l.ok()).unwrap_or_default();

                // Parse requested path
                let path = request_line.split_whitespace().nth(1).unwrap_or("/");
                let file_path = if path == "/" || path.is_empty() {
                    "web/index.html"
                } else {
                    &format!("web{}", path)
                };

                // Read file from web directory
                let web_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(file_path);
                let (status, content_type, body) = if web_dir.exists() && web_dir.is_file() {
                    let content = std::fs::read(&web_dir).unwrap_or_default();
                    let ct = if file_path.ends_with(".html") { "text/html" }
                        else if file_path.ends_with(".svg") { "image/svg+xml" }
                        else if file_path.ends_with(".js") { "application/javascript" }
                        else if file_path.ends_with(".css") { "text/css" }
                        else { "application/octet-stream" };
                    ("200 OK", ct, content)
                } else {
                    ("404 Not Found", "text/plain", b"Not Found".to_vec())
                };

                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
                    status, content_type, body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(&body);
            }
        }
    });
    println!("✓ Web client available at http://0.0.0.0:8082");

    // Start WebSocket server using kaosnet transport abstraction
    let mut ws_server = WsServerTransport::bind("0.0.0.0:7351")?;
    ws_server.set_nonblocking(true)?;
    println!("✓ WebSocket server on 0.0.0.0:7351 (web clients)");

    // Start RUDP server for native clients (bots, native apps)
    let mut rudp_server = RudpServer::bind("0.0.0.0:7354", 256)?;
    println!("✓ RUDP server on 0.0.0.0:7354 (native clients/bots)");
    println!("Waiting for players...\n");

    // Client tracking (game state is managed by Lua)
    let mut clients: HashMap<u64, Client> = HashMap::new();
    // Map RUDP address -> session_id for reverse lookup
    let mut rudp_sessions: HashMap<SocketAddr, u64> = HashMap::new();
    let mut next_session_id: u64 = 1;
    let mut tick: u64 = 0;

    let tick_duration = Duration::from_micros(1_000_000 / TICK_RATE as u64);

    // Main game loop
    loop {
        let tick_start = Instant::now();

        // ==================== Accept New Connections ====================
        match ws_server.accept() {
            Ok(Some(transport)) => {
                let session_id = next_session_id;
                next_session_id += 1;

                // Authenticate using device auth (creates account on first use)
                let device_id = format!("device_{}", session_id);
                let auth_request = DeviceAuthRequest {
                    device_id: device_id.clone(),
                    create: true,
                    username: Some(format!("Player{}", session_id)),
                };
                let auth_response = match auth.authenticate_device(&auth_request) {
                    Ok(response) => response,
                    Err(e) => {
                        eprintln!("[!] Auth failed for device {}: {:?}", device_id, e);
                        continue;
                    }
                };

                let user_id = auth_response.account.id.clone();
                let username = auth_response.account.username.clone()
                    .unwrap_or_else(|| format!("Player{}", session_id));

                // Execute match join hook
                let hook_ctx = HookContext {
                    user_id: Some(user_id.clone()),
                    username: Some(username.clone()),
                    session_id: Some(session_id),
                    ..Default::default()
                };
                let hook_payload = serde_json::json!({
                    "device_id": device_id,
                    "session_id": session_id
                });
                let hook_result_val = serde_json::json!({"status": "joined"});
                hooks.execute_after(
                    HookOperation::MatchJoin,
                    &hook_ctx,
                    &hook_payload,
                    &hook_result_val,
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

                println!("[+] {} joined (session={}, user={})", username, session_id, user_id);

                // Register session with KaosNet sessions service
                if let Some(peer_addr) = transport.peer_addr() {
                    let registered_id = sessions.create(peer_addr);
                    if let Some(mut session) = sessions.get_mut(registered_id) {
                        session.set_authenticated(user_id.clone(), Some(username.clone()));
                        session.join_room(game_room_id.clone());
                    }
                }

                // Track client connection
                clients.insert(session_id, Client {
                    transport: ClientTransportType::WebSocket(transport),
                    user_id,
                    username,
                });

                // Update metrics and room
                metrics.sessions_total.inc();
                if let Some(room) = rooms.get(&game_room_id) {
                    let _ = room.add_player(session_id);
                }
            }
            Ok(None) => {}
            Err(_) => {}
        }

        // ==================== Accept RUDP Connections ====================
        // Poll for incoming RUDP packets (creates client state on first packet)
        rudp_server.poll();

        // Handle new RUDP clients
        while let Some(rudp_addr) = rudp_server.accept() {
            // Check if we already know this client
            if rudp_sessions.contains_key(&rudp_addr) {
                continue;
            }

            let session_id = next_session_id;
            next_session_id += 1;

            // Create device ID from RUDP address
            let device_id = format!("rudp_{}", rudp_addr);
            let auth_request = DeviceAuthRequest {
                device_id: device_id.clone(),
                create: true,
                username: Some(format!("RudpPlayer{}", session_id)),
            };
            let auth_response = match auth.authenticate_device(&auth_request) {
                Ok(response) => response,
                Err(e) => {
                    eprintln!("[!] RUDP auth failed for {}: {:?}", rudp_addr, e);
                    rudp_server.disconnect(&rudp_addr);
                    continue;
                }
            };

            let user_id = auth_response.account.id.clone();
            let username = auth_response.account.username.clone()
                .unwrap_or_else(|| format!("RudpPlayer{}", session_id));

            // Execute match join hook
            let hook_ctx = HookContext {
                user_id: Some(user_id.clone()),
                username: Some(username.clone()),
                session_id: Some(session_id),
                ..Default::default()
            };
            let hook_payload = serde_json::json!({
                "device_id": device_id,
                "session_id": session_id,
                "transport": "rudp"
            });
            let hook_result_val = serde_json::json!({"status": "joined"});
            hooks.execute_after(
                HookOperation::MatchJoin,
                &hook_ctx,
                &hook_payload,
                &hook_result_val,
            );

            // Join the Lua match
            let presence = MatchPresence {
                user_id: user_id.clone(),
                session_id,
                username: username.clone(),
                node: None,
            };
            if let Err(e) = match_registry.join(&game_match_id, presence) {
                eprintln!("[!] RUDP failed to join match: {:?}", e);
                rudp_server.disconnect(&rudp_addr);
                continue;
            }

            println!("[+] {} joined via RUDP (session={}, addr={})", username, session_id, rudp_addr);

            // Register session with KaosNet sessions service (RUDP)
            let registered_id = sessions.create(rudp_addr);
            if let Some(mut session) = sessions.get_mut(registered_id) {
                session.set_authenticated(user_id.clone(), Some(username.clone()));
                session.join_room(game_room_id.clone());
            }

            // Track RUDP client
            rudp_sessions.insert(rudp_addr, session_id);
            clients.insert(session_id, Client {
                transport: ClientTransportType::Rudp(rudp_addr),
                user_id,
                username,
            });

            // Update metrics
            metrics.sessions_total.inc();
            if let Some(room) = rooms.get(&game_room_id) {
                let _ = room.add_player(session_id);
            }
        }

        // ==================== Process Client Messages ====================
        let mut disconnected: Vec<u64> = Vec::new();

        // Process WebSocket client messages
        for (session_id, client) in clients.iter_mut() {
            // Skip RUDP clients (handled separately below)
            if client.transport.is_rudp() {
                continue;
            }

            if !client.transport.is_open() {
                disconnected.push(*session_id);
                continue;
            }

            // Collect messages and forward to Lua match handler
            let session_id_copy = *session_id;
            let user_id = client.user_id.clone();
            let username = client.username.clone();
            let match_registry_ref = &match_registry;
            let game_match_id_ref = &game_match_id;

            // WebSocket receive
            if let ClientTransportType::WebSocket(ws) = &mut client.transport {
                ws.receive(&mut |data: &[u8]| {
                    let presence = MatchPresence {
                        user_id: user_id.clone(),
                        session_id: session_id_copy,
                        username: username.clone(),
                        node: None,
                    };

                    let message = MatchMessage {
                        sender: presence,
                        op_code: 1,
                        data: data.to_vec(),
                        reliable: true,
                        received_at: tick,
                    };

                    if let Err(e) = match_registry_ref.send_message(game_match_id_ref, message) {
                        eprintln!("[!] Failed to send message to match: {:?}", e);
                    }
                });
            }
        }

        // Process RUDP client messages (receive from all at once via server)
        for (&rudp_addr, &session_id) in &rudp_sessions {
            if let Some(client) = clients.get(&session_id) {
                let user_id = client.user_id.clone();
                let username = client.username.clone();

                // Receive messages from this RUDP client
                rudp_server.receive(&rudp_addr, |data| {
                    let presence = MatchPresence {
                        user_id: user_id.clone(),
                        session_id,
                        username: username.clone(),
                        node: None,
                    };

                    let message = MatchMessage {
                        sender: presence,
                        op_code: 1,
                        data: data.to_vec(),
                        reliable: true,
                        received_at: tick,
                    };

                    if let Err(e) = match_registry.send_message(&game_match_id, message) {
                        eprintln!("[!] RUDP message forward error: {:?}", e);
                    }
                });
            }
        }

        // ==================== Handle Disconnections ====================
        for session_id in &disconnected {
            if let Some(client) = clients.remove(session_id) {
                // Clean up session from KaosNet sessions service
                if let Some(addr) = client.transport.get_addr() {
                    if let Some(sid) = sessions.get_by_addr(&addr) {
                        sessions.remove(sid);
                    }
                }

                // Clean up RUDP session mapping if this was an RUDP client
                if let Some(addr) = client.transport.rudp_addr() {
                    rudp_sessions.remove(&addr);
                    rudp_server.disconnect(&addr);
                }

                // Execute match leave hook
                let hook_ctx = HookContext {
                    user_id: Some(client.user_id.clone()),
                    username: Some(client.username.clone()),
                    session_id: Some(*session_id),
                    ..Default::default()
                };
                let hook_payload = serde_json::json!({
                    "session_id": session_id,
                });
                let hook_result_val = serde_json::json!({"status": "left"});
                hooks.execute_after(
                    HookOperation::MatchLeave,
                    &hook_ctx,
                    &hook_payload,
                    &hook_result_val,
                );

                // Leave the Lua match - triggers match_leave in Lua (handles leaderboard submission)
                if let Err(e) = match_registry.leave(&game_match_id, *session_id) {
                    eprintln!("[!] Failed to leave match: {:?}", e);
                }

                let transport_type = if client.transport.is_rudp() { "RUDP" } else { "WS" };
                println!("[-] {} left via {} (session={})", client.username, transport_type, session_id);

                if let Some(room) = rooms.get(&game_room_id) {
                    room.remove_player(*session_id);
                }
            }
        }

        // ==================== Tick the Lua Match (Game Logic) ====================
        // This calls match_loop in Lua which handles ALL game logic:
        // - Player movement
        // - Food collisions
        // - Player-player collisions
        // - Respawning
        if let Err(e) = match_registry.tick(&game_match_id) {
            eprintln!("[!] Match tick error: {:?}", e);
        }

        // ==================== Broadcast State to Clients ====================
        // Get the current match state from Lua and broadcast to all clients
        if let Some(match_instance) = match_registry.get(&game_match_id) {
            // The Lua script puts broadcast-ready data in state.broadcast
            // This is formatted as arrays for the web client
            let broadcast_data = match_instance.state.state.get("broadcast").cloned()
                .unwrap_or_else(|| match_instance.state.state.clone());

            let state_json = serde_json::to_vec(&broadcast_data).unwrap_or_default();

            // Broadcast to WebSocket clients
            for client in clients.values_mut() {
                if let ClientTransportType::WebSocket(ws) = &mut client.transport {
                    if ws.is_open() {
                        let _ = ws.send(&state_json);
                    }
                }
            }

            // Broadcast to RUDP clients
            for &rudp_addr in rudp_sessions.keys() {
                let _ = rudp_server.send(&rudp_addr, &state_json);
            }
        }

        tick += 1;

        // Update metrics every tick
        let ws_count = clients.values().filter(|c| !c.transport.is_rudp()).count();
        let rudp_count = rudp_sessions.len();
        metrics.sessions_active.set(clients.len() as i64);
        metrics.websocket_connections.set(ws_count as i64);
        metrics.rooms_active.set(rooms.count() as i64);

        // Periodic status
        if tick % (TICK_RATE as u64 * 10) == 0 && !clients.is_empty() {
            let lb_count = leaderboards.count(LEADERBOARD_ID).unwrap_or(0);
            println!("[tick {}] {} players ({}WS + {}RUDP), {} on leaderboard",
                tick, clients.len(), ws_count, rudp_count, lb_count);
        }

        // Sleep for remaining tick time
        let elapsed = tick_start.elapsed();
        if elapsed < tick_duration {
            std::thread::sleep(tick_duration - elapsed);
        }
    }
}
