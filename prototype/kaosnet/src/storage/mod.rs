//! Storage layer for KaosNet.
//!
//! Provides persistent storage with multiple backends:
//! - In-memory (default, for development)
//! - PostgreSQL (for production, enable with `postgres` feature)
//!
//! ## Storage Types
//!
//! - **Key-Value**: Simple key-value pairs per user
//! - **Collections**: Document storage with queries (like MongoDB)
//! - **Objects**: Shared objects across users
//!
//! ## PostgreSQL Usage
//!
//! ```rust,ignore
//! use kaosnet::storage::{Storage, PostgresSyncBackend};
//!
//! // Connect to PostgreSQL
//! let backend = PostgresSyncBackend::connect("postgres://localhost/kaosnet")?;
//! backend.migrate()?;
//!
//! // Create storage with postgres backend
//! let storage = Storage::with_backend(Arc::new(backend));
//! ```

mod backend;
mod collections;
mod objects;

#[cfg(feature = "postgres")]
mod postgres;

pub use backend::{StorageBackend, MemoryBackend};
pub use collections::{Collection, Document, Query, QueryOp};
pub use objects::{StorageObject, ObjectPermission};

#[cfg(feature = "postgres")]
pub use postgres::{PostgresBackend, PostgresSyncBackend, AsyncStorageBackend};

use std::sync::Arc;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Storage error types.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("object not found: {collection}/{key}")]
    NotFound { collection: String, key: String },

    #[error("permission denied")]
    PermissionDenied,

    #[error("version conflict: expected {expected}, got {actual}")]
    VersionConflict { expected: u64, actual: u64 },

    #[error("collection not found: {0}")]
    CollectionNotFound(String),

    #[error("invalid query: {0}")]
    InvalidQuery(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("backend error: {0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// Main storage service.
pub struct Storage {
    backend: Arc<dyn StorageBackend>,
}

impl Storage {
    /// Create new storage with in-memory backend.
    pub fn new() -> Self {
        Self {
            backend: Arc::new(MemoryBackend::new()),
        }
    }

    /// Create storage with custom backend.
    pub fn with_backend(backend: Arc<dyn StorageBackend>) -> Self {
        Self { backend }
    }

    // ==================== Key-Value Operations ====================

    /// Get a value by key for a user.
    pub fn get(&self, user_id: &str, collection: &str, key: &str) -> Result<Option<StorageObject>> {
        self.backend.get(user_id, collection, key)
    }

    /// Set a value for a user.
    pub fn set(&self, user_id: &str, collection: &str, key: &str, value: serde_json::Value) -> Result<StorageObject> {
        self.backend.set(user_id, collection, key, value, None)
    }

    /// Set a value with version check (optimistic locking).
    pub fn set_if_version(&self, user_id: &str, collection: &str, key: &str, value: serde_json::Value, version: u64) -> Result<StorageObject> {
        self.backend.set(user_id, collection, key, value, Some(version))
    }

    /// Delete a key.
    pub fn delete(&self, user_id: &str, collection: &str, key: &str) -> Result<bool> {
        self.backend.delete(user_id, collection, key)
    }

    /// List all keys in a collection for a user.
    pub fn list(&self, user_id: &str, collection: &str, limit: usize, cursor: Option<&str>) -> Result<(Vec<StorageObject>, Option<String>)> {
        self.backend.list(user_id, collection, limit, cursor)
    }

    // ==================== Batch Operations ====================

    /// Get multiple objects at once.
    pub fn get_many(&self, reads: &[ObjectId]) -> Result<Vec<Option<StorageObject>>> {
        self.backend.get_many(reads)
    }

    /// Write multiple objects at once (atomic).
    pub fn write_many(&self, writes: &[WriteOp]) -> Result<Vec<StorageObject>> {
        self.backend.write_many(writes)
    }

    /// Delete multiple objects at once.
    pub fn delete_many(&self, deletes: &[ObjectId]) -> Result<usize> {
        self.backend.delete_many(deletes)
    }

    // ==================== Query Operations ====================

    /// Query objects in a collection.
    pub fn query(&self, collection: &str, query: Query, limit: usize) -> Result<Vec<StorageObject>> {
        self.backend.query(collection, query, limit)
    }

    /// Count objects matching a query.
    pub fn count(&self, collection: &str, query: Query) -> Result<u64> {
        self.backend.count(collection, query)
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

/// Object identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectId {
    pub user_id: String,
    pub collection: String,
    pub key: String,
}

impl ObjectId {
    pub fn new(user_id: impl Into<String>, collection: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            collection: collection.into(),
            key: key.into(),
        }
    }
}

/// Write operation for batch writes.
#[derive(Debug, Clone)]
pub struct WriteOp {
    pub id: ObjectId,
    pub value: serde_json::Value,
    pub version: Option<u64>,
    pub permission: ObjectPermission,
}

impl WriteOp {
    pub fn new(id: ObjectId, value: serde_json::Value) -> Self {
        Self {
            id,
            value,
            version: None,
            permission: ObjectPermission::OwnerOnly,
        }
    }

    pub fn with_version(mut self, version: u64) -> Self {
        self.version = Some(version);
        self
    }

    pub fn with_permission(mut self, permission: ObjectPermission) -> Self {
        self.permission = permission;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_storage() {
        let storage = Storage::new();

        // Set a value
        let obj = storage.set("user1", "saves", "slot1", serde_json::json!({
            "level": 5,
            "gold": 100
        })).unwrap();

        assert_eq!(obj.version, 1);

        // Get it back
        let retrieved = storage.get("user1", "saves", "slot1").unwrap().unwrap();
        assert_eq!(retrieved.value["level"], 5);

        // Update with version check
        let updated = storage.set_if_version("user1", "saves", "slot1", serde_json::json!({
            "level": 6,
            "gold": 150
        }), 1).unwrap();
        assert_eq!(updated.version, 2);

        // Version conflict
        let result = storage.set_if_version("user1", "saves", "slot1", serde_json::json!({}), 1);
        assert!(matches!(result, Err(StorageError::VersionConflict { .. })));
    }

    #[test]
    fn test_list_objects() {
        let storage = Storage::new();

        for i in 0..5 {
            storage.set("user1", "items", &format!("item{}", i), serde_json::json!({
                "name": format!("Item {}", i)
            })).unwrap();
        }

        let (items, cursor) = storage.list("user1", "items", 10, None).unwrap();
        assert_eq!(items.len(), 5);
        assert!(cursor.is_none());
    }
}
