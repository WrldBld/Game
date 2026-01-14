//! Neo4j player character repository implementation.
//!
//! Handles PlayerCharacter persistence for players in game worlds.

use std::sync::Arc;

use async_trait::async_trait;
use neo4rs::{query, Node, Row};
use crate::infrastructure::neo4j::Neo4jGraph;
use wrldbldr_domain::*;

use super::helpers::{parse_optional_typed_id, parse_typed_id, row_to_item, NodeExt};
use crate::infrastructure::ports::{ClockPort, PlayerCharacterRepo, RepoError};

pub struct Neo4jPlayerCharacterRepo {
    graph: Neo4jGraph,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jPlayerCharacterRepo {
    pub fn new(graph: Neo4jGraph, clock: Arc<dyn ClockPort>) -> Self {
        Self { graph, clock }
    }
}

#[async_trait]
impl PlayerCharacterRepo for Neo4jPlayerCharacterRepo {
    /// Get a player character by ID
    async fn get(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>, RepoError> {
        let q = query("MATCH (pc:PlayerCharacter {id: $id}) RETURN pc").param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            Ok(Some(row_to_player_character(row)?))
        } else {
            Ok(None)
        }
    }

    /// Save a player character (upsert)
    async fn save(&self, pc: &PlayerCharacter) -> Result<(), RepoError> {
        let sheet_data_json = pc
            .sheet_data()
            .map(|s| serde_json::to_string(s))
            .transpose()
            .map_err(|e| RepoError::Serialization(e.to_string()))?
            .unwrap_or_else(|| "{}".to_string());

        let current_region_id_str = pc
            .current_region_id()
            .map(|r| r.to_string())
            .unwrap_or_default();

        let q = query(
            "MERGE (pc:PlayerCharacter {id: $id})
            ON CREATE SET
                pc.user_id = $user_id,
                pc.world_id = $world_id,
                pc.name = $name,
                pc.description = $description,
                pc.sheet_data = $sheet_data,
                pc.current_location_id = $current_location_id,
                pc.current_region_id = $current_region_id,
                pc.starting_location_id = $starting_location_id,
                pc.sprite_asset = $sprite_asset,
                pc.portrait_asset = $portrait_asset,
                pc.is_alive = $is_alive,
                pc.is_active = $is_active,
                pc.created_at = $created_at,
                pc.last_active_at = $last_active_at
            ON MATCH SET
                pc.name = $name,
                pc.description = $description,
                pc.sheet_data = $sheet_data,
                pc.current_location_id = $current_location_id,
                pc.current_region_id = $current_region_id,
                pc.sprite_asset = $sprite_asset,
                pc.portrait_asset = $portrait_asset,
                pc.is_alive = $is_alive,
                pc.is_active = $is_active,
                pc.last_active_at = $last_active_at
            WITH pc
            MATCH (w:World {id: $world_id})
            MERGE (pc)-[:IN_WORLD]->(w)
            WITH pc
            MATCH (l:Location {id: $current_location_id})
            MERGE (pc)-[:AT_LOCATION]->(l)",
        )
        .param("id", pc.id().to_string())
        .param("user_id", pc.user_id().to_string())
        .param("world_id", pc.world_id().to_string())
        .param("name", pc.name().to_string())
        .param(
            "description",
            pc.description().unwrap_or_default().to_string(),
        )
        .param("sheet_data", sheet_data_json)
        .param("current_location_id", pc.current_location_id().to_string())
        .param("current_region_id", current_region_id_str)
        .param(
            "starting_location_id",
            pc.starting_location_id().to_string(),
        )
        .param(
            "sprite_asset",
            pc.sprite_asset().unwrap_or_default().to_string(),
        )
        .param(
            "portrait_asset",
            pc.portrait_asset().unwrap_or_default().to_string(),
        )
        .param("is_alive", pc.is_alive())
        .param("is_active", pc.is_active())
        .param("created_at", pc.created_at().to_rfc3339())
        .param("last_active_at", pc.last_active_at().to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }

    /// Delete a player character
    async fn delete(&self, id: PlayerCharacterId) -> Result<(), RepoError> {
        let q = query("MATCH (pc:PlayerCharacter {id: $id}) DETACH DELETE pc")
            .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }

    /// List all player characters in a world
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<PlayerCharacter>, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter)-[:IN_WORLD]->(w:World {id: $world_id})
            RETURN pc
            ORDER BY pc.last_active_at DESC",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        let mut pcs = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            pcs.push(row_to_player_character(row)?);
        }

        Ok(pcs)
    }

    /// Get a player character by user ID in a specific world
    async fn get_by_user(
        &self,
        world_id: WorldId,
        user_id: &str,
    ) -> Result<Option<PlayerCharacter>, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {user_id: $user_id})-[:IN_WORLD]->(w:World {id: $world_id})
            RETURN pc
            ORDER BY pc.last_active_at DESC
            LIMIT 1",
        )
        .param("user_id", user_id)
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            Ok(Some(row_to_player_character(row)?))
        } else {
            Ok(None)
        }
    }

    /// Update position (location and region)
    async fn update_position(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
        region_id: RegionId,
    ) -> Result<(), RepoError> {
        // Single atomic query: delete old relationship, create new one, and verify success
        // Returns the PC id if successful (location exists), null otherwise
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $id})
            MATCH (l:Location {id: $location_id})
            OPTIONAL MATCH (pc)-[old:AT_LOCATION]->()
            DELETE old
            CREATE (pc)-[:AT_LOCATION]->(l)
            SET pc.current_location_id = $location_id,
                pc.current_region_id = $region_id,
                pc.last_active_at = $last_active_at
            RETURN pc.id AS updated_id",
        )
        .param("id", id.to_string())
        .param("location_id", location_id.to_string())
        .param("region_id", region_id.to_string())
        .param("last_active_at", self.clock.now().to_rfc3339());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Check if the update succeeded (PC and Location both exist)
        if result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
            .is_none()
        {
            tracing::warn!(
                pc_id = %id,
                location_id = %location_id,
                "update_position failed: PC or Location not found"
            );
            return Err(RepoError::not_found("Entity", "unknown"));
        }

        Ok(())
    }

    /// Get inventory items for a player character
    async fn get_inventory(&self, id: PlayerCharacterId) -> Result<Vec<Item>, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[:POSSESSES]->(i:Item)
            RETURN i
            ORDER BY i.name",
        )
        .param("pc_id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        let mut items = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            items.push(row_to_item(row)?);
        }

        Ok(items)
    }

    /// Add an item to a player character's inventory (creates POSSESSES edge)
    async fn add_to_inventory(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})
            MATCH (i:Item {id: $item_id})
            MERGE (pc)-[:POSSESSES]->(i)",
        )
        .param("pc_id", pc_id.to_string())
        .param("item_id", item_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }

    /// Remove an item from a player character's inventory (removes POSSESSES edge)
    async fn remove_from_inventory(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[r:POSSESSES]->(i:Item {id: $item_id})
            DELETE r",
        )
        .param("pc_id", pc_id.to_string())
        .param("item_id", item_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }

    /// Modify a stat on a player character.
    /// Stats are stored in a JSON field `stats_json` on the PC node.
    /// Uses explicit transaction to ensure atomicity and prevent race conditions.
    async fn modify_stat(
        &self,
        id: PlayerCharacterId,
        stat: &str,
        modifier: i32,
    ) -> Result<(), RepoError> {
        // Use explicit transaction to ensure read-modify-write is atomic
        let mut txn = self
            .graph
            .start_txn()
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Step 1: Read current stats_json within transaction
        let read_q = query(
            "MATCH (pc:PlayerCharacter {id: $id})
            RETURN coalesce(pc.stats_json, '{}') as stats_json",
        )
        .param("id", id.to_string());

        let mut result = txn
            .execute(read_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let stats_json: String = if let Some(row) = result
            .next(txn.handle())
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            row.get("stats_json").unwrap_or_else(|_| "{}".to_string())
        } else {
            // Rollback not strictly needed for read-only, but good practice
            txn.rollback()
                .await
                .map_err(|e| RepoError::database("query", e))?;
            return Err(RepoError::not_found("Entity", "unknown"));
        };

        // Step 2: Parse JSON, modify stat in Rust
        let mut stats: std::collections::HashMap<String, i64> =
            serde_json::from_str(&stats_json).unwrap_or_default();

        let current_value = stats.get(stat).copied().unwrap_or(0);
        let new_value = current_value + modifier as i64;
        stats.insert(stat.to_string(), new_value);

        // Step 3: Write updated JSON back within same transaction
        let updated_json =
            serde_json::to_string(&stats).map_err(|e| RepoError::Serialization(e.to_string()))?;

        let write_q = query(
            "MATCH (pc:PlayerCharacter {id: $id})
            SET pc.stats_json = $stats_json",
        )
        .param("id", id.to_string())
        .param("stats_json", updated_json);

        txn.run(write_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Commit the transaction
        txn.commit()
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::info!(pc_id = %id, stat = %stat, modifier = %modifier, new_value = %new_value, "Modified stat");
        Ok(())
    }
}

// =============================================================================
// Row conversion helpers
// =============================================================================

fn row_to_player_character(row: Row) -> Result<PlayerCharacter, RepoError> {
    let node: Node = row.get("pc").map_err(|e| RepoError::database("query", e))?;

    let id: PlayerCharacterId =
        parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
    let user_id: String = node
        .get("user_id")
        .map_err(|e| RepoError::database("query", e))?;
    let world_id: WorldId =
        parse_typed_id(&node, "world_id").map_err(|e| RepoError::database("query", e))?;
    let name: String = node
        .get("name")
        .map_err(|e| RepoError::database("query", e))?;
    let description = node.get_optional_string("description");

    // Parse sheet_data from JSON
    let sheet_data_str = node.get_string_or("sheet_data", "{}");
    let sheet_data = if sheet_data_str.is_empty() || sheet_data_str == "{}" {
        None
    } else {
        serde_json::from_str(&sheet_data_str)
            .map_err(|e| RepoError::Serialization(format!("Invalid sheet_data: {}", e)))?
    };

    let current_location_id: LocationId = parse_typed_id(&node, "current_location_id")
        .map_err(|e| RepoError::database("query", e))?;
    let current_region_id: Option<RegionId> = parse_optional_typed_id(&node, "current_region_id")
        .map_err(|e| RepoError::database("query", e))?;
    let starting_location_id: LocationId = parse_typed_id(&node, "starting_location_id")
        .map_err(|e| RepoError::database("query", e))?;

    let sprite_asset = node.get_optional_string("sprite_asset");
    let portrait_asset = node.get_optional_string("portrait_asset");

    let created_at_str: String = node
        .get("created_at")
        .map_err(|e| RepoError::database("query", e))?;
    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
        .map_err(|e| RepoError::database("query", format!("Invalid created_at: {}", e)))?
        .with_timezone(&chrono::Utc);

    let last_active_at_str: String = node
        .get("last_active_at")
        .map_err(|e| RepoError::database("query", e))?;
    let last_active_at = chrono::DateTime::parse_from_rfc3339(&last_active_at_str)
        .map_err(|e| RepoError::database("query", format!("Invalid last_active_at: {}", e)))?
        .with_timezone(&chrono::Utc);

    // Status flags with defaults
    let is_alive: bool = node.get("is_alive").unwrap_or(true);
    let is_active: bool = node.get("is_active").unwrap_or(true);

    Ok(PlayerCharacter::new(
        user_id,
        world_id,
        wrldbldr_domain::CharacterName::new(&name)
            .map_err(|e| RepoError::database("query", e.to_string()))?,
        starting_location_id,
        created_at,
    )
    .with_id(id)
    .with_current_location(current_location_id)
    .with_current_region(current_region_id)
    .with_description(description.unwrap_or_default())
    .with_sheet_data(sheet_data.unwrap_or_default())
    .with_sprite(sprite_asset.unwrap_or_default())
    .with_portrait(portrait_asset.unwrap_or_default())
    .with_state(CharacterState::from_legacy(is_alive, is_active))
    .with_last_active_at(last_active_at))
}