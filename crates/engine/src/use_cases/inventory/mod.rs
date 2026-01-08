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
    pub actions: Arc<InventoryActions>,
}

impl InventoryUseCases {
    pub fn new(ops: Arc<InventoryOps>, actions: Arc<InventoryActions>) -> Self {
        Self { ops, actions }
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

    pub async fn get_pc_inventory(&self, pc_id: PlayerCharacterId) -> Result<Vec<Item>, RepoError> {
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
        self.inventory
            .place_item_in_region(item_id, region_id)
            .await
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

        self.inventory
            .create_and_place_in_region(item, region_id)
            .await
    }
}

/// Inventory action orchestration (equip/unequip/drop/pickup).
pub struct InventoryActions {
    inventory: Arc<Inventory>,
}

impl InventoryActions {
    pub fn new(inventory: Arc<Inventory>) -> Self {
        Self { inventory }
    }

    pub async fn equip(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<crate::entities::inventory::InventoryActionResult, InventoryActionError> {
        self.inventory
            .equip_item(pc_id, item_id)
            .await
            .map_err(InventoryActionError::from)
    }

    pub async fn unequip(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<crate::entities::inventory::InventoryActionResult, InventoryActionError> {
        self.inventory
            .unequip_item(pc_id, item_id)
            .await
            .map_err(InventoryActionError::from)
    }

    pub async fn drop_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
        quantity: u32,
    ) -> Result<crate::entities::inventory::InventoryActionResult, InventoryActionError> {
        self.inventory
            .drop_item(pc_id, item_id, quantity)
            .await
            .map_err(InventoryActionError::from)
    }

    pub async fn pickup(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<crate::entities::inventory::InventoryActionResult, InventoryActionError> {
        self.inventory
            .pickup_item(pc_id, item_id)
            .await
            .map_err(InventoryActionError::from)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InventoryActionError {
    #[error("Inventory error: {0}")]
    Inventory(#[from] crate::entities::inventory::InventoryError),
}

#[derive(Debug, Clone)]
pub struct CreateItemInput {
    pub name: String,
    pub description: Option<String>,
    pub item_type: Option<String>,
    pub properties: Option<serde_json::Value>,
}
