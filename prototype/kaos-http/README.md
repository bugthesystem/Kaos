# kaos-http

A minimal, fast HTTP/1.1 and HTTP/2 server for Kaos, built on hyper.

## Features

- **HTTP/2 support** - Full HTTP/2 via hyper
- **Path routing** - Express-style routing with parameters
- **Middleware** - Composable middleware chain (CORS, logging, auth)
- **JSON handling** - Built-in JSON request/response support
- **Async** - Fully async with Tokio runtime

## Quick Start

```rust
use kaos_http::{HttpServer, Router, Request, Response};

#[tokio::main]
async fn main() {
    let router = Router::new()
        .get("/", |_| async { Response::ok().text("Hello, World!") })
        .get("/api/status", |_| async {
            Response::ok().json(&serde_json::json!({ "status": "ok" }))
        })
        .get("/users/:id", |req| async move {
            let id = req.params().get("id").unwrap();
            Response::ok().json(&serde_json::json!({ "user_id": id }))
        });

    HttpServer::bind("127.0.0.1:8080")
        .router(router)
        .serve()
        .await
        .unwrap();
}
```

## Routing

```rust
let router = Router::new()
    .get("/users", list_users)
    .get("/users/:id", get_user)
    .post("/users", create_user)
    .put("/users/:id", update_user)
    .delete("/users/:id", delete_user);

// Mount sub-routers
let api = Router::new()
    .get("/status", get_status)
    .get("/config", get_config);

let router = Router::new().mount("/api", api);
```

## Middleware

```rust
use kaos_http::middleware::{CorsMiddleware, LoggingMiddleware};

HttpServer::bind("127.0.0.1:8080")
    .router(router)
    .middleware(LoggingMiddleware)
    .middleware(CorsMiddleware::permissive())
    .serve()
    .await?;
```

### Custom Middleware

```rust
use kaos_http::middleware::{Middleware, Next};
use kaos_http::{Request, Response};
use std::future::Future;
use std::pin::Pin;

struct AuthMiddleware;

impl Middleware for AuthMiddleware {
    fn handle<'a>(
        &'a self,
        req: Request,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            if req.headers().get("authorization").is_none() {
                return Response::unauthorized();
            }
            next.run(req).await
        })
    }
}
```

## Response Builder

```rust
// Status shortcuts
Response::ok()
Response::created()
Response::no_content()
Response::bad_request()
Response::unauthorized()
Response::forbidden()
Response::not_found()
Response::internal_error()

// Body types
Response::ok().text("Hello")
Response::ok().html("<h1>Hello</h1>")
Response::ok().json(&data)
Response::ok().body(bytes)

// Headers
Response::ok()
    .header("X-Custom", "value")
    .header("Cache-Control", "no-cache")
    .json(&data)

// Error responses
Response::error(StatusCode::BAD_REQUEST, "Invalid input")
Response::validation_error(&["field1 required", "field2 invalid"])
```

## Request Handling

```rust
async fn handler(req: Request) -> Response {
    // Path parameters
    let id = req.params().get("id").unwrap();

    // Query parameters
    let limit = req.query().get("limit");

    // Headers
    let auth = req.headers().get("authorization");

    // JSON body
    let body: MyRequest = req.json().unwrap();

    Response::ok().json(&MyResponse { ... })
}
```

## Features

| Feature | Description |
|---------|-------------|
| `tls` | Enable TLS/HTTPS support |
| `tracing` | Enable structured logging |

## License

MIT License
