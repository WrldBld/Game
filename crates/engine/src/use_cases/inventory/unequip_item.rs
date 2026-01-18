//! Unequip item use case.
//!
//! Removes the equipped status from an item in the player's inventory.

use std::sync::Arc;
use wrldbldr_domain::{ItemId, PlayerCharacterId};

use crate::infrastructure::ports::{ItemRepo, PlayerCharacterRepo};

use super::error::InventoryError;
use super::types::InventoryActionResult;

/// Unequip item use case.
///
/// Orchestrates: Item validation, inventory verification, equipment state change.
pub struct UnequipItem {
    item_repo: Arc<dyn ItemRepo>,
    pc_repo: Arc<dyn PlayerCharacterRepo>,
}

impl UnequipItem {
    pub fn new(item_repo: Arc<dyn ItemRepo>, pc_repo: Arc<dyn PlayerCharacterRepo>) -> Self {
        Self { item_repo, pc_repo }
    }

    /// Execute the unequip item use case.
    ///
    /// # Arguments
    /// * `pc_id` - The player character unequipping the item
    /// * `item_id` - The item to unequip
    ///
    /// # Returns
    /// * `Ok(InventoryActionResult)` - Item unequipped successfully
    /// * `Err(InventoryError)` - Failed to unequip item
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<InventoryActionResult, InventoryError> {
        // Get the item
        let item = self
            .item_repo
            .get(item_id)
            .await?
            .ok_or(InventoryError::ItemNotFound(item_id))?;

        // Verify the item is in the PC's inventory
        let inventory = self.pc_repo.get_inventory(pc_id).await?;
        if !inventory.iter().any(|i| i.id() == item_id) {
            return Err(InventoryError::ItemNotInInventory(item_id));
        }

        // Remove EQUIPPED_BY edge in graph
        self.item_repo.set_unequipped(pc_id, item_id).await?;

        Ok(InventoryActionResult {
            item_name: item.name().to_string(),
            quantity: 1,
        })
    }
}
