//! Tournament handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::types::PaginatedList;
use crate::tournament::{TournamentConfig, TournamentState};
use kaos_http::{Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct TournamentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub state: String,
    pub participant_count: u32,
    pub max_participants: u32,
    pub start_time: i64,
    pub end_time: i64,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct TournamentRecordInfo {
    pub user_id: String,
    pub username: String,
    pub score: i64,
    pub rank: u64,
    pub num_submissions: u32,
    pub joined_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateTournamentRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub duration_secs: u64,
    pub start_time: Option<i64>,
    pub max_participants: Option<usize>,
}

fn state_str(state: TournamentState) -> &'static str {
    match state {
        TournamentState::Upcoming => "upcoming",
        TournamentState::Open => "open",
        TournamentState::Active => "active",
        TournamentState::Ended => "ended",
        TournamentState::Cancelled => "cancelled",
    }
}

/// GET /api/tournaments
pub async fn list_tournaments(req: Request, ctx: Arc<ServerContext>) -> Response {
    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);
    let category = req.query_param("category");
    let state_filter = req.query_param("state");

    // Update states first
    ctx.tournaments.update_states();

    let all: Vec<TournamentInfo> = if let Some(cat) = category {
        ctx.tournaments.list_by_category(cat, 1000)
    } else {
        ctx.tournaments.list_active(1000)
    }
    .into_iter()
    .filter(|t| {
        if let Some(s) = state_filter {
            state_str(t.state) == s
        } else {
            true
        }
    })
    .map(|t| TournamentInfo {
        id: t.id,
        name: t.config.name,
        description: t.config.description,
        category: t.config.category,
        state: state_str(t.state).to_string(),
        participant_count: t.participant_count as u32,
        max_participants: t.config.max_participants as u32,
        start_time: t.start_time,
        end_time: t.end_time,
        created_at: t.created_at,
    })
    .collect();

    let total = all.len() as u32;
    let start = ((page - 1) * page_size) as usize;
    let items: Vec<_> = all.into_iter().skip(start).take(page_size as usize).collect();

    Response::ok().json(&PaginatedList {
        items,
        total,
        page,
        page_size,
    })
}

/// GET /api/tournaments/:id
pub async fn get_tournament(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing tournament id"
        })),
    };

    ctx.tournaments.update_states();

    match ctx.tournaments.get(id) {
        Some(t) => Response::ok().json(&TournamentInfo {
            id: t.id,
            name: t.config.name,
            description: t.config.description,
            category: t.config.category,
            state: state_str(t.state).to_string(),
            participant_count: t.participant_count as u32,
            max_participants: t.config.max_participants as u32,
            start_time: t.start_time,
            end_time: t.end_time,
            created_at: t.created_at,
        }),
        None => Response::not_found().json(&serde_json::json!({
            "error": "tournament not found"
        })),
    }
}

/// GET /api/tournaments/:id/records
pub async fn get_tournament_records(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing tournament id"
        })),
    };

    let limit: usize = req.query_param("limit").and_then(|p| p.parse().ok()).unwrap_or(100);

    match ctx.tournaments.get_top(id, limit) {
        Ok(records) => {
            let items: Vec<TournamentRecordInfo> = records
                .into_iter()
                .map(|r| TournamentRecordInfo {
                    user_id: r.user_id,
                    username: r.username,
                    score: r.score,
                    rank: r.rank,
                    num_submissions: r.num_submissions,
                    joined_at: r.joined_at,
                })
                .collect();
            Response::ok().json(&serde_json::json!({
                "records": items
            }))
        }
        Err(e) => Response::not_found().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// POST /api/tournaments
pub async fn create_tournament(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageTournaments) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let body: CreateTournamentRequest = match serde_json::from_slice(req.body()) {
        Ok(b) => b,
        Err(e) => return Response::bad_request().json(&serde_json::json!({
            "error": format!("invalid request: {}", e)
        })),
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let start_time = body.start_time.unwrap_or(now);

    let config = TournamentConfig {
        id: body.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        name: body.name,
        description: body.description.unwrap_or_default(),
        category: body.category.unwrap_or_else(|| "default".to_string()),
        sort_order: crate::tournament::TournamentSortOrder::Descending,
        operator: crate::tournament::ScoreOperator::Best,
        max_participants: body.max_participants.unwrap_or(0),
        max_submissions: 0,
        entry_fee: 0,
        metadata: None,
        reset: crate::tournament::TournamentReset::Never,
        duration_secs: body.duration_secs,
        join_window_secs: 0,
    };

    match ctx.tournaments.create(config, start_time) {
        Ok(t) => Response::ok().json(&TournamentInfo {
            id: t.id,
            name: t.config.name,
            description: t.config.description,
            category: t.config.category,
            state: state_str(t.state).to_string(),
            participant_count: t.participant_count as u32,
            max_participants: t.config.max_participants as u32,
            start_time: t.start_time,
            end_time: t.end_time,
            created_at: t.created_at,
        }),
        Err(e) => Response::bad_request().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// POST /api/tournaments/:id/cancel
pub async fn cancel_tournament(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageTournaments) {
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
            "error": "missing tournament id"
        })),
    };

    match ctx.tournaments.cancel(id) {
        Ok(()) => Response::ok().json(&serde_json::json!({
            "message": "tournament cancelled"
        })),
        Err(e) => Response::not_found().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}
