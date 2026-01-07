//! Inventory use cases.
//!
//! Handles inventory queries and item placement operations.

use std::sync::Arc;

use crate::entities::Inventory;
use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::{Item, ItemId, PlayerCharacterId, WorldId};

/// Container for inventory use cases.
pub struct InventoryUseCases {
    pub ops: Arc<InventoryOps>,
}

impl InventoryUseCases {
    pub fn new(ops: Arc<InventoryOps>) -> Self {
        Self { ops }
    }
}

/// Inventory operations.
pub struct InventoryOps {
    inventory: Arc<Inventory>,
}

impl InventoryOps {
    pub fn new(inventory: Arc<Inventory>) -> Self {
        Self { inventory }
    }

    pub async fn get_pc_inventory(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<Item>, RepoError> {
        self.inventory.get_pc_inventory(pc_id).await
    }

    pub async fn get_character_inventory(
        &self,
        character_id: wrldbldr_domain::CharacterId,
    ) -> Result<Vec<Item>, RepoError> {
        self.inventory.get_character_inventory(character_id).await
    }

    pub async fn place_item_in_region(
        &self,
        item_id: ItemId,
        region_id: wrldbldr_domain::RegionId,
    ) -> Result<(), crate::entities::inventory::InventoryError> {
        self.inventory.place_item_in_region(item_id, region_id).await
    }

    pub async fn list_in_region(
        &self,
        region_id: wrldbldr_domain::RegionId,
    ) -> Result<Vec<Item>, RepoError> {
        self.inventory.list_in_region(region_id).await
    }

    pub async fn create_and_place_item(
        &self,
        world_id: WorldId,
        region_id: wrldbldr_domain::RegionId,
        data: CreateItemInput,
    ) -> Result<ItemId, crate::entities::inventory::InventoryError> {
        let mut item = Item::new(world_id, data.name);
        if let Some(desc) = data.description {
            item = item.with_description(desc);
        }
        if let Some(item_type) = data.item_type {
            item = item.with_type(item_type);
        }
        if let Some(props) = data.properties {
            item = item.with_properties(props.to_string());
        }

        self.inventory.create_and_place_in_region(item, region_id).await
    }
}

#[derive(Debug, Clone)]
pub struct CreateItemInput {
    pub name: String,
    pub description: Option<String>,
    pub item_type: Option<String>,
    pub properties: Option<serde_json::Value>,
}
