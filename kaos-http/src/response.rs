//! HTTP response builder.

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode};

/// HTTP response with builder pattern.
#[derive(Debug)]
pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl Response {
    /// Create response with status.
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        }
    }

    // Status shortcuts

    /// 200 OK
    pub fn ok() -> Self {
        Self::new(StatusCode::OK)
    }

    /// 201 Created
    pub fn created() -> Self {
        Self::new(StatusCode::CREATED)
    }

    /// 204 No Content
    pub fn no_content() -> Self {
        Self::new(StatusCode::NO_CONTENT)
    }

    /// 400 Bad Request
    pub fn bad_request() -> Self {
        Self::new(StatusCode::BAD_REQUEST)
    }

    /// 401 Unauthorized
    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED)
    }

    /// 403 Forbidden
    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN)
    }

    /// 404 Not Found
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND)
    }

    /// 405 Method Not Allowed
    pub fn method_not_allowed() -> Self {
        Self::new(StatusCode::METHOD_NOT_ALLOWED)
    }

    /// 500 Internal Server Error
    pub fn internal_error() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR)
    }

    // Builder methods

    /// Set status code.
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Set header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        if let (Ok(n), Ok(v)) = (
            http::HeaderName::try_from(name),
            HeaderValue::from_str(value),
        ) {
            self.headers.insert(n, v);
        }
        self
    }

    /// Set Content-Type.
    pub fn content_type(self, content_type: &str) -> Self {
        self.header("content-type", content_type)
    }

    /// Set body bytes.
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    /// Set body as text.
    pub fn text(self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.content_type("text/plain; charset=utf-8")
            .body(text)
    }

    /// Set body as HTML.
    pub fn html(self, html: impl Into<String>) -> Self {
        let html = html.into();
        self.content_type("text/html; charset=utf-8")
            .body(html)
    }

    /// Set body as JSON.
    pub fn json<T: serde::Serialize>(self, value: &T) -> Self {
        match serde_json::to_vec(value) {
            Ok(json) => self
                .content_type("application/json")
                .body(json),
            Err(_) => Self::internal_error()
                .content_type("application/json")
                .body(r#"{"error":"serialization failed"}"#),
        }
    }

    /// Set CORS headers for all origins.
    pub fn cors(self) -> Self {
        self.header("access-control-allow-origin", "*")
            .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS")
            .header("access-control-allow-headers", "Content-Type, Authorization")
    }

    /// Set CORS headers for specific origin.
    pub fn cors_origin(self, origin: &str) -> Self {
        self.header("access-control-allow-origin", origin)
            .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS")
            .header("access-control-allow-headers", "Content-Type, Authorization")
            .header("access-control-allow-credentials", "true")
    }

    // Getters

    /// Get status code.
    pub fn status_code(&self) -> StatusCode {
        self.status
    }

    /// Get headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get body.
    pub fn body_bytes(&self) -> &Bytes {
        &self.body
    }

    /// Get Content-Type header.
    pub fn content_type_header(&self) -> Option<&str> {
        self.headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
    }

    /// Build hyper response.
    pub fn into_hyper(self) -> hyper::Response<http_body_util::Full<Bytes>> {
        let mut builder = hyper::Response::builder().status(self.status);

        for (name, value) in &self.headers {
            builder = builder.header(name, value);
        }

        builder
            .body(http_body_util::Full::new(self.body))
            .unwrap_or_else(|_| {
                hyper::Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(http_body_util::Full::new(Bytes::new()))
                    .unwrap()
            })
    }
}

// Error response helpers
impl Response {
    /// Create error response with JSON body.
    pub fn error(status: StatusCode, message: &str) -> Self {
        Self::new(status).json(&serde_json::json!({
            "error": message
        }))
    }

    /// Create validation error response.
    pub fn validation_error(errors: &[&str]) -> Self {
        Self::bad_request().json(&serde_json::json!({
            "error": "validation failed",
            "details": errors
        }))
    }
}
