//! # kaos-http
//!
//! HTTP/1.1 and HTTP/2 server for Kaos.
//!
//! ## Features
//!
//! - **HTTP/2**: Full HTTP/2 support via hyper
//! - **Routing**: Path-based routing with params
//! - **Middleware**: Composable middleware chain
//! - **JSON**: Built-in JSON request/response handling
//! - **Async**: Tokio-based async runtime
//!
//! ## Example
//!
//! ```rust,ignore
//! use kaos_http::{HttpServer, Router, Request, Response};
//!
//! #[tokio::main]
//! async fn main() {
//!     let router = Router::new()
//!         .get("/", |_| async { Response::ok().text("Hello, World!") })
//!         .get("/api/status", |_| async {
//!             Response::ok().json(&serde_json::json!({ "status": "ok" }))
//!         });
//!
//!     HttpServer::bind("127.0.0.1:8080")
//!         .router(router)
//!         .serve()
//!         .await
//!         .unwrap();
//! }
//! ```

mod error;
pub mod middleware;
mod request;
mod response;
mod router;
mod server;

pub use error::{HttpError, Result};
pub use middleware::{Middleware, Next};
pub use request::Request;
pub use response::Response;
pub use router::{Handler, Route, Router};
pub use server::HttpServer;

// Re-export http types
pub use http::{Method, StatusCode};

#[cfg(test)]
mod tests {
    use super::*;

    // Response tests
    #[test]
    fn test_response_builder() {
        let resp = Response::ok().text("hello");
        assert_eq!(resp.status_code(), StatusCode::OK);
    }

    #[test]
    fn test_response_json() {
        let resp = Response::ok().json(&serde_json::json!({"key": "value"}));
        assert_eq!(resp.status_code(), StatusCode::OK);
        assert!(resp.content_type_header().unwrap().contains("application/json"));
    }

    #[test]
    fn test_response_status_shortcuts() {
        assert_eq!(Response::ok().status_code(), StatusCode::OK);
        assert_eq!(Response::created().status_code(), StatusCode::CREATED);
        assert_eq!(Response::no_content().status_code(), StatusCode::NO_CONTENT);
        assert_eq!(Response::bad_request().status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(Response::unauthorized().status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(Response::forbidden().status_code(), StatusCode::FORBIDDEN);
        assert_eq!(Response::not_found().status_code(), StatusCode::NOT_FOUND);
        assert_eq!(Response::method_not_allowed().status_code(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(Response::internal_error().status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_response_headers() {
        let resp = Response::ok()
            .header("x-custom", "value")
            .header("x-another", "test");

        let headers = resp.headers();
        assert_eq!(headers.get("x-custom").unwrap(), "value");
        assert_eq!(headers.get("x-another").unwrap(), "test");
    }

    #[test]
    fn test_response_text() {
        let resp = Response::ok().text("hello world");
        assert_eq!(resp.body_bytes().as_ref(), b"hello world");
        assert!(resp.content_type_header().unwrap().contains("text/plain"));
    }

    #[test]
    fn test_response_html() {
        let resp = Response::ok().html("<h1>Hello</h1>");
        assert_eq!(resp.body_bytes().as_ref(), b"<h1>Hello</h1>");
        assert!(resp.content_type_header().unwrap().contains("text/html"));
    }

    #[test]
    fn test_response_error() {
        let resp = Response::error(StatusCode::BAD_REQUEST, "invalid input");
        assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
        let body = String::from_utf8_lossy(resp.body_bytes());
        assert!(body.contains("invalid input"));
    }

    #[test]
    fn test_response_validation_error() {
        let resp = Response::validation_error(&["field1 required", "field2 invalid"]);
        assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
        let body = String::from_utf8_lossy(resp.body_bytes());
        assert!(body.contains("validation failed"));
    }

    // Router tests
    #[test]
    fn test_router_basic() {
        let router = Router::new()
            .get("/", |_| async { Response::ok().text("root") })
            .get("/api/status", |_| async { Response::ok().text("status") });

        assert!(router.find(Method::GET, "/").is_some());
        assert!(router.find(Method::GET, "/api/status").is_some());
        assert!(router.find(Method::POST, "/").is_none());
    }

    #[test]
    fn test_router_params() {
        let router = Router::new()
            .get("/users/:id", |_| async { Response::ok().text("user") })
            .get("/users/:id/posts/:post_id", |_| async { Response::ok().text("post") });

        let (_, params) = router.find(Method::GET, "/users/123").unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));

        let (_, params) = router.find(Method::GET, "/users/456/posts/789").unwrap();
        assert_eq!(params.get("id"), Some(&"456".to_string()));
        assert_eq!(params.get("post_id"), Some(&"789".to_string()));
    }

    #[test]
    fn test_router_methods() {
        let router = Router::new()
            .get("/resource", |_| async { Response::ok() })
            .post("/resource", |_| async { Response::created() })
            .put("/resource", |_| async { Response::ok() })
            .delete("/resource", |_| async { Response::no_content() })
            .options("/resource", |_| async { Response::no_content() });

        assert!(router.find(Method::GET, "/resource").is_some());
        assert!(router.find(Method::POST, "/resource").is_some());
        assert!(router.find(Method::PUT, "/resource").is_some());
        assert!(router.find(Method::DELETE, "/resource").is_some());
        assert!(router.find(Method::OPTIONS, "/resource").is_some());
        assert!(router.find(Method::PATCH, "/resource").is_none());
    }

    #[test]
    fn test_router_mount() {
        let sub_router = Router::new()
            .get("/users", |_| async { Response::ok() })
            .get("/users/:id", |_| async { Response::ok() });

        let router = Router::new()
            .get("/", |_| async { Response::ok() })
            .mount("/api", sub_router);

        assert!(router.find(Method::GET, "/").is_some());
        assert!(router.find(Method::GET, "/api/users").is_some());
        assert!(router.find(Method::GET, "/api/users/123").is_some());
        assert!(router.find(Method::GET, "/users").is_none());
    }

    #[test]
    fn test_router_path_exists() {
        let router = Router::new()
            .get("/resource", |_| async { Response::ok() })
            .post("/resource", |_| async { Response::ok() });

        assert!(router.path_exists("/resource"));
        assert!(!router.path_exists("/nonexistent"));
    }

    // Error tests
    #[test]
    fn test_error_status_codes() {
        assert_eq!(HttpError::NotFound { method: "GET".into(), path: "/".into() }.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(HttpError::MethodNotAllowed { method: "POST".into(), path: "/".into() }.status_code(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(HttpError::BadRequest("bad".into()).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(HttpError::Unauthorized.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(HttpError::Forbidden.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(HttpError::Internal("err".into()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_display() {
        let err = HttpError::NotFound { method: "GET".into(), path: "/test".into() };
        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("/test"));

        let err = HttpError::BadRequest("invalid data".into());
        assert!(err.to_string().contains("invalid data"));
    }
}
