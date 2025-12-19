//! Player handlers for console API.
//!
//! Players are game client accounts from the auth system.
//! This integrates with both the client auth service and optional storage data.

use crate::auth::{AccountId, UserAccount};
use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use kaos_http::{Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Get pagination parameters from request.
fn get_pagination(req: &Request) -> (usize, usize) {
    let page: usize = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: usize = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);
    (page, page_size)
}

#[derive(Debug, Serialize)]
pub struct PlayerAccount {
    pub id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub devices: Vec<DeviceInfo>,
    pub custom_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub disabled: bool,
    pub metadata: serde_json::Value,
    // From storage (player profile data)
    pub online: bool,
}

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub linked_at: i64,
}

impl From<&UserAccount> for PlayerAccount {
    fn from(account: &UserAccount) -> Self {
        Self {
            id: account.id.0.clone(),
            username: account.username.clone(),
            display_name: account.display_name.clone(),
            email: account.email.clone(),
            avatar_url: account.avatar_url.clone(),
            devices: account.devices.iter().map(|d| DeviceInfo {
                device_id: d.device_id.clone(),
                linked_at: d.linked_at as i64,
            }).collect(),
            custom_id: account.custom_id.clone(),
            created_at: account.created_at as i64,
            updated_at: account.updated_at as i64,
            disabled: account.disabled,
            metadata: account.metadata.clone(),
            online: false, // Will be updated from session registry
        }
    }
}

// Legacy type for backward compatibility
#[derive(Debug, Serialize)]
pub struct SocialLink {
    pub provider: String,
    pub provider_id: String,
    pub linked_at: i64,
}

/// GET /api/players
pub async fn list_players(req: Request, ctx: Arc<ServerContext>) -> Response {
    let (page, page_size) = get_pagination(&req);
    let search = req.query_param("search");

    // Use search or list based on query
    let result = if let Some(query) = search {
        // Search accounts
        match ctx.client_auth.search_accounts(query, page_size) {
            Ok(accounts) => {
                let players: Vec<PlayerAccount> = accounts.iter().map(|a| {
                    let mut player: PlayerAccount = a.into();
                    // Check if player is online
                    // TODO: Track online status by user ID
                    player.online = false;
                    player
                }).collect();
                let total = players.len() as u64;
                Ok((players, total))
            }
            Err(e) => Err(e.to_string()),
        }
    } else {
        // List all accounts with pagination
        let cursor = if page > 1 {
            Some(((page - 1) * page_size).to_string())
        } else {
            None
        };

        match ctx.client_auth.list_accounts(page_size, cursor.as_deref()) {
            Ok((accounts, _next_cursor)) => {
                let players: Vec<PlayerAccount> = accounts.iter().map(|a| {
                    let mut player: PlayerAccount = a.into();
                    // Check if player is online
                    // TODO: Track online status by user ID
                    player.online = false;
                    player
                }).collect();

                let total = ctx.client_auth.count_accounts().unwrap_or(0);
                Ok((players, total))
            }
            Err(e) => Err(e.to_string()),
        }
    };

    match result {
        Ok((players, total)) => {
            Response::ok().json(&serde_json::json!({
                "items": players,
                "total": total,
                "page": page,
                "page_size": page_size
            }))
        }
        Err(e) => Response::internal_error().json(&serde_json::json!({
            "error": e
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

    match ctx.client_auth.get_account(&AccountId(player_id.to_string())) {
        Ok(Some(account)) => {
            let mut player: PlayerAccount = (&account).into();
            // TODO: Track online status by user ID
            player.online = false;
            Response::ok().json(&player)
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

    let _ban_req: BanRequest = match req.json() {
        Ok(b) => b,
        Err(_) => BanRequest { reason: None },
    };

    // Disable the account (ban)
    match ctx.client_auth.disable_account(&AccountId(player_id.to_string())) {
        Ok(()) => Response::ok().json(&serde_json::json!({
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

    // Enable the account (unban)
    match ctx.client_auth.enable_account(&AccountId(player_id.to_string())) {
        Ok(()) => Response::ok().json(&serde_json::json!({
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

    // Delete the account
    match ctx.client_auth.delete_account(&AccountId(player_id.to_string())) {
        Ok(true) => Response::ok().json(&serde_json::json!({
            "message": "player deleted"
        })),
        Ok(false) => Response::not_found().json(&serde_json::json!({
            "error": "player not found"
        })),
        Err(e) => Response::internal_error().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}
