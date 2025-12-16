//! Simple HTTP/2 server example.
//!
//! Run with: `cargo run --example hello_http2`
//! Test with: `curl http://localhost:8080/` or `curl http://localhost:8080/api/status`

use kaos_http::{HttpServer, Request, Response, Router};
use kaos_http::middleware::LoggingMiddleware;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build router
    let router = Router::new()
        .get("/", |_| async {
            Response::ok().text("Hello from kaos-http!")
        })
        .get("/api/status", |_| async {
            Response::ok().json(&serde_json::json!({
                "status": "ok",
                "version": env!("CARGO_PKG_VERSION")
            }))
        })
        .get("/api/users/:id", |req: Request| async move {
            let id = req.param("id").unwrap_or("unknown");
            Response::ok().json(&serde_json::json!({
                "user_id": id,
                "name": "Example User"
            }))
        })
        .post("/api/echo", |req: Request| async move {
            match req.json::<serde_json::Value>() {
                Ok(body) => Response::ok().json(&serde_json::json!({
                    "received": body
                })),
                Err(e) => Response::bad_request().json(&serde_json::json!({
                    "error": format!("invalid json: {}", e)
                })),
            }
        });

    // Start server
    println!("Starting server on http://127.0.0.1:8080");

    HttpServer::bind("127.0.0.1:8080")
        .middleware(LoggingMiddleware)
        .router(router)
        .serve()
        .await?;

    Ok(())
}
