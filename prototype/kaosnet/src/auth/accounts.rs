//! User account storage and management.

use super::{AuthError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

/// Unique account identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub String);

impl AccountId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for AccountId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for AccountId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for AccountId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Authentication provider type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthProvider {
    Device,
    Email,
    Custom,
    Google,
    Facebook,
    Apple,
    Steam,
}

/// Device link information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLink {
    pub device_id: String,
    pub linked_at: u64,
}

/// Complete user account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    /// Unique account ID
    pub id: AccountId,
    /// Optional username (unique if set)
    pub username: Option<String>,
    /// Display name (can be same as username)
    pub display_name: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Email address (unique if set)
    pub email: Option<String>,
    /// Hashed password (for email auth)
    pub password_hash: Option<String>,
    /// Custom auth ID (for external systems)
    pub custom_id: Option<String>,
    /// Linked devices
    pub devices: Vec<DeviceLink>,
    /// Account disabled flag
    pub disabled: bool,
    /// Custom metadata (JSON)
    pub metadata: serde_json::Value,
    /// Session variables (persistent)
    pub vars: std::collections::HashMap<String, String>,
    /// Creation timestamp (Unix)
    pub created_at: u64,
    /// Last update timestamp (Unix)
    pub updated_at: u64,
    /// Last login timestamp (Unix)
    pub last_login_at: Option<u64>,
    /// Verification state
    pub email_verified: bool,
}

impl UserAccount {
    pub fn new() -> Self {
        let now = unix_timestamp();
        Self {
            id: AccountId::new(),
            username: None,
            display_name: None,
            avatar_url: None,
            email: None,
            password_hash: None,
            custom_id: None,
            devices: Vec::new(),
            disabled: false,
            metadata: serde_json::Value::Null,
            vars: std::collections::HashMap::new(),
            created_at: now,
            updated_at: now,
            last_login_at: None,
            email_verified: false,
        }
    }

    /// Get the primary display identifier.
    pub fn display_identifier(&self) -> &str {
        self.display_name.as_deref()
            .or(self.username.as_deref())
            .or(self.email.as_deref())
            .unwrap_or(&self.id.0)
    }

    /// Check if account has any authentication method.
    pub fn has_auth(&self) -> bool {
        !self.devices.is_empty()
            || self.email.is_some()
            || self.custom_id.is_some()
    }
}

impl Default for UserAccount {
    fn default() -> Self {
        Self::new()
    }
}

/// Account storage trait for different backends.
pub trait AccountStore: Send + Sync {
    /// Get account by ID.
    fn get(&self, id: &AccountId) -> Result<Option<UserAccount>>;

    /// Get account by email.
    fn get_by_email(&self, email: &str) -> Result<Option<UserAccount>>;

    /// Get account by device ID.
    fn get_by_device(&self, device_id: &str) -> Result<Option<UserAccount>>;

    /// Get account by custom ID.
    fn get_by_custom(&self, custom_id: &str) -> Result<Option<UserAccount>>;

    /// Get account by username.
    fn get_by_username(&self, username: &str) -> Result<Option<UserAccount>>;

    /// Create new account.
    fn create(&self, account: &UserAccount) -> Result<()>;

    /// Update existing account.
    fn update(&self, account: &UserAccount) -> Result<()>;

    /// Delete account.
    fn delete(&self, id: &AccountId) -> Result<bool>;

    /// List accounts with pagination.
    fn list(&self, limit: usize, cursor: Option<&str>) -> Result<(Vec<UserAccount>, Option<String>)>;

    /// Count total accounts.
    fn count(&self) -> Result<u64>;

    /// Search accounts by query.
    fn search(&self, query: &str, limit: usize) -> Result<Vec<UserAccount>>;
}

/// In-memory account store (for development/testing).
pub struct MemoryAccountStore {
    accounts: DashMap<String, UserAccount>,
    by_email: DashMap<String, String>,
    by_device: DashMap<String, String>,
    by_custom: DashMap<String, String>,
    by_username: DashMap<String, String>,
    counter: AtomicU64,
}

impl MemoryAccountStore {
    pub fn new() -> Self {
        Self {
            accounts: DashMap::new(),
            by_email: DashMap::new(),
            by_device: DashMap::new(),
            by_custom: DashMap::new(),
            by_username: DashMap::new(),
            counter: AtomicU64::new(0),
        }
    }

    fn index_account(&self, account: &UserAccount) {
        // Index email
        if let Some(email) = &account.email {
            self.by_email.insert(email.to_lowercase(), account.id.0.clone());
        }

        // Index devices
        for device in &account.devices {
            self.by_device.insert(device.device_id.clone(), account.id.0.clone());
        }

        // Index custom ID
        if let Some(custom_id) = &account.custom_id {
            self.by_custom.insert(custom_id.clone(), account.id.0.clone());
        }

        // Index username
        if let Some(username) = &account.username {
            self.by_username.insert(username.to_lowercase(), account.id.0.clone());
        }
    }

    fn remove_indexes(&self, account: &UserAccount) {
        if let Some(email) = &account.email {
            self.by_email.remove(&email.to_lowercase());
        }
        for device in &account.devices {
            self.by_device.remove(&device.device_id);
        }
        if let Some(custom_id) = &account.custom_id {
            self.by_custom.remove(custom_id);
        }
        if let Some(username) = &account.username {
            self.by_username.remove(&username.to_lowercase());
        }
    }
}

impl Default for MemoryAccountStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountStore for MemoryAccountStore {
    fn get(&self, id: &AccountId) -> Result<Option<UserAccount>> {
        Ok(self.accounts.get(&id.0).map(|r| r.clone()))
    }

    fn get_by_email(&self, email: &str) -> Result<Option<UserAccount>> {
        let email_lower = email.to_lowercase();
        if let Some(id) = self.by_email.get(&email_lower) {
            return self.get(&AccountId(id.clone()));
        }
        Ok(None)
    }

    fn get_by_device(&self, device_id: &str) -> Result<Option<UserAccount>> {
        if let Some(id) = self.by_device.get(device_id) {
            return self.get(&AccountId(id.clone()));
        }
        Ok(None)
    }

    fn get_by_custom(&self, custom_id: &str) -> Result<Option<UserAccount>> {
        if let Some(id) = self.by_custom.get(custom_id) {
            return self.get(&AccountId(id.clone()));
        }
        Ok(None)
    }

    fn get_by_username(&self, username: &str) -> Result<Option<UserAccount>> {
        let username_lower = username.to_lowercase();
        if let Some(id) = self.by_username.get(&username_lower) {
            return self.get(&AccountId(id.clone()));
        }
        Ok(None)
    }

    fn create(&self, account: &UserAccount) -> Result<()> {
        // Check for uniqueness
        if let Some(email) = &account.email {
            if self.by_email.contains_key(&email.to_lowercase()) {
                return Err(AuthError::EmailAlreadyRegistered);
            }
        }

        for device in &account.devices {
            if self.by_device.contains_key(&device.device_id) {
                return Err(AuthError::DeviceAlreadyLinked);
            }
        }

        if let Some(username) = &account.username {
            if self.by_username.contains_key(&username.to_lowercase()) {
                return Err(AuthError::AccountExists);
            }
        }

        self.accounts.insert(account.id.0.clone(), account.clone());
        self.index_account(account);
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn update(&self, account: &UserAccount) -> Result<()> {
        // Get old account to remove old indexes
        if let Some(old) = self.accounts.get(&account.id.0) {
            self.remove_indexes(&old);
        }

        self.accounts.insert(account.id.0.clone(), account.clone());
        self.index_account(account);
        Ok(())
    }

    fn delete(&self, id: &AccountId) -> Result<bool> {
        if let Some((_, account)) = self.accounts.remove(&id.0) {
            self.remove_indexes(&account);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn list(&self, limit: usize, cursor: Option<&str>) -> Result<(Vec<UserAccount>, Option<String>)> {
        let mut accounts: Vec<UserAccount> = self.accounts
            .iter()
            .map(|r| r.value().clone())
            .collect();

        // Sort by created_at desc
        accounts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply cursor (simple offset-based for memory store)
        let start = cursor.and_then(|c| c.parse::<usize>().ok()).unwrap_or(0);
        let end = (start + limit).min(accounts.len());

        let page = accounts[start..end].to_vec();
        let next_cursor = if end < accounts.len() {
            Some(end.to_string())
        } else {
            None
        };

        Ok((page, next_cursor))
    }

    fn count(&self) -> Result<u64> {
        Ok(self.accounts.len() as u64)
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<UserAccount>> {
        let query_lower = query.to_lowercase();
        let results: Vec<UserAccount> = self.accounts
            .iter()
            .filter(|r| {
                let account = r.value();
                account.username.as_ref().map(|u| u.to_lowercase().contains(&query_lower)).unwrap_or(false)
                    || account.display_name.as_ref().map(|d| d.to_lowercase().contains(&query_lower)).unwrap_or(false)
                    || account.email.as_ref().map(|e| e.to_lowercase().contains(&query_lower)).unwrap_or(false)
                    || account.id.0.contains(&query_lower)
            })
            .take(limit)
            .map(|r| r.value().clone())
            .collect();

        Ok(results)
    }
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_account() {
        let store = MemoryAccountStore::new();
        let mut account = UserAccount::new();
        account.email = Some("test@example.com".to_string());
        account.username = Some("testuser".to_string());

        store.create(&account).unwrap();

        let retrieved = store.get(&account.id).unwrap().unwrap();
        assert_eq!(retrieved.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_get_by_email() {
        let store = MemoryAccountStore::new();
        let mut account = UserAccount::new();
        account.email = Some("Test@Example.com".to_string());

        store.create(&account).unwrap();

        // Case insensitive lookup
        let found = store.get_by_email("test@example.com").unwrap().unwrap();
        assert_eq!(found.id, account.id);
    }

    #[test]
    fn test_get_by_device() {
        let store = MemoryAccountStore::new();
        let mut account = UserAccount::new();
        account.devices.push(DeviceLink {
            device_id: "device-123".to_string(),
            linked_at: unix_timestamp(),
        });

        store.create(&account).unwrap();

        let found = store.get_by_device("device-123").unwrap().unwrap();
        assert_eq!(found.id, account.id);
    }

    #[test]
    fn test_unique_email() {
        let store = MemoryAccountStore::new();

        let mut account1 = UserAccount::new();
        account1.email = Some("unique@example.com".to_string());
        store.create(&account1).unwrap();

        let mut account2 = UserAccount::new();
        account2.email = Some("unique@example.com".to_string());
        let result = store.create(&account2);

        assert!(matches!(result, Err(AuthError::EmailAlreadyRegistered)));
    }

    #[test]
    fn test_update_account() {
        let store = MemoryAccountStore::new();

        let mut account = UserAccount::new();
        account.username = Some("oldname".to_string());
        store.create(&account).unwrap();

        account.username = Some("newname".to_string());
        store.update(&account).unwrap();

        let found = store.get_by_username("newname").unwrap().unwrap();
        assert_eq!(found.id, account.id);

        // Old username should not work
        let not_found = store.get_by_username("oldname").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_search() {
        let store = MemoryAccountStore::new();

        for i in 0..5 {
            let mut account = UserAccount::new();
            account.username = Some(format!("player{}", i));
            store.create(&account).unwrap();
        }

        let results = store.search("player", 10).unwrap();
        assert_eq!(results.len(), 5);

        let results = store.search("player2", 10).unwrap();
        assert_eq!(results.len(), 1);
    }
}
