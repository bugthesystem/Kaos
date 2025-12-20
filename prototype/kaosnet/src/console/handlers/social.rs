//! Social handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::types::PaginatedList;
use crate::social::{FriendState, GroupRole};
use kaos_http::{Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct FriendInfo {
    pub user_id: String,
    pub username: String,
    pub state: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub creator_id: String,
    pub member_count: u32,
    pub open: bool,
    pub max_members: usize,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct GroupMemberInfo {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub joined_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
    pub creator_id: String,
    pub creator_username: String,
    pub open: Option<bool>,
}

fn friend_state_str(s: FriendState) -> &'static str {
    match s {
        FriendState::Pending => "pending",
        FriendState::Accepted => "accepted",
        FriendState::Blocked => "blocked",
    }
}

fn group_role_str(r: GroupRole) -> &'static str {
    match r {
        GroupRole::Member => "member",
        GroupRole::Moderator => "mod",
        GroupRole::Admin => "admin",
        GroupRole::Owner => "owner",
    }
}

/// GET /api/social/friends
pub async fn list_friends(req: Request, ctx: Arc<ServerContext>) -> Response {
    let user_id = match req.query_param("user_id") {
        Some(uid) => uid.to_string(),
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "user_id required"
        })),
    };

    let state_filter = req.query_param("state");
    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);

    let friends: Vec<FriendInfo> = ctx.social.get_friends(&user_id)
        .into_iter()
        .filter(|f| {
            if let Some(s) = state_filter {
                friend_state_str(f.state) == s
            } else {
                true
            }
        })
        .map(|f| FriendInfo {
            user_id: f.user_id,
            username: f.username,
            state: friend_state_str(f.state).to_string(),
            created_at: f.created_at,
        })
        .collect();

    let total = friends.len() as u32;
    let start = ((page - 1) * page_size) as usize;
    let items: Vec<_> = friends.into_iter().skip(start).take(page_size as usize).collect();

    Response::ok().json(&PaginatedList {
        items,
        total,
        page,
        page_size,
    })
}

/// GET /api/social/groups
pub async fn list_groups(req: Request, ctx: Arc<ServerContext>) -> Response {
    let user_id = req.query_param("user_id");
    let search_query = req.query_param("q");
    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);

    let groups: Vec<GroupInfo> = if let Some(uid) = user_id {
        ctx.social.get_user_groups(uid)
            .into_iter()
            .map(|g| {
                let member_count = ctx.social.get_group_members(&g.id)
                    .map(|m| m.len())
                    .unwrap_or(0);
                GroupInfo {
                    id: g.id,
                    name: g.name,
                    description: g.description,
                    creator_id: g.creator_id,
                    member_count: member_count as u32,
                    open: g.open,
                    max_members: g.max_members,
                    created_at: g.created_at,
                }
            })
            .collect()
    } else {
        // List all groups (optionally filtered by search query)
        let query = search_query.unwrap_or("");
        ctx.social.search_groups(query, 10000)
            .into_iter()
            .map(|g| {
                let member_count = ctx.social.get_group_members(&g.id)
                    .map(|m| m.len())
                    .unwrap_or(0);
                GroupInfo {
                    id: g.id,
                    name: g.name,
                    description: g.description,
                    creator_id: g.creator_id,
                    member_count: member_count as u32,
                    open: g.open,
                    max_members: g.max_members,
                    created_at: g.created_at,
                }
            })
            .collect()
    };

    let total = groups.len() as u32;
    let start = ((page - 1) * page_size) as usize;
    let items: Vec<_> = groups.into_iter().skip(start).take(page_size as usize).collect();

    Response::ok().json(&PaginatedList {
        items,
        total,
        page,
        page_size,
    })
}

/// GET /api/social/groups/:id
pub async fn get_group(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing group id"
        })),
    };

    match ctx.social.get_group(id) {
        Some(g) => {
            let member_count = ctx.social.get_group_members(&g.id)
                .map(|m| m.len())
                .unwrap_or(0);
            Response::ok().json(&serde_json::json!({
                "id": g.id,
                "name": g.name,
                "description": g.description,
                "creator_id": g.creator_id,
                "open": g.open,
                "max_members": g.max_members,
                "member_count": member_count,
                "created_at": g.created_at
            }))
        }
        None => Response::not_found().json(&serde_json::json!({
            "error": "group not found"
        })),
    }
}

/// GET /api/social/groups/:id/members
pub async fn get_group_members(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing group id"
        })),
    };

    match ctx.social.get_group_members(id) {
        Ok(members) => {
            let items: Vec<GroupMemberInfo> = members
                .into_iter()
                .map(|m| GroupMemberInfo {
                    user_id: m.user_id,
                    username: m.username,
                    role: group_role_str(m.role).to_string(),
                    joined_at: m.joined_at,
                })
                .collect();
            Response::ok().json(&serde_json::json!({
                "members": items
            }))
        }
        Err(e) => Response::not_found().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// POST /api/social/groups
pub async fn create_group(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageSocial) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let body: CreateGroupRequest = match serde_json::from_slice(req.body()) {
        Ok(b) => b,
        Err(e) => return Response::bad_request().json(&serde_json::json!({
            "error": format!("invalid request: {}", e)
        })),
    };

    match ctx.social.create_group(
        &body.creator_id,
        &body.creator_username,
        body.name.clone(),
        body.description.unwrap_or_default(),
        body.open.unwrap_or(true),
    ) {
        Ok(g) => Response::ok().json(&GroupInfo {
            id: g.id,
            name: g.name,
            description: g.description,
            creator_id: g.creator_id,
            member_count: 1, // Creator is auto-joined
            open: g.open,
            max_members: g.max_members,
            created_at: g.created_at,
        }),
        Err(e) => Response::bad_request().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// DELETE /api/social/groups/:id
pub async fn delete_group(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageSocial) {
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
            "error": "missing group id"
        })),
    };

    match ctx.social.delete_group(id) {
        Ok(_) => Response::ok().json(&serde_json::json!({
            "success": true,
            "message": "group deleted"
        })),
        Err(e) => Response::not_found().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}
