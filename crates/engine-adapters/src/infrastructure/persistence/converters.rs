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
use wrldbldr_domain::{
    entities::{Item, MapBounds, Region, Want, WantVisibility},
    ItemId, LocationId, RegionId, WantId, WorldId,
};

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

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let item_type: String = node.get("item_type").unwrap_or_default();
    let is_unique: bool = node.get("is_unique").unwrap_or(false);
    let properties: String = node.get("properties").unwrap_or_default();
    let can_contain_items: bool = node.get("can_contain_items").unwrap_or(false);
    let container_limit: i64 = node.get("container_limit").unwrap_or(-1);

    let id = uuid::Uuid::parse_str(&id_str)?;
    let world_id = uuid::Uuid::parse_str(&world_id_str)?;

    Ok(Item {
        id: ItemId::from_uuid(id),
        world_id: WorldId::from_uuid(world_id),
        name,
        description: if description.is_empty() {
            None
        } else {
            Some(description)
        },
        item_type: if item_type.is_empty() {
            None
        } else {
            Some(item_type)
        },
        is_unique,
        properties: if properties.is_empty() {
            None
        } else {
            Some(properties)
        },
        can_contain_items,
        container_limit: if container_limit < 0 {
            None
        } else {
            Some(container_limit as u32)
        },
    })
}

/// Convert a Neo4j row to a Want entity.
///
/// Expects the row to contain a node with alias `w` representing the Want.
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
pub fn row_to_want(row: &Row) -> Result<Want> {
    let node: neo4rs::Node = row.get("w")?;

    let id_str: String = node.get("id")?;
    let description: String = node.get("description")?;
    let intensity: f64 = node.get("intensity")?;
    let created_at_str: String = node.get("created_at")?;

    // Handle visibility - try new field first, fall back to legacy known_to_player
    let visibility = if let Ok(vis_str) = node.get::<String>("visibility") {
        match vis_str.as_str() {
            "Known" => WantVisibility::Known,
            "Suspected" => WantVisibility::Suspected,
            _ => WantVisibility::Hidden,
        }
    } else {
        // Legacy: convert from known_to_player bool
        let known_to_player: bool = node.get("known_to_player").unwrap_or(false);
        WantVisibility::from_known_to_player(known_to_player)
    };

    // Behavioral guidance fields (optional)
    let deflection_behavior: Option<String> = node
        .get("deflection_behavior")
        .ok()
        .filter(|s: &String| !s.is_empty());
    let tells_json: String = node.get("tells").unwrap_or_else(|_| "[]".to_string());
    let tells: Vec<String> = serde_json::from_str(&tells_json).unwrap_or_default();

    let id = uuid::Uuid::parse_str(&id_str)?;
    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(Want {
        id: WantId::from_uuid(id),
        description,
        intensity: intensity as f32,
        visibility,
        created_at,
        deflection_behavior,
        tells,
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

    let id_str: String = node.get("id")?;
    let location_id_str: String = node.get("location_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let backdrop_asset: String = node.get("backdrop_asset").unwrap_or_default();
    let atmosphere: String = node.get("atmosphere").unwrap_or_default();
    let map_bounds_json: String = node.get("map_bounds").unwrap_or_default();
    let is_spawn_point: bool = node.get("is_spawn_point").unwrap_or(false);
    let order: i64 = node.get("order").unwrap_or(0);

    let id = uuid::Uuid::parse_str(&id_str)?;
    let location_id = uuid::Uuid::parse_str(&location_id_str)?;

    // Parse map_bounds from JSON
    let map_bounds = if map_bounds_json.is_empty() {
        None
    } else {
        serde_json::from_str::<serde_json::Value>(&map_bounds_json)
            .ok()
            .and_then(|v| {
                Some(MapBounds {
                    x: v.get("x")?.as_u64()? as u32,
                    y: v.get("y")?.as_u64()? as u32,
                    width: v.get("width")?.as_u64()? as u32,
                    height: v.get("height")?.as_u64()? as u32,
                })
            })
    };

    Ok(Region {
        id: RegionId::from_uuid(id),
        location_id: LocationId::from_uuid(location_id),
        name,
        description,
        backdrop_asset: if backdrop_asset.is_empty() {
            None
        } else {
            Some(backdrop_asset)
        },
        atmosphere: if atmosphere.is_empty() {
            None
        } else {
            Some(atmosphere)
        },
        map_bounds,
        is_spawn_point,
        order: order as u32,
    })
}
