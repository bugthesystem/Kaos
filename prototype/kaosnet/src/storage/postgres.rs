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
}

/// Sync wrapper for PostgresBackend that implements StorageBackend.
///
/// Uses a tokio runtime internally to bridge async to sync.
pub struct PostgresSyncBackend {
    inner: Arc<PostgresBackend>,
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
}

impl StorageBackend for PostgresSyncBackend {
    fn get(&self, user_id: &str, collection: &str, key: &str) -> Result<Option<StorageObject>> {
        self.runtime.block_on(self.inner.get_async(user_id, collection, key))
    }

    fn set(
        &self,
        user_id: &str,
        collection: &str,
        key: &str,
        value: Value,
        expected_version: Option<u64>,
    ) -> Result<StorageObject> {
        self.runtime.block_on(self.inner.set_async(user_id, collection, key, value, expected_version))
    }

    fn delete(&self, user_id: &str, collection: &str, key: &str) -> Result<bool> {
        self.runtime.block_on(self.inner.delete_async(user_id, collection, key))
    }

    fn list(
        &self,
        user_id: &str,
        collection: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<(Vec<StorageObject>, Option<String>)> {
        self.runtime.block_on(self.inner.list_async(user_id, collection, limit, cursor))
    }

    fn get_many(&self, reads: &[ObjectId]) -> Result<Vec<Option<StorageObject>>> {
        self.runtime.block_on(self.inner.get_many_async(reads))
    }

    fn write_many(&self, writes: &[WriteOp]) -> Result<Vec<StorageObject>> {
        self.runtime.block_on(self.inner.write_many_async(writes))
    }

    fn delete_many(&self, deletes: &[ObjectId]) -> Result<usize> {
        self.runtime.block_on(self.inner.delete_many_async(deletes))
    }

    fn query(&self, collection: &str, query: Query, limit: usize) -> Result<Vec<StorageObject>> {
        self.runtime.block_on(self.inner.query_async(collection, query, limit))
    }

    fn count(&self, collection: &str, query: Query) -> Result<u64> {
        self.runtime.block_on(self.inner.count_async(collection, query))
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
