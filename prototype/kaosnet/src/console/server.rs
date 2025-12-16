//! Console HTTP server.

use crate::console::auth::AuthService;
use crate::console::handlers;
use crate::console::storage::{AccountStore, ApiKeyStore};
use crate::ratelimit::{RateLimiter, RateLimitConfig, RateLimitPresets};
use crate::room::RoomRegistry;
use crate::session::SessionRegistry;
use kaos_http::middleware::{CorsMiddleware, LoggingMiddleware, Middleware, Next};
use kaos_http::{HttpServer, Request, Response, Router};
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
            bind_addr: "127.0.0.1:7350".to_string(),
            jwt_secret: "change-me-in-production".to_string(),
            allow_public_status: true,
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
}

/// Console HTTP server for admin interface.
pub struct ConsoleServer {
    config: ConsoleConfig,
    ctx: Arc<ServerContext>,
}

impl ConsoleServer {
    /// Create new console server.
    pub fn new(
        config: ConsoleConfig,
        sessions: Arc<SessionRegistry>,
        rooms: Arc<RoomRegistry>,
    ) -> Self {
        let accounts = Arc::new(AccountStore::new());
        let api_keys = Arc::new(ApiKeyStore::new());

        // Create default admin account if none exists
        if accounts.list().is_empty() {
            use crate::console::auth::Role;
            accounts.create("admin", "admin", Role::Admin);
            #[cfg(not(feature = "tracing"))]
            eprintln!("Created default admin account: admin/admin");
            #[cfg(feature = "tracing")]
            tracing::warn!("Created default admin account: admin/admin");
        }

        let auth = Arc::new(AuthService::new(
            &config.jwt_secret,
            Arc::clone(&accounts),
            Arc::clone(&api_keys),
        ));

        let ctx = Arc::new(ServerContext {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            sessions,
            rooms,
            auth,
            accounts,
            api_keys,
        });

        Self { config, ctx }
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

        Router::new()
            // Health check (no auth)
            .get("/health", |_| async { Response::ok().json(&serde_json::json!({"status": "ok"})) })

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
        if path == "/health" || path == "/api/auth/login" {
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
            let mut response = next.run(req).await;

            // Note: In a real implementation, we'd add these headers:
            // X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset
            response
        })
    }
}
