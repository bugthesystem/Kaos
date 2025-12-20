//! Status handlers.

use crate::console::server::ServerContext;
use crate::console::types::{RoomStats, ServerStatus, SessionStats};
use kaos_http::{Request, Response};
use std::sync::Arc;

/// GET /api/status
pub async fn get_status(_req: Request, ctx: Arc<ServerContext>) -> Response {
    let uptime = ctx.start_time.elapsed().as_secs();

    // Count sessions by state
    let counts = ctx.sessions.count_by_state();
    let session_stats = SessionStats {
        total: counts.total(),
        connecting: counts.connecting,
        connected: counts.connected,
        authenticated: counts.authenticated,
    };

    // Count rooms
    let total_rooms = ctx.rooms.count() as u32;
    let total_players = ctx.rooms.total_players() as u32;
    let room_stats = RoomStats {
        total: total_rooms,
        players: total_players,
    };

    Response::ok().json(&ServerStatus {
        version: ctx.version.clone(),
        uptime_secs: uptime,
        sessions: session_stats,
        rooms: room_stats,
    })
}

/// GET /api/config
pub async fn get_config(_req: Request, ctx: Arc<ServerContext>) -> Response {
    // Return sanitized config (no secrets)
    Response::ok().json(&serde_json::json!({
        "version": ctx.version,
        "uptime_secs": ctx.start_time.elapsed().as_secs(),
        "features": {
            "lua": cfg!(feature = "lua"),
            "console": true
        }
    }))
}

/// GET /api/metrics - Returns parsed metrics data for the Console UI
#[cfg(feature = "metrics")]
pub async fn get_metrics(_req: Request, ctx: Arc<ServerContext>) -> Response {
    use crate::console::types::MetricsData;

    let Some(ref metrics) = ctx.metrics else {
        return Response::ok().json(&serde_json::json!({
            "error": "Metrics not enabled"
        }));
    };

    // Get session counts by state
    let counts = ctx.sessions.count_by_state();
    let mut sessions_by_state = std::collections::HashMap::new();
    sessions_by_state.insert("connecting".to_string(), counts.connecting as i64);
    sessions_by_state.insert("connected".to_string(), counts.connected as i64);
    sessions_by_state.insert("authenticated".to_string(), counts.authenticated as i64);

    let data = MetricsData {
        uptime_seconds: metrics.uptime_seconds.get(),
        sessions_active: metrics.sessions_active.get(),
        sessions_total: metrics.sessions_total.get() as i64,
        sessions_by_state,
        rooms_active: metrics.rooms_active.get(),
        rooms_total: metrics.rooms_total.get() as i64,
        websocket_connections: metrics.websocket_connections.get(),
        bytes_received_total: metrics.bytes_received_total.get() as i64,
        bytes_sent_total: metrics.bytes_sent_total.get() as i64,
        udp_packets_received_total: metrics.udp_packets_received_total.get() as i64,
        udp_packets_sent_total: metrics.udp_packets_sent_total.get() as i64,
        chat_messages_total: metrics.chat_messages_total.get() as i64,
        leaderboard_submissions_total: metrics.leaderboard_submissions_total.get() as i64,
        matchmaker_queue_size: metrics.matchmaker_queue_size.get(),
        matchmaker_matches_total: metrics.matchmaker_matches_total.get() as i64,
        notifications_total: metrics.notifications_total.get() as i64,
    };

    Response::ok().json(&data)
}

#[cfg(not(feature = "metrics"))]
pub async fn get_metrics(_req: Request, _ctx: Arc<ServerContext>) -> Response {
    Response::ok().json(&serde_json::json!({
        "error": "Metrics feature not enabled. Compile with --features metrics"
    }))
}
