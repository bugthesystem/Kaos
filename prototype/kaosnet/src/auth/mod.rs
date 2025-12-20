//! Client Authentication System
//!
//! Provides authentication methods for game clients (separate from console auth).
//!
//! ## Authentication Methods
//!
//! - **Device**: Anonymous auth via unique device ID (mobile, desktop)
//! - **Email**: Email/password registration and login
//! - **Custom**: Developer-defined auth (e.g., Steam, custom backend)
//!
//! ## Example
//!
//! ```rust,ignore
//! use kaosnet::auth::{AuthService, DeviceAuthRequest};
//!
//! let auth = AuthService::new("secret-key");
//!
//! // Device auth (anonymous)
//! let session = auth.authenticate_device(&DeviceAuthRequest {
//!     device_id: "device-abc123".to_string(),
//!     create: true,
//! })?;
//!
//! // Email auth
//! let session = auth.authenticate_email(&EmailAuthRequest {
//!     email: "player@game.com".to_string(),
//!     password: "secret".to_string(),
//!     create: false,
//! })?;
//! ```

mod accounts;
mod tokens;

pub use accounts::{
    UserAccount, AccountId, DeviceLink, AuthProvider,
    AccountStore, MemoryAccountStore,
};
pub use tokens::{ClientToken, ClientClaims, RefreshToken};

use std::sync::Arc;
use thiserror::Error;
use serde::{Serialize, Deserialize};

/// Authentication errors.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("account not found")]
    AccountNotFound,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("account already exists")]
    AccountExists,

    #[error("device already linked to another account")]
    DeviceAlreadyLinked,

    #[error("email already registered")]
    EmailAlreadyRegistered,

    #[error("invalid token")]
    InvalidToken,

    #[error("token expired")]
    TokenExpired,

    #[error("account disabled")]
    AccountDisabled,

    #[error("weak password: {0}")]
    WeakPassword(String),

    #[error("invalid email format")]
    InvalidEmail,

    #[error("custom auth failed: {0}")]
    CustomAuthFailed(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AuthError>;

/// Device authentication request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAuthRequest {
    /// Unique device identifier (e.g., hardware ID, vendor ID)
    pub device_id: String,
    /// Create account if device is not registered
    pub create: bool,
    /// Optional username for new accounts
    pub username: Option<String>,
}

/// Email authentication request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAuthRequest {
    /// Email address
    pub email: String,
    /// Password
    pub password: String,
    /// Create account if email is not registered
    pub create: bool,
    /// Optional username for new accounts
    pub username: Option<String>,
}

/// Custom authentication request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAuthRequest {
    /// Custom identifier from external system
    pub id: String,
    /// Optional username
    pub username: Option<String>,
    /// Create account if not found
    pub create: bool,
    /// Additional vars from custom auth
    #[serde(default)]
    pub vars: std::collections::HashMap<String, String>,
}

/// Authentication response with session tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Access token for API requests
    pub token: String,
    /// Refresh token for getting new access tokens
    pub refresh_token: String,
    /// User account information
    pub account: UserAccountInfo,
    /// Whether this is a newly created account
    pub created: bool,
}

/// User account information (safe to expose to client).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccountInfo {
    pub id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    /// Custom metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl From<&UserAccount> for UserAccountInfo {
    fn from(account: &UserAccount) -> Self {
        Self {
            id: account.id.0.clone(),
            username: account.username.clone(),
            display_name: account.display_name.clone(),
            avatar_url: account.avatar_url.clone(),
            created_at: account.created_at,
            updated_at: account.updated_at,
            metadata: account.metadata.clone(),
        }
    }
}

/// Account linking request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkDeviceRequest {
    /// Device ID to link
    pub device_id: String,
}

/// Link email to existing account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkEmailRequest {
    /// Email to link
    pub email: String,
    /// Password for the email
    pub password: String,
}

/// Configuration for the auth service.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWT secret key
    pub secret: String,
    /// Access token expiry in seconds (default: 1 hour)
    pub token_expiry_secs: u64,
    /// Refresh token expiry in seconds (default: 7 days)
    pub refresh_expiry_secs: u64,
    /// Minimum password length
    pub min_password_length: usize,
    /// Allow anonymous device auth
    pub allow_device_auth: bool,
    /// Allow email registration
    pub allow_email_registration: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            secret: "change-me-in-production-32-chars".to_string(),
            token_expiry_secs: 3600,        // 1 hour
            refresh_expiry_secs: 604800,    // 7 days
            min_password_length: 8,
            allow_device_auth: true,
            allow_email_registration: true,
        }
    }
}

/// Main authentication service.
pub struct AuthService {
    config: AuthConfig,
    accounts: Arc<dyn AccountStore>,
    token_service: tokens::TokenService,
}

impl AuthService {
    /// Create new auth service with default in-memory store.
    pub fn new(secret: &str) -> Self {
        let config = AuthConfig {
            secret: secret.to_string(),
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Create auth service with custom config.
    pub fn with_config(config: AuthConfig) -> Self {
        let token_service = tokens::TokenService::new(
            &config.secret,
            config.token_expiry_secs,
            config.refresh_expiry_secs,
        );
        Self {
            config,
            accounts: Arc::new(MemoryAccountStore::new()),
            token_service,
        }
    }

    /// Create auth service with custom account store.
    pub fn with_store(config: AuthConfig, store: Arc<dyn AccountStore>) -> Self {
        let token_service = tokens::TokenService::new(
            &config.secret,
            config.token_expiry_secs,
            config.refresh_expiry_secs,
        );
        Self {
            config,
            accounts: store,
            token_service,
        }
    }

    // ==================== Device Authentication ====================

    /// Authenticate via device ID (anonymous auth).
    pub fn authenticate_device(&self, req: &DeviceAuthRequest) -> Result<AuthResponse> {
        if !self.config.allow_device_auth {
            return Err(AuthError::CustomAuthFailed("Device auth disabled".into()));
        }

        // Check if device is already linked to an account
        if let Some(account) = self.accounts.get_by_device(&req.device_id)? {
            if account.disabled {
                return Err(AuthError::AccountDisabled);
            }
            return self.create_auth_response(&account, false);
        }

        // Device not found - create if requested
        if !req.create {
            return Err(AuthError::AccountNotFound);
        }

        // Create new account with device link
        let mut account = UserAccount::new();
        account.username = req.username.clone();
        account.devices.push(DeviceLink {
            device_id: req.device_id.clone(),
            linked_at: unix_timestamp(),
        });

        self.accounts.create(&account)?;
        self.create_auth_response(&account, true)
    }

    // ==================== Email Authentication ====================

    /// Authenticate via email/password.
    pub fn authenticate_email(&self, req: &EmailAuthRequest) -> Result<AuthResponse> {
        // Validate email format
        if !is_valid_email(&req.email) {
            return Err(AuthError::InvalidEmail);
        }

        // Check if email exists
        if let Some(account) = self.accounts.get_by_email(&req.email)? {
            // Login flow
            if account.disabled {
                return Err(AuthError::AccountDisabled);
            }

            // Verify password
            let password_hash = account.password_hash.as_ref()
                .ok_or(AuthError::InvalidCredentials)?;

            if !verify_password(&req.password, password_hash) {
                return Err(AuthError::InvalidCredentials);
            }

            return self.create_auth_response(&account, false);
        }

        // Email not found - create if requested
        if !req.create {
            return Err(AuthError::AccountNotFound);
        }

        if !self.config.allow_email_registration {
            return Err(AuthError::CustomAuthFailed("Email registration disabled".into()));
        }

        // Validate password strength
        self.validate_password(&req.password)?;

        // Create new account with email
        let mut account = UserAccount::new();
        account.email = Some(req.email.clone().to_lowercase());
        account.password_hash = Some(hash_password(&req.password));
        account.username = req.username.clone();

        self.accounts.create(&account)?;
        self.create_auth_response(&account, true)
    }

    // ==================== Custom Authentication ====================

    /// Authenticate via custom ID (for external auth systems).
    pub fn authenticate_custom(&self, req: &CustomAuthRequest) -> Result<AuthResponse> {
        // Check if custom ID exists
        if let Some(account) = self.accounts.get_by_custom(&req.id)? {
            if account.disabled {
                return Err(AuthError::AccountDisabled);
            }
            return self.create_auth_response(&account, false);
        }

        // Custom ID not found - create if requested
        if !req.create {
            return Err(AuthError::AccountNotFound);
        }

        let mut account = UserAccount::new();
        account.custom_id = Some(req.id.clone());
        account.username = req.username.clone();

        self.accounts.create(&account)?;
        self.create_auth_response(&account, true)
    }

    // ==================== Token Operations ====================

    /// Refresh access token using refresh token.
    pub fn refresh_token(&self, refresh_token: &str) -> Result<AuthResponse> {
        let claims = self.token_service.verify_refresh(refresh_token)?;

        let account = self.accounts.get(&AccountId(claims.sub.clone()))?
            .ok_or(AuthError::AccountNotFound)?;

        if account.disabled {
            return Err(AuthError::AccountDisabled);
        }

        self.create_auth_response(&account, false)
    }

    /// Verify and decode access token.
    pub fn verify_token(&self, token: &str) -> Result<ClientClaims> {
        self.token_service.verify(token)
    }

    /// Get account by ID.
    pub fn get_account(&self, id: &AccountId) -> Result<Option<UserAccount>> {
        self.accounts.get(id)
    }

    /// List all accounts with pagination.
    pub fn list_accounts(&self, limit: usize, cursor: Option<&str>) -> Result<(Vec<UserAccount>, Option<String>)> {
        self.accounts.list(limit, cursor)
    }

    /// Count total accounts.
    pub fn count_accounts(&self) -> Result<u64> {
        self.accounts.count()
    }

    /// Search accounts by query.
    pub fn search_accounts(&self, query: &str, limit: usize) -> Result<Vec<UserAccount>> {
        self.accounts.search(query, limit)
    }

    /// Delete an account.
    pub fn delete_account(&self, id: &AccountId) -> Result<bool> {
        self.accounts.delete(id)
    }

    // ==================== Account Linking ====================

    /// Link a device to an existing account.
    pub fn link_device(&self, account_id: &AccountId, req: &LinkDeviceRequest) -> Result<()> {
        // Check if device is already linked elsewhere
        if let Some(existing) = self.accounts.get_by_device(&req.device_id)? {
            if existing.id != *account_id {
                return Err(AuthError::DeviceAlreadyLinked);
            }
            // Already linked to this account
            return Ok(());
        }

        let mut account = self.accounts.get(account_id)?
            .ok_or(AuthError::AccountNotFound)?;

        account.devices.push(DeviceLink {
            device_id: req.device_id.clone(),
            linked_at: unix_timestamp(),
        });
        account.updated_at = unix_timestamp();

        self.accounts.update(&account)?;
        Ok(())
    }

    /// Link email to an existing account.
    pub fn link_email(&self, account_id: &AccountId, req: &LinkEmailRequest) -> Result<()> {
        if !is_valid_email(&req.email) {
            return Err(AuthError::InvalidEmail);
        }

        // Check if email is already registered
        if self.accounts.get_by_email(&req.email)?.is_some() {
            return Err(AuthError::EmailAlreadyRegistered);
        }

        self.validate_password(&req.password)?;

        let mut account = self.accounts.get(account_id)?
            .ok_or(AuthError::AccountNotFound)?;

        account.email = Some(req.email.clone().to_lowercase());
        account.password_hash = Some(hash_password(&req.password));
        account.updated_at = unix_timestamp();

        self.accounts.update(&account)?;
        Ok(())
    }

    /// Unlink a device from an account.
    pub fn unlink_device(&self, account_id: &AccountId, device_id: &str) -> Result<()> {
        let mut account = self.accounts.get(account_id)?
            .ok_or(AuthError::AccountNotFound)?;

        let original_len = account.devices.len();
        account.devices.retain(|d| d.device_id != device_id);

        if account.devices.len() == original_len {
            // Device wasn't linked to this account
            return Err(AuthError::AccountNotFound);
        }

        account.updated_at = unix_timestamp();
        self.accounts.update(&account)?;
        Ok(())
    }

    /// Validate token and return claims (alias for verify_token).
    pub fn validate_token(&self, token: &str) -> Result<ClientClaims> {
        self.verify_token(token)
    }

    // ==================== Account Management ====================

    /// Update account profile.
    pub fn update_account(
        &self,
        account_id: &AccountId,
        username: Option<String>,
        display_name: Option<String>,
        avatar_url: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> Result<UserAccount> {
        let mut account = self.accounts.get(account_id)?
            .ok_or(AuthError::AccountNotFound)?;

        if let Some(u) = username {
            account.username = Some(u);
        }
        if let Some(d) = display_name {
            account.display_name = Some(d);
        }
        if let Some(a) = avatar_url {
            account.avatar_url = Some(a);
        }
        if let Some(m) = metadata {
            account.metadata = m;
        }
        account.updated_at = unix_timestamp();

        self.accounts.update(&account)?;
        Ok(account)
    }

    /// Disable an account.
    pub fn disable_account(&self, account_id: &AccountId) -> Result<()> {
        let mut account = self.accounts.get(account_id)?
            .ok_or(AuthError::AccountNotFound)?;

        account.disabled = true;
        account.updated_at = unix_timestamp();
        self.accounts.update(&account)?;
        Ok(())
    }

    /// Enable a disabled account.
    pub fn enable_account(&self, account_id: &AccountId) -> Result<()> {
        let mut account = self.accounts.get(account_id)?
            .ok_or(AuthError::AccountNotFound)?;

        account.disabled = false;
        account.updated_at = unix_timestamp();
        self.accounts.update(&account)?;
        Ok(())
    }

    // ==================== Private Helpers ====================

    fn create_auth_response(&self, account: &UserAccount, created: bool) -> Result<AuthResponse> {
        let token = self.token_service.generate(
            &account.id.0,
            account.username.as_deref(),
        )?;

        let refresh_token = self.token_service.generate_refresh(&account.id.0)?;

        Ok(AuthResponse {
            token,
            refresh_token,
            account: account.into(),
            created,
        })
    }

    fn validate_password(&self, password: &str) -> Result<()> {
        if password.len() < self.config.min_password_length {
            return Err(AuthError::WeakPassword(format!(
                "Password must be at least {} characters",
                self.config.min_password_length
            )));
        }
        Ok(())
    }
}

// ==================== Helper Functions ====================

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn is_valid_email(email: &str) -> bool {
    // Basic email validation
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

fn hash_password(password: &str) -> String {
    use sha2::{Sha256, Digest};
    // In production, use bcrypt or argon2
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    // Add salt in production!
    hex::encode(hasher.finalize())
}

fn verify_password(password: &str, hash: &str) -> bool {
    hash_password(password) == hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_auth_create() {
        let auth = AuthService::new("test-secret-key-at-least-32-ch");

        let response = auth.authenticate_device(&DeviceAuthRequest {
            device_id: "device-123".to_string(),
            create: true,
            username: Some("Player1".to_string()),
        }).unwrap();

        assert!(response.created);
        assert!(!response.token.is_empty());
        assert!(!response.refresh_token.is_empty());
    }

    #[test]
    fn test_device_auth_existing() {
        let auth = AuthService::new("test-secret-key-at-least-32-ch");

        // Create account
        let first = auth.authenticate_device(&DeviceAuthRequest {
            device_id: "device-456".to_string(),
            create: true,
            username: None,
        }).unwrap();

        // Login with same device
        let second = auth.authenticate_device(&DeviceAuthRequest {
            device_id: "device-456".to_string(),
            create: false,
            username: None,
        }).unwrap();

        assert!(first.created);
        assert!(!second.created);
        assert_eq!(first.account.id, second.account.id);
    }

    #[test]
    fn test_email_auth() {
        let auth = AuthService::new("test-secret-key-at-least-32-ch");

        // Register
        let response = auth.authenticate_email(&EmailAuthRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            create: true,
            username: Some("EmailUser".to_string()),
        }).unwrap();

        assert!(response.created);

        // Login
        let login = auth.authenticate_email(&EmailAuthRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            create: false,
            username: None,
        }).unwrap();

        assert!(!login.created);
        assert_eq!(response.account.id, login.account.id);

        // Wrong password
        let bad = auth.authenticate_email(&EmailAuthRequest {
            email: "test@example.com".to_string(),
            password: "wrongpassword".to_string(),
            create: false,
            username: None,
        });

        assert!(matches!(bad, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn test_token_refresh() {
        let auth = AuthService::new("test-secret-key-at-least-32-ch");

        let response = auth.authenticate_device(&DeviceAuthRequest {
            device_id: "device-refresh".to_string(),
            create: true,
            username: None,
        }).unwrap();

        let refreshed = auth.refresh_token(&response.refresh_token).unwrap();
        assert_eq!(response.account.id, refreshed.account.id);
    }

    #[test]
    fn test_link_email_to_device_account() {
        let auth = AuthService::new("test-secret-key-at-least-32-ch");

        // Create device account
        let response = auth.authenticate_device(&DeviceAuthRequest {
            device_id: "device-link".to_string(),
            create: true,
            username: None,
        }).unwrap();

        let account_id = AccountId(response.account.id.clone());

        // Link email
        auth.link_email(&account_id, &LinkEmailRequest {
            email: "linked@example.com".to_string(),
            password: "password123".to_string(),
        }).unwrap();

        // Now can login with email
        let login = auth.authenticate_email(&EmailAuthRequest {
            email: "linked@example.com".to_string(),
            password: "password123".to_string(),
            create: false,
            username: None,
        }).unwrap();

        assert_eq!(response.account.id, login.account.id);
    }

    #[test]
    fn test_invalid_email() {
        let auth = AuthService::new("test-secret-key-at-least-32-ch");

        let result = auth.authenticate_email(&EmailAuthRequest {
            email: "not-an-email".to_string(),
            password: "password123".to_string(),
            create: true,
            username: None,
        });

        assert!(matches!(result, Err(AuthError::InvalidEmail)));
    }

    #[test]
    fn test_weak_password() {
        let auth = AuthService::new("test-secret-key-at-least-32-ch");

        let result = auth.authenticate_email(&EmailAuthRequest {
            email: "weak@example.com".to_string(),
            password: "short".to_string(),
            create: true,
            username: None,
        });

        assert!(matches!(result, Err(AuthError::WeakPassword(_))));
    }
}
