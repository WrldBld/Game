//! Inventory management operations for PlayerCharacter entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{AcquisitionMethod, InventoryItem, ItemId, PlayerCharacterId};

/// Inventory management operations for player characters.
///
/// This trait covers CRUD operations for the items possessed
/// by a player character (the POSSESSES edge in the graph).
#[async_trait]
pub trait PlayerCharacterInventoryPort: Send + Sync {
    /// Add an item to PC's inventory (creates POSSESSES edge)
    async fn add_inventory_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
        quantity: u32,
        is_equipped: bool,
        acquisition_method: Option<AcquisitionMethod>,
    ) -> Result<()>;

    /// Get all items in PC's inventory
    async fn get_inventory(&self, pc_id: PlayerCharacterId) -> Result<Vec<InventoryItem>>;

    /// Get a specific item from PC's inventory
    async fn get_inventory_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<Option<InventoryItem>>;

    /// Update quantity/equipped status of item in PC's inventory
    async fn update_inventory_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
        quantity: u32,
        is_equipped: bool,
    ) -> Result<()>;

    /// Remove an item from PC's inventory (deletes POSSESSES edge)
    async fn remove_inventory_item(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> Result<()>;
}
