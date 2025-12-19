//! Matchmaker handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::matchmaker::{MatchmakerPlayer, MatchmakerTicket};
use kaos_http::{Request, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct TicketInfo {
    pub id: String,
    pub queue: String,
    pub players: Vec<PlayerInfo>,
    pub properties: HashMap<String, serde_json::Value>,
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

/// Request to add a ticket to the matchmaker.
/// Follows Nakama-style API with string/numeric properties.
#[derive(Debug, Deserialize)]
pub struct MatchmakerAddRequest {
    /// Queue name (e.g., "ranked", "casual")
    pub queue: String,
    /// Query string for matching (e.g., "+region:us +mode:ranked")
    #[serde(default)]
    pub query: Option<String>,
    /// Minimum players needed
    #[serde(default = "default_min_count")]
    pub min_count: usize,
    /// Maximum players allowed
    #[serde(default = "default_max_count")]
    pub max_count: usize,
    /// String properties for exact matching
    #[serde(default)]
    pub string_properties: HashMap<String, String>,
    /// Numeric properties for range matching
    #[serde(default)]
    pub numeric_properties: HashMap<String, f64>,
}

fn default_min_count() -> usize { 2 }
fn default_max_count() -> usize { 8 }

/// Response when adding to matchmaker.
#[derive(Debug, Serialize)]
pub struct MatchmakerAddResponse {
    pub ticket: TicketInfo,
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// POST /api/matchmaker/add
/// Add a player to the matchmaker queue with properties.
/// Requires client auth (session token).
pub async fn add_to_matchmaker(req: Request, ctx: Arc<ServerContext>) -> Response {
    // Require authentication and extract user info
    let (user_id, username) = match req.ext::<Identity>() {
        Some(Identity::User { id, username, .. }) => (id.to_string(), username.clone()),
        Some(Identity::ApiKey { id, name, .. }) => (id.to_string(), name.clone()),
        None => return Response::unauthorized().json(&serde_json::json!({
            "error": "authentication required"
        })),
    };

    // Parse request body
    let body: MatchmakerAddRequest = match req.json() {
        Ok(b) => b,
        Err(e) => return Response::bad_request().json(&serde_json::json!({
            "error": format!("invalid request: {}", e)
        })),
    };

    // Build properties map from string and numeric properties
    let mut properties: HashMap<String, serde_json::Value> = HashMap::new();
    for (k, v) in body.string_properties {
        properties.insert(k, serde_json::Value::String(v));
    }
    for (k, v) in &body.numeric_properties {
        properties.insert(k.clone(), serde_json::json!(v));
    }
    // Store query if provided
    if let Some(ref query) = body.query {
        properties.insert("_query".to_string(), serde_json::Value::String(query.clone()));
    }
    // Store min/max count
    properties.insert("_min_count".to_string(), serde_json::json!(body.min_count));
    properties.insert("_max_count".to_string(), serde_json::json!(body.max_count));

    // Create ticket
    let ticket = MatchmakerTicket {
        id: Uuid::new_v4().to_string(),
        queue: body.queue.clone(),
        players: vec![MatchmakerPlayer {
            user_id: user_id.clone(),
            session_id: 0, // HTTP doesn't have session ID
            username,
            skill: body.numeric_properties.get("skill").copied().unwrap_or(1000.0),
        }],
        created_at: now_millis(),
        properties: properties.clone(),
    };

    // Add to matchmaker
    match ctx.matchmaker.add(ticket.clone()) {
        Ok(ticket_id) => {
            Response::ok().json(&MatchmakerAddResponse {
                ticket: TicketInfo {
                    id: ticket_id,
                    queue: ticket.queue,
                    players: ticket.players.into_iter().map(|p| PlayerInfo {
                        user_id: p.user_id,
                        username: p.username,
                        skill: p.skill,
                    }).collect(),
                    properties,
                    created_at: ticket.created_at,
                },
            })
        }
        Err(e) => Response::bad_request().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// DELETE /api/matchmaker/remove
/// Remove a player from matchmaker queue.
pub async fn remove_from_matchmaker(req: Request, ctx: Arc<ServerContext>) -> Response {
    // Require authentication and extract user id
    let user_id = match req.ext::<Identity>() {
        Some(Identity::User { id, .. }) => id.to_string(),
        Some(Identity::ApiKey { id, .. }) => id.to_string(),
        None => return Response::unauthorized().json(&serde_json::json!({
            "error": "authentication required"
        })),
    };

    // Remove player's ticket
    match ctx.matchmaker.remove_player(&user_id) {
        Ok(ticket) => Response::ok().json(&serde_json::json!({
            "message": "removed from matchmaker",
            "ticket_id": ticket.id
        })),
        Err(e) => Response::not_found().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
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
                        properties: t.properties,
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
            properties: t.properties,
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

/// GET /api/matchmaker/queues
pub async fn list_matchmaker_queues(_req: Request, ctx: Arc<ServerContext>) -> Response {
    let queues: Vec<QueueStatsInfo> = ctx.matchmaker.list_queues()
        .into_iter()
        .map(|stats| QueueStatsInfo {
            queue: stats.queue,
            tickets: stats.tickets,
            players: stats.players,
            longest_wait_secs: stats.longest_wait_secs,
        })
        .collect();

    Response::ok().json(&serde_json::json!({
        "queues": queues
    }))
}
