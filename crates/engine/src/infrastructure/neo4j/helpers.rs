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

    let id: ItemId = parse_typed_id(&node, "id")
        .map_err(|e| RepoError::database("query", format!("Failed to parse ItemId: {}", e)))?;
    let world_id: WorldId = parse_typed_id(&node, "world_id").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to parse world_id for Item {}: {}", id, e),
        )
    })?;
    let name_str: String = node.get("name").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'name' for Item {}: {}", id, e),
        )
    })?;
    let name = ItemName::new(name_str.clone()).map_err(|e| {
        RepoError::database(
            "parse",
            format!("Failed to parse name '{}' for Item {}: {}", name_str, id, e),
        )
    })?;
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

// =============================================================================
// Common Neo4j Query Fragments
// =============================================================================

/// Fragment for creating a parameter binding for a character ID.
///
/// Returns the `.param("id", ...)` call as a string helper.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::param_character_id;
///
/// let id = param_character_id(character_id);
/// ```
///
/// Note: This is a documentation helper only - the actual param call
/// uses neo4rs QueryBuilder, not this function.
#[allow(dead_code)]
pub fn param_character_id(id: impl std::fmt::Display) -> String {
    format!("{}", id)
}

/// Fragment for character fields including optional relationship mood in a COLLECT map.
///
/// Returns a comma-separated list of character field mappings for use in
/// COLLECT({ ... }) clauses, with an optional mood field from a relationship
/// (e.g., `rel.mood` with fallback to `c.default_mood`).
///
/// # Fields
/// - `character_id: c.id`
/// - `name: c.name`
/// - `sprite_asset: c.sprite_asset`
/// - `portrait_asset: c.portrait_asset`
/// - `mood: COALESCE(rel.mood, c.default_mood, 'calm')`
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::collect_character_fields_with_rel_mood;
///
/// let q = query(&format!(
///     "MATCH (s:Staging)-[rel:INCLUDES_NPC]->(c:Character)
///     WITH s, COLLECT({{ {} }}) as npcs
///     RETURN s, npcs",
///     collect_character_fields_with_rel_mood()
/// ))
/// .param("staging_id", staging_id.to_string());
/// ```
pub const fn collect_character_fields_with_rel_mood() -> &'static str {
    "character_id: c.id, name: c.name, sprite_asset: c.sprite_asset, portrait_asset: c.portrait_asset, mood: COALESCE(rel.mood, c.default_mood, 'calm')"
}

/// Fragment for returning character fields with relationship properties (for staging queries).
///
/// Returns a comma-separated list of character and relationship fields for RETURN clauses.
/// Designed for use in staging queries that return StagedNpc data.
///
/// # Fields
/// - Character fields: `c.id as character_id`, `c.name as name`, `c.sprite_asset as sprite_asset`, `c.portrait_asset as portrait_asset`
/// - Relationship fields: `rel.is_present as is_present`, `COALESCE(rel.is_hidden_from_players, false) as is_hidden_from_players`, `rel.reasoning as reasoning`, `COALESCE(rel.mood, c.default_mood, 'calm') as mood`, `COALESCE(rel.has_incomplete_data, false) as has_incomplete_data`
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::return_staged_character_fields;
///
/// let q = query(&format!(
///     "MATCH (r:Region {{id: $region_id}})-[:CURRENT_STAGING]->(s:Staging)-[rel:INCLUDES_NPC]->(c:Character)
///     WHERE s.is_active = true AND rel.is_present = true
///     {}",
///     return_staged_character_fields()
/// ))
/// .param("region_id", region_id.to_string());
/// ```
pub const fn return_staged_character_fields() -> &'static str {
    "RETURN c.id as character_id, c.name as name, c.sprite_asset as sprite_asset, c.portrait_asset as portrait_asset, rel.is_present as is_present, COALESCE(rel.is_hidden_from_players, false) as is_hidden_from_players, rel.reasoning as reasoning, COALESCE(rel.mood, c.default_mood, 'calm') as mood, COALESCE(rel.has_incomplete_data, false) as has_incomplete_data"
}

/// Fragment for collecting character fields with relationship properties (for staging COLLECT queries).
///
/// Returns a comma-separated list of character and relationship field mappings for use in
/// COLLECT({ ... }) clauses in staging queries.
///
/// # Fields
/// - Character fields: `character_id: c.id`, `name: c.name`, `sprite_asset: c.sprite_asset`, `portrait_asset: c.portrait_asset`
/// - Relationship fields: `is_present: rel.is_present`, `is_hidden_from_players: COALESCE(rel.is_hidden_from_players, false)`, `reasoning: rel.reasoning`, `mood: COALESCE(rel.mood, c.default_mood, 'calm')`, `has_incomplete_data: COALESCE(rel.has_incomplete_data, false)`
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::collect_staged_character_fields;
///
/// let q = query(&format!(
///     "MATCH (s:Staging)-[rel:INCLUDES_NPC]->(c:Character)
///     WITH s, COLLECT({{ {} }}) as npcs
///     RETURN s, npcs",
///     collect_staged_character_fields()
/// ))
/// .param("staging_id", staging_id.to_string());
/// ```
pub const fn collect_staged_character_fields() -> &'static str {
    "character_id: c.id, name: c.name, sprite_asset: c.sprite_asset, portrait_asset: c.portrait_asset, is_present: rel.is_present, is_hidden_from_players: COALESCE(rel.is_hidden_from_players, false), reasoning: rel.reasoning, mood: COALESCE(rel.mood, c.default_mood, 'calm'), has_incomplete_data: COALESCE(rel.has_incomplete_data, false)"
}

// =============================================================================
// Common Character MATCH Fragments
// =============================================================================

/// Fragment for matching a Character node by ID (using parameter `$id`).
///
/// Returns the MATCH clause `MATCH (c:Character {id: $id})`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_character_by_id;
///
/// let q = query(&format!("{} RETURN c", match_character_by_id()))
///     .param("id", character_id.to_string());
/// ```
pub const fn match_character_by_id() -> &'static str {
    "MATCH (c:Character {id: $id})"
}

/// Fragment for matching a Character node by ID with custom parameter name.
///
/// Returns MATCH clause with specified parameter name.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_character_by_param;
///
/// let q = query(&format!("{} RETURN c", match_character_by_param("character_id")))
///     .param("character_id", character_id.to_string());
/// ```
pub fn match_character_by_param(param_name: &str) -> String {
    format!("MATCH (c:Character {{id: ${}}})", param_name)
}
 
// =============================================================================
// Common World MATCH Fragments
// =============================================================================
 
/// Fragment for matching a World node by ID (using parameter `$id`).
///
/// Returns the MATCH clause `MATCH (w:World {id: $id})`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_world_by_id;
///
/// let q = query(&format!("{} RETURN w", match_world_by_id()))
///     .param("id", world_id.to_string());
/// ```
pub const fn match_world_by_id() -> &'static str {
    "MATCH (w:World {id: $id})"
}
 
// =============================================================================
// Common Item MATCH Fragments
// =============================================================================
 
/// Fragment for matching an Item node by ID (using parameter `$id`).
///
/// Returns the MATCH clause `MATCH (i:Item {id: $id})`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_item_by_id;
///
/// let q = query(&format!("{} RETURN i", match_item_by_id()))
///     .param("id", item_id.to_string());
/// ```
pub const fn match_item_by_id() -> &'static str {
    "MATCH (i:Item {id: $id})"
}
 
// =============================================================================
// Common PlayerCharacter MATCH Fragments
// =============================================================================
  
/// Fragment for matching a PlayerCharacter node by ID (using parameter `$id`).
///
/// Returns the MATCH clause `MATCH (pc:PlayerCharacter {id: $id})`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_player_character_by_id;
///
/// let q = query(&format!("{} RETURN pc", match_player_character_by_id()))
///     .param("id", pc_id.to_string());
/// ```
pub const fn match_player_character_by_id() -> &'static str {
    "MATCH (pc:PlayerCharacter {id: $id})"
}
 
// =============================================================================
// Common Lore MATCH Fragments
// =============================================================================


// =============================================================================
// Common Lore MATCH Fragments
// =============================================================================

/// Fragment for matching a Lore node by ID (using parameter `$id`).
///
/// Returns the MATCH clause `MATCH (l:Lore {id: $id})`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_lore_by_id;
///
/// let q = query(&format!("{} RETURN l", match_lore_by_id()))
///     .param("id", lore_id.to_string());
/// ```
pub const fn match_lore_by_id() -> &'static str {
    "MATCH (l:Lore {id: $id})"
}

/// Fragment for matching a Lore node by world ID.
///
/// Returns the MATCH clause `MATCH (l:Lore {world_id: $world_id})`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_lore_by_world;
///
/// let q = query(&format!("{} RETURN l ORDER BY l.title", match_lore_by_world()))
///     .param("world_id", world_id.to_string());
/// ```
pub const fn match_lore_by_world() -> &'static str {
    "MATCH (l:Lore {world_id: $world_id})"
}

// =============================================================================
// Common Lore Knowledge RETURN Fragments
// =============================================================================

/// Fragment for returning lore knowledge relationship fields.
///
/// Returns a comma-separated list of knowledge relationship fields for RETURN clauses.
///
/// # Fields
/// - `l.id as lore_id`
/// - `c.id as character_id`
/// - `k.known_chunk_ids as known_chunk_ids`
/// - `k.discovery_source as discovery_source`
/// - `k.discovered_at as discovered_at`
/// - `k.notes as notes`
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::return_knowledge_fields;
///
/// let q = query(&format!(
///     "MATCH (c:Character {id: $character_id})-[k:KNOWS_LORE]->(l:Lore)
///      {}",
///     return_knowledge_fields()
/// ))
/// .param("character_id", character_id.to_string());
/// ```
pub const fn return_knowledge_fields() -> &'static str {
    "RETURN l.id as lore_id, c.id as character_id, k.known_chunk_ids as known_chunk_ids, k.discovery_source as discovery_source, k.discovered_at as discovered_at, k.notes as notes"
}

// =============================================================================
// Common Scene MATCH Fragments
// =============================================================================

/// Fragment for matching a Scene node by ID (using parameter `$id`).
///
/// Returns the MATCH clause `MATCH (s:Scene {id: $id})`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_scene_by_id;
///
/// let q = query(&format!("{} RETURN s", match_scene_by_id()))
///     .param("id", scene_id.to_string());
/// ```
pub const fn match_scene_by_id() -> &'static str {
    "MATCH (s:Scene {id: $id})"
}

// =============================================================================
// Common NarrativeEvent MATCH Fragments
// =============================================================================

/// Fragment for matching a NarrativeEvent node by ID (using parameter `$id`).
///
/// Returns the MATCH clause `MATCH (e:NarrativeEvent {id: $id})`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_narrative_event_by_id;
///
/// let q = query(&format!("{} RETURN e", match_narrative_event_by_id()))
///     .param("id", event_id.to_string());
/// ```
pub const fn match_narrative_event_by_id() -> &'static str {
    "MATCH (e:NarrativeEvent {id: $id})"
}

// =============================================================================
// Common EventChain MATCH Fragments
// =============================================================================

/// Fragment for matching an EventChain node by ID (using parameter `$id`).
///
/// Returns the MATCH clause `MATCH (c:EventChain {id: $id})`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_event_chain_by_id;
///
/// let q = query(&format!("{} RETURN c", match_event_chain_by_id()))
///     .param("id", chain_id.to_string());
/// ```
pub const fn match_event_chain_by_id() -> &'static str {
    "MATCH (c:EventChain {id: $id})"
}

// =============================================================================
// Common StoryEvent MATCH Fragments
// =============================================================================

/// Fragment for matching a StoryEvent node by ID (using parameter `$id`).
///
/// Returns the MATCH clause `MATCH (e:StoryEvent {id: $id})`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::match_story_event_by_id;
///
/// let q = query(&format!("{} RETURN e", match_story_event_by_id()))
///     .param("id", event_id.to_string());
/// ```
pub const fn match_story_event_by_id() -> &'static str {
    "MATCH (e:StoryEvent {id: $id})"
}

// =============================================================================
// Common RETURN Field Fragments
// =============================================================================

/// Fragment for returning a Scene node as 's'.
///
/// Returns `RETURN s` for RETURN clauses.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::return_scene;
///
/// let q = query(&format!("{} {}", match_scene_by_id(), return_scene()))
///     .param("id", scene_id.to_string());
/// ```
pub const fn return_scene() -> &'static str {
    "RETURN s"
}

/// Fragment for returning a World node as 'w'.
///
/// Returns `RETURN w` for RETURN clauses.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::return_world;
///
/// let q = query(&format!("{} {}", match_world_by_id(), return_world()))
///     .param("id", world_id.to_string());
/// ```
pub const fn return_world() -> &'static str {
    "RETURN w"
}

/// Fragment for returning an Item node as 'i'.
///
/// Returns `RETURN i` for RETURN clauses.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::return_item;
///
/// let q = query(&format!("{} {}", match_item_by_id(), return_item()))
///     .param("id", item_id.to_string());
/// ```
pub const fn return_item() -> &'static str {
    "RETURN i"
}

/// Fragment for returning a NarrativeEvent node as 'e'.
///
/// Returns `RETURN e` for RETURN clauses.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::return_narrative_event;
///
/// let q = query(&format!("{} {}", match_narrative_event_by_id(), return_narrative_event()))
///     .param("id", event_id.to_string());
/// ```
pub const fn return_narrative_event() -> &'static str {
    "RETURN e"
}

/// Fragment for returning an EventChain node as 'c'.
///
/// Returns `RETURN c` for RETURN clauses.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::return_event_chain;
///
/// let q = query(&format!("{} {}", match_event_chain_by_id(), return_event_chain()))
///     .param("id", chain_id.to_string());
/// ```
pub const fn return_event_chain() -> &'static str {
    "RETURN c"
}

/// Fragment for returning a StoryEvent node as 'e'.
///
/// Returns `RETURN e` for RETURN clauses.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::return_story_event;
///
/// let q = query(&format!("{} {}", match_story_event_by_id(), return_story_event()))
///     .param("id", event_id.to_string());
/// ```
pub const fn return_story_event() -> &'static str {
    "RETURN e"
}

// =============================================================================
// Common DELETE Fragments (Basic Node Deletes)
// =============================================================================

/// Fragment for deleting a Scene node and all relationships.
///
/// Returns DETACH DELETE clause `DETACH DELETE s`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::delete_scene;
///
/// let q = query(&format!("{} {}", match_scene_by_id(), delete_scene()))
///     .param("id", scene_id.to_string());
/// ```
pub const fn delete_scene() -> &'static str {
    "DETACH DELETE s"
}

/// Fragment for deleting a NarrativeEvent node and all relationships.
///
/// Returns DETACH DELETE clause `DETACH DELETE e`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::delete_narrative_event;
///
/// let q = query(&format!("{} {}", match_narrative_event_by_id(), delete_narrative_event()))
///     .param("id", event_id.to_string());
/// ```
pub const fn delete_narrative_event() -> &'static str {
    "DETACH DELETE e"
}

/// Fragment for deleting an EventChain node and all relationships.
///
/// Returns DETACH DELETE clause `DETACH DELETE c`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::delete_event_chain;
///
/// let q = query(&format!("{} {}", match_event_chain_by_id(), delete_event_chain()))
///     .param("id", chain_id.to_string());
/// ```
pub const fn delete_event_chain() -> &'static str {
    "DETACH DELETE c"
}

/// Fragment for deleting a StoryEvent node and all relationships.
///
/// Returns DETACH DELETE clause `DETACH DELETE e`.
///
/// # Example
/// ```ignore
/// use crate::infrastructure::neo4j::helpers::delete_story_event;
///
/// let q = query(&format!("{} {}", match_story_event_by_id(), delete_story_event()))
///     .param("id", event_id.to_string());
/// ```
pub const fn delete_story_event() -> &'static str {
    "DETACH DELETE e"
}
