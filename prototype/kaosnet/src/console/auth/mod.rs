//! Authentication and authorization.

mod api_key;
mod jwt;
mod rbac;

pub use api_key::ApiKeyService;
pub use jwt::JwtService;
pub use rbac::{Identity, Permission, Role};

use crate::console::storage::{AccountStore, ApiKeyStore};
use crate::console::types::Account;
use kaos_http::Request;
use std::sync::Arc;
use uuid::Uuid;

/// Combined authentication service.
pub struct AuthService {
    jwt: JwtService,
    api_key: ApiKeyService,
    accounts: Arc<AccountStore>,
    api_keys: Arc<ApiKeyStore>,
}

impl AuthService {
    /// Create new auth service.
    pub fn new(
        jwt_secret: &str,
        accounts: Arc<AccountStore>,
        api_keys: Arc<ApiKeyStore>,
    ) -> Self {
        Self {
            jwt: JwtService::new(jwt_secret),
            api_key: ApiKeyService::new(),
            accounts,
            api_keys,
        }
    }

    /// Authenticate request, returns identity if valid.
    pub fn authenticate(&self, req: &Request) -> Option<Identity> {
        // Try JWT first (from Authorization: Bearer <token>)
        if let Some(token) = req.bearer_token() {
            // Check if it looks like a JWT (contains dots)
            if token.contains('.') {
                if let Some(claims) = self.jwt.verify(token) {
                    if let Ok(user_id) = claims.sub.parse::<Uuid>() {
                        if let Some(account) = self.accounts.get_by_id(&user_id) {
                            if !account.disabled {
                                return Some(Identity::User {
                                    id: account.id,
                                    username: account.username.clone(),
                                    role: account.role,
                                });
                            }
                        }
                    }
                }
            } else {
                // Try as API key
                if let Some(key) = self.api_keys.verify(token) {
                    if !key.disabled && !key.is_expired() {
                        self.api_keys.record_usage(&key.id);
                        return Some(Identity::ApiKey {
                            id: key.id,
                            name: key.name.clone(),
                            scopes: key.scopes,
                        });
                    }
                }
            }
        }

        None
    }

    /// Login with username and password, returns JWT token.
    pub fn login(&self, username: &str, password: &str) -> Option<(String, Account)> {
        let account = self.accounts.get_by_username(username)?;

        if account.disabled {
            return None;
        }

        if !self.accounts.verify_password(&account, password) {
            return None;
        }

        self.accounts.update_last_login(&account.id);

        let token = self.jwt.generate(&account.id.to_string(), account.role)?;
        Some((token, account))
    }

    /// Get JWT service for token operations.
    pub fn jwt(&self) -> &JwtService {
        &self.jwt
    }

    /// Get API key service.
    pub fn api_key(&self) -> &ApiKeyService {
        &self.api_key
    }
}
