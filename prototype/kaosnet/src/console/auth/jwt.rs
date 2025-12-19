//! JWT token handling.

use crate::console::auth::Role;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT claims.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Role
    pub role: String,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issued at (Unix timestamp)
    pub iat: u64,
}

/// JWT service for token generation and verification.
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    expiry_secs: u64,
}

impl JwtService {
    /// Create new JWT service.
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            expiry_secs: 86400, // 24 hours (console sessions should persist longer)
        }
    }

    /// Set token expiry time in seconds.
    pub fn with_expiry(mut self, secs: u64) -> Self {
        self.expiry_secs = secs;
        self
    }

    /// Generate a JWT token.
    pub fn generate(&self, user_id: &str, role: Role) -> Option<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();

        let claims = Claims {
            sub: user_id.to_string(),
            role: role.as_str().to_string(),
            exp: now + self.expiry_secs,
            iat: now,
        };

        encode(&Header::default(), &claims, &self.encoding_key).ok()
    }

    /// Verify and decode a JWT token.
    pub fn verify(&self, token: &str) -> Option<Claims> {
        let validation = Validation::default();
        decode::<Claims>(token, &self.decoding_key, &validation)
            .ok()
            .map(|data| data.claims)
    }

    /// Get expiry time in seconds.
    pub fn expiry_secs(&self) -> u64 {
        self.expiry_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_roundtrip() {
        let service = JwtService::new("test-secret-key-at-least-32-chars");
        let token = service.generate("user-123", Role::Admin).unwrap();
        let claims = service.verify(&token).unwrap();

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.role, "admin");
    }

    #[test]
    fn test_jwt_invalid_token() {
        let service = JwtService::new("test-secret-key-at-least-32-chars");
        assert!(service.verify("invalid-token").is_none());
    }

    #[test]
    fn test_jwt_wrong_secret() {
        let service1 = JwtService::new("secret-key-one-at-least-32-chars");
        let service2 = JwtService::new("secret-key-two-at-least-32-chars");

        let token = service1.generate("user-123", Role::Admin).unwrap();
        assert!(service2.verify(&token).is_none());
    }
}
