//! Room handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::types::{PaginatedList, RoomInfo};
use crate::room::RoomState;
use kaos_http::{Request, Response};
use std::sync::Arc;

fn room_state_str(state: RoomState) -> &'static str {
    match state {
        RoomState::Open => "open",
        RoomState::Closed => "closed",
        RoomState::Running => "running",
    }
}

fn instant_to_epoch(instant: std::time::Instant, reference: std::time::Instant) -> i64 {
    let now = std::time::SystemTime::now();
    let elapsed = reference.elapsed();
    let room_elapsed = instant.elapsed();
    let diff = elapsed.as_secs() as i64 - room_elapsed.as_secs() as i64;
    now.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 - (elapsed.as_secs() as i64 - diff))
        .unwrap_or(0)
}

/// GET /api/rooms
pub async fn list_rooms(req: Request, ctx: Arc<ServerContext>) -> Response {
    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);
    let search = req.query_param("search");

    let all_rooms = ctx.rooms.list_all();
    let reference = ctx.start_time;
    let mut rooms: Vec<RoomInfo> = all_rooms
        .iter()
        .filter(|r| {
            if let Some(q) = search {
                r.id.contains(q) || r.config.label.contains(q)
            } else {
                true
            }
        })
        .map(|r| RoomInfo {
            id: r.id.clone(),
            label: if r.config.label.is_empty() { None } else { Some(r.config.label.clone()) },
            state: room_state_str(r.state).to_string(),
            tick_rate: r.config.tick_rate,
            player_count: r.player_count() as u32,
            max_players: r.config.max_players as u32,
            created_at: instant_to_epoch(r.created_at, reference),
        })
        .collect();

    let total = rooms.len() as u32;

    // Paginate
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(rooms.len());
    rooms = rooms.into_iter().skip(start).take(end - start).collect();

    Response::ok().json(&PaginatedList {
        items: rooms,
        total,
        page,
        page_size,
    })
}

/// GET /api/rooms/:id
pub async fn get_room(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing room id"
        })),
    };

    let reference = ctx.start_time;
    match ctx.rooms.get(id) {
        Some(room) => {
            Response::ok().json(&RoomInfo {
                id: room.id.clone(),
                label: if room.config.label.is_empty() { None } else { Some(room.config.label.clone()) },
                state: room_state_str(room.state).to_string(),
                tick_rate: room.config.tick_rate,
                player_count: room.player_count() as u32,
                max_players: room.config.max_players as u32,
                created_at: instant_to_epoch(room.created_at, reference),
            })
        }
        None => Response::not_found().json(&serde_json::json!({
            "error": "room not found"
        })),
    }
}

/// GET /api/rooms/:id/state
pub async fn get_room_state(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing room id"
        })),
    };

    match ctx.rooms.get(id) {
        Some(room) => {
            // Return room's custom state as JSON (binary state as base64)
            use base64::prelude::*;
            let state_bytes = room.get_state();
            let state_b64 = BASE64_STANDARD.encode(&state_bytes);
            Response::ok().json(&serde_json::json!({
                "room_id": room.id,
                "state": state_b64,
                "state_size": state_bytes.len(),
                "players": room.player_ids()
            }))
        }
        None => Response::not_found().json(&serde_json::json!({
            "error": "room not found"
        })),
    }
}

/// GET /api/rooms/:id/players
pub async fn get_room_players(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing room id"
        })),
    };

    match ctx.rooms.get(id) {
        Some(room) => {
            Response::ok().json(&serde_json::json!({
                "room_id": room.id,
                "players": room.player_ids(),
                "player_count": room.player_count()
            }))
        }
        None => Response::not_found().json(&serde_json::json!({
            "error": "room not found"
        })),
    }
}

/// POST /api/rooms/:id/terminate
pub async fn terminate_room(req: Request, ctx: Arc<ServerContext>) -> Response {
    // Check permission
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::TerminateRoom) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing room id"
        })),
    };

    if ctx.rooms.get(id).is_some() {
        ctx.rooms.remove(id);
        Response::ok().json(&serde_json::json!({
            "message": "room terminated"
        }))
    } else {
        Response::not_found().json(&serde_json::json!({
            "error": "room not found"
        }))
    }
}
