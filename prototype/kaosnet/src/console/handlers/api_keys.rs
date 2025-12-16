//! API key management handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::storage::ApiKeyStore;
use crate::console::types::{
    ApiKeyInfo, ApiKeyScope, CreateApiKeyRequest, CreateApiKeyResponse, PaginatedList,
};
use kaos_http::{Request, Response};
use std::sync::Arc;
use uuid::Uuid;

/// GET /api/keys
pub async fn list_keys(req: Request, keys: Arc<ApiKeyStore>) -> Response {
    if !check_permission(&req, Permission::ManageApiKeys) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);

    let all_keys = keys.list();
    let total = all_keys.len() as u32;

    let items: Vec<ApiKeyInfo> = all_keys
        .iter()
        .skip(((page - 1) * page_size) as usize)
        .take(page_size as usize)
        .map(ApiKeyInfo::from)
        .collect();

    Response::ok().json(&PaginatedList {
        items,
        total,
        page,
        page_size,
    })
}

/// POST /api/keys
pub async fn create_key(req: Request, keys: Arc<ApiKeyStore>) -> Response {
    if !check_permission(&req, Permission::ManageApiKeys) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let user_id = match req.ext::<Identity>() {
        Some(Identity::User { id, .. }) => *id,
        _ => return Response::unauthorized().json(&serde_json::json!({
            "error": "user authentication required"
        })),
    };

    let body: CreateApiKeyRequest = match req.json() {
        Ok(b) => b,
        Err(_) => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid request body"
        })),
    };

    let scope_strs: Vec<&str> = body.scopes.iter().map(|s| s.as_str()).collect();
    let scopes = ApiKeyScope::from_vec(&scope_strs);

    let (key, raw_key) = keys.create(&body.name, scopes, user_id, body.expires_in_days);

    Response::created().json(&CreateApiKeyResponse {
        id: key.id,
        key: raw_key, // Only time the raw key is shown!
        name: key.name,
        scopes: key.scopes.to_vec().into_iter().map(String::from).collect(),
        expires_at: key.expires_at,
    })
}

/// GET /api/keys/:id
pub async fn get_key(req: Request, keys: Arc<ApiKeyStore>) -> Response {
    if !check_permission(&req, Permission::ManageApiKeys) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let id: Uuid = match req.param("id").and_then(|p| p.parse().ok()) {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid key id"
        })),
    };

    match keys.get(&id) {
        Some(key) => Response::ok().json(&ApiKeyInfo::from(&key)),
        None => Response::not_found().json(&serde_json::json!({
            "error": "key not found"
        })),
    }
}

/// DELETE /api/keys/:id
pub async fn delete_key(req: Request, keys: Arc<ApiKeyStore>) -> Response {
    if !check_permission(&req, Permission::ManageApiKeys) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let id: Uuid = match req.param("id").and_then(|p| p.parse().ok()) {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid key id"
        })),
    };

    match keys.delete(&id) {
        Some(_) => Response::ok().json(&serde_json::json!({
            "message": "key deleted"
        })),
        None => Response::not_found().json(&serde_json::json!({
            "error": "key not found"
        })),
    }
}

/// GET /api/keys/:id/usage
pub async fn get_key_usage(req: Request, keys: Arc<ApiKeyStore>) -> Response {
    if !check_permission(&req, Permission::ManageApiKeys) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let id: Uuid = match req.param("id").and_then(|p| p.parse().ok()) {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid key id"
        })),
    };

    match keys.get(&id) {
        Some(key) => Response::ok().json(&serde_json::json!({
            "id": key.id,
            "name": key.name,
            "request_count": key.request_count,
            "last_used": key.last_used
        })),
        None => Response::not_found().json(&serde_json::json!({
            "error": "key not found"
        })),
    }
}

fn check_permission(req: &Request, permission: Permission) -> bool {
    req.ext::<Identity>()
        .map(|i| i.has_permission(permission))
        .unwrap_or(false)
}
