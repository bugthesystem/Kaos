//! Account storage.

use crate::console::auth::Role;
use crate::console::types::Account;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use dashmap::DashMap;
use uuid::Uuid;

/// In-memory account storage.
pub struct AccountStore {
    accounts: DashMap<Uuid, Account>,
    by_username: DashMap<String, Uuid>,
}

impl AccountStore {
    /// Create new store.
    pub fn new() -> Self {
        Self {
            accounts: DashMap::new(),
            by_username: DashMap::new(),
        }
    }

    /// Create store with default admin account.
    pub fn with_default_admin(admin_password: &str) -> Self {
        let store = Self::new();
        store
            .create("admin", admin_password, Role::Admin)
            .expect("failed to create admin account");
        store
    }

    /// Create a new account.
    pub fn create(&self, username: &str, password: &str, role: Role) -> Option<Account> {
        // Check username not taken
        if self.by_username.contains_key(username) {
            return None;
        }

        let password_hash = hash_password(password)?;
        let now = unix_timestamp();

        let account = Account {
            id: Uuid::new_v4(),
            username: username.to_string(),
            password_hash,
            role,
            created_at: now,
            last_login: None,
            disabled: false,
        };

        self.by_username.insert(username.to_string(), account.id);
        self.accounts.insert(account.id, account.clone());

        Some(account)
    }

    /// Get account by ID.
    pub fn get_by_id(&self, id: &Uuid) -> Option<Account> {
        self.accounts.get(id).map(|a| a.clone())
    }

    /// Get account by username.
    pub fn get_by_username(&self, username: &str) -> Option<Account> {
        let id = self.by_username.get(username)?;
        self.get_by_id(&id)
    }

    /// List all accounts.
    pub fn list(&self) -> Vec<Account> {
        self.accounts.iter().map(|a| a.clone()).collect()
    }

    /// Update account.
    pub fn update(&self, id: &Uuid, username: Option<&str>, role: Option<Role>, disabled: Option<bool>) -> Option<Account> {
        let mut account = self.accounts.get_mut(id)?;

        if let Some(new_username) = username {
            // Check not taken by another
            if let Some(existing_id) = self.by_username.get(new_username) {
                if *existing_id != *id {
                    return None;
                }
            }
            // Remove old username mapping
            self.by_username.remove(&account.username);
            // Add new mapping
            self.by_username.insert(new_username.to_string(), *id);
            account.username = new_username.to_string();
        }

        if let Some(new_role) = role {
            account.role = new_role;
        }

        if let Some(new_disabled) = disabled {
            account.disabled = new_disabled;
        }

        Some(account.clone())
    }

    /// Change password.
    pub fn change_password(&self, id: &Uuid, password: &str) -> bool {
        if let Some(hash) = hash_password(password) {
            if let Some(mut account) = self.accounts.get_mut(id) {
                account.password_hash = hash;
                return true;
            }
        }
        false
    }

    /// Delete account.
    pub fn delete(&self, id: &Uuid) -> Option<Account> {
        let (_, account) = self.accounts.remove(id)?;
        self.by_username.remove(&account.username);
        Some(account)
    }

    /// Verify password.
    pub fn verify_password(&self, account: &Account, password: &str) -> bool {
        verify_password(password, &account.password_hash)
    }

    /// Update last login time.
    pub fn update_last_login(&self, id: &Uuid) {
        if let Some(mut account) = self.accounts.get_mut(id) {
            account.last_login = Some(unix_timestamp());
        }
    }

    /// Count accounts.
    pub fn count(&self) -> usize {
        self.accounts.len()
    }
}

impl Default for AccountStore {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_password(password: &str) -> Option<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .ok()
        .map(|h| h.to_string())
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .ok()
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
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
    fn test_create_account() {
        let store = AccountStore::new();
        let account = store.create("testuser", "password123", Role::Developer).unwrap();

        assert_eq!(account.username, "testuser");
        assert_eq!(account.role, Role::Developer);
        assert!(!account.disabled);
    }

    #[test]
    fn test_duplicate_username() {
        let store = AccountStore::new();
        store.create("testuser", "password123", Role::Developer).unwrap();
        assert!(store.create("testuser", "different", Role::Viewer).is_none());
    }

    #[test]
    fn test_verify_password() {
        let store = AccountStore::new();
        let account = store.create("testuser", "password123", Role::Developer).unwrap();

        assert!(store.verify_password(&account, "password123"));
        assert!(!store.verify_password(&account, "wrongpassword"));
    }

    #[test]
    fn test_get_by_username() {
        let store = AccountStore::new();
        store.create("testuser", "password123", Role::Developer).unwrap();

        let account = store.get_by_username("testuser").unwrap();
        assert_eq!(account.username, "testuser");

        assert!(store.get_by_username("nonexistent").is_none());
    }

    #[test]
    fn test_update_account() {
        let store = AccountStore::new();
        let account = store.create("testuser", "password123", Role::Developer).unwrap();

        let updated = store.update(&account.id, Some("newname"), Some(Role::Admin), None).unwrap();
        assert_eq!(updated.username, "newname");
        assert_eq!(updated.role, Role::Admin);

        // Old username should not exist
        assert!(store.get_by_username("testuser").is_none());
        // New username should work
        assert!(store.get_by_username("newname").is_some());
    }

    #[test]
    fn test_delete_account() {
        let store = AccountStore::new();
        let account = store.create("testuser", "password123", Role::Developer).unwrap();

        store.delete(&account.id);

        assert!(store.get_by_id(&account.id).is_none());
        assert!(store.get_by_username("testuser").is_none());
    }

    #[test]
    fn test_default_admin() {
        let store = AccountStore::with_default_admin("admin123");
        let admin = store.get_by_username("admin").unwrap();

        assert_eq!(admin.role, Role::Admin);
        assert!(store.verify_password(&admin, "admin123"));
    }
}
