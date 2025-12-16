//! PostgreSQL storage backend implementation.
//!
//! Provides persistent storage using PostgreSQL with connection pooling.

use super::{ObjectId, ObjectPermission, Query, QueryOp, Result, StorageError, StorageObject, WriteOp};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::{SystemTime, UNIX_EPOCH};

/// PostgreSQL storage backend.
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

    /// Create with custom pool options.
    pub async fn with_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run migrations to set up the schema.
    pub async fn migrate(&self) -> std::result::Result<(), sqlx::Error> {
        sqlx::query(include_str!("postgres_schema.sql"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    fn now_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    fn permission_to_int(p: &ObjectPermission) -> i32 {
        match p {
            ObjectPermission::OwnerOnly => 0,
            ObjectPermission::PublicRead => 1,
            ObjectPermission::PublicReadWrite => 2,
        }
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
        .map_err(|e| StorageError::Internal(e.to_string()))?;

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
        let now = Self::now_millis();

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
            VALUES ($1, $2, $3, $4, 1, 1, NOW(), NOW())
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
        .map_err(|e| StorageError::Internal(e.to_string()))?;

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
        .map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_async(
        &self,
        user_id: &str,
        collection: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<(Vec<StorageObject>, Option<String>)> {
        let query = match cursor {
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
            }
        };

        let rows = query.fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Internal(e.to_string()))?;

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
        // Build WHERE clause from query
        let (where_clause, params) = self.build_query_where(&query);

        let sql = format!(
            r#"
            SELECT user_id, collection, key, value, version, permission,
                   EXTRACT(EPOCH FROM created_at)::bigint * 1000 as created_at,
                   EXTRACT(EPOCH FROM updated_at)::bigint * 1000 as updated_at
            FROM storage_objects
            WHERE collection = $1 {}
            LIMIT $2
            "#,
            if where_clause.is_empty() { String::new() } else { format!("AND {}", where_clause) }
        );

        // For now, use a simpler approach - fetch all and filter in Rust
        // This is not optimal but works for the initial implementation
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
        .bind((limit * 10) as i64) // Fetch more to account for filtering
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))?;

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
        // Simple implementation - can be optimized with proper SQL query building
        let rows = sqlx::query(
            r#"
            SELECT value FROM storage_objects WHERE collection = $1
            "#
        )
        .bind(collection)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))?;

        let count = rows.iter()
            .filter(|r| {
                let value: Value = r.get("value");
                query.matches(&value)
            })
            .count();

        Ok(count as u64)
    }

    fn build_query_where(&self, _query: &Query) -> (String, Vec<Value>) {
        // TODO: Build proper JSONB query clauses
        // For now, return empty to use Rust-side filtering
        (String::new(), vec![])
    }
}

/// Async storage backend trait for PostgreSQL and other async backends.
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
