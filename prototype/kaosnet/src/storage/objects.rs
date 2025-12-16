//! Storage object types.

use serde::{Deserialize, Serialize};

/// Permission level for storage objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectPermission {
    /// Only the owner can read/write.
    OwnerOnly,
    /// Anyone can read, only owner can write.
    PublicRead,
    /// Anyone can read/write.
    PublicReadWrite,
}

impl Default for ObjectPermission {
    fn default() -> Self {
        Self::OwnerOnly
    }
}

/// A stored object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageObject {
    /// Owner user ID.
    pub user_id: String,
    /// Collection name.
    pub collection: String,
    /// Object key within collection.
    pub key: String,
    /// JSON value.
    pub value: serde_json::Value,
    /// Version number (incremented on each update).
    pub version: u64,
    /// Read/write permissions.
    pub permission: ObjectPermission,
    /// Creation timestamp (milliseconds since epoch).
    pub created_at: i64,
    /// Last update timestamp.
    pub updated_at: i64,
}

impl StorageObject {
    /// Check if a user can read this object.
    pub fn can_read(&self, user_id: &str) -> bool {
        match self.permission {
            ObjectPermission::OwnerOnly => self.user_id == user_id,
            ObjectPermission::PublicRead | ObjectPermission::PublicReadWrite => true,
        }
    }

    /// Check if a user can write this object.
    pub fn can_write(&self, user_id: &str) -> bool {
        match self.permission {
            ObjectPermission::OwnerOnly | ObjectPermission::PublicRead => self.user_id == user_id,
            ObjectPermission::PublicReadWrite => true,
        }
    }

    /// Get a typed value from the object.
    pub fn get<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.value.clone())
    }

    /// Get a field from the value.
    pub fn get_field(&self, field: &str) -> Option<&serde_json::Value> {
        self.value.get(field)
    }

    /// Get a string field.
    pub fn get_str(&self, field: &str) -> Option<&str> {
        self.value.get(field).and_then(|v| v.as_str())
    }

    /// Get an i64 field.
    pub fn get_i64(&self, field: &str) -> Option<i64> {
        self.value.get(field).and_then(|v| v.as_i64())
    }

    /// Get an f64 field.
    pub fn get_f64(&self, field: &str) -> Option<f64> {
        self.value.get(field).and_then(|v| v.as_f64())
    }

    /// Get a bool field.
    pub fn get_bool(&self, field: &str) -> Option<bool> {
        self.value.get(field).and_then(|v| v.as_bool())
    }
}
