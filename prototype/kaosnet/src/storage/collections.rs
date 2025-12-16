//! Collection and query types.

use serde::{Deserialize, Serialize};

/// A collection of documents.
#[derive(Debug, Clone)]
pub struct Collection {
    pub name: String,
}

/// A document in a collection (alias for StorageObject).
pub type Document = super::StorageObject;

/// Query operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryOp {
    /// Equal to value.
    Eq(serde_json::Value),
    /// Not equal to value.
    Ne(serde_json::Value),
    /// Greater than value.
    Gt(serde_json::Value),
    /// Greater than or equal to value.
    Gte(serde_json::Value),
    /// Less than value.
    Lt(serde_json::Value),
    /// Less than or equal to value.
    Lte(serde_json::Value),
    /// Value is in list.
    In(Vec<serde_json::Value>),
    /// Value is not in list.
    NotIn(Vec<serde_json::Value>),
    /// String contains substring.
    Contains(String),
    /// String starts with prefix.
    StartsWith(String),
    /// String ends with suffix.
    EndsWith(String),
    /// Field exists.
    Exists,
    /// Field does not exist.
    NotExists,
}

/// A query filter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Query {
    /// Field conditions (AND).
    pub conditions: Vec<(String, QueryOp)>,
    /// Sort field and direction.
    pub sort: Option<(String, SortDir)>,
    /// Offset for pagination.
    pub offset: usize,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDir {
    Asc,
    Desc,
}

impl Query {
    /// Create empty query (matches all).
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an equality condition.
    pub fn eq(mut self, field: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.conditions.push((field.into(), QueryOp::Eq(value.into())));
        self
    }

    /// Add a not-equal condition.
    pub fn ne(mut self, field: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.conditions.push((field.into(), QueryOp::Ne(value.into())));
        self
    }

    /// Add a greater-than condition.
    pub fn gt(mut self, field: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.conditions.push((field.into(), QueryOp::Gt(value.into())));
        self
    }

    /// Add a greater-than-or-equal condition.
    pub fn gte(mut self, field: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.conditions.push((field.into(), QueryOp::Gte(value.into())));
        self
    }

    /// Add a less-than condition.
    pub fn lt(mut self, field: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.conditions.push((field.into(), QueryOp::Lt(value.into())));
        self
    }

    /// Add a less-than-or-equal condition.
    pub fn lte(mut self, field: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.conditions.push((field.into(), QueryOp::Lte(value.into())));
        self
    }

    /// Add an "in list" condition.
    pub fn in_list(mut self, field: impl Into<String>, values: Vec<serde_json::Value>) -> Self {
        self.conditions.push((field.into(), QueryOp::In(values)));
        self
    }

    /// Add a contains condition.
    pub fn contains(mut self, field: impl Into<String>, substring: impl Into<String>) -> Self {
        self.conditions.push((field.into(), QueryOp::Contains(substring.into())));
        self
    }

    /// Add sort.
    pub fn sort(mut self, field: impl Into<String>, dir: SortDir) -> Self {
        self.sort = Some((field.into(), dir));
        self
    }

    /// Sort ascending.
    pub fn sort_asc(self, field: impl Into<String>) -> Self {
        self.sort(field, SortDir::Asc)
    }

    /// Sort descending.
    pub fn sort_desc(self, field: impl Into<String>) -> Self {
        self.sort(field, SortDir::Desc)
    }

    /// Set offset for pagination.
    pub fn skip(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Check if a value matches this query.
    pub fn matches(&self, value: &serde_json::Value) -> bool {
        for (field, op) in &self.conditions {
            let field_value = value.get(field);

            let matches = match op {
                QueryOp::Eq(expected) => field_value == Some(expected),
                QueryOp::Ne(expected) => field_value != Some(expected),
                QueryOp::Gt(expected) => compare_values(field_value, Some(expected)) == Some(std::cmp::Ordering::Greater),
                QueryOp::Gte(expected) => matches!(compare_values(field_value, Some(expected)), Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)),
                QueryOp::Lt(expected) => compare_values(field_value, Some(expected)) == Some(std::cmp::Ordering::Less),
                QueryOp::Lte(expected) => matches!(compare_values(field_value, Some(expected)), Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)),
                QueryOp::In(values) => field_value.map(|v| values.contains(v)).unwrap_or(false),
                QueryOp::NotIn(values) => field_value.map(|v| !values.contains(v)).unwrap_or(true),
                QueryOp::Contains(substring) => {
                    field_value
                        .and_then(|v| v.as_str())
                        .map(|s| s.contains(substring.as_str()))
                        .unwrap_or(false)
                }
                QueryOp::StartsWith(prefix) => {
                    field_value
                        .and_then(|v| v.as_str())
                        .map(|s| s.starts_with(prefix.as_str()))
                        .unwrap_or(false)
                }
                QueryOp::EndsWith(suffix) => {
                    field_value
                        .and_then(|v| v.as_str())
                        .map(|s| s.ends_with(suffix.as_str()))
                        .unwrap_or(false)
                }
                QueryOp::Exists => field_value.is_some(),
                QueryOp::NotExists => field_value.is_none(),
            };

            if !matches {
                return false;
            }
        }
        true
    }
}

/// Compare two JSON values for ordering.
fn compare_values(a: Option<&serde_json::Value>, b: Option<&serde_json::Value>) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Some(serde_json::Value::Number(a)), Some(serde_json::Value::Number(b))) => {
            a.as_f64().partial_cmp(&b.as_f64())
        }
        (Some(serde_json::Value::String(a)), Some(serde_json::Value::String(b))) => {
            Some(a.cmp(b))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_eq() {
        let query = Query::new().eq("name", "test");
        assert!(query.matches(&serde_json::json!({"name": "test"})));
        assert!(!query.matches(&serde_json::json!({"name": "other"})));
    }

    #[test]
    fn test_query_gt() {
        let query = Query::new().gt("score", 100);
        assert!(query.matches(&serde_json::json!({"score": 150})));
        assert!(!query.matches(&serde_json::json!({"score": 50})));
        assert!(!query.matches(&serde_json::json!({"score": 100})));
    }

    #[test]
    fn test_query_contains() {
        let query = Query::new().contains("name", "test");
        assert!(query.matches(&serde_json::json!({"name": "testing123"})));
        assert!(!query.matches(&serde_json::json!({"name": "hello"})));
    }

    #[test]
    fn test_query_multiple_conditions() {
        let query = Query::new()
            .eq("status", "active")
            .gte("level", 5);

        assert!(query.matches(&serde_json::json!({"status": "active", "level": 10})));
        assert!(!query.matches(&serde_json::json!({"status": "active", "level": 3})));
        assert!(!query.matches(&serde_json::json!({"status": "inactive", "level": 10})));
    }
}
