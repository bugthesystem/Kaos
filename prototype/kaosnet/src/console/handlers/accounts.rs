//! Account management handlers.

use crate::console::auth::{Identity, Permission, Role};
use crate::console::storage::AccountStore;
use crate::console::types::{
    AccountInfo, ChangePasswordRequest, CreateAccountRequest, PaginatedList, UpdateAccountRequest,
};
use kaos_http::{Request, Response};
use std::sync::Arc;
use uuid::Uuid;

/// GET /api/accounts
pub async fn list_accounts(req: Request, accounts: Arc<AccountStore>) -> Response {
    // Check admin permission
    if !check_permission(&req, Permission::ManageAccounts) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let page: u32 = req.query_param("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size: u32 = req.query_param("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);

    let all_accounts = accounts.list();
    let total = all_accounts.len() as u32;

    let items: Vec<AccountInfo> = all_accounts
        .iter()
        .skip(((page - 1) * page_size) as usize)
        .take(page_size as usize)
        .map(AccountInfo::from)
        .collect();

    Response::ok().json(&PaginatedList {
        items,
        total,
        page,
        page_size,
    })
}

/// POST /api/accounts
pub async fn create_account(req: Request, accounts: Arc<AccountStore>) -> Response {
    if !check_permission(&req, Permission::ManageAccounts) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let body: CreateAccountRequest = match req.json() {
        Ok(b) => b,
        Err(_) => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid request body"
        })),
    };

    let role = match Role::from_str(&body.role) {
        Some(r) => r,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid role"
        })),
    };

    match accounts.create(&body.username, &body.password, role) {
        Some(account) => Response::created().json(&AccountInfo::from(&account)),
        None => Response::bad_request().json(&serde_json::json!({
            "error": "username already exists"
        })),
    }
}

/// GET /api/accounts/:id
pub async fn get_account(req: Request, accounts: Arc<AccountStore>) -> Response {
    if !check_permission(&req, Permission::ManageAccounts) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let id: Uuid = match req.param("id").and_then(|p| p.parse().ok()) {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid account id"
        })),
    };

    match accounts.get_by_id(&id) {
        Some(account) => Response::ok().json(&AccountInfo::from(&account)),
        None => Response::not_found().json(&serde_json::json!({
            "error": "account not found"
        })),
    }
}

/// PUT /api/accounts/:id
pub async fn update_account(req: Request, accounts: Arc<AccountStore>) -> Response {
    if !check_permission(&req, Permission::ManageAccounts) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let id: Uuid = match req.param("id").and_then(|p| p.parse().ok()) {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid account id"
        })),
    };

    let body: UpdateAccountRequest = match req.json() {
        Ok(b) => b,
        Err(_) => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid request body"
        })),
    };

    let role = body.role.as_ref().and_then(|r| Role::from_str(r));
    if body.role.is_some() && role.is_none() {
        return Response::bad_request().json(&serde_json::json!({
            "error": "invalid role"
        }));
    }

    match accounts.update(&id, body.username.as_deref(), role, body.disabled) {
        Some(account) => Response::ok().json(&AccountInfo::from(&account)),
        None => Response::not_found().json(&serde_json::json!({
            "error": "account not found or username taken"
        })),
    }
}

/// DELETE /api/accounts/:id
pub async fn delete_account(req: Request, accounts: Arc<AccountStore>) -> Response {
    if !check_permission(&req, Permission::ManageAccounts) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let id: Uuid = match req.param("id").and_then(|p| p.parse().ok()) {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid account id"
        })),
    };

    // Don't allow deleting self
    if let Some(Identity::User { id: user_id, .. }) = req.ext::<Identity>() {
        if *user_id == id {
            return Response::bad_request().json(&serde_json::json!({
                "error": "cannot delete own account"
            }));
        }
    }

    match accounts.delete(&id) {
        Some(_) => Response::ok().json(&serde_json::json!({
            "message": "account deleted"
        })),
        None => Response::not_found().json(&serde_json::json!({
            "error": "account not found"
        })),
    }
}

/// POST /api/accounts/:id/password
pub async fn change_password(req: Request, accounts: Arc<AccountStore>) -> Response {
    if !check_permission(&req, Permission::ManageAccounts) {
        return Response::forbidden().json(&serde_json::json!({
            "error": "admin access required"
        }));
    }

    let id: Uuid = match req.param("id").and_then(|p| p.parse().ok()) {
        Some(id) => id,
        None => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid account id"
        })),
    };

    let body: ChangePasswordRequest = match req.json() {
        Ok(b) => b,
        Err(_) => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid request body"
        })),
    };

    if accounts.change_password(&id, &body.password) {
        Response::ok().json(&serde_json::json!({
            "message": "password changed"
        }))
    } else {
        Response::not_found().json(&serde_json::json!({
            "error": "account not found"
        }))
    }
}

fn check_permission(req: &Request, permission: Permission) -> bool {
    req.ext::<Identity>()
        .map(|i| i.has_permission(permission))
        .unwrap_or(false)
}
