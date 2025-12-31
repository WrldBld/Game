//! Neo4j deserialization helpers for row conversion functions.
//!
//! This module provides extension traits and helper functions to reduce
//! boilerplate when converting Neo4j nodes to domain entities.
//!
//! # Usage
//!
//! ```ignore
//! use super::neo4j_helpers::{NodeExt, parse_typed_id};
//!
//! fn row_to_entity(row: &Row) -> Result<Entity> {
//!     let node: Node = row.get("e")?;
//!     Ok(Entity {
//!         id: parse_typed_id(&node, "id")?,
//!         name: node.get("name")?,
//!         description: node.get_optional_string("description"),
//!         created_at: node.get_datetime_or("created_at", fallback),
//!     })
//! }
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use neo4rs::{Node, Row};
use uuid::Uuid;
use wrldbldr_common::datetime::parse_datetime_or;
use wrldbldr_common::StringExt;

/// Extension trait for Neo4j Node to simplify common deserialization patterns.
///
/// These methods reduce the boilerplate of:
/// 1. Extracting fields from nodes
/// 2. Converting empty strings to None
/// 3. Parsing UUIDs and timestamps
/// 4. Deserializing JSON fields
pub trait NodeExt {
    /// Get a required UUID field and parse it.
    ///
    /// Returns an error if the field is missing or not a valid UUID.
    fn get_uuid(&self, field: &str) -> Result<Uuid>;

    /// Get an optional string field, returning None if empty or missing.
    ///
    /// This replaces the common pattern:
    /// ```ignore
    /// let val: String = node.get("field").unwrap_or_default();
    /// let opt = if val.is_empty() { None } else { Some(val) };
    /// ```
    fn get_optional_string(&self, field: &str) -> Option<String>;

    /// Get a string field with a default value if missing.
    fn get_string_or(&self, field: &str, default: &str) -> String;

    /// Get a timestamp field, falling back to provided time on parse error.
    ///
    /// Uses `wrldbldr_common::datetime::parse_datetime_or` for parsing.
    fn get_datetime_or(&self, field: &str, fallback: DateTime<Utc>) -> DateTime<Utc>;

    /// Get and deserialize a JSON field.
    ///
    /// Returns an error if the field is missing or JSON is invalid.
    fn get_json<T: serde::de::DeserializeOwned>(&self, field: &str) -> Result<T>;

    /// Get and deserialize a JSON field with default on error.
    ///
    /// Returns `T::default()` if the field is missing, empty, or contains invalid JSON.
    fn get_json_or_default<T: serde::de::DeserializeOwned + Default>(&self, field: &str) -> T;

    /// Get an optional i64 field, returning None if negative or missing.
    ///
    /// Useful for optional count/limit fields stored as -1 for "none".
    fn get_positive_i64(&self, field: &str) -> Option<u32>;

    /// Get a bool field with a default value if missing.
    fn get_bool_or(&self, field: &str, default: bool) -> bool;

    /// Get an i64 field with a default value if missing.
    fn get_i64_or(&self, field: &str, default: i64) -> i64;

    /// Get an f64 field with a default value if missing.
    fn get_f64_or(&self, field: &str, default: f64) -> f64;
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
}

/// Extension trait for Neo4j Row to simplify common deserialization patterns.
///
/// These methods work with column projections in Cypher queries (e.g., `r.id as id`).
/// Use this when your query projects individual properties as columns rather than
/// returning entire nodes.
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
}

/// Parse a typed ID from a Neo4j node field.
///
/// This is a convenience function that combines UUID parsing with typed ID construction.
///
/// # Example
///
/// ```ignore
/// let character_id: CharacterId = parse_typed_id(&node, "id")?;
/// let world_id: WorldId = parse_typed_id(&node, "world_id")?;
/// ```
pub fn parse_typed_id<T>(node: &Node, field: &str) -> Result<T>
where
    T: From<Uuid>,
{
    let uuid = node.get_uuid(field)?;
    Ok(T::from(uuid))
}

/// Parse an optional typed ID from a Neo4j node field.
///
/// Returns None if the field is missing or empty.
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
///
/// This is similar to `parse_typed_id` but works with row column projections
/// instead of node fields. Use this when your Cypher query projects properties
/// as columns (e.g., `RETURN r.id as id`).
///
/// # Example
///
/// ```ignore
/// let character_id: CharacterId = parse_typed_id_from_row(&row, "id")?;
/// let world_id: WorldId = parse_typed_id_from_row(&row, "world_id")?;
/// ```
pub fn parse_typed_id_from_row<T>(row: &Row, column: &str) -> Result<T>
where
    T: From<Uuid>,
{
    let uuid = row.get_uuid(column)?;
    Ok(T::from(uuid))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Testing NodeExt requires a mock Node implementation.
    // These tests verify the typed ID parsing utilities.

    #[test]
    fn test_parse_typed_id_concept() {
        // This test verifies the type constraint works.
        // Actual Node tests would require neo4rs test infrastructure.
        let uuid = Uuid::new_v4();

        // Verify the From trait bound works with domain IDs
        fn accepts_from_uuid<T: From<Uuid>>(uuid: Uuid) -> T {
            T::from(uuid)
        }

        let _: wrldbldr_domain::WorldId = accepts_from_uuid(uuid);
        let _: wrldbldr_domain::CharacterId = accepts_from_uuid(uuid);
    }
}
