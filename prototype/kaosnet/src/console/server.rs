//! Console HTTP server.

use crate::chat::Chat;
use crate::console::auth::AuthService;
use crate::console::handlers;
use crate::console::storage::{AccountStore, ApiKeyStore};
use crate::leaderboard::Leaderboards;
use crate::matchmaker::Matchmaker;
#[cfg(feature = "metrics")]
use crate::metrics::Metrics;
use crate::notifications::Notifications;
use crate::ratelimit::{RateLimiter, RateLimitPresets};
use crate::room::RoomRegistry;
use crate::session::SessionRegistry;
use crate::social::Social;
use crate::storage::Storage;
use crate::tournament::Tournaments;
use kaos_http::middleware::{CorsMiddleware, LoggingMiddleware, Middleware, Next};
use kaos_http::{HttpServer, Method, Request, Response, Router};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

/// Console server configuration.
pub struct ConsoleConfig {
    /// HTTP bind address.
    pub bind_addr: String,
    /// JWT secret for authentication.
    pub jwt_secret: String,
    /// Allow public access (no auth required for status endpoints).
    pub allow_public_status: bool,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            bind_addr: std::env::var("KAOS_CONSOLE_BIND")
                .unwrap_or_else(|_| "127.0.0.1:7350".to_string()),
            jwt_secret: std::env::var("KAOS_JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            allow_public_status: std::env::var("KAOS_PUBLIC_STATUS")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
        }
    }
}

impl ConsoleConfig {
    /// Validates the configuration, returning errors for insecure defaults in production.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check for insecure JWT secret
        if self.jwt_secret == "change-me-in-production" {
            if std::env::var("KAOS_ENV").map(|v| v == "production").unwrap_or(false) {
                errors.push("KAOS_JWT_SECRET must be set in production".to_string());
            } else {
                #[cfg(feature = "telemetry")]
                tracing::warn!("Using default JWT secret - set KAOS_JWT_SECRET in production");
                #[cfg(not(feature = "telemetry"))]
                eprintln!("Warning: Using default JWT secret - set KAOS_JWT_SECRET in production");
            }
        }

        // Check JWT secret length
        if self.jwt_secret.len() < 32 {
            errors.push("KAOS_JWT_SECRET should be at least 32 characters".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Server context shared across handlers.
pub struct ServerContext {
    pub start_time: Instant,
    pub version: String,
    pub sessions: Arc<SessionRegistry>,
    pub rooms: Arc<RoomRegistry>,
    pub auth: Arc<AuthService>,
    pub accounts: Arc<AccountStore>,
    pub api_keys: Arc<ApiKeyStore>,
    // Game services
    pub storage: Arc<Storage>,
    pub leaderboards: Arc<Leaderboards>,
    pub tournaments: Arc<Tournaments>,
    pub social: Arc<Social>,
    pub chat: Arc<Chat>,
    pub matchmaker: Arc<Matchmaker>,
    pub notifications: Arc<Notifications>,
    // Lua script path (optional)
    pub lua_script_path: Option<String>,
    // Metrics (optional)
    #[cfg(feature = "metrics")]
    pub metrics: Option<Arc<Metrics>>,
}

/// Console HTTP server for admin interface.
pub struct ConsoleServer {
    config: ConsoleConfig,
    ctx: Arc<ServerContext>,
}

/// Builder for ConsoleServer with optional services.
pub struct ConsoleServerBuilder {
    config: ConsoleConfig,
    sessions: Arc<SessionRegistry>,
    rooms: Arc<RoomRegistry>,
    storage: Option<Arc<Storage>>,
    leaderboards: Option<Arc<Leaderboards>>,
    tournaments: Option<Arc<Tournaments>>,
    social: Option<Arc<Social>>,
    chat: Option<Arc<Chat>>,
    matchmaker: Option<Arc<Matchmaker>>,
    notifications: Option<Arc<Notifications>>,
    lua_script_path: Option<String>,
    #[cfg(feature = "metrics")]
    metrics: Option<Arc<Metrics>>,
}

impl ConsoleServerBuilder {
    /// Create new builder with required services.
    pub fn new(
        config: ConsoleConfig,
        sessions: Arc<SessionRegistry>,
        rooms: Arc<RoomRegistry>,
    ) -> Self {
        Self {
            config,
            sessions,
            rooms,
            storage: None,
            leaderboards: None,
            tournaments: None,
            social: None,
            chat: None,
            matchmaker: None,
            notifications: None,
            lua_script_path: None,
            #[cfg(feature = "metrics")]
            metrics: None,
        }
    }

    /// Set storage service.
    pub fn storage(mut self, storage: Arc<Storage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set leaderboards service.
    pub fn leaderboards(mut self, leaderboards: Arc<Leaderboards>) -> Self {
        self.leaderboards = Some(leaderboards);
        self
    }

    /// Set tournaments service.
    pub fn tournaments(mut self, tournaments: Arc<Tournaments>) -> Self {
        self.tournaments = Some(tournaments);
        self
    }

    /// Set social service.
    pub fn social(mut self, social: Arc<Social>) -> Self {
        self.social = Some(social);
        self
    }

    /// Set chat service.
    pub fn chat(mut self, chat: Arc<Chat>) -> Self {
        self.chat = Some(chat);
        self
    }

    /// Set matchmaker service.
    pub fn matchmaker(mut self, matchmaker: Arc<Matchmaker>) -> Self {
        self.matchmaker = Some(matchmaker);
        self
    }

    /// Set notifications service.
    pub fn notifications(mut self, notifications: Arc<Notifications>) -> Self {
        self.notifications = Some(notifications);
        self
    }

    /// Set Lua script path for serving script contents.
    pub fn lua_script_path(mut self, path: impl Into<String>) -> Self {
        self.lua_script_path = Some(path.into());
        self
    }

    /// Set metrics service.
    #[cfg(feature = "metrics")]
    pub fn metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Build the console server.
    pub fn build(self) -> ConsoleServer {
        let accounts = Arc::new(AccountStore::new());
        let api_keys = Arc::new(ApiKeyStore::new());

        // Create default admin account if none exists
        if accounts.list().is_empty() {
            use crate::console::auth::Role;
            let admin_password = std::env::var("KAOS_ADMIN_PASSWORD")
                .unwrap_or_else(|_| {
                    #[cfg(not(feature = "telemetry"))]
                    eprintln!("Warning: KAOS_ADMIN_PASSWORD not set, using insecure default");
                    #[cfg(feature = "telemetry")]
                    tracing::warn!("KAOS_ADMIN_PASSWORD not set, using insecure default");
                    "admin".to_string()
                });
            let admin_username = std::env::var("KAOS_ADMIN_USERNAME")
                .unwrap_or_else(|_| "admin".to_string());
            accounts.create(&admin_username, &admin_password, Role::Admin);
            #[cfg(not(feature = "telemetry"))]
            eprintln!("Created admin account: {}", admin_username);
            #[cfg(feature = "telemetry")]
            tracing::info!("Created admin account: {}", admin_username);
        }

        let auth = Arc::new(AuthService::new(
            &self.config.jwt_secret,
            Arc::clone(&accounts),
            Arc::clone(&api_keys),
        ));

        let ctx = Arc::new(ServerContext {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            sessions: self.sessions,
            rooms: self.rooms,
            auth,
            accounts,
            api_keys,
            storage: self.storage.unwrap_or_else(|| Arc::new(Storage::new())),
            leaderboards: self.leaderboards.unwrap_or_else(|| Arc::new(Leaderboards::new())),
            tournaments: self.tournaments.unwrap_or_else(|| Arc::new(Tournaments::new())),
            social: self.social.unwrap_or_else(|| Arc::new(Social::new())),
            chat: self.chat.unwrap_or_else(|| Arc::new(Chat::new())),
            matchmaker: self.matchmaker.unwrap_or_else(|| Arc::new(Matchmaker::new())),
            notifications: self.notifications.unwrap_or_else(|| Arc::new(Notifications::new())),
            lua_script_path: self.lua_script_path,
            #[cfg(feature = "metrics")]
            metrics: self.metrics,
        });

        ConsoleServer { config: self.config, ctx }
    }
}

impl ConsoleServer {
    /// Create new console server with default services.
    pub fn new(
        config: ConsoleConfig,
        sessions: Arc<SessionRegistry>,
        rooms: Arc<RoomRegistry>,
    ) -> Self {
        ConsoleServerBuilder::new(config, sessions, rooms).build()
    }

    /// Create a builder for customized console server.
    pub fn builder(
        config: ConsoleConfig,
        sessions: Arc<SessionRegistry>,
        rooms: Arc<RoomRegistry>,
    ) -> ConsoleServerBuilder {
        ConsoleServerBuilder::new(config, sessions, rooms)
    }

    /// Start serving requests.
    pub async fn serve(self) -> kaos_http::Result<()> {
        let router = self.build_router();

        HttpServer::bind(&self.config.bind_addr)
            .router(router)
            .middleware(LoggingMiddleware)
            .middleware(CorsMiddleware::permissive())
            .middleware(RateLimitMiddleware::new())  // Rate limiting before auth
            .middleware(AuthMiddleware::new(
                Arc::clone(&self.ctx.auth),
                self.config.allow_public_status,
            ))
            .serve()
            .await
    }

    fn build_router(&self) -> Router {
        let ctx = Arc::clone(&self.ctx);

        let mut router = Router::new()
            // Health check (no auth)
            .get("/health", |_| async { Response::ok().json(&serde_json::json!({"status": "ok"})) });

        // Metrics endpoint (Prometheus scrape, no auth)
        #[cfg(feature = "metrics")]
        {
            let metrics = ctx.metrics.clone();
            router = router.get("/metrics", move |_| {
                let metrics = metrics.clone();
                async move {
                    match metrics {
                        Some(m) => Response::ok()
                            .header("Content-Type", "text/plain; version=0.0.4")
                            .body(m.gather()),
                        None => Response::ok()
                            .header("Content-Type", "text/plain; version=0.0.4")
                            .body("# No metrics configured\n".to_string()),
                    }
                }
            });
        }

        router
            // Auth routes
            .post("/api/auth/login", {
                let auth = Arc::clone(&ctx.auth);
                move |req| {
                    let auth = Arc::clone(&auth);
                    async move { handlers::login(req, auth).await }
                }
            })
            .post("/api/auth/logout", |req| async move { handlers::logout(req).await })
            .get("/api/auth/me", |req| async move { handlers::me(req).await })

            // Status routes (may be public)
            .get("/api/status", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_status(req, ctx).await }
                }
            })
            .get("/api/config", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_config(req, ctx).await }
                }
            })

            // Session routes
            .get("/api/sessions", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_sessions(req, ctx).await }
                }
            })
            .get("/api/sessions/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_session(req, ctx).await }
                }
            })
            .post("/api/sessions/:id/kick", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::kick_session(req, ctx).await }
                }
            })

            // Room routes
            .get("/api/rooms", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_rooms(req, ctx).await }
                }
            })
            .get("/api/rooms/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_room(req, ctx).await }
                }
            })
            .get("/api/rooms/:id/state", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_room_state(req, ctx).await }
                }
            })
            .get("/api/rooms/:id/players", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_room_players(req, ctx).await }
                }
            })
            .post("/api/rooms/:id/terminate", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::terminate_room(req, ctx).await }
                }
            })

            // Account routes (admin only)
            .get("/api/accounts", {
                let accounts = Arc::clone(&ctx.accounts);
                move |req| {
                    let accounts = Arc::clone(&accounts);
                    async move { handlers::list_accounts(req, accounts).await }
                }
            })
            .post("/api/accounts", {
                let accounts = Arc::clone(&ctx.accounts);
                move |req| {
                    let accounts = Arc::clone(&accounts);
                    async move { handlers::create_account(req, accounts).await }
                }
            })
            .get("/api/accounts/:id", {
                let accounts = Arc::clone(&ctx.accounts);
                move |req| {
                    let accounts = Arc::clone(&accounts);
                    async move { handlers::get_account(req, accounts).await }
                }
            })
            .put("/api/accounts/:id", {
                let accounts = Arc::clone(&ctx.accounts);
                move |req| {
                    let accounts = Arc::clone(&accounts);
                    async move { handlers::update_account(req, accounts).await }
                }
            })
            .delete("/api/accounts/:id", {
                let accounts = Arc::clone(&ctx.accounts);
                move |req| {
                    let accounts = Arc::clone(&accounts);
                    async move { handlers::delete_account(req, accounts).await }
                }
            })
            .post("/api/accounts/:id/password", {
                let accounts = Arc::clone(&ctx.accounts);
                move |req| {
                    let accounts = Arc::clone(&accounts);
                    async move { handlers::change_password(req, accounts).await }
                }
            })

            // API Key routes (admin only)
            .get("/api/keys", {
                let keys = Arc::clone(&ctx.api_keys);
                move |req| {
                    let keys = Arc::clone(&keys);
                    async move { handlers::list_keys(req, keys).await }
                }
            })
            .post("/api/keys", {
                let keys = Arc::clone(&ctx.api_keys);
                move |req| {
                    let keys = Arc::clone(&keys);
                    async move { handlers::create_key(req, keys).await }
                }
            })
            .get("/api/keys/:id", {
                let keys = Arc::clone(&ctx.api_keys);
                move |req| {
                    let keys = Arc::clone(&keys);
                    async move { handlers::get_key(req, keys).await }
                }
            })
            .delete("/api/keys/:id", {
                let keys = Arc::clone(&ctx.api_keys);
                move |req| {
                    let keys = Arc::clone(&keys);
                    async move { handlers::delete_key(req, keys).await }
                }
            })
            .get("/api/keys/:id/usage", {
                let keys = Arc::clone(&ctx.api_keys);
                move |req| {
                    let keys = Arc::clone(&keys);
                    async move { handlers::get_key_usage(req, keys).await }
                }
            })

            // Player routes
            .get("/api/players", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_players(req, ctx).await }
                }
            })
            .get("/api/players/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_player(req, ctx).await }
                }
            })
            .post("/api/players/:id/ban", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::ban_player(req, ctx).await }
                }
            })
            .post("/api/players/:id/unban", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::unban_player(req, ctx).await }
                }
            })
            .delete("/api/players/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::delete_player(req, ctx).await }
                }
            })

            // Storage routes
            .get("/api/storage/collections", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_collections(req, ctx).await }
                }
            })
            .get("/api/storage/objects", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_storage_objects(req, ctx).await }
                }
            })
            .get("/api/storage", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_storage(req, ctx).await }
                }
            })
            .get("/api/storage/:user_id/:collection/:key", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_storage_object(req, ctx).await }
                }
            })
            .post("/api/storage", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::write_storage_object(req, ctx).await }
                }
            })
            .delete("/api/storage/:user_id/:collection/:key", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::delete_storage_object(req, ctx).await }
                }
            })

            // Leaderboard routes
            .get("/api/leaderboards", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_leaderboards(req, ctx).await }
                }
            })
            .get("/api/leaderboards/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_leaderboard(req, ctx).await }
                }
            })
            .get("/api/leaderboards/:id/records", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_leaderboard_records(req, ctx).await }
                }
            })
            .post("/api/leaderboards", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::create_leaderboard(req, ctx).await }
                }
            })
            .delete("/api/leaderboards/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::delete_leaderboard(req, ctx).await }
                }
            })

            // Tournament routes
            .get("/api/tournaments", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_tournaments(req, ctx).await }
                }
            })
            .get("/api/tournaments/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_tournament(req, ctx).await }
                }
            })
            .get("/api/tournaments/:id/records", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_tournament_records(req, ctx).await }
                }
            })
            .post("/api/tournaments", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::create_tournament(req, ctx).await }
                }
            })
            .post("/api/tournaments/:id/cancel", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::cancel_tournament(req, ctx).await }
                }
            })

            // Social routes
            .get("/api/social/friends", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_friends(req, ctx).await }
                }
            })
            .get("/api/social/groups", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_groups(req, ctx).await }
                }
            })
            .get("/api/social/groups/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_group(req, ctx).await }
                }
            })
            .get("/api/social/groups/:id/members", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_group_members(req, ctx).await }
                }
            })
            .post("/api/social/groups", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::create_group(req, ctx).await }
                }
            })
            .delete("/api/social/groups/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::delete_group(req, ctx).await }
                }
            })

            // Chat routes
            .get("/api/chat/channels", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_channels(req, ctx).await }
                }
            })
            .get("/api/chat/channels/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_channel(req, ctx).await }
                }
            })
            .get("/api/chat/channels/:id/messages", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_channel_messages(req, ctx).await }
                }
            })
            .delete("/api/chat/channels/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::delete_channel(req, ctx).await }
                }
            })
            .post("/api/chat/channels/:id/send", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::send_system_message(req, ctx).await }
                }
            })

            // Matchmaker routes
            .get("/api/matchmaker/queues", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_matchmaker_queues(req, ctx).await }
                }
            })
            .get("/api/matchmaker/tickets", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_matchmaker_tickets(req, ctx).await }
                }
            })
            .get("/api/matchmaker/tickets/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_matchmaker_ticket(req, ctx).await }
                }
            })
            .delete("/api/matchmaker/tickets/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::cancel_matchmaker_ticket(req, ctx).await }
                }
            })
            .get("/api/matchmaker/stats", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_matchmaker_stats(req, ctx).await }
                }
            })

            // Notification routes
            .get("/api/notifications", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_notifications(req, ctx).await }
                }
            })
            .get("/api/notifications/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_notification(req, ctx).await }
                }
            })
            .post("/api/notifications", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::send_notification(req, ctx).await }
                }
            })
            .post("/api/notifications/:id/read", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::mark_notification_read(req, ctx).await }
                }
            })
            .delete("/api/notifications/:id", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::delete_notification(req, ctx).await }
                }
            })

            // Lua routes
            .get("/api/lua/scripts", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_scripts(req, ctx).await }
                }
            })
            .get("/api/lua/scripts/:name", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_script(req, ctx).await }
                }
            })
            .get("/api/lua/scripts/:name/content", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::get_script_content(req, ctx).await }
                }
            })
            .get("/api/lua/rpcs", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::list_rpcs(req, ctx).await }
                }
            })
            .post("/api/lua/rpcs/:name/execute", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::execute_rpc(req, ctx).await }
                }
            })
            .post("/api/lua/reload", {
                let ctx = Arc::clone(&ctx);
                move |req| {
                    let ctx = Arc::clone(&ctx);
                    async move { handlers::reload_scripts(req, ctx).await }
                }
            })
    }
}

/// Authentication middleware.
struct AuthMiddleware {
    auth: Arc<AuthService>,
    allow_public_status: bool,
}

impl AuthMiddleware {
    fn new(auth: Arc<AuthService>, allow_public_status: bool) -> Self {
        Self {
            auth,
            allow_public_status,
        }
    }

    fn is_public_path(&self, path: &str) -> bool {
        // Always public
        if path == "/health" || path == "/api/auth/login" || path == "/metrics" {
            return true;
        }

        // Conditionally public
        if self.allow_public_status && (path == "/api/status" || path == "/api/config") {
            return true;
        }

        false
    }
}

impl Middleware for AuthMiddleware {
    fn handle<'a>(
        &'a self,
        mut req: Request,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            let path = req.path().to_string();

            // Skip auth for OPTIONS requests (CORS preflight)
            if req.method() == &Method::OPTIONS {
                return next.run(req).await;
            }

            // Skip auth for public paths
            if self.is_public_path(&path) {
                return next.run(req).await;
            }

            // Authenticate request
            match self.auth.authenticate(&req) {
                Some(identity) => {
                    req.insert_ext(identity);
                    next.run(req).await
                }
                None => {
                    // No auth required for logout
                    if path == "/api/auth/logout" {
                        return next.run(req).await;
                    }

                    Response::unauthorized().json(&serde_json::json!({
                        "error": "authentication required"
                    }))
                }
            }
        })
    }
}

/// Rate limiting middleware for console API.
struct RateLimitMiddleware {
    /// Limiter for auth endpoints (stricter)
    auth_limiter: RateLimiter,
    /// Limiter for general API endpoints
    api_limiter: RateLimiter,
}

impl RateLimitMiddleware {
    fn new() -> Self {
        Self {
            // Strict rate limit for login (5 req/sec, 3 burst) to prevent brute force
            auth_limiter: RateLimiter::new(RateLimitPresets::strict()),
            // Standard rate limit for other API endpoints (100 req/sec)
            api_limiter: RateLimiter::new(RateLimitPresets::standard()),
        }
    }

    fn get_client_id(req: &Request) -> String {
        // Use IP address as client identifier for rate limiting
        req.headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

impl Middleware for RateLimitMiddleware {
    fn handle<'a>(
        &'a self,
        req: Request,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            // Skip rate limiting for OPTIONS requests (CORS preflight)
            if req.method() == &Method::OPTIONS {
                return next.run(req).await;
            }

            let path = req.path();
            let client_id = Self::get_client_id(&req);

            // Select limiter based on path
            let limiter = if path.starts_with("/api/auth/") {
                &self.auth_limiter
            } else {
                &self.api_limiter
            };

            // Check rate limit
            let result = limiter.check_with_info(&client_id);

            if !result.allowed {
                return Response::too_many_requests().json(&serde_json::json!({
                    "error": "rate limit exceeded",
                    "retry_after_ms": result.reset_after_ms,
                    "limit": result.limit
                }));
            }

            // Add rate limit headers to response
            let response = next.run(req).await;

            // Note: In a real implementation, we'd add these headers:
            // X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset
            response
        })
    }
}
