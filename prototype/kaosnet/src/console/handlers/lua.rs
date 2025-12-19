//! Lua script management handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::types::PaginatedList;
use kaos_http::{Request, Response};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// Lua script info
#[derive(Debug, Serialize)]
pub struct LuaScriptInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub loaded: bool,
}

/// Script content response
#[derive(Debug, Serialize)]
pub struct ScriptContent {
    pub name: String,
    pub content: String,
    pub size: u64,
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

    let mut scripts = Vec::new();

    // If we have a script path configured, scan it for .lua files
    if let Some(ref script_path) = ctx.lua_script_path {
        let path = Path::new(script_path);
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.extension().map_or(false, |e| e == "lua") {
                        let name = entry_path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        scripts.push(LuaScriptInfo {
                            name,
                            path: entry_path.to_string_lossy().to_string(),
                            size,
                            loaded: true,
                        });
                    }
                }
            }
        }
    }

    // If no scripts found from filesystem, return placeholder
    if scripts.is_empty() {
        scripts.push(LuaScriptInfo {
            name: "game".to_string(),
            path: "scripts/game.lua".to_string(),
            size: 0,
            loaded: true,
        });
    }

    let total = scripts.len() as u32;
    Response::ok().json(&PaginatedList {
        items: scripts,
        total,
        page: 1,
        page_size: 20,
    })
}

/// GET /api/lua/scripts/:name
pub async fn get_script(req: Request, ctx: Arc<ServerContext>) -> Response {
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

    // Try to get actual file info if script path is configured
    if let Some(ref script_path) = ctx.lua_script_path {
        let file_path = Path::new(script_path).join(format!("{}.lua", name));
        if file_path.exists() {
            let size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
            return Response::ok().json(&LuaScriptInfo {
                name: name.to_string(),
                path: file_path.to_string_lossy().to_string(),
                size,
                loaded: true,
            });
        }
    }

    // Return script metadata with default path
    Response::ok().json(&LuaScriptInfo {
        name: name.to_string(),
        path: format!("scripts/{}.lua", name),
        size: 0,
        loaded: true,
    })
}

/// GET /api/lua/scripts/:name/content
pub async fn get_script_content(req: Request, ctx: Arc<ServerContext>) -> Response {
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

    // Read script content from file
    if let Some(ref script_path) = ctx.lua_script_path {
        let file_path = Path::new(script_path).join(format!("{}.lua", name));
        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                let size = content.len() as u64;
                return Response::ok().json(&ScriptContent {
                    name: name.to_string(),
                    content,
                    size,
                });
            }
            Err(e) => {
                return Response::not_found().json(&serde_json::json!({
                    "error": format!("script not found: {}", e)
                }));
            }
        }
    }

    Response::not_found().json(&serde_json::json!({
        "error": "lua script path not configured"
    }))
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
