//! Client session tokens (separate from console JWT).
//!
//! Provides access tokens and refresh tokens for game clients.

use super::{AuthError, Result};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Client JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientClaims {
    /// Subject (user/account ID)
    pub sub: String,
    /// Username (optional)
    pub username: Option<String>,
    /// Token type: "access" or "refresh"
    pub typ: String,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// JWT ID (unique token identifier)
    pub jti: String,
}

/// Access token wrapper for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientToken {
    pub token: String,
    pub expires_at: u64,
}

/// Refresh token wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token: String,
    pub expires_at: u64,
}

/// Token service for generating and verifying client tokens.
pub struct TokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_expiry_secs: u64,
    refresh_expiry_secs: u64,
}

impl TokenService {
    /// Create new token service.
    pub fn new(secret: &str, access_expiry_secs: u64, refresh_expiry_secs: u64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_expiry_secs,
            refresh_expiry_secs,
        }
    }

    /// Generate an access token.
    pub fn generate(&self, user_id: &str, username: Option<&str>) -> Result<String> {
        let now = unix_timestamp();
        let exp = now + self.access_expiry_secs;

        let claims = ClientClaims {
            sub: user_id.to_string(),
            username: username.map(|s| s.to_string()),
            typ: "access".to_string(),
            exp,
            iat: now,
            jti: generate_jti(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::Internal(format!("Token generation failed: {}", e)))
    }

    /// Generate a refresh token.
    pub fn generate_refresh(&self, user_id: &str) -> Result<String> {
        let now = unix_timestamp();
        let exp = now + self.refresh_expiry_secs;

        let claims = ClientClaims {
            sub: user_id.to_string(),
            username: None,
            typ: "refresh".to_string(),
            exp,
            iat: now,
            jti: generate_jti(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::Internal(format!("Refresh token generation failed: {}", e)))
    }

    /// Verify and decode an access token.
    pub fn verify(&self, token: &str) -> Result<ClientClaims> {
        let validation = Validation::default();
        let data = decode::<ClientClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::InvalidToken,
            })?;

        if data.claims.typ != "access" {
            return Err(AuthError::InvalidToken);
        }

        Ok(data.claims)
    }

    /// Verify and decode a refresh token.
    pub fn verify_refresh(&self, token: &str) -> Result<ClientClaims> {
        let validation = Validation::default();
        let data = decode::<ClientClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::InvalidToken,
            })?;

        if data.claims.typ != "refresh" {
            return Err(AuthError::InvalidToken);
        }

        Ok(data.claims)
    }

    /// Get access token expiry in seconds.
    #[allow(dead_code)]
    pub fn access_expiry_secs(&self) -> u64 {
        self.access_expiry_secs
    }

    /// Get refresh token expiry in seconds.
    #[allow(dead_code)]
    pub fn refresh_expiry_secs(&self) -> u64 {
        self.refresh_expiry_secs
    }
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn generate_jti() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = unix_timestamp();
    format!("{:x}-{:x}", timestamp, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_token_roundtrip() {
        let service = TokenService::new("test-secret-key-at-least-32-ch", 3600, 604800);

        let token = service.generate("user-123", Some("Player1")).unwrap();
        let claims = service.verify(&token).unwrap();

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.username, Some("Player1".to_string()));
        assert_eq!(claims.typ, "access");
    }

    #[test]
    fn test_refresh_token_roundtrip() {
        let service = TokenService::new("test-secret-key-at-least-32-ch", 3600, 604800);

        let token = service.generate_refresh("user-456").unwrap();
        let claims = service.verify_refresh(&token).unwrap();

        assert_eq!(claims.sub, "user-456");
        assert_eq!(claims.typ, "refresh");
    }

    #[test]
    fn test_access_token_rejected_as_refresh() {
        let service = TokenService::new("test-secret-key-at-least-32-ch", 3600, 604800);

        let access_token = service.generate("user-789", None).unwrap();
        let result = service.verify_refresh(&access_token);

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_refresh_token_rejected_as_access() {
        let service = TokenService::new("test-secret-key-at-least-32-ch", 3600, 604800);

        let refresh_token = service.generate_refresh("user-abc").unwrap();
        let result = service.verify(&refresh_token);

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_invalid_token() {
        let service = TokenService::new("test-secret-key-at-least-32-ch", 3600, 604800);

        let result = service.verify("not.a.valid.token");
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_wrong_secret() {
        let service1 = TokenService::new("secret-key-one-at-least-32-chars", 3600, 604800);
        let service2 = TokenService::new("secret-key-two-at-least-32-chars", 3600, 604800);

        let token = service1.generate("user-xyz", None).unwrap();
        let result = service2.verify(&token);

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_unique_jti() {
        let service = TokenService::new("test-secret-key-at-least-32-ch", 3600, 604800);

        let token1 = service.generate("user-1", None).unwrap();
        let token2 = service.generate("user-1", None).unwrap();

        let claims1 = service.verify(&token1).unwrap();
        let claims2 = service.verify(&token2).unwrap();

        assert_ne!(claims1.jti, claims2.jti);
    }
}
