//! PostgreSQL storage backend implementation.
//!
//! Provides persistent storage using PostgreSQL with connection pooling.
//!
//! Enable with `postgres` feature flag:
//! ```toml
//! kaosnet = { version = "0.1", features = ["postgres"] }
//! ```

use super::{ObjectId, ObjectPermission, Query, Result, StorageBackend, StorageError, StorageObject, WriteOp};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// PostgreSQL storage backend (async).
pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend with connection string.
    pub async fn new(database_url: &str) -> std::result::Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Create with existing pool.
    pub fn with_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run migrations to set up the schema.
    pub async fn migrate(&self) -> std::result::Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS storage_objects (
                user_id TEXT NOT NULL,
                collection TEXT NOT NULL,
                key TEXT NOT NULL,
                value JSONB NOT NULL,
                version BIGINT NOT NULL DEFAULT 1,
                permission INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                PRIMARY KEY (user_id, collection, key)
            );

            CREATE INDEX IF NOT EXISTS idx_storage_collection ON storage_objects(collection);
            CREATE INDEX IF NOT EXISTS idx_storage_user_collection ON storage_objects(user_id, collection);
            "#
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    fn int_to_permission(i: i32) -> ObjectPermission {
        match i {
            1 => ObjectPermission::PublicRead,
            2 => ObjectPermission::PublicReadWrite,
            _ => ObjectPermission::OwnerOnly,
        }
    }

    // Async implementations
    pub async fn get_async(&self, user_id: &str, collection: &str, key: &str) -> Result<Option<StorageObject>> {
        let row = sqlx::query(
            r#"
            SELECT user_id, collection, key, value, version, permission,
                   EXTRACT(EPOCH FROM created_at)::bigint * 1000 as created_at,
                   EXTRACT(EPOCH FROM updated_at)::bigint * 1000 as updated_at
            FROM storage_objects
            WHERE user_id = $1 AND collection = $2 AND key = $3
            "#
        )
        .bind(user_id)
        .bind(collection)
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(row.map(|r| StorageObject {
            user_id: r.get("user_id"),
            collection: r.get("collection"),
            key: r.get("key"),
            value: r.get("value"),
            version: r.get::<i64, _>("version") as u64,
            permission: Self::int_to_permission(r.get("permission")),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    pub async fn set_async(
        &self,
        user_id: &str,
        collection: &str,
        key: &str,
        value: Value,
        expected_version: Option<u64>,
    ) -> Result<StorageObject> {
        // Check version if specified
        if let Some(expected) = expected_version {
            let existing = self.get_async(user_id, collection, key).await?;
            match existing {
                Some(obj) if obj.version != expected => {
                    return Err(StorageError::VersionConflict {
                        expected,
                        actual: obj.version,
                    });
                }
                None if expected != 0 => {
                    return Err(StorageError::VersionConflict {
                        expected,
                        actual: 0,
                    });
                }
                _ => {}
            }
        }

        // Upsert with version increment
        let row = sqlx::query(
            r#"
            INSERT INTO storage_objects (user_id, collection, key, value, version, permission, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 1, 0, NOW(), NOW())
            ON CONFLICT (user_id, collection, key)
            DO UPDATE SET
                value = $4,
                version = storage_objects.version + 1,
                updated_at = NOW()
            RETURNING user_id, collection, key, value, version, permission,
                      EXTRACT(EPOCH FROM created_at)::bigint * 1000 as created_at,
                      EXTRACT(EPOCH FROM updated_at)::bigint * 1000 as updated_at
            "#
        )
        .bind(user_id)
        .bind(collection)
        .bind(key)
        .bind(&value)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(StorageObject {
            user_id: row.get("user_id"),
            collection: row.get("collection"),
            key: row.get("key"),
            value: row.get("value"),
            version: row.get::<i64, _>("version") as u64,
            permission: Self::int_to_permission(row.get("permission")),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn delete_async(&self, user_id: &str, collection: &str, key: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM storage_objects WHERE user_id = $1 AND collection = $2 AND key = $3"
        )
        .bind(user_id)
        .bind(collection)
        .bind(key)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_async(
        &self,
        user_id: &str,
        collection: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<(Vec<StorageObject>, Option<String>)> {
        let rows = match cursor {
            Some(cursor_key) => {
                sqlx::query(
                    r#"
                    SELECT user_id, collection, key, value, version, permission,
                           EXTRACT(EPOCH FROM created_at)::bigint * 1000 as created_at,
                           EXTRACT(EPOCH FROM updated_at)::bigint * 1000 as updated_at
                    FROM storage_objects
                    WHERE user_id = $1 AND collection = $2 AND key > $3
                    ORDER BY key
                    LIMIT $4
                    "#
                )
                .bind(user_id)
                .bind(collection)
                .bind(cursor_key)
                .bind((limit + 1) as i64)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query(
                    r#"
                    SELECT user_id, collection, key, value, version, permission,
                           EXTRACT(EPOCH FROM created_at)::bigint * 1000 as created_at,
                           EXTRACT(EPOCH FROM updated_at)::bigint * 1000 as updated_at
                    FROM storage_objects
                    WHERE user_id = $1 AND collection = $2
                    ORDER BY key
                    LIMIT $3
                    "#
                )
                .bind(user_id)
                .bind(collection)
                .bind((limit + 1) as i64)
                .fetch_all(&self.pool)
                .await
            }
        }.map_err(|e| StorageError::Backend(e.to_string()))?;

        let mut objects: Vec<StorageObject> = rows.iter().map(|r| StorageObject {
            user_id: r.get("user_id"),
            collection: r.get("collection"),
            key: r.get("key"),
            value: r.get("value"),
            version: r.get::<i64, _>("version") as u64,
            permission: Self::int_to_permission(r.get("permission")),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }).collect();

        let has_more = objects.len() > limit;
        objects.truncate(limit);

        let next_cursor = if has_more {
            objects.last().map(|o| o.key.clone())
        } else {
            None
        };

        Ok((objects, next_cursor))
    }

    pub async fn query_async(&self, collection: &str, query: Query, limit: usize) -> Result<Vec<StorageObject>> {
        // Fetch and filter in Rust (can be optimized with JSONB queries later)
        let rows = sqlx::query(
            r#"
            SELECT user_id, collection, key, value, version, permission,
                   EXTRACT(EPOCH FROM created_at)::bigint * 1000 as created_at,
                   EXTRACT(EPOCH FROM updated_at)::bigint * 1000 as updated_at
            FROM storage_objects
            WHERE collection = $1
            LIMIT $2
            "#
        )
        .bind(collection)
        .bind((limit * 10) as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        let mut results = Vec::new();
        for r in rows {
            let value: Value = r.get("value");
            if query.matches(&value) {
                results.push(StorageObject {
                    user_id: r.get("user_id"),
                    collection: r.get("collection"),
                    key: r.get("key"),
                    value,
                    version: r.get::<i64, _>("version") as u64,
                    permission: Self::int_to_permission(r.get("permission")),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                });
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    pub async fn count_async(&self, collection: &str, query: Query) -> Result<u64> {
        let rows = sqlx::query("SELECT value FROM storage_objects WHERE collection = $1")
            .bind(collection)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let count = rows.iter()
            .filter(|r| {
                let value: Value = r.get("value");
                query.matches(&value)
            })
            .count();

        Ok(count as u64)
    }

    pub async fn get_many_async(&self, reads: &[ObjectId]) -> Result<Vec<Option<StorageObject>>> {
        let mut results = Vec::with_capacity(reads.len());
        for id in reads {
            results.push(self.get_async(&id.user_id, &id.collection, &id.key).await?);
        }
        Ok(results)
    }

    pub async fn write_many_async(&self, writes: &[WriteOp]) -> Result<Vec<StorageObject>> {
        let mut results = Vec::with_capacity(writes.len());
        for write in writes {
            let obj = self.set_async(
                &write.id.user_id,
                &write.id.collection,
                &write.id.key,
                write.value.clone(),
                write.version,
            ).await?;
            results.push(obj);
        }
        Ok(results)
    }

    pub async fn delete_many_async(&self, deletes: &[ObjectId]) -> Result<usize> {
        let mut count = 0;
        for id in deletes {
            if self.delete_async(&id.user_id, &id.collection, &id.key).await? {
                count += 1;
            }
        }
        Ok(count)
    }

    /// List all unique collection names.
    pub async fn list_collections_async(&self) -> Vec<String> {
        let rows = sqlx::query("SELECT DISTINCT collection FROM storage_objects ORDER BY collection")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        rows.iter().map(|r| r.get("collection")).collect()
    }

    /// List all objects in a collection (across all users).
    pub async fn list_all_in_collection_async(&self, collection: &str) -> Vec<StorageObject> {
        let rows = sqlx::query(
            r#"
            SELECT user_id, collection, key, value, version, permission,
                   EXTRACT(EPOCH FROM created_at)::bigint * 1000 as created_at,
                   EXTRACT(EPOCH FROM updated_at)::bigint * 1000 as updated_at
            FROM storage_objects
            WHERE collection = $1
            ORDER BY key
            "#
        )
        .bind(collection)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows.iter().map(|r| StorageObject {
            user_id: r.get("user_id"),
            collection: r.get("collection"),
            key: r.get("key"),
            value: r.get("value"),
            version: r.get::<i64, _>("version") as u64,
            permission: Self::int_to_permission(r.get("permission")),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }).collect()
    }
}

/// Sync wrapper for PostgresBackend that implements StorageBackend.
///
/// Uses `tokio::task::block_in_place` to safely run async operations from sync context,
/// even when called from within an async runtime.
pub struct PostgresSyncBackend {
    inner: Arc<PostgresBackend>,
    /// Handle to the runtime (for operations outside async context)
    runtime: Arc<Runtime>,
}

impl PostgresSyncBackend {
    /// Create a new sync backend from an async backend.
    pub fn new(backend: PostgresBackend, runtime: Runtime) -> Self {
        Self {
            inner: Arc::new(backend),
            runtime: Arc::new(runtime),
        }
    }

    /// Create from a database URL.
    ///
    /// This creates a new tokio runtime internally.
    pub fn connect(database_url: &str) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let runtime = Runtime::new()?;
        let backend = runtime.block_on(PostgresBackend::new(database_url))?;
        Ok(Self::new(backend, runtime))
    }

    /// Run database migrations.
    pub fn migrate(&self) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.runtime.block_on(self.inner.migrate())?;
        Ok(())
    }

    /// Helper to run async code from sync context, handling both
    /// cases where we're inside or outside an async runtime.
    fn block_on_async<F, T>(&self, f: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        // Check if we're inside a tokio runtime
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // We're inside a runtime - use block_in_place to avoid nested runtime panic
            tokio::task::block_in_place(|| handle.block_on(f))
        } else {
            // We're outside a runtime - use our own runtime
            self.runtime.block_on(f)
        }
    }
}

impl StorageBackend for PostgresSyncBackend {
    fn get(&self, user_id: &str, collection: &str, key: &str) -> Result<Option<StorageObject>> {
        self.block_on_async(self.inner.get_async(user_id, collection, key))
    }

    fn set(
        &self,
        user_id: &str,
        collection: &str,
        key: &str,
        value: Value,
        expected_version: Option<u64>,
    ) -> Result<StorageObject> {
        self.block_on_async(self.inner.set_async(user_id, collection, key, value, expected_version))
    }

    fn delete(&self, user_id: &str, collection: &str, key: &str) -> Result<bool> {
        self.block_on_async(self.inner.delete_async(user_id, collection, key))
    }

    fn list(
        &self,
        user_id: &str,
        collection: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<(Vec<StorageObject>, Option<String>)> {
        self.block_on_async(self.inner.list_async(user_id, collection, limit, cursor))
    }

    fn get_many(&self, reads: &[ObjectId]) -> Result<Vec<Option<StorageObject>>> {
        self.block_on_async(self.inner.get_many_async(reads))
    }

    fn write_many(&self, writes: &[WriteOp]) -> Result<Vec<StorageObject>> {
        self.block_on_async(self.inner.write_many_async(writes))
    }

    fn delete_many(&self, deletes: &[ObjectId]) -> Result<usize> {
        self.block_on_async(self.inner.delete_many_async(deletes))
    }

    fn query(&self, collection: &str, query: Query, limit: usize) -> Result<Vec<StorageObject>> {
        self.block_on_async(self.inner.query_async(collection, query, limit))
    }

    fn count(&self, collection: &str, query: Query) -> Result<u64> {
        self.block_on_async(self.inner.count_async(collection, query))
    }

    fn list_collections(&self) -> Vec<String> {
        self.block_on_async(self.inner.list_collections_async())
    }

    fn list_all_in_collection(&self, collection: &str) -> Vec<StorageObject> {
        self.block_on_async(self.inner.list_all_in_collection_async(collection))
    }
}

/// Async storage backend trait.
#[async_trait::async_trait]
pub trait AsyncStorageBackend: Send + Sync {
    async fn get(&self, user_id: &str, collection: &str, key: &str) -> Result<Option<StorageObject>>;
    async fn set(&self, user_id: &str, collection: &str, key: &str, value: Value, expected_version: Option<u64>) -> Result<StorageObject>;
    async fn delete(&self, user_id: &str, collection: &str, key: &str) -> Result<bool>;
    async fn list(&self, user_id: &str, collection: &str, limit: usize, cursor: Option<&str>) -> Result<(Vec<StorageObject>, Option<String>)>;
    async fn query(&self, collection: &str, query: Query, limit: usize) -> Result<Vec<StorageObject>>;
    async fn count(&self, collection: &str, query: Query) -> Result<u64>;
}

#[async_trait::async_trait]
impl AsyncStorageBackend for PostgresBackend {
    async fn get(&self, user_id: &str, collection: &str, key: &str) -> Result<Option<StorageObject>> {
        self.get_async(user_id, collection, key).await
    }

    async fn set(&self, user_id: &str, collection: &str, key: &str, value: Value, expected_version: Option<u64>) -> Result<StorageObject> {
        self.set_async(user_id, collection, key, value, expected_version).await
    }

    async fn delete(&self, user_id: &str, collection: &str, key: &str) -> Result<bool> {
        self.delete_async(user_id, collection, key).await
    }

    async fn list(&self, user_id: &str, collection: &str, limit: usize, cursor: Option<&str>) -> Result<(Vec<StorageObject>, Option<String>)> {
        self.list_async(user_id, collection, limit, cursor).await
    }

    async fn query(&self, collection: &str, query: Query, limit: usize) -> Result<Vec<StorageObject>> {
        self.query_async(collection, query, limit).await
    }

    async fn count(&self, collection: &str, query: Query) -> Result<u64> {
        self.count_async(collection, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Get database URL from environment or skip test.
    /// Set POSTGRES_TEST_URL=postgres://user:pass@localhost/test_db to run these tests.
    fn get_test_db_url() -> Option<String> {
        std::env::var("POSTGRES_TEST_URL").ok()
    }

    /// Helper to create a test backend (async).
    async fn setup_test_backend() -> Option<PostgresBackend> {
        let url = get_test_db_url()?;
        let backend = PostgresBackend::new(&url).await.ok()?;
        backend.migrate().await.ok()?;
        // Clean up test data
        sqlx::query("DELETE FROM storage_objects WHERE user_id LIKE 'test_%'")
            .execute(&backend.pool)
            .await
            .ok()?;
        Some(backend)
    }

    #[tokio::test]
    async fn test_postgres_crud() {
        let Some(backend) = setup_test_backend().await else {
            eprintln!("Skipping test_postgres_crud: POSTGRES_TEST_URL not set");
            return;
        };

        let user_id = "test_user_1";
        let collection = "profiles";
        let key = "profile";
        let value = json!({"name": "Test User", "level": 5});

        // Create
        let obj = backend.set_async(user_id, collection, key, value.clone(), None)
            .await
            .expect("set should succeed");
        assert_eq!(obj.user_id, user_id);
        assert_eq!(obj.collection, collection);
        assert_eq!(obj.key, key);
        assert_eq!(obj.value, value);
        assert_eq!(obj.version, 1);

        // Read
        let read = backend.get_async(user_id, collection, key)
            .await
            .expect("get should succeed")
            .expect("object should exist");
        assert_eq!(read.value, value);
        assert_eq!(read.version, 1);

        // Update
        let new_value = json!({"name": "Test User", "level": 10});
        let updated = backend.set_async(user_id, collection, key, new_value.clone(), None)
            .await
            .expect("update should succeed");
        assert_eq!(updated.value, new_value);
        assert_eq!(updated.version, 2);

        // Delete
        let deleted = backend.delete_async(user_id, collection, key)
            .await
            .expect("delete should succeed");
        assert!(deleted);

        // Verify deleted
        let after_delete = backend.get_async(user_id, collection, key)
            .await
            .expect("get after delete should succeed");
        assert!(after_delete.is_none());
    }

    #[tokio::test]
    async fn test_postgres_version_conflict() {
        let Some(backend) = setup_test_backend().await else {
            eprintln!("Skipping test_postgres_version_conflict: POSTGRES_TEST_URL not set");
            return;
        };

        let user_id = "test_user_version";
        let collection = "items";
        let key = "sword";

        // Create initial object
        let obj = backend.set_async(user_id, collection, key, json!({"damage": 10}), None)
            .await
            .expect("initial set should succeed");
        assert_eq!(obj.version, 1);

        // Update with correct version
        let updated = backend.set_async(user_id, collection, key, json!({"damage": 15}), Some(1))
            .await
            .expect("versioned update should succeed");
        assert_eq!(updated.version, 2);

        // Update with wrong version should fail
        let result = backend.set_async(user_id, collection, key, json!({"damage": 20}), Some(1))
            .await;
        assert!(matches!(result, Err(StorageError::VersionConflict { expected: 1, actual: 2 })));

        // Cleanup
        backend.delete_async(user_id, collection, key).await.ok();
    }

    #[tokio::test]
    async fn test_postgres_list() {
        let Some(backend) = setup_test_backend().await else {
            eprintln!("Skipping test_postgres_list: POSTGRES_TEST_URL not set");
            return;
        };

        let user_id = "test_user_list";
        let collection = "inventory";

        // Create multiple objects
        for i in 0..5 {
            backend.set_async(user_id, collection, &format!("item_{}", i), json!({"index": i}), None)
                .await
                .expect("set should succeed");
        }

        // List all
        let (objects, cursor) = backend.list_async(user_id, collection, 10, None)
            .await
            .expect("list should succeed");
        assert_eq!(objects.len(), 5);
        assert!(cursor.is_none());

        // List with pagination
        let (page1, cursor1) = backend.list_async(user_id, collection, 2, None)
            .await
            .expect("list page 1 should succeed");
        assert_eq!(page1.len(), 2);
        assert!(cursor1.is_some());

        let (page2, cursor2) = backend.list_async(user_id, collection, 2, cursor1.as_deref())
            .await
            .expect("list page 2 should succeed");
        assert_eq!(page2.len(), 2);
        assert!(cursor2.is_some());

        // Cleanup
        for i in 0..5 {
            backend.delete_async(user_id, collection, &format!("item_{}", i)).await.ok();
        }
    }

    #[tokio::test]
    async fn test_postgres_query() {
        let Some(backend) = setup_test_backend().await else {
            eprintln!("Skipping test_postgres_query: POSTGRES_TEST_URL not set");
            return;
        };

        let collection = "test_players";

        // Create players with different levels
        for i in 0..5 {
            backend.set_async(
                &format!("test_player_{}", i),
                collection,
                "stats",
                json!({"level": i * 10, "name": format!("Player {}", i)}),
                None
            ).await.expect("set should succeed");
        }

        // Query players with level >= 20
        let query = Query::new().gte("level", 20);
        let results = backend.query_async(collection, query, 10)
            .await
            .expect("query should succeed");
        assert_eq!(results.len(), 3); // levels 20, 30, 40

        // Count query
        let query = Query::new().gte("level", 30);
        let count = backend.count_async(collection, query)
            .await
            .expect("count should succeed");
        assert_eq!(count, 2); // levels 30, 40

        // Cleanup
        for i in 0..5 {
            backend.delete_async(&format!("test_player_{}", i), collection, "stats").await.ok();
        }
    }

    #[tokio::test]
    async fn test_postgres_batch_operations() {
        let Some(backend) = setup_test_backend().await else {
            eprintln!("Skipping test_postgres_batch_operations: POSTGRES_TEST_URL not set");
            return;
        };

        // Batch write
        let writes: Vec<WriteOp> = (0..3).map(|i| WriteOp {
            id: ObjectId {
                user_id: format!("test_batch_user_{}", i),
                collection: "batch_test".to_string(),
                key: "data".to_string(),
            },
            value: json!({"batch_index": i}),
            permission: ObjectPermission::OwnerOnly,
            version: None,
        }).collect();

        let written = backend.write_many_async(&writes)
            .await
            .expect("batch write should succeed");
        assert_eq!(written.len(), 3);

        // Batch read
        let reads: Vec<ObjectId> = (0..3).map(|i| ObjectId {
            user_id: format!("test_batch_user_{}", i),
            collection: "batch_test".to_string(),
            key: "data".to_string(),
        }).collect();

        let read_results = backend.get_many_async(&reads)
            .await
            .expect("batch read should succeed");
        assert_eq!(read_results.len(), 3);
        assert!(read_results.iter().all(|r| r.is_some()));

        // Batch delete
        let deleted = backend.delete_many_async(&reads)
            .await
            .expect("batch delete should succeed");
        assert_eq!(deleted, 3);
    }

    #[test]
    fn test_postgres_sync_wrapper() {
        let Some(url) = get_test_db_url() else {
            eprintln!("Skipping test_postgres_sync_wrapper: POSTGRES_TEST_URL not set");
            return;
        };

        let backend = PostgresSyncBackend::connect(&url)
            .expect("connect should succeed");
        backend.migrate().expect("migrate should succeed");

        let user_id = "test_sync_user";
        let collection = "sync_test";
        let key = "data";

        // Test sync operations
        let obj = backend.set(user_id, collection, key, json!({"sync": true}), None)
            .expect("sync set should succeed");
        assert_eq!(obj.version, 1);

        let read = backend.get(user_id, collection, key)
            .expect("sync get should succeed")
            .expect("object should exist");
        assert_eq!(read.value["sync"], true);

        backend.delete(user_id, collection, key)
            .expect("sync delete should succeed");
    }
}
