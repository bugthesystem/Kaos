//! Lua script management handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::types::PaginatedList;
use kaos_http::{Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Lua script info
#[derive(Debug, Serialize)]
pub struct LuaScriptInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub loaded: bool,
}

/// RPC function info
#[derive(Debug, Serialize)]
pub struct RpcInfo {
    pub name: String,
    pub module: String,
}

/// Execute RPC request
#[derive(Debug, Deserialize)]
pub struct ExecuteRpcRequest {
    pub name: String,
    pub payload: Option<serde_json::Value>,
}

/// GET /api/lua/scripts
pub async fn list_scripts(req: Request, ctx: Arc<ServerContext>) -> Response {
    // Check permission
    if !check_permission(&req, Permission::ViewLua) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "insufficient permissions"
        }));
    }

    // In production, this would scan the scripts directory
    // For now, return info about loaded modules from the Lua runtime
    let scripts = vec![
        LuaScriptInfo {
            name: "game".to_string(),
            path: "scripts/game.lua".to_string(),
            size: 0,
            loaded: true,
        },
    ];

    Response::ok().json(&PaginatedList {
        items: scripts,
        total: 1,
        page: 1,
        page_size: 20,
    })
}

/// GET /api/lua/scripts/:name
pub async fn get_script(req: Request, _ctx: Arc<ServerContext>) -> Response {
    if !check_permission(&req, Permission::ViewLua) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "insufficient permissions"
        }));
    }

    let name = match req.param("name") {
        Some(n) => n,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing script name"
        })),
    };

    // Return script metadata
    Response::ok().json(&LuaScriptInfo {
        name: name.to_string(),
        path: format!("scripts/{}.lua", name),
        size: 0,
        loaded: true,
    })
}

/// GET /api/lua/rpcs
pub async fn list_rpcs(req: Request, _ctx: Arc<ServerContext>) -> Response {
    if !check_permission(&req, Permission::ViewLua) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "insufficient permissions"
        }));
    }

    // Return registered RPC functions
    // In production, this would query the Lua runtime
    let rpcs: Vec<RpcInfo> = vec![];

    Response::ok().json(&PaginatedList {
        items: rpcs,
        total: 0,
        page: 1,
        page_size: 20,
    })
}

/// POST /api/lua/rpcs/:name/execute
pub async fn execute_rpc(req: Request, _ctx: Arc<ServerContext>) -> Response {
    // Check permission
    if !check_permission(&req, Permission::ExecuteRpc) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "insufficient permissions"
        }));
    }

    let name = match req.param("name") {
        Some(n) => n,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing RPC name"
        })),
    };

    let body: ExecuteRpcRequest = match req.json() {
        Ok(b) => b,
        Err(_) => ExecuteRpcRequest {
            name: name.to_string(),
            payload: None,
        },
    };

    // In production, this would execute the RPC via LuaRuntime
    // For now, return a mock response
    Response::ok().json(&serde_json::json!({
        "rpc": body.name,
        "result": null,
        "duration_ms": 0
    }))
}

/// POST /api/lua/reload
pub async fn reload_scripts(req: Request, _ctx: Arc<ServerContext>) -> Response {
    // Check permission - need developer or higher
    if !check_permission(&req, Permission::ExecuteRpc) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "insufficient permissions"
        }));
    }

    // In production, this would reload Lua scripts
    Response::ok().json(&serde_json::json!({
        "message": "scripts reloaded",
        "count": 1
    }))
}

fn check_permission(req: &Request, permission: Permission) -> bool {
    req.ext::<Identity>()
        .map(|i| i.has_permission(permission))
        .unwrap_or(false)
}
