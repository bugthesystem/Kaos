//! HTTP request wrapper.

use bytes::Bytes;
use http::{Extensions, HeaderMap, Method, Uri, Version};
use std::collections::HashMap;
use std::net::SocketAddr;

/// HTTP request with body and extensions.
#[derive(Debug)]
pub struct Request {
    method: Method,
    uri: Uri,
    version: Version,
    headers: HeaderMap,
    body: Bytes,
    params: HashMap<String, String>,
    query: HashMap<String, String>,
    extensions: Extensions,
    remote_addr: Option<SocketAddr>,
}

impl Request {
    /// Create request from hyper parts.
    pub fn new(
        method: Method,
        uri: Uri,
        version: Version,
        headers: HeaderMap,
        body: Bytes,
    ) -> Self {
        let query = parse_query(uri.query());

        Self {
            method,
            uri,
            version,
            headers,
            body,
            params: HashMap::new(),
            query,
            extensions: Extensions::new(),
            remote_addr: None,
        }
    }

    /// HTTP method.
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Request URI.
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    /// Request path.
    pub fn path(&self) -> &str {
        self.uri.path()
    }

    /// HTTP version.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Request headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get header value.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Request body bytes.
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    /// Parse body as JSON.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    /// Body as string.
    pub fn text(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }

    /// Path parameters.
    pub fn params(&self) -> &HashMap<String, String> {
        &self.params
    }

    /// Get path parameter.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(|s| s.as_str())
    }

    /// Set path parameters.
    pub fn set_params(&mut self, params: HashMap<String, String>) {
        self.params = params;
    }

    /// Query parameters.
    pub fn query(&self) -> &HashMap<String, String> {
        &self.query
    }

    /// Get query parameter.
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(|s| s.as_str())
    }

    /// Extensions for storing request-scoped data.
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    /// Mutable extensions.
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    /// Get extension.
    pub fn ext<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions.get::<T>()
    }

    /// Insert extension.
    pub fn insert_ext<T: Clone + Send + Sync + 'static>(&mut self, val: T) {
        self.extensions.insert(val);
    }

    /// Remote address.
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// Set remote address.
    pub fn set_remote_addr(&mut self, addr: SocketAddr) {
        self.remote_addr = Some(addr);
    }

    /// Content-Type header.
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    /// Authorization header.
    pub fn authorization(&self) -> Option<&str> {
        self.header("authorization")
    }

    /// Extract Bearer token from Authorization header.
    pub fn bearer_token(&self) -> Option<&str> {
        self.authorization()
            .and_then(|auth| auth.strip_prefix("Bearer "))
    }
}

fn parse_query(query: Option<&str>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(q) = query {
        for pair in q.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            }
        }
    }
    map
}
