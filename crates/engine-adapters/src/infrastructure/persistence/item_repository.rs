//! Neo4j Item Repository
//!
//! Implements the ItemRepositoryPort for Neo4j persistence.
//!
//! # Neo4j Relationships
//! - `(World)-[:CONTAINS_ITEM]->(Item)` - World contains item
//! - `(Item)-[:CONTAINS {quantity, added_at}]->(Item)` - Container items

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;

use wrldbldr_domain::entities::Item;
use wrldbldr_domain::{ItemId, WorldId};
use wrldbldr_engine_ports::outbound::{ContainerInfo, ItemRepositoryPort};

use super::converters::row_to_item;
use super::Neo4jConnection;

/// Neo4j implementation of ItemRepositoryPort
pub struct Neo4jItemRepository {
    connection: Neo4jConnection,
}

impl Neo4jItemRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl ItemRepositoryPort for Neo4jItemRepository {
    async fn create(&self, item: &Item) -> Result<()> {
        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (i:Item {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description,
                item_type: $item_type,
                is_unique: $is_unique,
                properties: $properties,
                can_contain_items: $can_contain_items,
                container_limit: $container_limit
            })
            CREATE (w)-[:CONTAINS_ITEM]->(i)
            RETURN i.id as id",
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

        self.connection.graph().run(q).await?;
        Ok(())
    }

    async fn get(&self, id: ItemId) -> Result<Option<Item>> {
        let q = query(
            "MATCH (i:Item {id: $id})
            RETURN i",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let item = row_to_item(&row)?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    async fn list(&self, world_id: WorldId) -> Result<Vec<Item>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_ITEM]->(i:Item)
            RETURN i
            ORDER BY i.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut items = Vec::new();

        while let Some(row) = result.next().await? {
            items.push(row_to_item(&row)?);
        }

        Ok(items)
    }

    async fn update(&self, item: &Item) -> Result<()> {
        let q = query(
            "MATCH (i:Item {id: $id})
            SET i.name = $name,
                i.description = $description,
                i.item_type = $item_type,
                i.is_unique = $is_unique,
                i.properties = $properties,
                i.can_contain_items = $can_contain_items,
                i.container_limit = $container_limit
            RETURN i.id as id",
        )
        .param("id", item.id.to_string())
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

        self.connection.graph().run(q).await?;
        Ok(())
    }

    async fn delete(&self, id: ItemId) -> Result<()> {
        let q = query(
            "MATCH (i:Item {id: $id})
            DETACH DELETE i",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    async fn get_by_type(&self, world_id: WorldId, item_type: &str) -> Result<Vec<Item>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_ITEM]->(i:Item)
            WHERE i.item_type = $item_type
            RETURN i
            ORDER BY i.name",
        )
        .param("world_id", world_id.to_string())
        .param("item_type", item_type);

        let mut result = self.connection.graph().execute(q).await?;
        let mut items = Vec::new();

        while let Some(row) = result.next().await? {
            items.push(row_to_item(&row)?);
        }

        Ok(items)
    }

    // =========================================================================
    // Container Operations
    // =========================================================================

    async fn add_item_to_container(
        &self,
        container_id: ItemId,
        item_id: ItemId,
        quantity: u32,
    ) -> Result<()> {
        // Pure data access - validation should be done by the service layer
        let add_q = query(
            "MATCH (container:Item {id: $container_id}), (item:Item {id: $item_id})
            CREATE (container)-[:CONTAINS {
                quantity: $quantity,
                added_at: $added_at
            }]->(item)
            RETURN item.id as id",
        )
        .param("container_id", container_id.to_string())
        .param("item_id", item_id.to_string())
        .param("quantity", quantity as i64)
        .param("added_at", chrono::Utc::now().to_rfc3339());

        self.connection.graph().run(add_q).await?;
        Ok(())
    }

    async fn get_container_contents(&self, container_id: ItemId) -> Result<Vec<(Item, u32)>> {
        let q = query(
            "MATCH (container:Item {id: $container_id})-[r:CONTAINS]->(item:Item)
            RETURN item as i, r.quantity as quantity
            ORDER BY item.name",
        )
        .param("container_id", container_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut contents = Vec::new();

        while let Some(row) = result.next().await? {
            let item = row_to_item(&row)?;
            let quantity: i64 = row.get("quantity").unwrap_or(1);
            contents.push((item, quantity as u32));
        }

        Ok(contents)
    }

    async fn remove_item_from_container(
        &self,
        container_id: ItemId,
        item_id: ItemId,
        quantity: u32,
    ) -> Result<()> {
        // Get current quantity
        let check_q = query(
            "MATCH (container:Item {id: $container_id})-[r:CONTAINS]->(item:Item {id: $item_id})
            RETURN r.quantity as quantity",
        )
        .param("container_id", container_id.to_string())
        .param("item_id", item_id.to_string());

        let mut result = self.connection.graph().execute(check_q).await?;

        if let Some(row) = result.next().await? {
            let current_quantity: i64 = row.get("quantity").unwrap_or(1);
            let new_quantity = current_quantity - quantity as i64;

            if new_quantity <= 0 {
                // Remove the relationship entirely
                let delete_q = query(
                    "MATCH (container:Item {id: $container_id})-[r:CONTAINS]->(item:Item {id: $item_id})
                    DELETE r",
                )
                .param("container_id", container_id.to_string())
                .param("item_id", item_id.to_string());

                self.connection.graph().run(delete_q).await?;
            } else {
                // Update quantity
                let update_q = query(
                    "MATCH (container:Item {id: $container_id})-[r:CONTAINS]->(item:Item {id: $item_id})
                    SET r.quantity = $quantity
                    RETURN item.id as id",
                )
                .param("container_id", container_id.to_string())
                .param("item_id", item_id.to_string())
                .param("quantity", new_quantity);

                self.connection.graph().run(update_q).await?;
            }
        }

        Ok(())
    }

    async fn get_container_info(&self, container_id: ItemId) -> Result<ContainerInfo> {
        let q = query(
            "MATCH (container:Item {id: $container_id})
            OPTIONAL MATCH (container)-[r:CONTAINS]->(item:Item)
            WITH container, count(item) as current_count
            RETURN container.can_contain_items as can_contain, 
                   container.container_limit as max_limit, 
                   current_count",
        )
        .param("container_id", container_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let can_contain: bool = row.get("can_contain").unwrap_or(false);
            let max_limit: i64 = row.get("max_limit").unwrap_or(-1);
            let current_count: i64 = row.get("current_count")?;

            let max = if max_limit < 0 {
                None
            } else {
                Some(max_limit as u32)
            };

            Ok(ContainerInfo {
                can_contain_items: can_contain,
                current_count: current_count as u32,
                max_limit: max,
            })
        } else {
            Err(anyhow::anyhow!("Container not found"))
        }
    }
}
