//! Prometheus metrics for KaosNet game server.
//!
//! Provides observability into server health and performance.
//!
//! # Example
//!
//! ```rust,ignore
//! use kaosnet::metrics::Metrics;
//!
//! let metrics = Metrics::new();
//! metrics.sessions_active.set(42);
//! metrics.http_requests_total.inc();
//!
//! // Get Prometheus text format
//! let output = metrics.gather();
//! ```

use prometheus::{
    Histogram, HistogramOpts, HistogramVec,
    IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
    TextEncoder, Encoder,
};

/// Game server metrics.
#[derive(Clone)]
pub struct Metrics {
    registry: Registry,

    // Server info
    /// Server uptime in seconds.
    pub uptime_seconds: IntGauge,

    // Session metrics
    /// Number of active sessions.
    pub sessions_active: IntGauge,
    /// Total sessions created.
    pub sessions_total: IntCounter,
    /// Sessions by state (connected, authenticated, etc).
    pub sessions_by_state: IntGaugeVec,

    // Room metrics
    /// Number of active rooms.
    pub rooms_active: IntGauge,
    /// Total rooms created.
    pub rooms_total: IntCounter,
    /// Players per room (histogram).
    pub room_players: Histogram,

    // HTTP API metrics
    /// Total HTTP requests.
    pub http_requests_total: IntCounterVec,
    /// HTTP request duration in seconds.
    pub http_request_duration: HistogramVec,
    /// HTTP response status codes.
    pub http_response_codes: IntCounterVec,

    // Game service metrics
    /// Chat messages sent.
    pub chat_messages_total: IntCounter,
    /// Leaderboard submissions.
    pub leaderboard_submissions_total: IntCounter,
    /// Matchmaker tickets in queue.
    pub matchmaker_queue_size: IntGauge,
    /// Matches created by matchmaker.
    pub matchmaker_matches_total: IntCounter,
    /// Storage operations.
    pub storage_operations_total: IntCounterVec,
    /// Notifications sent.
    pub notifications_total: IntCounter,

    // Network metrics
    /// Bytes received.
    pub bytes_received_total: IntCounter,
    /// Bytes sent.
    pub bytes_sent_total: IntCounter,
    /// WebSocket connections.
    pub websocket_connections: IntGauge,
    /// UDP packets received.
    pub udp_packets_received_total: IntCounter,
    /// UDP packets sent.
    pub udp_packets_sent_total: IntCounter,
}

impl Metrics {
    /// Create a new metrics instance with all metrics registered.
    pub fn new() -> Self {
        let registry = Registry::new();

        // Server info
        let uptime_seconds = IntGauge::new("kaosnet_uptime_seconds", "Server uptime in seconds")
            .expect("metric can be created");
        registry.register(Box::new(uptime_seconds.clone())).unwrap();

        // Session metrics
        let sessions_active = IntGauge::new("kaosnet_sessions_active", "Number of active sessions")
            .expect("metric can be created");
        registry.register(Box::new(sessions_active.clone())).unwrap();

        let sessions_total = IntCounter::new("kaosnet_sessions_total", "Total sessions created")
            .expect("metric can be created");
        registry.register(Box::new(sessions_total.clone())).unwrap();

        let sessions_by_state = IntGaugeVec::new(
            Opts::new("kaosnet_sessions_by_state", "Sessions by state"),
            &["state"],
        ).expect("metric can be created");
        registry.register(Box::new(sessions_by_state.clone())).unwrap();

        // Room metrics
        let rooms_active = IntGauge::new("kaosnet_rooms_active", "Number of active rooms")
            .expect("metric can be created");
        registry.register(Box::new(rooms_active.clone())).unwrap();

        let rooms_total = IntCounter::new("kaosnet_rooms_total", "Total rooms created")
            .expect("metric can be created");
        registry.register(Box::new(rooms_total.clone())).unwrap();

        let room_players = Histogram::with_opts(
            HistogramOpts::new("kaosnet_room_players", "Players per room")
                .buckets(vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0]),
        ).expect("metric can be created");
        registry.register(Box::new(room_players.clone())).unwrap();

        // HTTP API metrics
        let http_requests_total = IntCounterVec::new(
            Opts::new("kaosnet_http_requests_total", "Total HTTP requests"),
            &["method", "path"],
        ).expect("metric can be created");
        registry.register(Box::new(http_requests_total.clone())).unwrap();

        let http_request_duration = HistogramVec::new(
            HistogramOpts::new("kaosnet_http_request_duration_seconds", "HTTP request duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
            &["method", "path"],
        ).expect("metric can be created");
        registry.register(Box::new(http_request_duration.clone())).unwrap();

        let http_response_codes = IntCounterVec::new(
            Opts::new("kaosnet_http_response_codes_total", "HTTP response status codes"),
            &["code"],
        ).expect("metric can be created");
        registry.register(Box::new(http_response_codes.clone())).unwrap();

        // Game service metrics
        let chat_messages_total = IntCounter::new("kaosnet_chat_messages_total", "Chat messages sent")
            .expect("metric can be created");
        registry.register(Box::new(chat_messages_total.clone())).unwrap();

        let leaderboard_submissions_total = IntCounter::new(
            "kaosnet_leaderboard_submissions_total",
            "Leaderboard submissions",
        ).expect("metric can be created");
        registry.register(Box::new(leaderboard_submissions_total.clone())).unwrap();

        let matchmaker_queue_size = IntGauge::new(
            "kaosnet_matchmaker_queue_size",
            "Matchmaker tickets in queue",
        ).expect("metric can be created");
        registry.register(Box::new(matchmaker_queue_size.clone())).unwrap();

        let matchmaker_matches_total = IntCounter::new(
            "kaosnet_matchmaker_matches_total",
            "Matches created by matchmaker",
        ).expect("metric can be created");
        registry.register(Box::new(matchmaker_matches_total.clone())).unwrap();

        let storage_operations_total = IntCounterVec::new(
            Opts::new("kaosnet_storage_operations_total", "Storage operations"),
            &["operation"],
        ).expect("metric can be created");
        registry.register(Box::new(storage_operations_total.clone())).unwrap();

        let notifications_total = IntCounter::new(
            "kaosnet_notifications_total",
            "Notifications sent",
        ).expect("metric can be created");
        registry.register(Box::new(notifications_total.clone())).unwrap();

        // Network metrics
        let bytes_received_total = IntCounter::new(
            "kaosnet_bytes_received_total",
            "Total bytes received",
        ).expect("metric can be created");
        registry.register(Box::new(bytes_received_total.clone())).unwrap();

        let bytes_sent_total = IntCounter::new(
            "kaosnet_bytes_sent_total",
            "Total bytes sent",
        ).expect("metric can be created");
        registry.register(Box::new(bytes_sent_total.clone())).unwrap();

        let websocket_connections = IntGauge::new(
            "kaosnet_websocket_connections",
            "Active WebSocket connections",
        ).expect("metric can be created");
        registry.register(Box::new(websocket_connections.clone())).unwrap();

        let udp_packets_received_total = IntCounter::new(
            "kaosnet_udp_packets_received_total",
            "Total UDP packets received",
        ).expect("metric can be created");
        registry.register(Box::new(udp_packets_received_total.clone())).unwrap();

        let udp_packets_sent_total = IntCounter::new(
            "kaosnet_udp_packets_sent_total",
            "Total UDP packets sent",
        ).expect("metric can be created");
        registry.register(Box::new(udp_packets_sent_total.clone())).unwrap();

        Self {
            registry,
            uptime_seconds,
            sessions_active,
            sessions_total,
            sessions_by_state,
            rooms_active,
            rooms_total,
            room_players,
            http_requests_total,
            http_request_duration,
            http_response_codes,
            chat_messages_total,
            leaderboard_submissions_total,
            matchmaker_queue_size,
            matchmaker_matches_total,
            storage_operations_total,
            notifications_total,
            bytes_received_total,
            bytes_sent_total,
            websocket_connections,
            udp_packets_received_total,
            udp_packets_sent_total,
        }
    }

    /// Gather all metrics in Prometheus text format.
    pub fn gather(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    /// Record an HTTP request.
    pub fn record_http_request(&self, method: &str, path: &str, status_code: u16, duration_secs: f64) {
        self.http_requests_total
            .with_label_values(&[method, path])
            .inc();
        self.http_request_duration
            .with_label_values(&[method, path])
            .observe(duration_secs);
        self.http_response_codes
            .with_label_values(&[&status_code.to_string()])
            .inc();
    }

    /// Record a storage operation.
    pub fn record_storage_operation(&self, operation: &str) {
        self.storage_operations_total
            .with_label_values(&[operation])
            .inc();
    }

    /// Update session counts by state.
    pub fn update_session_state(&self, state: &str, count: i64) {
        self.sessions_by_state
            .with_label_values(&[state])
            .set(count);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new();
        metrics.sessions_active.set(5);
        metrics.sessions_total.inc();
        metrics.rooms_active.set(2);

        let output = metrics.gather();
        assert!(output.contains("kaosnet_sessions_active 5"));
        assert!(output.contains("kaosnet_sessions_total 1"));
        assert!(output.contains("kaosnet_rooms_active 2"));
    }

    #[test]
    fn test_http_metrics() {
        let metrics = Metrics::new();
        metrics.record_http_request("GET", "/api/status", 200, 0.015);
        metrics.record_http_request("POST", "/api/auth/login", 200, 0.045);
        metrics.record_http_request("GET", "/api/status", 200, 0.012);

        let output = metrics.gather();
        assert!(output.contains("kaosnet_http_requests_total"));
        assert!(output.contains("kaosnet_http_request_duration_seconds"));
    }

    #[test]
    fn test_storage_metrics() {
        let metrics = Metrics::new();
        metrics.record_storage_operation("get");
        metrics.record_storage_operation("set");
        metrics.record_storage_operation("get");

        let output = metrics.gather();
        assert!(output.contains("kaosnet_storage_operations_total"));
    }
}
