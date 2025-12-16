//! API key storage.

use crate::console::auth::ApiKeyService;
use crate::console::types::{ApiKey, ApiKeyScope};
use dashmap::DashMap;
use uuid::Uuid;

/// In-memory API key storage.
pub struct ApiKeyStore {
    keys: DashMap<Uuid, ApiKey>,
    service: ApiKeyService,
}

impl ApiKeyStore {
    /// Create new store.
    pub fn new() -> Self {
        Self {
            keys: DashMap::new(),
            service: ApiKeyService::new(),
        }
    }

    /// Create a new API key.
    /// Returns (api_key_info, raw_key).
    pub fn create(
        &self,
        name: &str,
        scopes: ApiKeyScope,
        created_by: Uuid,
        expires_in_days: Option<u32>,
    ) -> (ApiKey, String) {
        let (raw_key, key_hash, key_prefix) = self.service.generate();

        let now = unix_timestamp();
        let expires_at = expires_in_days.map(|days| now + (days as i64 * 24 * 60 * 60));

        let key = ApiKey {
            id: Uuid::new_v4(),
            name: name.to_string(),
            key_hash,
            key_prefix,
            scopes,
            created_by,
            created_at: now,
            expires_at,
            last_used: None,
            request_count: 0,
            disabled: false,
        };

        self.keys.insert(key.id, key.clone());

        (key, raw_key)
    }

    /// Verify a raw API key and return the key info if valid.
    pub fn verify(&self, raw_key: &str) -> Option<ApiKey> {
        for entry in self.keys.iter() {
            if self.service.verify(raw_key, &entry.key_hash) {
                return Some(entry.clone());
            }
        }
        None
    }

    /// Get key by ID.
    pub fn get(&self, id: &Uuid) -> Option<ApiKey> {
        self.keys.get(id).map(|k| k.clone())
    }

    /// List all keys.
    pub fn list(&self) -> Vec<ApiKey> {
        self.keys.iter().map(|k| k.clone()).collect()
    }

    /// List keys created by a user.
    pub fn list_by_creator(&self, user_id: &Uuid) -> Vec<ApiKey> {
        self.keys
            .iter()
            .filter(|k| k.created_by == *user_id)
            .map(|k| k.clone())
            .collect()
    }

    /// Delete a key.
    pub fn delete(&self, id: &Uuid) -> Option<ApiKey> {
        self.keys.remove(id).map(|(_, k)| k)
    }

    /// Disable/enable a key.
    pub fn set_disabled(&self, id: &Uuid, disabled: bool) -> bool {
        if let Some(mut key) = self.keys.get_mut(id) {
            key.disabled = disabled;
            true
        } else {
            false
        }
    }

    /// Record usage of a key.
    pub fn record_usage(&self, id: &Uuid) {
        if let Some(mut key) = self.keys.get_mut(id) {
            key.last_used = Some(unix_timestamp());
            key.request_count += 1;
        }
    }

    /// Count keys.
    pub fn count(&self) -> usize {
        self.keys.len()
    }
}

impl Default for ApiKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_key() {
        let store = ApiKeyStore::new();
        let user_id = Uuid::new_v4();

        let (key, raw_key) = store.create("test-key", ApiKeyScope::READ_ALL, user_id, None);

        assert_eq!(key.name, "test-key");
        assert!(raw_key.starts_with("kn_"));
        assert!(key.expires_at.is_none());
    }

    #[test]
    fn test_verify_key() {
        let store = ApiKeyStore::new();
        let user_id = Uuid::new_v4();

        let (key, raw_key) = store.create("test-key", ApiKeyScope::READ_ALL, user_id, None);

        let verified = store.verify(&raw_key).unwrap();
        assert_eq!(verified.id, key.id);

        assert!(store.verify("invalid-key").is_none());
    }

    #[test]
    fn test_key_with_expiry() {
        let store = ApiKeyStore::new();
        let user_id = Uuid::new_v4();

        let (key, _) = store.create("test-key", ApiKeyScope::READ_ALL, user_id, Some(30));

        assert!(key.expires_at.is_some());
        assert!(!key.is_expired());
    }

    #[test]
    fn test_delete_key() {
        let store = ApiKeyStore::new();
        let user_id = Uuid::new_v4();

        let (key, raw_key) = store.create("test-key", ApiKeyScope::READ_ALL, user_id, None);

        store.delete(&key.id);

        assert!(store.verify(&raw_key).is_none());
        assert!(store.get(&key.id).is_none());
    }

    #[test]
    fn test_record_usage() {
        let store = ApiKeyStore::new();
        let user_id = Uuid::new_v4();

        let (key, _) = store.create("test-key", ApiKeyScope::READ_ALL, user_id, None);

        assert_eq!(key.request_count, 0);
        assert!(key.last_used.is_none());

        store.record_usage(&key.id);
        store.record_usage(&key.id);

        let updated = store.get(&key.id).unwrap();
        assert_eq!(updated.request_count, 2);
        assert!(updated.last_used.is_some());
    }

    #[test]
    fn test_list_by_creator() {
        let store = ApiKeyStore::new();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        store.create("key1", ApiKeyScope::READ_ALL, user1, None);
        store.create("key2", ApiKeyScope::READ_ALL, user1, None);
        store.create("key3", ApiKeyScope::READ_ALL, user2, None);

        let user1_keys = store.list_by_creator(&user1);
        assert_eq!(user1_keys.len(), 2);

        let user2_keys = store.list_by_creator(&user2);
        assert_eq!(user2_keys.len(), 1);
    }
}
