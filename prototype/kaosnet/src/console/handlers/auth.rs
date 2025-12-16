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
