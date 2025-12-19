//! Game client authentication handlers.
//!
//! These endpoints are for game clients (SDK), separate from console admin auth.

use crate::auth::{
    AuthService as ClientAuthService, DeviceAuthRequest, EmailAuthRequest, CustomAuthRequest,
    AuthResponse, AuthError,
};
use crate::console::server::ServerContext;
use kaos_http::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ============================================================================
// Request/Response types for SDK
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DeviceAuthApiRequest {
    pub device_id: String,
    #[serde(default = "default_true")]
    pub create: bool,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EmailAuthApiRequest {
    pub email: String,
    pub password: String,
    #[serde(default = "default_true")]
    pub create: bool,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CustomAuthApiRequest {
    pub id: String,
    #[serde(default = "default_true")]
    pub create: bool,
    pub username: Option<String>,
    #[serde(default)]
    pub vars: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session: SessionInfo,
    pub new_account: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub username: Option<String>,
    pub expires_at: u64,
    pub created_at: u64,
}

impl From<AuthResponse> for SessionResponse {
    fn from(resp: AuthResponse) -> Self {
        // Decode token to get expiry (simplified - in production decode JWT properly)
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() + 3600) // 1 hour default
            .unwrap_or(0);

        Self {
            session: SessionInfo {
                token: resp.token,
                refresh_token: resp.refresh_token,
                user_id: resp.account.id,
                username: resp.account.username,
                expires_at,
                created_at: resp.account.created_at,
            },
            new_account: resp.created,
        }
    }
}

fn auth_error_response(err: AuthError) -> Response {
    let (status, code) = match &err {
        AuthError::AccountNotFound => (StatusCode::NOT_FOUND, "ACCOUNT_NOT_FOUND"),
        AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"),
        AuthError::AccountExists => (StatusCode::CONFLICT, "ACCOUNT_EXISTS"),
        AuthError::DeviceAlreadyLinked => (StatusCode::CONFLICT, "DEVICE_ALREADY_LINKED"),
        AuthError::EmailAlreadyRegistered => (StatusCode::CONFLICT, "EMAIL_ALREADY_REGISTERED"),
        AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN"),
        AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "TOKEN_EXPIRED"),
        AuthError::AccountDisabled => (StatusCode::FORBIDDEN, "ACCOUNT_DISABLED"),
        AuthError::WeakPassword(_) => (StatusCode::BAD_REQUEST, "WEAK_PASSWORD"),
        AuthError::InvalidEmail => (StatusCode::BAD_REQUEST, "INVALID_EMAIL"),
        AuthError::CustomAuthFailed(_) => (StatusCode::UNAUTHORIZED, "CUSTOM_AUTH_FAILED"),
        AuthError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
    };

    Response::new(status).json(&serde_json::json!({
        "error": err.to_string(),
        "code": code,
    }))
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/auth/device
/// Authenticate with a device ID (anonymous auth).
pub async fn authenticate_device(req: Request, ctx: Arc<ServerContext>) -> Response {
    let body: DeviceAuthApiRequest = match req.json() {
        Ok(b) => b,
        Err(e) => return Response::bad_request().json(&serde_json::json!({
            "error": format!("Invalid request: {}", e),
            "code": "INVALID_REQUEST",
        })),
    };

    let auth_req = DeviceAuthRequest {
        device_id: body.device_id,
        create: body.create,
        username: body.username,
    };

    match ctx.client_auth.authenticate_device(&auth_req) {
        Ok(response) => Response::ok().json(&SessionResponse::from(response)),
        Err(e) => auth_error_response(e),
    }
}

/// POST /api/auth/email
/// Authenticate with email and password.
pub async fn authenticate_email(req: Request, ctx: Arc<ServerContext>) -> Response {
    let body: EmailAuthApiRequest = match req.json() {
        Ok(b) => b,
        Err(e) => return Response::bad_request().json(&serde_json::json!({
            "error": format!("Invalid request: {}", e),
            "code": "INVALID_REQUEST",
        })),
    };

    let auth_req = EmailAuthRequest {
        email: body.email,
        password: body.password,
        create: body.create,
        username: body.username,
    };

    match ctx.client_auth.authenticate_email(&auth_req) {
        Ok(response) => Response::ok().json(&SessionResponse::from(response)),
        Err(e) => auth_error_response(e),
    }
}

/// POST /api/auth/custom
/// Authenticate with a custom auth method.
pub async fn authenticate_custom(req: Request, ctx: Arc<ServerContext>) -> Response {
    let body: CustomAuthApiRequest = match req.json() {
        Ok(b) => b,
        Err(e) => return Response::bad_request().json(&serde_json::json!({
            "error": format!("Invalid request: {}", e),
            "code": "INVALID_REQUEST",
        })),
    };

    let auth_req = CustomAuthRequest {
        id: body.id,
        create: body.create,
        username: body.username,
        vars: body.vars,
    };

    match ctx.client_auth.authenticate_custom(&auth_req) {
        Ok(response) => Response::ok().json(&SessionResponse::from(response)),
        Err(e) => auth_error_response(e),
    }
}

/// POST /api/auth/refresh
/// Refresh an expired session token.
pub async fn refresh_token(req: Request, ctx: Arc<ServerContext>) -> Response {
    let body: RefreshTokenRequest = match req.json() {
        Ok(b) => b,
        Err(e) => return Response::bad_request().json(&serde_json::json!({
            "error": format!("Invalid request: {}", e),
            "code": "INVALID_REQUEST",
        })),
    };

    match ctx.client_auth.refresh_token(&body.refresh_token) {
        Ok(response) => Response::ok().json(&SessionResponse::from(response)),
        Err(e) => auth_error_response(e),
    }
}
