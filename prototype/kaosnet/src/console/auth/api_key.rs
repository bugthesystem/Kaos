//! API key handling.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// API key service for generation and hashing.
pub struct ApiKeyService {
    key_length: usize,
}

impl ApiKeyService {
    /// Create new API key service.
    pub fn new() -> Self {
        Self { key_length: 32 }
    }

    /// Generate a new API key.
    /// Returns (raw_key, key_hash, key_prefix).
    pub fn generate(&self) -> (String, String, String) {
        let mut bytes = vec![0u8; self.key_length];
        rand::thread_rng().fill_bytes(&mut bytes);

        let raw_key = format!("kn_{}", URL_SAFE_NO_PAD.encode(&bytes));
        let key_hash = self.hash(&raw_key);
        let key_prefix = raw_key.chars().take(11).collect(); // "kn_" + 8 chars

        (raw_key, key_hash, key_prefix)
    }

    /// Hash an API key for storage.
    pub fn hash(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        URL_SAFE_NO_PAD.encode(result)
    }

    /// Verify a key against a hash.
    pub fn verify(&self, key: &str, hash: &str) -> bool {
        self.hash(key) == hash
    }
}

impl Default for ApiKeyService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_generation() {
        let service = ApiKeyService::new();
        let (key, hash, prefix) = service.generate();

        assert!(key.starts_with("kn_"));
        assert!(!hash.is_empty());
        assert!(prefix.starts_with("kn_"));
        assert!(key.starts_with(&prefix));
    }

    #[test]
    fn test_api_key_verification() {
        let service = ApiKeyService::new();
        let (key, hash, _) = service.generate();

        assert!(service.verify(&key, &hash));
        assert!(!service.verify("wrong-key", &hash));
    }

    #[test]
    fn test_api_key_uniqueness() {
        let service = ApiKeyService::new();
        let (key1, _, _) = service.generate();
        let (key2, _, _) = service.generate();

        assert_ne!(key1, key2);
    }
}
