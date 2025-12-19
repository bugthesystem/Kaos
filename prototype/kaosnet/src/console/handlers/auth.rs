//! Authentication handlers.

use crate::console::auth::{AuthService, Identity};
use crate::console::types::{AccountInfo, LoginRequest, LoginResponse};
use kaos_http::{Request, Response};
use std::sync::Arc;

/// POST /api/auth/login
pub async fn login(req: Request, auth: Arc<AuthService>) -> Response {
    let body: LoginRequest = match req.json() {
        Ok(b) => b,
        Err(_) => return Response::bad_request().json(&serde_json::json!({
            "error": "invalid request body"
        })),
    };

    match auth.login(&body.username, &body.password) {
        Some((token, account)) => {
            let expires_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64 + auth.jwt().expiry_secs() as i64)
                .unwrap_or(0);

            Response::ok().json(&LoginResponse {
                token,
                expires_at,
                user: AccountInfo::from(&account),
            })
        }
        None => Response::unauthorized().json(&serde_json::json!({
            "error": "invalid credentials"
        })),
    }
}

/// POST /api/auth/logout
pub async fn logout(_req: Request) -> Response {
    // JWT is stateless, so logout is a no-op on server side
    // Client should discard the token
    Response::ok().json(&serde_json::json!({
        "message": "logged out"
    }))
}

/// GET /api/auth/me
pub async fn me(req: Request) -> Response {
    match req.ext::<Identity>() {
        Some(Identity::User { id, username, role }) => {
            Response::ok().json(&serde_json::json!({
                "id": id.to_string(),
                "username": username,
                "role": role.as_str(),
                "type": "user"
            }))
        }
        Some(Identity::ApiKey { id, name, scopes }) => {
            Response::ok().json(&serde_json::json!({
                "id": id.to_string(),
                "name": name,
                "scopes": scopes.to_vec(),
                "type": "api_key"
            }))
        }
        None => Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        })),
    }
}

/// POST /api/auth/refresh
/// Refresh the JWT token if the current one is still valid.
/// Returns a new token with extended expiry.
pub async fn refresh(req: Request, auth: Arc<AuthService>) -> Response {
    match req.ext::<Identity>() {
        Some(Identity::User { id, username, role }) => {
            // Generate a fresh token for this user
            if let Some(token) = auth.jwt().generate(&id.to_string(), *role) {
                let expires_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64 + auth.jwt().expiry_secs() as i64)
                    .unwrap_or(0);

                Response::ok().json(&serde_json::json!({
                    "token": token,
                    "expires_at": expires_at,
                    "user": {
                        "id": id.to_string(),
                        "username": username,
                        "role": role.as_str()
                    }
                }))
            } else {
                Response::internal_error().json(&serde_json::json!({
                    "error": "failed to generate token"
                }))
            }
        }
        Some(Identity::ApiKey { .. }) => {
            // API keys can't be refreshed - they have their own expiry
            Response::bad_request().json(&serde_json::json!({
                "error": "API keys cannot be refreshed"
            }))
        }
        None => Response::unauthorized().json(&serde_json::json!({
            "error": "not authenticated"
        })),
    }
}
