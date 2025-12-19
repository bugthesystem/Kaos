//! Chat handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::types::PaginatedList;
use crate::chat::ChannelType;
use kaos_http::{Request, Response};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub member_count: u32,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct MessageInfo {
    pub id: String,
    pub channel_id: String,
    pub sender_id: String,
    pub sender_username: String,
    pub content: String,
    pub code: i32,
    pub created_at: i64,
}

fn channel_type_str(ct: ChannelType) -> &'static str {
    match ct {
        ChannelType::Room => "room",
        ChannelType::Group => "group",
        ChannelType::DirectMessage => "dm",
    }
}

/// GET /api/chat/channels
pub async fn list_channels(req: Request, ctx: Arc<ServerContext>) -> Response {
    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);
    let user_id = req.query_param("user_id");
    let channel_type = req.query_param("channel_type");

    let channels: Vec<ChannelInfo> = if let Some(uid) = user_id {
        // List channels for a specific user
        ctx.chat.get_user_channels(uid)
            .into_iter()
            .map(|c| {
                let member_count = ctx.chat.get_members(&c.id).map(|m| m.len()).unwrap_or(0);
                ChannelInfo {
                    id: c.id,
                    name: c.name,
                    channel_type: channel_type_str(c.channel_type).to_string(),
                    member_count: member_count as u32,
                    created_at: c.created_at,
                }
            })
            .collect()
    } else {
        // List all channels
        ctx.chat.list_channels_with_counts()
            .into_iter()
            .map(|(c, member_count)| ChannelInfo {
                id: c.id,
                name: c.name,
                channel_type: channel_type_str(c.channel_type).to_string(),
                member_count: member_count as u32,
                created_at: c.created_at,
            })
            .collect()
    };

    // Filter by channel type if specified
    let channels: Vec<ChannelInfo> = match channel_type {
        Some(t) => channels.into_iter().filter(|c| c.channel_type == t).collect(),
        None => channels,
    };

    let total = channels.len() as u32;
    let start = ((page - 1) * page_size) as usize;
    let items: Vec<_> = channels.into_iter().skip(start).take(page_size as usize).collect();

    Response::ok().json(&PaginatedList {
        items,
        total,
        page,
        page_size,
    })
}

/// GET /api/chat/channels/:id
pub async fn get_channel(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing channel id"
        })),
    };

    match ctx.chat.get_channel(id) {
        Some(c) => {
            let members = ctx.chat.get_members(&c.id).unwrap_or_default();
            Response::ok().json(&serde_json::json!({
                "id": c.id,
                "name": c.name,
                "channel_type": channel_type_str(c.channel_type),
                "room_id": c.room_id,
                "group_id": c.group_id,
                "member_count": members.len(),
                "members": members,
                "created_at": c.created_at
            }))
        }
        None => Response::not_found().json(&serde_json::json!({
            "error": "channel not found"
        })),
    }
}

/// GET /api/chat/channels/:id/messages
pub async fn get_channel_messages(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing channel id"
        })),
    };

    let limit: usize = req.query_param("limit").and_then(|p| p.parse().ok()).unwrap_or(50);
    let before = req.query_param("before");

    match ctx.chat.get_history(id, limit, before) {
        Ok(messages) => {
            let items: Vec<MessageInfo> = messages
                .into_iter()
                .map(|m| MessageInfo {
                    id: m.id,
                    channel_id: m.channel_id,
                    sender_id: m.sender_id,
                    sender_username: m.sender_username,
                    content: m.content,
                    code: m.code,
                    created_at: m.created_at,
                })
                .collect();
            Response::ok().json(&serde_json::json!({
                "messages": items
            }))
        }
        Err(e) => Response::not_found().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// DELETE /api/chat/channels/:id
pub async fn delete_channel(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageChat) {
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
            "error": "missing channel id"
        })),
    };

    if ctx.chat.delete_channel(id) {
        Response::ok().json(&serde_json::json!({
            "message": "channel deleted"
        }))
    } else {
        Response::not_found().json(&serde_json::json!({
            "error": "channel not found"
        }))
    }
}

/// POST /api/chat/channels/:id/send
pub async fn send_system_message(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageChat) {
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
            "error": "missing channel id"
        })),
    };

    #[derive(serde::Deserialize)]
    struct SendRequest {
        content: String,
        code: Option<i32>,
    }

    let body: SendRequest = match serde_json::from_slice(req.body()) {
        Ok(b) => b,
        Err(e) => return Response::bad_request().json(&serde_json::json!({
            "error": format!("invalid request: {}", e)
        })),
    };

    match ctx.chat.send_system(id, body.code.unwrap_or(100), &body.content) {
        Ok(msg) => Response::ok().json(&MessageInfo {
            id: msg.id,
            channel_id: msg.channel_id,
            sender_id: msg.sender_id,
            sender_username: msg.sender_username,
            content: msg.content,
            code: msg.code,
            created_at: msg.created_at,
        }),
        Err(e) => Response::not_found().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}
