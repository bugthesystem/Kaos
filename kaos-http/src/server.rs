//! HTTP/2 server implementation.

use crate::middleware::{Middleware, MiddlewareChain, Next};
use crate::{Request, Response, Router};
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// HTTP server configuration.
pub struct HttpServer {
    addr: SocketAddr,
    router: Router,
    middleware: MiddlewareChain,
    http2_only: bool,
}

impl HttpServer {
    /// Bind to address.
    pub fn bind(addr: impl std::net::ToSocketAddrs) -> Self {
        let addr = addr
            .to_socket_addrs()
            .expect("invalid address")
            .next()
            .expect("no address");

        Self {
            addr,
            router: Router::new(),
            middleware: MiddlewareChain::new(),
            http2_only: false,
        }
    }

    /// Set router.
    pub fn router(mut self, router: Router) -> Self {
        self.router = router;
        self
    }

    /// Add middleware.
    pub fn middleware<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middleware = self.middleware.add(middleware);
        self
    }

    /// Use HTTP/2 only (no HTTP/1.1 fallback).
    pub fn http2_only(mut self) -> Self {
        self.http2_only = true;
        self
    }

    /// Start serving requests.
    pub async fn serve(self) -> crate::Result<()> {
        let listener = TcpListener::bind(self.addr).await?;

        #[cfg(feature = "tracing")]
        tracing::info!("listening on {}", self.addr);

        #[cfg(not(feature = "tracing"))]
        eprintln!("kaos-http listening on {}", self.addr);

        let router = Arc::new(self.router);
        let middleware = Arc::new(self.middleware);
        let http2_only = self.http2_only;

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let router = Arc::clone(&router);
            let middleware = Arc::clone(&middleware);

            tokio::spawn(async move {
                let service = service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                    let router = Arc::clone(&router);
                    let middleware = Arc::clone(&middleware);

                    async move {
                        let resp = handle_request(req, remote_addr, &router, &middleware).await;
                        Ok::<_, Infallible>(resp.into_hyper())
                    }
                });

                let result = if http2_only {
                    http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                        .serve_connection(io, service)
                        .await
                } else {
                    http1::Builder::new()
                        .serve_connection(io, service)
                        .with_upgrades()
                        .await
                };

                if let Err(e) = result {
                    #[cfg(feature = "tracing")]
                    tracing::error!("connection error: {}", e);

                    #[cfg(not(feature = "tracing"))]
                    eprintln!("connection error: {}", e);
                }
            });
        }
    }

    /// Get bound address.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}

async fn handle_request(
    req: hyper::Request<hyper::body::Incoming>,
    remote_addr: SocketAddr,
    router: &Router,
    middleware: &MiddlewareChain,
) -> Response {
    // Collect body
    let (parts, body) = req.into_parts();
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => Bytes::new(),
    };

    // Build request
    let mut request = Request::new(
        parts.method.clone(),
        parts.uri.clone(),
        parts.version,
        parts.headers,
        body_bytes,
    );
    request.set_remote_addr(remote_addr);

    // Find route
    let path = request.path().to_string();
    let method = request.method().clone();

    match router.find(method.clone(), &path) {
        Some((handler, params)) => {
            request.set_params(params);

            // Build middleware chain
            let middlewares = middleware.middlewares();
            if middlewares.is_empty() {
                // No middleware, call handler directly
                handler(request).await
            } else {
                // Build chain from end to start
                let mut next = Next::handler(handler);
                for mw in middlewares.iter().rev() {
                    next = Next::middleware(mw.as_ref(), next);
                }
                next.run(request).await
            }
        }
        None => {
            // Check if path exists with different method
            if router.path_exists(&path) {
                Response::method_not_allowed().json(&serde_json::json!({
                    "error": "method not allowed",
                    "method": method.as_str(),
                    "path": path
                }))
            } else {
                Response::not_found().json(&serde_json::json!({
                    "error": "not found",
                    "path": path
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_create() {
        let server = HttpServer::bind("127.0.0.1:8080");
        assert_eq!(server.local_addr().port(), 8080);
    }
}
