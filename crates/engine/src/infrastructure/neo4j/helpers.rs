//! Neo4j deserialization helpers for row conversion functions.
//!
//! This module provides extension traits and helper functions to reduce
//! boilerplate when converting Neo4j nodes to domain entities.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use neo4rs::{Node, Query, Row};
use uuid::Uuid;
use wrldbldr_domain::common::{parse_datetime_or, StringExt};

use crate::infrastructure::neo4j::Neo4jGraph;
use crate::infrastructure::ports::RepoError;

// =============================================================================
// Graph Extension Trait (Error Mapping)
// =============================================================================

/// Extension trait for Neo4j Graph with standardized error mapping to RepoError.
///
/// This trait reduces the common pattern of:
/// ```ignore
/// graph.run(query).await.map_err(|e| RepoError::database("query", e))?;
/// ```
///
/// Usage (can be adopted incrementally):
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::GraphExt;
///
/// graph.run_or_err(query).await?;
/// ```
///
/// Note: `execute` is not wrapped because neo4rs 0.8 doesn't export
/// `DetachedRowStream` publicly. Use the standard pattern for queries:
/// ```ignore
/// let mut result = graph.execute(q).await.map_err(|e| RepoError::database("query", e))?;
/// ```
#[allow(dead_code)]
#[async_trait::async_trait]
pub trait GraphExt {
    /// Execute a query that doesn't return results (INSERT, UPDATE, DELETE).
    ///
    /// Maps neo4rs errors to `RepoError::Database`.
    async fn run_or_err(&self, query: Query) -> Result<(), RepoError>;
}

#[async_trait::async_trait]
impl GraphExt for Neo4jGraph {
    async fn run_or_err(&self, query: Query) -> Result<(), RepoError> {
        self.run(query)
            .await
            .map_err(|e| RepoError::database("query", e))
    }
}

/// Extension trait for Neo4j Node to simplify common deserialization patterns.
pub trait NodeExt {
    /// Get a required UUID field and parse it.
    fn get_uuid(&self, field: &str) -> Result<Uuid>;

    /// Get an optional string field, returning None if empty or missing.
    fn get_optional_string(&self, field: &str) -> Option<String>;

    /// Get a string field with a default value if missing.
    fn get_string_or(&self, field: &str, default: &str) -> String;

    /// Get a timestamp field, falling back to provided time on parse error.
    fn get_datetime_or(&self, field: &str, fallback: DateTime<Utc>) -> DateTime<Utc>;

    /// Get and deserialize a JSON field.
    fn get_json<T: serde::de::DeserializeOwned>(&self, field: &str) -> Result<T>;

    /// Get and deserialize a JSON field with default on error.
    fn get_json_or_default<T: serde::de::DeserializeOwned + Default>(&self, field: &str) -> T;

    /// Get an optional i64 field, returning None if negative or missing.
    fn get_positive_i64(&self, field: &str) -> Option<u32>;

    /// Get a bool field with a default value if missing.
    fn get_bool_or(&self, field: &str, default: bool) -> bool;

    /// Get an i64 field with a default value if missing.
    fn get_i64_or(&self, field: &str, default: i64) -> i64;

    /// Get an f64 field with a default value if missing.
    fn get_f64_or(&self, field: &str, default: f64) -> f64;

    /// Get a required JSON field with strict error handling (fail-fast).
    fn get_json_strict<T: serde::de::DeserializeOwned>(&self, field: &str) -> Result<T, RepoError>;

    /// Get a required string field with strict error handling (fail-fast).
    fn get_string_strict(&self, field: &str) -> Result<String, RepoError>;

    /// Get a required datetime field with strict error handling (fail-fast).
    /// Currently unused but kept for symmetry with other strict methods.
    #[allow(dead_code)]
    fn get_datetime_strict(&self, field: &str) -> Result<DateTime<Utc>, RepoError>;
}

impl NodeExt for Node {
    fn get_uuid(&self, field: &str) -> Result<Uuid> {
        let s: String = self
            .get(field)
            .with_context(|| format!("Missing field: {}", field))?;
        Uuid::parse_str(&s).with_context(|| format!("Invalid UUID in field '{}': {}", field, s))
    }

    fn get_optional_string(&self, field: &str) -> Option<String> {
        self.get::<String>(field).ok().and_then(|s| s.into_option())
    }

    fn get_string_or(&self, field: &str, default: &str) -> String {
        self.get(field).unwrap_or_else(|_| default.to_string())
    }

    fn get_datetime_or(&self, field: &str, fallback: DateTime<Utc>) -> DateTime<Utc> {
        self.get::<String>(field)
            .ok()
            .map(|s| parse_datetime_or(&s, fallback))
            .unwrap_or(fallback)
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, field: &str) -> Result<T> {
        let s: String = self
            .get(field)
            .with_context(|| format!("Missing field: {}", field))?;
        serde_json::from_str(&s)
            .with_context(|| format!("Invalid JSON in field '{}': {}", field, s))
    }

    fn get_json_or_default<T: serde::de::DeserializeOwned + Default>(&self, field: &str) -> T {
        self.get::<String>(field)
            .ok()
            .filter(|s| !s.is_empty())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn get_positive_i64(&self, field: &str) -> Option<u32> {
        self.get::<i64>(field)
            .ok()
            .filter(|&n| n >= 0)
            .map(|n| n as u32)
    }

    fn get_bool_or(&self, field: &str, default: bool) -> bool {
        self.get(field).unwrap_or(default)
    }

    fn get_i64_or(&self, field: &str, default: i64) -> i64 {
        self.get(field).unwrap_or(default)
    }

    fn get_f64_or(&self, field: &str, default: f64) -> f64 {
        self.get(field).unwrap_or(default)
    }

    fn get_json_strict<T: serde::de::DeserializeOwned>(&self, field: &str) -> Result<T, RepoError> {
        let s: String = self.get(field).map_err(|e| {
            RepoError::database(
                "query",
                format!("Missing required field '{}': {}", field, e),
            )
        })?;
        serde_json::from_str(&s).map_err(|e| {
            RepoError::database(
                "parse",
                format!("Invalid JSON in field '{}': {} (value: '{}')", field, e, s),
            )
        })
    }

    fn get_string_strict(&self, field: &str) -> Result<String, RepoError> {
        self.get(field).map_err(|e| {
            RepoError::database(
                "query",
                format!("Missing required field '{}': {}", field, e),
            )
        })
    }

    fn get_datetime_strict(&self, field: &str) -> Result<DateTime<Utc>, RepoError> {
        let s: String = self.get(field).map_err(|e| {
            RepoError::database(
                "query",
                format!("Missing required field '{}': {}", field, e),
            )
        })?;
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Invalid datetime in field '{}': {} (value: '{}')",
                        field, e, s
                    ),
                )
            })
    }
}

/// Extension trait for Neo4j Row to simplify common deserialization patterns.
#[allow(dead_code)]
pub trait RowExt {
    /// Get a required UUID column and parse it.
    fn get_uuid(&self, column: &str) -> Result<Uuid>;

    /// Get an optional string column, returning None if empty or missing.
    fn get_optional_string(&self, column: &str) -> Option<String>;

    /// Get a string column with a default value if missing.
    fn get_string_or(&self, column: &str, default: &str) -> String;

    /// Get a timestamp column, falling back to provided time on parse error.
    fn get_datetime_or(&self, column: &str, fallback: DateTime<Utc>) -> DateTime<Utc>;

    /// Get and deserialize a JSON column.
    fn get_json<T: serde::de::DeserializeOwned>(&self, column: &str) -> Result<T>;

    /// Get and deserialize a JSON column with default on error.
    fn get_json_or_default<T: serde::de::DeserializeOwned + Default>(&self, column: &str) -> T;

    /// Get a bool column with a default value if missing.
    fn get_bool_or(&self, column: &str, default: bool) -> bool;

    /// Get an f64 column with a default value if missing.
    fn get_f64_or(&self, column: &str, default: f64) -> f64;

    /// Get a required JSON column with strict error handling (fail-fast).
    fn get_json_strict<T: serde::de::DeserializeOwned>(&self, column: &str)
        -> Result<T, RepoError>;

    /// Get a required string column with strict error handling (fail-fast).
    fn get_string_strict(&self, column: &str) -> Result<String, RepoError>;
}

impl RowExt for Row {
    fn get_uuid(&self, column: &str) -> Result<Uuid> {
        let s: String = self
            .get(column)
            .with_context(|| format!("Missing column: {}", column))?;
        Uuid::parse_str(&s).with_context(|| format!("Invalid UUID in column '{}': {}", column, s))
    }

    fn get_optional_string(&self, column: &str) -> Option<String> {
        self.get::<String>(column)
            .ok()
            .and_then(|s| s.into_option())
    }

    fn get_string_or(&self, column: &str, default: &str) -> String {
        self.get(column).unwrap_or_else(|_| default.to_string())
    }

    fn get_datetime_or(&self, column: &str, fallback: DateTime<Utc>) -> DateTime<Utc> {
        self.get::<String>(column)
            .ok()
            .map(|s| parse_datetime_or(&s, fallback))
            .unwrap_or(fallback)
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, column: &str) -> Result<T> {
        let s: String = self
            .get(column)
            .with_context(|| format!("Missing column: {}", column))?;
        serde_json::from_str(&s)
            .with_context(|| format!("Invalid JSON in column '{}': {}", column, s))
    }

    fn get_json_or_default<T: serde::de::DeserializeOwned + Default>(&self, column: &str) -> T {
        self.get::<String>(column)
            .ok()
            .filter(|s| !s.is_empty())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn get_bool_or(&self, column: &str, default: bool) -> bool {
        self.get(column).unwrap_or(default)
    }

    fn get_f64_or(&self, column: &str, default: f64) -> f64 {
        self.get(column).unwrap_or(default)
    }

    fn get_json_strict<T: serde::de::DeserializeOwned>(
        &self,
        column: &str,
    ) -> Result<T, RepoError> {
        let s: String = self.get(column).map_err(|e| {
            RepoError::database(
                "query",
                format!("Missing required column '{}': {}", column, e),
            )
        })?;
        serde_json::from_str(&s).map_err(|e| {
            RepoError::database(
                "parse",
                format!(
                    "Invalid JSON in column '{}': {} (value: '{}')",
                    column, e, s
                ),
            )
        })
    }

    fn get_string_strict(&self, column: &str) -> Result<String, RepoError> {
        self.get(column).map_err(|e| {
            RepoError::database(
                "query",
                format!("Missing required column '{}': {}", column, e),
            )
        })
    }
}

/// Parse a typed ID from a Neo4j node field.
pub fn parse_typed_id<T>(node: &Node, field: &str) -> Result<T>
where
    T: From<Uuid>,
{
    let uuid = node.get_uuid(field)?;
    Ok(T::from(uuid))
}

/// Parse an optional typed ID from a Neo4j node field.
pub fn parse_optional_typed_id<T>(node: &Node, field: &str) -> Result<Option<T>>
where
    T: From<Uuid>,
{
    let s: String = match node.get(field) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    if s.is_empty() {
        return Ok(None);
    }

    let uuid = Uuid::parse_str(&s)
        .with_context(|| format!("Invalid UUID in optional field '{}': {}", field, s))?;
    Ok(Some(T::from(uuid)))
}

/// Parse a typed ID from a Neo4j row column.
pub fn parse_typed_id_from_row<T>(row: &Row, column: &str) -> Result<T>
where
    T: From<Uuid>,
{
    let uuid = row.get_uuid(column)?;
    Ok(T::from(uuid))
}

// =============================================================================
// Common Row-to-Entity Converters
// =============================================================================

use wrldbldr_domain::{Item, ItemId, ItemName, WorldId};

/// Convert a Neo4j row containing an Item node (aliased as 'i') to an Item entity.
///
/// This helper reduces duplication across character_repo, player_character_repo, and item_repo.
pub fn row_to_item(row: Row) -> Result<Item, RepoError> {
    let node: Node = row.get("i").map_err(|e| RepoError::database("query", e))?;

    let id: ItemId = parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
    let world_id: WorldId =
        parse_typed_id(&node, "world_id").map_err(|e| RepoError::database("query", e))?;
    let name_str: String = node
        .get("name")
        .map_err(|e| RepoError::database("query", e))?;
    let name = ItemName::new(name_str).map_err(|e| RepoError::database("parse", e))?;
    let description = node.get_optional_string("description");
    let item_type = node.get_optional_string("item_type");
    let is_unique = node.get_bool_or("is_unique", false);
    let properties = node.get_optional_string("properties");
    let can_contain_items = node.get_bool_or("can_contain_items", false);

    let container_limit_raw = node.get_i64_or("container_limit", -1);
    let container_limit = if container_limit_raw < 0 {
        None
    } else {
        Some(container_limit_raw as u32)
    };

    let item = Item {
        id,
        world_id,
        name,
        description,
        item_type,
        is_unique,
        properties,
        can_contain_items,
        container_limit,
    };
    Ok(item)
}

// =============================================================================
// Description Parsing Helpers
// =============================================================================

use wrldbldr_domain::value_objects::Description;

/// Convert an optional string to an optional Description.
///
/// Since descriptions are loaded from the database (already validated on save),
/// we use `unwrap_or_default()` for any validation failures. This should never
/// happen in practice, but provides a safe fallback.
pub fn parse_optional_description(s: Option<String>) -> Option<Description> {
    s.map(|text| Description::new(text).unwrap_or_default())
}

/// Convert an optional string to a Description, returning default if None.
pub fn parse_description_or_default(s: Option<String>) -> Description {
    s.map(|text| Description::new(text).unwrap_or_default())
        .unwrap_or_default()
}
