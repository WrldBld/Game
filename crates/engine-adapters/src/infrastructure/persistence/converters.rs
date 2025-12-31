//! Shared row-to-entity converter functions for Neo4j query results
//!
//! This module consolidates duplicate converter functions that were
//! previously scattered across multiple repository files. These converters
//! handle the standard deserialization of Neo4j rows into domain entities.
//!
//! # Usage
//!
//! Repository files should import the converters they need:
//! ```ignore
//! use super::converters::{row_to_item, row_to_want, row_to_region};
//! ```
//!
//! # Node Aliases
//!
//! These converters expect specific node aliases in the Cypher query:
//! - `row_to_item`: expects node alias `i`
//! - `row_to_want`: expects node alias `w`
//! - `row_to_region`: expects node alias `r`

use anyhow::Result;
use chrono::{DateTime, Utc};
use neo4rs::Row;
use wrldbldr_domain::entities::{Item, MapBounds, Region, Want, WantVisibility};

use super::neo4j_helpers::{parse_typed_id, NodeExt};

/// Convert a Neo4j row to an Item entity.
///
/// Expects the row to contain a node with alias `i` representing the Item.
///
/// # Node Properties
/// - `id` (String): UUID of the item
/// - `world_id` (String): UUID of the world
/// - `name` (String): Item name
/// - `description` (String, optional): Item description
/// - `item_type` (String, optional): Type classification
/// - `is_unique` (bool, optional): Whether item is unique, defaults to false
/// - `properties` (String, optional): JSON properties
/// - `can_contain_items` (bool, optional): Container capability, defaults to false
/// - `container_limit` (i64, optional): Max items if container
pub fn row_to_item(row: &Row) -> Result<Item> {
    let node: neo4rs::Node = row.get("i")?;

    Ok(Item {
        id: parse_typed_id(&node, "id")?,
        world_id: parse_typed_id(&node, "world_id")?,
        name: node.get("name")?,
        description: node.get_optional_string("description"),
        item_type: node.get_optional_string("item_type"),
        is_unique: node.get_bool_or("is_unique", false),
        properties: node.get_optional_string("properties"),
        can_contain_items: node.get_bool_or("can_contain_items", false),
        container_limit: node.get_positive_i64("container_limit"),
    })
}

/// Convert a Neo4j row to a Want entity.
///
/// Expects the row to contain a node with alias `w` representing the Want.
///
/// # Arguments
/// * `row` - The Neo4j row containing the want node
/// * `fallback_time` - Fallback timestamp to use if created_at parsing fails
///
/// # Node Properties
/// - `id` (String): UUID of the want
/// - `description` (String): What the character wants
/// - `intensity` (f64): How strongly they want it (0.0-1.0)
/// - `created_at` (String): RFC3339 timestamp
/// - `visibility` (String, optional): "Known", "Suspected", or "Hidden"
/// - `known_to_player` (bool, optional): Legacy field, converted to visibility
/// - `deflection_behavior` (String, optional): How character hides this want
/// - `tells` (String, optional): JSON array of behavioral tells
pub fn row_to_want(row: &Row, fallback_time: DateTime<Utc>) -> Result<Want> {
    let node: neo4rs::Node = row.get("w")?;

    // Handle visibility - try new field first, fall back to legacy known_to_player
    let visibility = if let Some(vis_str) = node.get_optional_string("visibility") {
        match vis_str.as_str() {
            "Known" => WantVisibility::Known,
            "Suspected" => WantVisibility::Suspected,
            _ => WantVisibility::Hidden,
        }
    } else {
        // Legacy: convert from known_to_player bool
        let known_to_player = node.get_bool_or("known_to_player", false);
        WantVisibility::from_known_to_player(known_to_player)
    };

    Ok(Want {
        id: parse_typed_id(&node, "id")?,
        description: node.get("description")?,
        intensity: node.get_f64_or("intensity", 0.5) as f32,
        visibility,
        created_at: node.get_datetime_or("created_at", fallback_time),
        deflection_behavior: node.get_optional_string("deflection_behavior"),
        tells: node.get_json_or_default("tells"),
    })
}

/// Convert a Neo4j row to a Region entity.
///
/// Expects the row to contain a node with alias `r` representing the Region.
///
/// # Node Properties
/// - `id` (String): UUID of the region
/// - `location_id` (String): UUID of the parent location
/// - `name` (String): Region name
/// - `description` (String, optional): Region description
/// - `backdrop_asset` (String, optional): Background image asset
/// - `atmosphere` (String, optional): Mood/atmosphere description
/// - `map_bounds` (String, optional): JSON with x, y, width, height
/// - `is_spawn_point` (bool, optional): Whether PCs can spawn here
/// - `order` (i64, optional): Display order, defaults to 0
pub fn row_to_region(row: &Row) -> Result<Region> {
    let node: neo4rs::Node = row.get("r")?;

    // Parse map_bounds from JSON - needs custom handling for nested structure
    let map_bounds: Option<MapBounds> = node
        .get_optional_string("map_bounds")
        .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
        .and_then(|v| {
            Some(MapBounds {
                x: v.get("x")?.as_u64()? as u32,
                y: v.get("y")?.as_u64()? as u32,
                width: v.get("width")?.as_u64()? as u32,
                height: v.get("height")?.as_u64()? as u32,
            })
        });

    Ok(Region {
        id: parse_typed_id(&node, "id")?,
        location_id: parse_typed_id(&node, "location_id")?,
        name: node.get("name")?,
        description: node.get_string_or("description", ""),
        backdrop_asset: node.get_optional_string("backdrop_asset"),
        atmosphere: node.get_optional_string("atmosphere"),
        map_bounds,
        is_spawn_point: node.get_bool_or("is_spawn_point", false),
        order: node.get_i64_or("order", 0) as u32,
    })
}
