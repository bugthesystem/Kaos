//! Notification handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::types::PaginatedList;
use crate::notifications::{Notification, NotificationCode};
use kaos_http::{Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct NotificationInfo {
    pub id: String,
    pub user_id: String,
    pub code: i32,
    pub subject: String,
    pub content: String,
    pub sender_id: Option<String>,
    pub read: bool,
    pub persistent: bool,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct SendNotificationRequest {
    pub user_id: String,
    pub code: i32,
    pub subject: String,
    pub content: String,
    pub sender_id: Option<String>,
    pub persistent: Option<bool>,
}

fn code_to_i32(code: NotificationCode) -> i32 {
    code as i32
}

/// GET /api/notifications
pub async fn list_notifications(req: Request, ctx: Arc<ServerContext>) -> Response {
    let user_id = match req.query_param("user_id") {
        Some(uid) => uid.to_string(),
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "user_id required"
        })),
    };

    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);
    let unread_only: bool = req.query_param("unread").and_then(|p| p.parse().ok()).unwrap_or(false);

    let offset = ((page - 1) * page_size) as usize;
    let limit = page_size as usize;

    let notifications: Vec<NotificationInfo> = ctx.notifications.list(&user_id, limit + offset, 0, unread_only)
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|n| NotificationInfo {
            id: n.id,
            user_id: n.user_id,
            code: code_to_i32(n.code),
            subject: n.subject,
            content: n.content,
            sender_id: n.sender_id,
            read: n.read,
            persistent: n.persistent,
            created_at: n.created_at,
        })
        .collect();

    // Get total count
    let total = ctx.notifications.list(&user_id, 10000, 0, unread_only).len() as u32;

    Response::ok().json(&PaginatedList {
        items: notifications,
        total,
        page,
        page_size,
    })
}

/// GET /api/notifications/:id
pub async fn get_notification(req: Request, ctx: Arc<ServerContext>) -> Response {
    let id = match req.param("id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing notification id"
        })),
    };

    let user_id = match req.query_param("user_id") {
        Some(uid) => uid.to_string(),
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "user_id required"
        })),
    };

    let notifications = ctx.notifications.list(&user_id, 1000, 0, false);
    match notifications.into_iter().find(|n| n.id == id) {
        Some(n) => Response::ok().json(&NotificationInfo {
            id: n.id,
            user_id: n.user_id,
            code: code_to_i32(n.code),
            subject: n.subject,
            content: n.content,
            sender_id: n.sender_id,
            read: n.read,
            persistent: n.persistent,
            created_at: n.created_at,
        }),
        None => Response::not_found().json(&serde_json::json!({
            "error": "notification not found"
        })),
    }
}

/// POST /api/notifications
pub async fn send_notification(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageNotifications) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let body: SendNotificationRequest = match serde_json::from_slice(req.body()) {
        Ok(b) => b,
        Err(e) => return Response::bad_request().json(&serde_json::json!({
            "error": format!("invalid request: {}", e)
        })),
    };

    let code = NotificationCode::from(body.code);
    let mut notification = Notification::new(&body.user_id, code, &body.subject, &body.content);
    notification.sender_id = body.sender_id;
    notification.persistent = body.persistent.unwrap_or(true);

    match ctx.notifications.send(notification) {
        Ok(()) => {
            // Since send doesn't return the notification, create a response manually
            Response::ok().json(&serde_json::json!({
                "message": "notification sent"
            }))
        }
        Err(e) => Response::bad_request().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// POST /api/notifications/:id/read
pub async fn mark_notification_read(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageNotifications) {
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
            "error": "missing notification id"
        })),
    };

    let user_id = match req.query_param("user_id") {
        Some(uid) => uid.to_string(),
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "user_id required"
        })),
    };

    if ctx.notifications.mark_read(&user_id, id) {
        Response::ok().json(&serde_json::json!({
            "message": "notification marked as read"
        }))
    } else {
        Response::not_found().json(&serde_json::json!({
            "error": "notification not found"
        }))
    }
}

/// DELETE /api/notifications/:id
pub async fn delete_notification(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageNotifications) {
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
            "error": "missing notification id"
        })),
    };

    let user_id = match req.query_param("user_id") {
        Some(uid) => uid.to_string(),
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "user_id required"
        })),
    };

    if ctx.notifications.delete(&user_id, id) {
        Response::ok().json(&serde_json::json!({
            "message": "notification deleted"
        }))
    } else {
        Response::not_found().json(&serde_json::json!({
            "error": "notification not found"
        }))
    }
}
