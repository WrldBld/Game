//! CharacterInventoryPort implementation

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;
use wrldbldr_common::datetime::parse_datetime_or;
use wrldbldr_domain::entities::{AcquisitionMethod, InventoryItem};
use wrldbldr_domain::{CharacterId, ItemId};
use wrldbldr_engine_ports::outbound::CharacterInventoryPort;

use super::super::converters::row_to_item;
use super::Neo4jCharacterRepository;

impl Neo4jCharacterRepository {
    /// Add an item to character's inventory
    pub(crate) async fn add_inventory_item_impl(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
        acquisition_method: Option<AcquisitionMethod>,
    ) -> Result<()> {
        let method_str = acquisition_method
            .map(|m| m.to_string())
            .unwrap_or_default();

        let q = query(
            "MATCH (c:Character {id: $character_id}), (i:Item {id: $item_id})
            CREATE (c)-[:POSSESSES {
                quantity: $quantity,
                equipped: $equipped,
                acquired_at: $acquired_at,
                acquisition_method: $acquisition_method
            }]->(i)
            RETURN i.id as id",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string())
        .param("quantity", quantity as i64)
        .param("equipped", equipped)
        .param("acquired_at", self.clock.now_rfc3339())
        .param("acquisition_method", method_str);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get character's inventory
    pub(crate) async fn get_inventory_impl(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<InventoryItem>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item)
            RETURN i, r.quantity as quantity, r.equipped as equipped, 
                   r.acquired_at as acquired_at, r.acquisition_method as acquisition_method",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut inventory = Vec::new();

        while let Some(row) = result.next().await? {
            let item = row_to_item(&row)?;
            let quantity: i64 = row.get("quantity")?;
            let equipped: bool = row.get("equipped")?;
            let acquired_at_str: String = row.get("acquired_at")?;
            let acquisition_method_str: String = row.get("acquisition_method").unwrap_or_default();

            let acquired_at = parse_datetime_or(&acquired_at_str, self.clock.now());

            let acquisition_method = if acquisition_method_str.is_empty() {
                None
            } else {
                acquisition_method_str.parse().ok()
            };

            inventory.push(InventoryItem {
                item,
                quantity: quantity as u32,
                equipped,
                acquired_at,
                acquisition_method,
            });
        }

        Ok(inventory)
    }

    /// Get a single inventory item by ID
    pub(crate) async fn get_inventory_item_impl(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<Option<InventoryItem>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item {id: $item_id})
            RETURN i, r.quantity as quantity, r.equipped as equipped, 
                   r.acquired_at as acquired_at, r.acquisition_method as acquisition_method",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let item = row_to_item(&row)?;
            let quantity: i64 = row.get("quantity")?;
            let equipped: bool = row.get("equipped")?;
            let acquired_at_str: String = row.get("acquired_at")?;
            let acquisition_method_str: String = row.get("acquisition_method").unwrap_or_default();

            let acquired_at = parse_datetime_or(&acquired_at_str, self.clock.now());

            let acquisition_method = if acquisition_method_str.is_empty() {
                None
            } else {
                acquisition_method_str.parse().ok()
            };

            Ok(Some(InventoryItem {
                item,
                quantity: quantity as u32,
                equipped,
                acquired_at,
                acquisition_method,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update inventory item
    pub(crate) async fn update_inventory_item_impl(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item {id: $item_id})
            SET r.quantity = $quantity, r.equipped = $equipped
            RETURN i.id as id",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string())
        .param("quantity", quantity as i64)
        .param("equipped", equipped);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove an item from inventory
    pub(crate) async fn remove_inventory_item_impl(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:POSSESSES]->(i:Item {id: $item_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("item_id", item_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }
}

#[async_trait]
impl CharacterInventoryPort for Neo4jCharacterRepository {
    async fn add_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
        acquisition_method: Option<AcquisitionMethod>,
    ) -> Result<()> {
        self.add_inventory_item_impl(character_id, item_id, quantity, equipped, acquisition_method)
            .await
    }

    async fn get_inventory(&self, character_id: CharacterId) -> Result<Vec<InventoryItem>> {
        self.get_inventory_impl(character_id).await
    }

    async fn get_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<Option<InventoryItem>> {
        self.get_inventory_item_impl(character_id, item_id).await
    }

    async fn update_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
    ) -> Result<()> {
        self.update_inventory_item_impl(character_id, item_id, quantity, equipped)
            .await
    }

    async fn remove_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<()> {
        self.remove_inventory_item_impl(character_id, item_id).await
    }
}
