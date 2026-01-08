//! Neo4j item repository implementation.
//!
//! Handles item persistence in the game world.

use async_trait::async_trait;
use neo4rs::{query, Graph};
use wrldbldr_domain::*;

use super::helpers::row_to_item;
use crate::infrastructure::ports::{ItemRepo, RepoError};

#[allow(unused_imports)]
use wrldbldr_domain::PlayerCharacterId;

pub struct Neo4jItemRepo {
    graph: Graph,
}

impl Neo4jItemRepo {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }
}

#[async_trait]
impl ItemRepo for Neo4jItemRepo {
    /// Get an item by ID
    async fn get(&self, id: ItemId) -> Result<Option<Item>, RepoError> {
        let q = query("MATCH (i:Item {id: $id}) RETURN i").param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            Ok(Some(row_to_item(row)?))
        } else {
            Ok(None)
        }
    }

    /// Save an item (upsert)
    async fn save(&self, item: &Item) -> Result<(), RepoError> {
        let q = query(
            "MERGE (i:Item {id: $id})
            ON CREATE SET
                i.world_id = $world_id,
                i.name = $name,
                i.description = $description,
                i.item_type = $item_type,
                i.is_unique = $is_unique,
                i.properties = $properties,
                i.can_contain_items = $can_contain_items,
                i.container_limit = $container_limit
            ON MATCH SET
                i.name = $name,
                i.description = $description,
                i.item_type = $item_type,
                i.is_unique = $is_unique,
                i.properties = $properties,
                i.can_contain_items = $can_contain_items,
                i.container_limit = $container_limit
            WITH i
            MATCH (w:World {id: $world_id})
            MERGE (w)-[:CONTAINS_ITEM]->(i)",
        )
        .param("id", item.id.to_string())
        .param("world_id", item.world_id.to_string())
        .param("name", item.name.clone())
        .param("description", item.description.clone().unwrap_or_default())
        .param("item_type", item.item_type.clone().unwrap_or_default())
        .param("is_unique", item.is_unique)
        .param("properties", item.properties.clone().unwrap_or_default())
        .param("can_contain_items", item.can_contain_items)
        .param(
            "container_limit",
            item.container_limit.map(|l| l as i64).unwrap_or(-1),
        );

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete an item by ID.
    /// Uses DETACH DELETE to remove all relationships.
    async fn delete(&self, id: ItemId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (i:Item {id: $id})
            DETACH DELETE i",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Deleted item: {}", id);
        Ok(())
    }

    /// List items in a region (items that are "in" the region via some relationship)
    async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<Item>, RepoError> {
        // Items in a region could be:
        // - Items possessed by characters in the region (via CURRENTLY_IN)
        // - Items placed in the region directly (via IN_REGION)
        // Note: ORDER BY must be outside UNION in Cypher
        let q = query(
            "MATCH (r:Region {id: $region_id})<-[:IN_REGION]-(i:Item)
            RETURN i
            UNION
            MATCH (r:Region {id: $region_id})<-[:CURRENTLY_IN]-(c:Character)-[:POSSESSES]->(i:Item)
            RETURN i
            ORDER BY i.name",
        )
        .param("region_id", region_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut items = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            items.push(row_to_item(row)?);
        }

        Ok(items)
    }

    /// List all items in a world
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<Item>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_ITEM]->(i:Item)
            RETURN i
            ORDER BY i.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut items = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            items.push(row_to_item(row)?);
        }

        Ok(items)
    }

    /// Mark an item as equipped by a player character (creates EQUIPPED_BY edge)
    async fn set_equipped(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})
            MATCH (i:Item {id: $item_id})
            MERGE (i)-[:EQUIPPED_BY]->(pc)",
        )
        .param("pc_id", pc_id.to_string())
        .param("item_id", item_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        Ok(())
    }

    /// Unequip an item (removes EQUIPPED_BY edge)
    async fn set_unequipped(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (i:Item {id: $item_id})-[r:EQUIPPED_BY]->(pc:PlayerCharacter {id: $pc_id})
            DELETE r",
        )
        .param("pc_id", pc_id.to_string())
        .param("item_id", item_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        Ok(())
    }

    /// Place an item in a region (creates IN_REGION edge for dropped items)
    async fn place_in_region(&self, item_id: ItemId, region_id: RegionId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (i:Item {id: $item_id})
            MATCH (r:Region {id: $region_id})
            MERGE (i)-[:IN_REGION]->(r)",
        )
        .param("item_id", item_id.to_string())
        .param("region_id", region_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        Ok(())
    }

    /// Remove an item from any region (removes IN_REGION edge)
    async fn remove_from_region(&self, item_id: ItemId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (i:Item {id: $item_id})-[r:IN_REGION]->()
            DELETE r",
        )
        .param("item_id", item_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        Ok(())
    }
}
