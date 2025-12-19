//! Rate limiting middleware for kaosnet.
//!
//! Provides token bucket and sliding window rate limiters for protecting
//! game server resources from abuse.
//!
//! # Example
//!
//! ```rust,ignore
//! use kaosnet::ratelimit::{RateLimiter, RateLimitConfig};
//!
//! let limiter = RateLimiter::new(RateLimitConfig {
//!     requests_per_second: 100,
//!     burst_size: 20,
//! });
//!
//! // Check if request is allowed
//! if limiter.check("user123") {
//!     // Process request
//! } else {
//!     // Rate limited, return error
//! }
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub requests_per_second: u32,
    /// Burst size (max tokens that can accumulate)
    pub burst_size: u32,
    /// Time window for sliding window limiter (default: 1 second)
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            burst_size: 20,
            window_size: Duration::from_secs(1),
        }
    }
}

impl RateLimitConfig {
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            requests_per_second,
            burst_size: requests_per_second / 5, // 20% burst
            window_size: Duration::from_secs(1),
        }
    }

    pub fn with_burst(mut self, burst_size: u32) -> Self {
        self.burst_size = burst_size;
        self
    }

    pub fn with_window(mut self, window: Duration) -> Self {
        self.window_size = window;
        self
    }
}

/// Token bucket state for a single client
#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    last_update: Instant,
}

impl TokenBucket {
    fn new(initial_tokens: f64) -> Self {
        Self {
            tokens: initial_tokens,
            last_update: Instant::now(),
        }
    }

    /// Try to consume a token, returns true if successful
    fn try_consume(&mut self, rate: f64, burst: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();

        // Add tokens based on elapsed time
        self.tokens = (self.tokens + elapsed * rate).min(burst);
        self.last_update = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get remaining tokens
    fn remaining(&self, rate: f64, burst: f64) -> f64 {
        let elapsed = self.last_update.elapsed().as_secs_f64();
        (self.tokens + elapsed * rate).min(burst)
    }
}

/// Token bucket rate limiter - allows bursts within limits
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: DashMap<String, TokenBucket>,
    global_counter: AtomicU64,
    global_window_start: std::sync::Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: DashMap::new(),
            global_counter: AtomicU64::new(0),
            global_window_start: std::sync::Mutex::new(Instant::now()),
        }
    }

    /// Check if a request from a client is allowed
    pub fn check(&self, client_id: &str) -> bool {
        let rate = self.config.requests_per_second as f64;
        let burst = self.config.burst_size as f64;

        let mut entry = self.buckets
            .entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(burst));

        entry.try_consume(rate, burst)
    }

    /// Check rate limit and return result with metadata
    pub fn check_with_info(&self, client_id: &str) -> RateLimitResult {
        let rate = self.config.requests_per_second as f64;
        let burst = self.config.burst_size as f64;

        let mut entry = self.buckets
            .entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(burst));

        let allowed = entry.try_consume(rate, burst);
        let remaining = entry.remaining(rate, burst) as u32;

        RateLimitResult {
            allowed,
            remaining,
            limit: self.config.requests_per_second,
            reset_after_ms: if allowed { 0 } else { (1000.0 / rate) as u64 },
        }
    }

    /// Check global rate limit (all clients combined)
    pub fn check_global(&self, max_requests_per_second: u64) -> bool {
        // Use unwrap_or_else to handle poisoned mutex gracefully
        // If the mutex is poisoned (another thread panicked while holding it),
        // we still try to continue by resetting the window
        let mut window_start = self.global_window_start
            .lock()
            .unwrap_or_else(|poisoned| {
                // Recover from poisoned mutex - this is a best-effort approach
                // The rate limiting may be slightly off after recovery
                poisoned.into_inner()
            });
        let now = Instant::now();

        // Reset counter if window has passed
        if now.duration_since(*window_start) >= Duration::from_secs(1) {
            self.global_counter.store(0, Ordering::SeqCst);
            *window_start = now;
        }

        let count = self.global_counter.fetch_add(1, Ordering::SeqCst);
        count < max_requests_per_second
    }

    /// Get current tokens remaining for a client
    pub fn remaining(&self, client_id: &str) -> u32 {
        let rate = self.config.requests_per_second as f64;
        let burst = self.config.burst_size as f64;

        self.buckets
            .get(client_id)
            .map(|b| b.remaining(rate, burst) as u32)
            .unwrap_or(self.config.burst_size)
    }

    /// Clean up stale entries (clients that haven't been seen recently)
    pub fn cleanup(&self, max_age: Duration) {
        let now = Instant::now();
        self.buckets.retain(|_, bucket| {
            now.duration_since(bucket.last_update) < max_age
        });
    }

    /// Get number of tracked clients
    pub fn client_count(&self) -> usize {
        self.buckets.len()
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in current window
    pub remaining: u32,
    /// Maximum requests per window
    pub limit: u32,
    /// Milliseconds until rate limit resets
    pub reset_after_ms: u64,
}

/// Per-operation rate limiter with different limits for different operations
pub struct OperationRateLimiter {
    limiters: HashMap<String, RateLimiter>,
    default_limiter: RateLimiter,
}

impl OperationRateLimiter {
    pub fn new(default_config: RateLimitConfig) -> Self {
        Self {
            limiters: HashMap::new(),
            default_limiter: RateLimiter::new(default_config),
        }
    }

    /// Add a rate limiter for a specific operation
    pub fn add_operation(&mut self, operation: &str, config: RateLimitConfig) {
        self.limiters.insert(operation.to_string(), RateLimiter::new(config));
    }

    /// Check rate limit for a client performing an operation
    pub fn check(&self, client_id: &str, operation: &str) -> bool {
        self.limiters
            .get(operation)
            .unwrap_or(&self.default_limiter)
            .check(client_id)
    }

    /// Check rate limit with detailed result
    pub fn check_with_info(&self, client_id: &str, operation: &str) -> RateLimitResult {
        self.limiters
            .get(operation)
            .unwrap_or(&self.default_limiter)
            .check_with_info(client_id)
    }
}

/// Builder for common rate limit configurations
pub struct RateLimitPresets;

impl RateLimitPresets {
    /// Strict rate limit for sensitive operations (login, auth)
    pub fn strict() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_second: 5,
            burst_size: 3,
            window_size: Duration::from_secs(1),
        }
    }

    /// Standard rate limit for normal operations
    pub fn standard() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_second: 100,
            burst_size: 20,
            window_size: Duration::from_secs(1),
        }
    }

    /// Relaxed rate limit for read-heavy operations
    pub fn relaxed() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_second: 500,
            burst_size: 100,
            window_size: Duration::from_secs(1),
        }
    }

    /// Rate limit for real-time game data (high frequency)
    pub fn realtime() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_second: 60, // 60 updates per second (60Hz)
            burst_size: 10,
            window_size: Duration::from_secs(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_token_bucket_basic() {
        let config = RateLimitConfig::new(10).with_burst(5);
        let limiter = RateLimiter::new(config);

        // Burst should allow initial requests
        for _ in 0..5 {
            assert!(limiter.check("user1"));
        }

        // Should be rate limited after burst
        assert!(!limiter.check("user1"));
    }

    #[test]
    fn test_token_bucket_refill() {
        let config = RateLimitConfig::new(10).with_burst(2);
        let limiter = RateLimiter::new(config);

        // Use up burst
        assert!(limiter.check("user1"));
        assert!(limiter.check("user1"));
        assert!(!limiter.check("user1"));

        // Wait for refill (100ms should give us 1 token at 10/sec)
        thread::sleep(Duration::from_millis(150));

        // Should have tokens again
        assert!(limiter.check("user1"));
    }

    #[test]
    fn test_multiple_clients() {
        let config = RateLimitConfig::new(10).with_burst(3);
        let limiter = RateLimiter::new(config);

        // Each client has independent limits
        for _ in 0..3 {
            assert!(limiter.check("user1"));
            assert!(limiter.check("user2"));
        }

        // Both should be limited
        assert!(!limiter.check("user1"));
        assert!(!limiter.check("user2"));
    }

    #[test]
    fn test_check_with_info() {
        let config = RateLimitConfig::new(10).with_burst(3);
        let limiter = RateLimiter::new(config);

        let result = limiter.check_with_info("user1");
        assert!(result.allowed);
        assert_eq!(result.limit, 10);

        // Use remaining
        limiter.check("user1");
        limiter.check("user1");

        let result = limiter.check_with_info("user1");
        assert!(!result.allowed);
        assert!(result.reset_after_ms > 0);
    }

    #[test]
    fn test_operation_rate_limiter() {
        let mut limiter = OperationRateLimiter::new(RateLimitPresets::standard());
        limiter.add_operation("login", RateLimitPresets::strict());

        // Login has strict limits
        for _ in 0..3 {
            assert!(limiter.check("user1", "login"));
        }
        assert!(!limiter.check("user1", "login"));

        // Other operations use default limits
        for _ in 0..20 {
            assert!(limiter.check("user1", "other"));
        }
    }

    #[test]
    fn test_cleanup() {
        let config = RateLimitConfig::new(10).with_burst(5);
        let limiter = RateLimiter::new(config);

        limiter.check("user1");
        limiter.check("user2");
        assert_eq!(limiter.client_count(), 2);

        // Cleanup with short max age
        limiter.cleanup(Duration::from_millis(0));
        assert_eq!(limiter.client_count(), 0);
    }

    #[test]
    fn test_presets() {
        let strict = RateLimitPresets::strict();
        assert_eq!(strict.requests_per_second, 5);

        let standard = RateLimitPresets::standard();
        assert_eq!(standard.requests_per_second, 100);

        let relaxed = RateLimitPresets::relaxed();
        assert_eq!(relaxed.requests_per_second, 500);

        let realtime = RateLimitPresets::realtime();
        assert_eq!(realtime.requests_per_second, 60);
    }
}
