//! Middleware support for request/response processing.

use crate::{Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Middleware trait for processing requests.
pub trait Middleware: Send + Sync {
    /// Process request, call next to continue chain.
    fn handle<'a>(
        &'a self,
        req: Request,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>>;
}

/// Next middleware/handler in the chain.
pub struct Next<'a> {
    inner: NextInner<'a>,
}

enum NextInner<'a> {
    Middleware {
        middleware: &'a dyn Middleware,
        next: Box<Next<'a>>,
    },
    Handler {
        handler: &'a crate::Handler,
    },
}

impl<'a> Next<'a> {
    /// Create Next from handler.
    pub fn handler(handler: &'a crate::Handler) -> Self {
        Self {
            inner: NextInner::Handler { handler },
        }
    }

    /// Create Next from middleware.
    pub fn middleware(middleware: &'a dyn Middleware, next: Next<'a>) -> Self {
        Self {
            inner: NextInner::Middleware {
                middleware,
                next: Box::new(next),
            },
        }
    }

    /// Run the next handler in chain.
    pub fn run(self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        match self.inner {
            NextInner::Handler { handler } => handler(req),
            NextInner::Middleware { middleware, next } => middleware.handle(req, *next),
        }
    }
}

/// Middleware chain builder.
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareChain {
    /// Create empty chain.
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add middleware to chain.
    pub fn add<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    /// Get middlewares.
    pub fn middlewares(&self) -> &[Arc<dyn Middleware>] {
        &self.middlewares
    }
}

// Common middleware implementations

/// CORS middleware.
pub struct CorsMiddleware {
    origin: String,
    allow_credentials: bool,
}

impl CorsMiddleware {
    /// Allow all origins.
    pub fn permissive() -> Self {
        Self {
            origin: "*".to_string(),
            allow_credentials: false,
        }
    }

    /// Allow specific origin.
    pub fn origin(origin: impl Into<String>) -> Self {
        Self {
            origin: origin.into(),
            allow_credentials: true,
        }
    }
}

impl Middleware for CorsMiddleware {
    fn handle<'a>(
        &'a self,
        req: Request,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        let origin = self.origin.clone();
        let allow_credentials = self.allow_credentials;

        Box::pin(async move {
            // Handle preflight
            if req.method() == http::Method::OPTIONS {
                let mut resp = Response::no_content()
                    .header("access-control-allow-origin", &origin)
                    .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS")
                    .header("access-control-allow-headers", "Content-Type, Authorization")
                    .header("access-control-max-age", "86400");

                if allow_credentials {
                    resp = resp.header("access-control-allow-credentials", "true");
                }

                return resp;
            }

            // Add CORS headers to response
            let mut resp = next.run(req).await;

            // Modify response headers (we need to rebuild since Response is immutable)
            if origin == "*" {
                resp = Response::new(resp.status_code())
                    .header("access-control-allow-origin", "*")
                    .body(resp.body_bytes().clone());
            } else {
                resp = Response::new(resp.status_code())
                    .header("access-control-allow-origin", &origin)
                    .header("access-control-allow-credentials", "true")
                    .body(resp.body_bytes().clone());
            }

            resp
        })
    }
}

/// Logging middleware.
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn handle<'a>(
        &'a self,
        req: Request,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        let method = req.method().clone();
        let path = req.path().to_string();

        Box::pin(async move {
            let start = std::time::Instant::now();
            let resp = next.run(req).await;
            let duration = start.elapsed();

            #[cfg(feature = "tracing")]
            tracing::info!(
                method = %method,
                path = %path,
                status = %resp.status_code().as_u16(),
                duration_ms = %duration.as_millis(),
                "request"
            );

            #[cfg(not(feature = "tracing"))]
            eprintln!(
                "{} {} {} {}ms",
                method,
                path,
                resp.status_code().as_u16(),
                duration.as_millis()
            );

            resp
        })
    }
}
