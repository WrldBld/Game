//! Equip item use case.
//!
//! Marks an item in the player's inventory as equipped.

use std::sync::Arc;
use wrldbldr_domain::{ItemId, PlayerCharacterId};

use crate::infrastructure::ports::{ItemRepo, PlayerCharacterRepo};

use super::error::InventoryError;
use super::types::InventoryActionResult;

/// Equip item use case.
///
/// Orchestrates: Item validation, inventory verification, equipment state change.
pub struct EquipItem {
    item_repo: Arc<dyn ItemRepo>,
    pc_repo: Arc<dyn PlayerCharacterRepo>,
}

impl EquipItem {
    pub fn new(item_repo: Arc<dyn ItemRepo>, pc_repo: Arc<dyn PlayerCharacterRepo>) -> Self {
        Self { item_repo, pc_repo }
    }

    /// Execute the equip item use case.
    ///
    /// # Arguments
    /// * `pc_id` - The player character equipping the item
    /// * `item_id` - The item to equip
    ///
    /// # Returns
    /// * `Ok(InventoryActionResult)` - Item equipped successfully
    /// * `Err(InventoryError)` - Failed to equip item
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<InventoryActionResult, InventoryError> {
        // Get the item to verify it exists
        let item = self
            .item_repo
            .get(item_id)
            .await?
            .ok_or(InventoryError::ItemNotFound)?;

        // Verify the item is in the PC's inventory
        let inventory = self.pc_repo.get_inventory(pc_id).await?;
        if !inventory.iter().any(|i| i.id() == item_id) {
            return Err(InventoryError::ItemNotInInventory);
        }

        // Create EQUIPPED_BY edge in graph
        self.item_repo.set_equipped(pc_id, item_id).await?;

        Ok(InventoryActionResult {
            item_name: item.name().to_string(),
            quantity: 1,
        })
    }
}
