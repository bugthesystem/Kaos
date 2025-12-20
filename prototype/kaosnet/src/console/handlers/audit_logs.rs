//! Audit log handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::types::{AuditLogInfo, PaginatedList};
use kaos_http::{Request, Response};
use std::sync::Arc;

/// GET /api/audit-logs
pub async fn list_audit_logs(req: Request, ctx: Arc<ServerContext>) -> Response {
    // Check permission - require admin
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ViewStatus) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(50);
    let action_filter = req.query_param("action");
    let actor_filter = req.query_param("actor");

    // Get audit logs from context
    let logs = ctx.audit_logs.list(page, page_size, action_filter, actor_filter);
    let total = ctx.audit_logs.count();

    let items: Vec<AuditLogInfo> = logs.iter().map(|e| e.into()).collect();

    Response::ok().json(&PaginatedList {
        items,
        total,
        page,
        page_size,
    })
}

/// GET /api/audit-logs/:id
pub async fn get_audit_log(req: Request, ctx: Arc<ServerContext>) -> Response {
    // Check permission
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ViewStatus) {
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
        Some(id) => id.to_string(),
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing audit log id"
        })),
    };

    match ctx.audit_logs.get(&id) {
        Some(entry) => Response::ok().json(&AuditLogInfo::from(&entry)),
        None => Response::not_found().json(&serde_json::json!({
            "error": "audit log not found"
        })),
    }
}
