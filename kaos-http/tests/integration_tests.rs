//! Integration tests for kaos-http

use kaos_http::{HttpServer, Request, Response, Router};
use kaos_http::middleware::{CorsMiddleware, Middleware, Next};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Test basic GET request
#[tokio::test]
async fn test_basic_get() {
    let router = Router::new()
        .get("/", |_| async { Response::ok().text("Hello, World!") });

    let addr = spawn_server(router).await;

    let resp = reqwest::get(format!("http://{}/", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "Hello, World!");
}

/// Test JSON response
#[tokio::test]
async fn test_json_response() {
    let router = Router::new()
        .get("/api/status", |_| async {
            Response::ok().json(&serde_json::json!({
                "status": "ok",
                "version": "1.0.0"
            }))
        });

    let addr = spawn_server(router).await;

    let resp = reqwest::get(format!("http://{}/api/status", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert!(resp.headers().get("content-type").unwrap().to_str().unwrap().contains("application/json"));

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["version"], "1.0.0");
}

/// Test path parameters
#[tokio::test]
async fn test_path_params() {
    let router = Router::new()
        .get("/users/:id", |req: Request| async move {
            let id = req.param("id").unwrap_or("unknown");
            Response::ok().json(&serde_json::json!({ "user_id": id }))
        });

    let addr = spawn_server(router).await;

    let resp = reqwest::get(format!("http://{}/users/12345", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["user_id"], "12345");
}

/// Test multiple path parameters
#[tokio::test]
async fn test_multiple_path_params() {
    let router = Router::new()
        .get("/users/:user_id/posts/:post_id", |req: Request| async move {
            let user_id = req.param("user_id").unwrap_or("?");
            let post_id = req.param("post_id").unwrap_or("?");
            Response::ok().json(&serde_json::json!({
                "user_id": user_id,
                "post_id": post_id
            }))
        });

    let addr = spawn_server(router).await;

    let resp = reqwest::get(format!("http://{}/users/42/posts/99", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["user_id"], "42");
    assert_eq!(json["post_id"], "99");
}

/// Test POST with JSON body
#[tokio::test]
async fn test_post_json() {
    let router = Router::new()
        .post("/api/echo", |req: Request| async move {
            match req.json::<serde_json::Value>() {
                Ok(body) => Response::ok().json(&serde_json::json!({ "received": body })),
                Err(e) => Response::bad_request().json(&serde_json::json!({
                    "error": format!("invalid json: {}", e)
                })),
            }
        });

    let addr = spawn_server(router).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/api/echo", addr))
        .json(&serde_json::json!({ "message": "hello" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["received"]["message"], "hello");
}

/// Test 404 Not Found
#[tokio::test]
async fn test_not_found() {
    let router = Router::new()
        .get("/exists", |_| async { Response::ok().text("found") });

    let addr = spawn_server(router).await;

    let resp = reqwest::get(format!("http://{}/does-not-exist", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

/// Test 405 Method Not Allowed
#[tokio::test]
async fn test_method_not_allowed() {
    let router = Router::new()
        .get("/resource", |_| async { Response::ok().text("ok") });

    let addr = spawn_server(router).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/resource", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 405);
}

/// Test query parameters
#[tokio::test]
async fn test_query_params() {
    let router = Router::new()
        .get("/search", |req: Request| async move {
            let q = req.query_param("q").unwrap_or("none");
            let page = req.query_param("page").unwrap_or("1");
            Response::ok().json(&serde_json::json!({
                "query": q,
                "page": page
            }))
        });

    let addr = spawn_server(router).await;

    let resp = reqwest::get(format!("http://{}/search?q=rust&page=2", addr))
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["query"], "rust");
    assert_eq!(json["page"], "2");
}

/// Test different HTTP methods
#[tokio::test]
async fn test_http_methods() {
    let router = Router::new()
        .get("/resource", |_| async { Response::ok().json(&serde_json::json!({"method": "GET"})) })
        .post("/resource", |_| async { Response::created().json(&serde_json::json!({"method": "POST"})) })
        .put("/resource", |_| async { Response::ok().json(&serde_json::json!({"method": "PUT"})) })
        .delete("/resource", |_| async { Response::no_content() });

    let addr = spawn_server(router).await;
    let client = reqwest::Client::new();

    // GET
    let resp = client.get(format!("http://{}/resource", addr)).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["method"], "GET");

    // POST
    let resp = client.post(format!("http://{}/resource", addr)).send().await.unwrap();
    assert_eq!(resp.status(), 201);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["method"], "POST");

    // PUT
    let resp = client.put(format!("http://{}/resource", addr)).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["method"], "PUT");

    // DELETE
    let resp = client.delete(format!("http://{}/resource", addr)).send().await.unwrap();
    assert_eq!(resp.status(), 204);
}

/// Test response status codes
#[tokio::test]
async fn test_status_codes() {
    let router = Router::new()
        .get("/ok", |_| async { Response::ok().text("ok") })
        .get("/created", |_| async { Response::created().text("created") })
        .get("/bad-request", |_| async { Response::bad_request().text("bad") })
        .get("/unauthorized", |_| async { Response::unauthorized().text("unauth") })
        .get("/forbidden", |_| async { Response::forbidden().text("forbidden") })
        .get("/error", |_| async { Response::internal_error().text("error") });

    let addr = spawn_server(router).await;

    assert_eq!(reqwest::get(format!("http://{}/ok", addr)).await.unwrap().status(), 200);
    assert_eq!(reqwest::get(format!("http://{}/created", addr)).await.unwrap().status(), 201);
    assert_eq!(reqwest::get(format!("http://{}/bad-request", addr)).await.unwrap().status(), 400);
    assert_eq!(reqwest::get(format!("http://{}/unauthorized", addr)).await.unwrap().status(), 401);
    assert_eq!(reqwest::get(format!("http://{}/forbidden", addr)).await.unwrap().status(), 403);
    assert_eq!(reqwest::get(format!("http://{}/error", addr)).await.unwrap().status(), 500);
}

/// Test custom middleware
#[tokio::test]
async fn test_middleware() {
    let counter = Arc::new(AtomicUsize::new(0));

    struct CounterMiddleware {
        counter: Arc<AtomicUsize>,
    }

    impl Middleware for CounterMiddleware {
        fn handle<'a>(
            &'a self,
            req: Request,
            next: Next<'a>,
        ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            next.run(req)
        }
    }

    let router = Router::new()
        .get("/", |_| async { Response::ok().text("ok") });

    let addr = spawn_server_with_middleware(
        router,
        CounterMiddleware { counter: counter.clone() },
    ).await;

    // Make 3 requests
    for _ in 0..3 {
        reqwest::get(format!("http://{}/", addr)).await.unwrap();
    }

    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

/// Test CORS middleware
#[tokio::test]
async fn test_cors_middleware() {
    let router = Router::new()
        .get("/api/data", |_| async { Response::ok().json(&serde_json::json!({"data": "test"})) })
        .options("/api/data", |_| async { Response::no_content() });

    let addr = spawn_server_with_middleware(router, CorsMiddleware::permissive()).await;

    let client = reqwest::Client::new();

    // OPTIONS preflight
    let resp = client
        .request(reqwest::Method::OPTIONS, format!("http://{}/api/data", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 204);
    assert!(resp.headers().get("access-control-allow-origin").is_some());

    // GET with CORS
    let resp = client
        .get(format!("http://{}/api/data", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert!(resp.headers().get("access-control-allow-origin").is_some());
}

/// Test router mounting
#[tokio::test]
async fn test_router_mount() {
    let api_router = Router::new()
        .get("/users", |_| async { Response::ok().json(&serde_json::json!({"users": []})) })
        .get("/users/:id", |req: Request| async move {
            let id = req.param("id").unwrap_or("?");
            Response::ok().json(&serde_json::json!({"user_id": id}))
        });

    let main_router = Router::new()
        .get("/", |_| async { Response::ok().text("home") })
        .mount("/api/v1", api_router);

    let addr = spawn_server(main_router).await;

    // Root
    let resp = reqwest::get(format!("http://{}/", addr)).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "home");

    // Mounted /api/v1/users
    let resp = reqwest::get(format!("http://{}/api/v1/users", addr)).await.unwrap();
    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert!(json["users"].is_array());

    // Mounted /api/v1/users/:id
    let resp = reqwest::get(format!("http://{}/api/v1/users/123", addr)).await.unwrap();
    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["user_id"], "123");
}

/// Test concurrent requests
#[tokio::test]
async fn test_concurrent_requests() {
    let router = Router::new()
        .get("/slow", |_| async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Response::ok().text("done")
        });

    let addr = spawn_server(router).await;

    // Make 10 concurrent requests
    let mut handles = vec![];
    for _ in 0..10 {
        let url = format!("http://{}/slow", addr);
        handles.push(tokio::spawn(async move {
            reqwest::get(&url).await.unwrap()
        }));
    }

    // All should complete successfully
    for handle in handles {
        let resp = handle.await.unwrap();
        assert_eq!(resp.status(), 200);
    }
}

/// Test large request body
#[tokio::test]
async fn test_large_body() {
    let router = Router::new()
        .post("/upload", |req: Request| async move {
            let len = req.body().len();
            Response::ok().json(&serde_json::json!({"size": len}))
        });

    let addr = spawn_server(router).await;

    let client = reqwest::Client::new();
    let large_body = vec![b'x'; 100_000]; // 100KB

    let resp = client
        .post(format!("http://{}/upload", addr))
        .body(large_body.clone())
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["size"], 100_000);
}

/// Test custom headers
#[tokio::test]
async fn test_custom_headers() {
    let router = Router::new()
        .get("/headers", |req: Request| async move {
            let custom = req.header("x-custom-header").unwrap_or("none");
            Response::ok()
                .header("x-response-header", "value")
                .json(&serde_json::json!({"received": custom}))
        });

    let addr = spawn_server(router).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/headers", addr))
        .header("x-custom-header", "test-value")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers().get("x-response-header").unwrap(), "value");

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["received"], "test-value");
}

// Helper function to spawn a test server
async fn spawn_server(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let addr_str = addr.to_string();
    tokio::spawn(async move {
        HttpServer::bind(addr)
            .router(router)
            .serve()
            .await
            .unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;
    addr_str
}

// Helper function to spawn a test server with middleware
async fn spawn_server_with_middleware<M: Middleware + 'static>(router: Router, middleware: M) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let addr_str = addr.to_string();
    tokio::spawn(async move {
        HttpServer::bind(addr)
            .middleware(middleware)
            .router(router)
            .serve()
            .await
            .unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;
    addr_str
}
