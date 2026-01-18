//! Pickup item use case.
//!
//! Picks up an item from the current region into the player's inventory.

use std::sync::Arc;
use wrldbldr_domain::{ItemId, PlayerCharacterId};

use crate::infrastructure::ports::{ItemRepo, PlayerCharacterRepo};

use super::error::InventoryError;
use super::types::InventoryActionResult;

/// Pickup item use case.
///
/// Orchestrates: PC validation, item validation, region verification,
/// removal from region, addition to inventory.
pub struct PickupItem {
    item_repo: Arc<dyn ItemRepo>,
    pc_repo: Arc<dyn PlayerCharacterRepo>,
}

impl PickupItem {
    pub fn new(item_repo: Arc<dyn ItemRepo>, pc_repo: Arc<dyn PlayerCharacterRepo>) -> Self {
        Self { item_repo, pc_repo }
    }

    /// Execute the pickup item use case.
    ///
    /// # Arguments
    /// * `pc_id` - The player character picking up the item
    /// * `item_id` - The item to pick up
    ///
    /// # Returns
    /// * `Ok(InventoryActionResult)` - Item picked up successfully
    /// * `Err(InventoryError)` - Failed to pick up item
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<InventoryActionResult, InventoryError> {
        // Get the PC
        let pc = self
            .pc_repo
            .get(pc_id)
            .await?
            .ok_or(InventoryError::CharacterNotFound(pc_id))?;

        // Get the item
        let item = self
            .item_repo
            .get(item_id)
            .await?
            .ok_or(InventoryError::ItemNotFound(item_id))?;

        // Verify the item is in the PC's current region
        let pc_region = pc.current_region_id().ok_or(InventoryError::NotInRegion)?;
        let items_in_region = self.item_repo.list_in_region(pc_region).await?;
        if !items_in_region.iter().any(|i| i.id() == item_id) {
            return Err(InventoryError::ItemNotInRegion);
        }

        // Remove IN_REGION edge (item is no longer on the ground)
        self.item_repo.remove_from_region(item_id).await?;

        // Add POSSESSES edge (add to inventory)
        self.pc_repo.add_to_inventory(pc_id, item_id).await?;

        Ok(InventoryActionResult {
            item_name: item.name().to_string(),
            quantity: 1,
        })
    }
}
