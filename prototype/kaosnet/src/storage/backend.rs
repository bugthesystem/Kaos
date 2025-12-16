//! Storage backend implementations.

use super::{ObjectId, Query, Result, StorageError, StorageObject, WriteOp, ObjectPermission};
use dashmap::DashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Storage backend trait.
pub trait StorageBackend: Send + Sync {
    /// Get a single object.
    fn get(&self, user_id: &str, collection: &str, key: &str) -> Result<Option<StorageObject>>;

    /// Set/update an object.
    fn set(
        &self,
        user_id: &str,
        collection: &str,
        key: &str,
        value: serde_json::Value,
        expected_version: Option<u64>,
    ) -> Result<StorageObject>;

    /// Delete an object.
    fn delete(&self, user_id: &str, collection: &str, key: &str) -> Result<bool>;

    /// List objects in a collection for a user.
    fn list(
        &self,
        user_id: &str,
        collection: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<(Vec<StorageObject>, Option<String>)>;

    /// Get multiple objects.
    fn get_many(&self, reads: &[ObjectId]) -> Result<Vec<Option<StorageObject>>>;

    /// Write multiple objects atomically.
    fn write_many(&self, writes: &[WriteOp]) -> Result<Vec<StorageObject>>;

    /// Delete multiple objects.
    fn delete_many(&self, deletes: &[ObjectId]) -> Result<usize>;

    /// Query objects across all users.
    fn query(&self, collection: &str, query: Query, limit: usize) -> Result<Vec<StorageObject>>;

    /// Count objects matching query.
    fn count(&self, collection: &str, query: Query) -> Result<u64>;
}

/// In-memory storage backend.
pub struct MemoryBackend {
    /// Storage: user_id -> collection -> key -> object
    data: DashMap<String, DashMap<String, DashMap<String, StorageObject>>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
        }
    }

    fn get_or_create_user(&self, user_id: &str) -> dashmap::mapref::one::Ref<'_, String, DashMap<String, DashMap<String, StorageObject>>> {
        self.data.entry(user_id.to_string()).or_insert_with(DashMap::new);
        self.data.get(user_id).unwrap()
    }

    fn now_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for MemoryBackend {
    fn get(&self, user_id: &str, collection: &str, key: &str) -> Result<Option<StorageObject>> {
        let user_data = match self.data.get(user_id) {
            Some(d) => d,
            None => return Ok(None),
        };

        let collection_data = match user_data.get(collection) {
            Some(c) => c,
            None => return Ok(None),
        };

        Ok(collection_data.get(key).map(|r| r.clone()))
    }

    fn set(
        &self,
        user_id: &str,
        collection: &str,
        key: &str,
        value: serde_json::Value,
        expected_version: Option<u64>,
    ) -> Result<StorageObject> {
        let user_data = self.get_or_create_user(user_id);
        user_data.entry(collection.to_string()).or_insert_with(DashMap::new);
        let collection_data = user_data.get(collection).unwrap();

        let now = Self::now_millis();

        // Check version if specified
        if let Some(expected) = expected_version {
            if let Some(existing) = collection_data.get(key) {
                if existing.version != expected {
                    return Err(StorageError::VersionConflict {
                        expected,
                        actual: existing.version,
                    });
                }
            } else if expected != 0 {
                return Err(StorageError::VersionConflict {
                    expected,
                    actual: 0,
                });
            }
        }

        let new_version = collection_data
            .get(key)
            .map(|r| r.version + 1)
            .unwrap_or(1);

        let obj = StorageObject {
            user_id: user_id.to_string(),
            collection: collection.to_string(),
            key: key.to_string(),
            value,
            version: new_version,
            permission: ObjectPermission::OwnerOnly,
            created_at: collection_data
                .get(key)
                .map(|r| r.created_at)
                .unwrap_or(now),
            updated_at: now,
        };

        collection_data.insert(key.to_string(), obj.clone());
        Ok(obj)
    }

    fn delete(&self, user_id: &str, collection: &str, key: &str) -> Result<bool> {
        let user_data = match self.data.get(user_id) {
            Some(d) => d,
            None => return Ok(false),
        };

        let collection_data = match user_data.get(collection) {
            Some(c) => c,
            None => return Ok(false),
        };

        Ok(collection_data.remove(key).is_some())
    }

    fn list(
        &self,
        user_id: &str,
        collection: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<(Vec<StorageObject>, Option<String>)> {
        let user_data = match self.data.get(user_id) {
            Some(d) => d,
            None => return Ok((vec![], None)),
        };

        let collection_data = match user_data.get(collection) {
            Some(c) => c,
            None => return Ok((vec![], None)),
        };

        let mut objects: Vec<StorageObject> = collection_data
            .iter()
            .map(|r| r.value().clone())
            .collect();

        // Sort by key for consistent ordering
        objects.sort_by(|a, b| a.key.cmp(&b.key));

        // Apply cursor (skip items before cursor)
        if let Some(cursor_key) = cursor {
            objects.retain(|o| o.key.as_str() > cursor_key);
        }

        // Limit + 1 to check if there are more
        let has_more = objects.len() > limit;
        objects.truncate(limit);

        let next_cursor = if has_more {
            objects.last().map(|o| o.key.clone())
        } else {
            None
        };

        Ok((objects, next_cursor))
    }

    fn get_many(&self, reads: &[ObjectId]) -> Result<Vec<Option<StorageObject>>> {
        reads.iter().map(|id| self.get(&id.user_id, &id.collection, &id.key)).collect()
    }

    fn write_many(&self, writes: &[WriteOp]) -> Result<Vec<StorageObject>> {
        // For memory backend, we just do sequential writes
        // A real backend would make this atomic
        let mut results = Vec::with_capacity(writes.len());
        for write in writes {
            let obj = self.set(
                &write.id.user_id,
                &write.id.collection,
                &write.id.key,
                write.value.clone(),
                write.version,
            )?;
            results.push(obj);
        }
        Ok(results)
    }

    fn delete_many(&self, deletes: &[ObjectId]) -> Result<usize> {
        let mut count = 0;
        for id in deletes {
            if self.delete(&id.user_id, &id.collection, &id.key)? {
                count += 1;
            }
        }
        Ok(count)
    }

    fn query(&self, collection: &str, query: Query, limit: usize) -> Result<Vec<StorageObject>> {
        let mut results = Vec::new();

        for user_entry in self.data.iter() {
            if let Some(collection_data) = user_entry.value().get(collection) {
                for obj_entry in collection_data.iter() {
                    let obj = obj_entry.value();
                    if query.matches(&obj.value) {
                        results.push(obj.clone());
                        if results.len() >= limit {
                            return Ok(results);
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    fn count(&self, collection: &str, query: Query) -> Result<u64> {
        let mut count = 0u64;

        for user_entry in self.data.iter() {
            if let Some(collection_data) = user_entry.value().get(collection) {
                for obj_entry in collection_data.iter() {
                    if query.matches(&obj_entry.value().value) {
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }
}
