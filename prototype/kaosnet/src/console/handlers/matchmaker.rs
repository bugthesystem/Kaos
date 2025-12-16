//! Matchmaker handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use kaos_http::{Request, Response};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct TicketInfo {
    pub id: String,
    pub queue: String,
    pub players: Vec<PlayerInfo>,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct PlayerInfo {
    pub user_id: String,
    pub username: String,
    pub skill: f64,
}

#[derive(Debug, Serialize)]
pub struct QueueStatsInfo {
    pub queue: String,
    pub tickets: usize,
    pub players: usize,
    pub longest_wait_secs: u64,
}

/// GET /api/matchmaker/tickets
/// Note: This API requires a user_id to look up their ticket
pub async fn list_matchmaker_tickets(req: Request, ctx: Arc<ServerContext>) -> Response {
    let user_id = req.query_param("user_id");

    match user_id {
        Some(uid) => {
            match ctx.matchmaker.get_ticket(uid) {
                Some(t) => {
                    let ticket = TicketInfo {
                        id: t.id,
                        queue: t.queue,
                        players: t.players.into_iter().map(|p| PlayerInfo {
                            user_id: p.user_id,
                            username: p.username,
                            skill: p.skill,
                        }).collect(),
                        created_at: t.created_at,
                    };
                    Response::ok().json(&serde_json::json!({
                        "tickets": [ticket]
                    }))
                }
                None => Response::ok().json(&serde_json::json!({
                    "tickets": []
                })),
            }
        }
        None => {
            // Without a user_id, we can't list tickets
            // Return empty list with explanation
            Response::ok().json(&serde_json::json!({
                "tickets": [],
                "note": "provide user_id query param to look up a user's ticket"
            }))
        }
    }
}

/// GET /api/matchmaker/tickets/:id
/// Note: The :id here is interpreted as user_id since that's how the API works
pub async fn get_matchmaker_ticket(req: Request, ctx: Arc<ServerContext>) -> Response {
    let user_id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing user_id"
        })),
    };

    match ctx.matchmaker.get_ticket(user_id) {
        Some(t) => Response::ok().json(&TicketInfo {
            id: t.id,
            queue: t.queue,
            players: t.players.into_iter().map(|p| PlayerInfo {
                user_id: p.user_id,
                username: p.username,
                skill: p.skill,
            }).collect(),
            created_at: t.created_at,
        }),
        None => Response::not_found().json(&serde_json::json!({
            "error": "ticket not found for user"
        })),
    }
}

/// DELETE /api/matchmaker/tickets/:id
pub async fn cancel_matchmaker_ticket(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageMatchmaker) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let ticket_id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing ticket id"
        })),
    };

    match ctx.matchmaker.remove(ticket_id) {
        Ok(_ticket) => Response::ok().json(&serde_json::json!({
            "message": "ticket cancelled"
        })),
        Err(e) => Response::not_found().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// GET /api/matchmaker/stats
pub async fn get_matchmaker_stats(req: Request, ctx: Arc<ServerContext>) -> Response {
    let queue = req.query_param("queue");

    match queue {
        Some(q) => {
            match ctx.matchmaker.stats(q) {
                Some(stats) => Response::ok().json(&QueueStatsInfo {
                    queue: stats.queue,
                    tickets: stats.tickets,
                    players: stats.players,
                    longest_wait_secs: stats.longest_wait_secs,
                }),
                None => Response::not_found().json(&serde_json::json!({
                    "error": "queue not found"
                })),
            }
        }
        None => {
            // Without a queue parameter, return a message
            Response::ok().json(&serde_json::json!({
                "message": "provide queue query param to get stats for a specific queue"
            }))
        }
    }
}
