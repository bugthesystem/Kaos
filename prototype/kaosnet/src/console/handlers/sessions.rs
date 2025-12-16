//! Session handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::types::{PaginatedList, SessionInfo};
use crate::session::SessionState;
use kaos_http::{Request, Response};
use std::sync::Arc;

fn instant_to_epoch(instant: std::time::Instant, reference: std::time::Instant) -> i64 {
    // Convert Instant to approximate epoch by using reference point
    let now = std::time::SystemTime::now();
    let elapsed = reference.elapsed();
    let session_elapsed = instant.elapsed();
    let diff = elapsed.as_secs() as i64 - session_elapsed.as_secs() as i64;
    now.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 - (elapsed.as_secs() as i64 - diff))
        .unwrap_or(0)
}

fn session_state_str(state: SessionState) -> &'static str {
    match state {
        SessionState::Connecting => "connecting",
        SessionState::Connected => "connected",
        SessionState::Authenticated => "authenticated",
        SessionState::Disconnecting => "disconnecting",
    }
}

/// GET /api/sessions
pub async fn list_sessions(req: Request, ctx: Arc<ServerContext>) -> Response {
    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);
    let search = req.query_param("search");

    let all_sessions = ctx.sessions.list();
    let reference = ctx.start_time;
    let mut sessions: Vec<SessionInfo> = all_sessions
        .iter()
        .filter(|s| {
            if let Some(q) = search {
                s.addr.to_string().contains(q)
                    || s.user_id.as_ref().map(|u| u.contains(q)).unwrap_or(false)
            } else {
                true
            }
        })
        .map(|s| SessionInfo {
            id: s.id,
            address: s.addr.to_string(),
            state: session_state_str(s.state).to_string(),
            user_id: s.user_id.clone(),
            username: s.username.clone(),
            connected_at: instant_to_epoch(s.created_at, reference),
            last_heartbeat: instant_to_epoch(s.last_heartbeat, reference),
        })
        .collect();

    let total = sessions.len() as u32;

    // Paginate
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(sessions.len());
    sessions = sessions.into_iter().skip(start).take(end - start).collect();

    Response::ok().json(&PaginatedList {
        items: sessions,
        total,
        page,
        page_size,
    })
}

/// GET /api/sessions/:id
pub async fn get_session(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id: u64 = match req.param("id").and_then(|p| p.parse().ok()) {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid session id"
        })),
    };

    let reference = ctx.start_time;
    match ctx.sessions.get(id) {
        Some(session) => {
            Response::ok().json(&SessionInfo {
                id: session.id,
                address: session.addr.to_string(),
                state: session_state_str(session.state).to_string(),
                user_id: session.user_id.clone(),
                username: session.username.clone(),
                connected_at: instant_to_epoch(session.created_at, reference),
                last_heartbeat: instant_to_epoch(session.last_heartbeat, reference),
            })
        }
        None => Response::not_found().json(&serde_json::json!({
            "error": "session not found"
        })),
    }
}

/// POST /api/sessions/:id/kick
pub async fn kick_session(req: Request, ctx: Arc<ServerContext>) -> Response {
    // Check permission
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::KickSession) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let id: u64 = match req.param("id").and_then(|p| p.parse().ok()) {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid session id"
        })),
    };

    if ctx.sessions.get(id).is_some() {
        ctx.sessions.remove(id);
        Response::ok().json(&serde_json::json!({
            "message": "session kicked"
        }))
    } else {
        Response::not_found().json(&serde_json::json!({
            "error": "session not found"
        }))
    }
}
