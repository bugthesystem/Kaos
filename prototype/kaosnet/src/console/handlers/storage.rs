//! Storage handlers.

use crate::console::auth::{Identity, Permission};
use crate::console::server::ServerContext;
use crate::console::types::PaginatedList;
use crate::storage::{ObjectId, ObjectPermission, WriteOp};
use kaos_http::{Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct StorageObjectInfo {
    pub user_id: String,
    pub collection: String,
    pub key: String,
    pub value: serde_json::Value,
    pub version: u64,
    pub permission: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct WriteObjectRequest {
    pub user_id: String,
    pub collection: String,
    pub key: String,
    pub value: serde_json::Value,
    pub permission: Option<String>,
}

fn permission_str(p: ObjectPermission) -> &'static str {
    match p {
        ObjectPermission::OwnerOnly => "owner_only",
        ObjectPermission::PublicRead => "public_read",
        ObjectPermission::PublicReadWrite => "public_read_write",
    }
}

/// GET /api/storage
pub async fn list_storage(req: Request, ctx: Arc<ServerContext>) -> Response {
    let user_id = req.query_param("user_id");
    let collection = req.query_param("collection");
    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);

    let objects: Vec<StorageObjectInfo> = match (user_id, collection) {
        (Some(uid), Some(col)) => {
            match ctx.storage.list(uid, col, 1000, None) {
                Ok((items, _cursor)) => items
                    .into_iter()
                    .map(|o| StorageObjectInfo {
                        user_id: o.user_id,
                        collection: o.collection,
                        key: o.key,
                        value: o.value,
                        version: o.version,
                        permission: permission_str(o.permission).to_string(),
                        created_at: o.created_at,
                        updated_at: o.updated_at,
                    })
                    .collect(),
                Err(_) => vec![],
            }
        }
        (Some(uid), None) => {
            // List all collections for user
            match ctx.storage.list(uid, "", 1000, None) {
                Ok((items, _cursor)) => items
                    .into_iter()
                    .map(|o| StorageObjectInfo {
                        user_id: o.user_id,
                        collection: o.collection,
                        key: o.key,
                        value: o.value,
                        version: o.version,
                        permission: permission_str(o.permission).to_string(),
                        created_at: o.created_at,
                        updated_at: o.updated_at,
                    })
                    .collect(),
                Err(_) => vec![],
            }
        }
        _ => vec![],
    };

    let total = objects.len() as u32;
    let start = ((page - 1) * page_size) as usize;
    let items: Vec<_> = objects.into_iter().skip(start).take(page_size as usize).collect();

    Response::ok().json(&PaginatedList {
        items,
        total,
        page,
        page_size,
    })
}

/// GET /api/storage/:user_id/:collection/:key
pub async fn get_storage_object(req: Request, ctx: Arc<ServerContext>) -> Response {
    let user_id = match req.param("user_id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing user_id"
        })),
    };
    let collection = match req.param("collection") {
        Some(c) => c,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing collection"
        })),
    };
    let key = match req.param("key") {
        Some(k) => k,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing key"
        })),
    };

    match ctx.storage.get(user_id, collection, key) {
        Ok(Some(o)) => Response::ok().json(&StorageObjectInfo {
            user_id: o.user_id,
            collection: o.collection,
            key: o.key,
            value: o.value,
            version: o.version,
            permission: permission_str(o.permission).to_string(),
            created_at: o.created_at,
            updated_at: o.updated_at,
        }),
        Ok(None) => Response::not_found().json(&serde_json::json!({
            "error": "object not found"
        })),
        Err(e) => Response::internal_error().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// POST /api/storage
pub async fn write_storage_object(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageStorage) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let body: WriteObjectRequest = match serde_json::from_slice(req.body()) {
        Ok(b) => b,
        Err(e) => return Response::bad_request().json(&serde_json::json!({
            "error": format!("invalid request: {}", e)
        })),
    };

    let permission = match body.permission.as_deref() {
        Some("owner_only") => ObjectPermission::OwnerOnly,
        Some("public_read_write") => ObjectPermission::PublicReadWrite,
        _ => ObjectPermission::PublicRead,
    };

    let id = ObjectId::new(&body.user_id, &body.collection, &body.key);
    let write_op = WriteOp {
        id,
        value: body.value.clone(),
        version: None,
        permission,
    };

    match ctx.storage.write_many(&[write_op]) {
        Ok(objects) if !objects.is_empty() => {
            let o = &objects[0];
            Response::ok().json(&StorageObjectInfo {
                user_id: o.user_id.clone(),
                collection: o.collection.clone(),
                key: o.key.clone(),
                value: o.value.clone(),
                version: o.version,
                permission: permission_str(o.permission).to_string(),
                created_at: o.created_at,
                updated_at: o.updated_at,
            })
        }
        Ok(_) => Response::bad_request().json(&serde_json::json!({
            "error": "write failed"
        })),
        Err(e) => Response::bad_request().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// DELETE /api/storage/:user_id/:collection/:key
pub async fn delete_storage_object(req: Request, ctx: Arc<ServerContext>) -> Response {
    if let Some(identity) = req.ext::<Identity>() {
        if !identity.has_permission(Permission::ManageStorage) {
            return Response::forbidden().json(&serde_json::json!({
                "error": "insufficient permissions"
            }));
        }
    } else {
        return Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        }));
    }

    let user_id = match req.param("user_id") {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing user_id"
        })),
    };
    let collection = match req.param("collection") {
        Some(c) => c,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing collection"
        })),
    };
    let key = match req.param("key") {
        Some(k) => k,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "missing key"
        })),
    };

    match ctx.storage.delete(user_id, collection, key) {
        Ok(true) => Response::ok().json(&serde_json::json!({
            "message": "object deleted"
        })),
        Ok(false) => Response::not_found().json(&serde_json::json!({
            "error": "object not found"
        })),
        Err(e) => Response::internal_error().json(&serde_json::json!({
            "error": e.to_string()
        })),
    }
}
