//! Player handlers for console API.
//!
//! Players are game users stored in the storage service under the "players" collection.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::utils::get_pagination;
use crate::storage::Query;
use kaos_http::{Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct PlayerAccount {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub email_verified: bool,
    pub devices: Vec<String>,
    pub social_links: Vec<SocialLink>,
    pub created_at: i64,
    pub updated_at: i64,
    pub banned: bool,
    pub ban_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SocialLink {
    pub provider: String,
    pub provider_id: String,
    pub linked_at: i64,
}

/// GET /api/players
pub async fn list_players(req: Request, ctx: Arc<ServerContext>) -> Response {
    let (page, page_size) = get_pagination(&req);
    let offset = (page - 1) * page_size;

    // List players from the "players" collection in storage
    match ctx.storage.query("players", Query::new().skip(offset), page_size) {
        Ok(objects) => {
            let players: Vec<PlayerAccount> = objects.into_iter()
                .map(|obj| {
                    let value = &obj.value;
                    PlayerAccount {
                        id: obj.user_id.clone(),
                        username: value.get("username")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&obj.user_id)
                            .to_string(),
                        display_name: value.get("display_name")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        email: value.get("email")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        email_verified: value.get("email_verified")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        devices: value.get("devices")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|d| d.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                        social_links: value.get("social_links")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|link| {
                                Some(SocialLink {
                                    provider: link.get("provider")?.as_str()?.to_string(),
                                    provider_id: link.get("provider_id")?.as_str()?.to_string(),
                                    linked_at: link.get("linked_at")?.as_i64().unwrap_or(0),
                                })
                            }).collect())
                            .unwrap_or_default(),
                        created_at: value.get("created_at")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0),
                        updated_at: value.get("updated_at")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0),
                        banned: value.get("banned")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        ban_reason: value.get("ban_reason")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    }
                })
                .collect();

            // Count total (approximate)
            let total = ctx.storage.query("players", Query::new(), 1000)
                .map(|v| v.len())
                .unwrap_or(0);

            Response::ok().json(&serde_json::json!({
                "items": players,
                "total": total,
                "page": page,
                "page_size": page_size
            }))
        }
        Err(e) => Response::internal_error().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// GET /api/players/:id
pub async fn get_player(req: Request, ctx: Arc<ServerContext>) -> Response {
    let player_id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing player id"
        })),
    };

    match ctx.storage.get(player_id, "players", "profile") {
        Ok(Some(obj)) => {
            let value = &obj.value;
            Response::ok().json(&PlayerAccount {
                id: obj.user_id.clone(),
                username: value.get("username")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&obj.user_id)
                    .to_string(),
                display_name: value.get("display_name")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                email: value.get("email")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                email_verified: value.get("email_verified")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                devices: value.get("devices")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|d| d.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                social_links: vec![],
                created_at: value.get("created_at")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                updated_at: value.get("updated_at")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                banned: value.get("banned")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                ban_reason: value.get("ban_reason")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            })
        }
        Ok(None) => Response::not_found().json(&serde_json::json!({
            "error": "player not found"
        })),
        Err(e) => Response::internal_error().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[derive(Debug, Deserialize)]
pub struct BanRequest {
    pub reason: Option<String>,
}

/// POST /api/players/:id/ban
pub async fn ban_player(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManagePlayers) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let player_id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing player id"
        })),
    };

    let ban_req: BanRequest = match req.json() {
        Ok(b) => b,
        Err(_) => BanRequest { reason: None },
    };

    // Get existing player data
    let mut player_data = match ctx.storage.get(player_id, "players", "profile") {
        Ok(Some(obj)) => obj.value.clone(),
        Ok(None) => serde_json::json!({}),
        Err(e) => return Response::internal_error().json(&serde_json::json!({
            "error": e.to_string()
        })),
    };

    // Update ban status
    player_data["banned"] = serde_json::json!(true);
    if let Some(reason) = ban_req.reason {
        player_data["ban_reason"] = serde_json::json!(reason);
    }

    match ctx.storage.set(player_id, "players", "profile", player_data) {
        Ok(_) => Response::ok().json(&serde_json::json!({
            "message": "player banned"
        })),
        Err(e) => Response::internal_error().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// POST /api/players/:id/unban
pub async fn unban_player(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManagePlayers) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let player_id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing player id"
        })),
    };

    // Get existing player data
    let mut player_data = match ctx.storage.get(player_id, "players", "profile") {
        Ok(Some(obj)) => obj.value.clone(),
        Ok(None) => return Response::not_found().json(&serde_json::json!({
            "error": "player not found"
        })),
        Err(e) => return Response::internal_error().json(&serde_json::json!({
            "error": e.to_string()
        })),
    };

    // Update ban status
    player_data["banned"] = serde_json::json!(false);
    player_data["ban_reason"] = serde_json::Value::Null;

    match ctx.storage.set(player_id, "players", "profile", player_data) {
        Ok(_) => Response::ok().json(&serde_json::json!({
            "message": "player unbanned"
        })),
        Err(e) => Response::internal_error().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// DELETE /api/players/:id
pub async fn delete_player(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManagePlayers) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let player_id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing player id"
        })),
    };

    match ctx.storage.delete(player_id, "players", "profile") {
        Ok(_) => Response::ok().json(&serde_json::json!({
            "message": "player deleted"
        })),
        Err(e) => Response::internal_error().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}
