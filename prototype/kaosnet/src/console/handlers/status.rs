//! Status handlers.

use crate::console::server::ServerContext;
use crate::console::types::{RoomStats, ServerStatus, SessionStats};
use kaos_http::{Request, Response};
use std::sync::Arc;

/// GET /api/status
pub async fn get_status(_req: Request, ctx: Arc<ServerContext>) -> Response {
    let uptime = ctx.start_time.elapsed().as_secs();

    // Count sessions by state
    let total_sessions = ctx.sessions.count() as u32;
    let session_stats = SessionStats {
        total: total_sessions,
        connecting: 0,    // TODO: track by state
        connected: total_sessions,
        authenticated: 0, // TODO: track authenticated
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
