//! HTTP router with path parameters.

use crate::{Request, Response};
use http::Method;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Async handler function type.
pub type Handler = Arc<
    dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync,
>;

/// A single route entry.
#[derive(Clone)]
pub struct Route {
    method: Method,
    pattern: String,
    segments: Vec<Segment>,
    handler: Handler,
}

#[derive(Clone, Debug)]
enum Segment {
    Static(String),
    Param(String),
    Wildcard,
}

impl Route {
    fn new(method: Method, pattern: &str, handler: Handler) -> Self {
        let segments = parse_pattern(pattern);
        Self {
            method,
            pattern: pattern.to_string(),
            segments,
            handler,
        }
    }

    fn matches(&self, method: &Method, path: &str) -> Option<HashMap<String, String>> {
        if self.method != *method {
            return None;
        }

        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut params = HashMap::new();
        let mut path_idx = 0;

        for segment in &self.segments {
            match segment {
                Segment::Static(s) => {
                    if path_segments.get(path_idx) != Some(&s.as_str()) {
                        return None;
                    }
                    path_idx += 1;
                }
                Segment::Param(name) => {
                    let value = path_segments.get(path_idx)?;
                    params.insert(name.clone(), value.to_string());
                    path_idx += 1;
                }
                Segment::Wildcard => {
                    // Match rest of path
                    let rest: Vec<&str> = path_segments[path_idx..].to_vec();
                    params.insert("*".to_string(), rest.join("/"));
                    return Some(params);
                }
            }
        }

        // All segments must be consumed
        if path_idx == path_segments.len() {
            Some(params)
        } else {
            None
        }
    }
}

fn parse_pattern(pattern: &str) -> Vec<Segment> {
    pattern
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s == "*" {
                Segment::Wildcard
            } else if let Some(name) = s.strip_prefix(':') {
                Segment::Param(name.to_string())
            } else {
                Segment::Static(s.to_string())
            }
        })
        .collect()
}

/// HTTP router.
#[derive(Default, Clone)]
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    /// Create new router.
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add GET route.
    pub fn get<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.route(Method::GET, path, handler)
    }

    /// Add POST route.
    pub fn post<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.route(Method::POST, path, handler)
    }

    /// Add PUT route.
    pub fn put<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.route(Method::PUT, path, handler)
    }

    /// Add DELETE route.
    pub fn delete<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.route(Method::DELETE, path, handler)
    }

    /// Add OPTIONS route.
    pub fn options<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.route(Method::OPTIONS, path, handler)
    }

    /// Add route with method.
    pub fn route<F, Fut>(mut self, method: Method, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let handler: Handler = Arc::new(move |req| Box::pin(handler(req)));
        self.routes.push(Route::new(method, path, handler));
        self
    }

    /// Mount another router at prefix.
    pub fn mount(mut self, prefix: &str, other: Router) -> Self {
        let prefix = prefix.trim_end_matches('/');
        for mut route in other.routes {
            route.pattern = format!("{}{}", prefix, route.pattern);
            route.segments = parse_pattern(&route.pattern);
            self.routes.push(route);
        }
        self
    }

    /// Find matching route.
    pub fn find(&self, method: Method, path: &str) -> Option<(&Handler, HashMap<String, String>)> {
        for route in &self.routes {
            if let Some(params) = route.matches(&method, path) {
                return Some((&route.handler, params));
            }
        }
        None
    }

    /// Check if any route matches path (any method).
    pub fn path_exists(&self, path: &str) -> bool {
        for route in &self.routes {
            // Check pattern match ignoring method
            let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            let mut path_idx = 0;
            let mut matches = true;

            for segment in &route.segments {
                match segment {
                    Segment::Static(s) => {
                        if path_segments.get(path_idx) != Some(&s.as_str()) {
                            matches = false;
                            break;
                        }
                        path_idx += 1;
                    }
                    Segment::Param(_) => {
                        if path_segments.get(path_idx).is_none() {
                            matches = false;
                            break;
                        }
                        path_idx += 1;
                    }
                    Segment::Wildcard => {
                        return true;
                    }
                }
            }

            if matches && path_idx == path_segments.len() {
                return true;
            }
        }
        false
    }

    /// Get all routes (for debugging/introspection).
    pub fn routes(&self) -> impl Iterator<Item = (&Method, &str)> {
        self.routes.iter().map(|r| (&r.method, r.pattern.as_str()))
    }
}
