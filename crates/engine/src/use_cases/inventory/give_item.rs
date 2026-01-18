//! Give item use case.
//!
//! Creates a new item and gives it to a player character.
//! Used by the GiveItem trigger in challenge outcomes.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, PlayerCharacterId};

use crate::infrastructure::ports::{ItemRepo, PlayerCharacterRepo};

use super::error::InventoryError;
use super::types::InventoryActionResult;

/// Give item use case.
///
/// Orchestrates: PC validation, item creation, inventory addition.
pub struct GiveItem {
    item_repo: Arc<dyn ItemRepo>,
    pc_repo: Arc<dyn PlayerCharacterRepo>,
}

impl GiveItem {
    pub fn new(item_repo: Arc<dyn ItemRepo>, pc_repo: Arc<dyn PlayerCharacterRepo>) -> Self {
        Self { item_repo, pc_repo }
    }

    /// Execute the give item use case.
    ///
    /// Creates a new item with the given name/description and adds it to the PC's inventory.
    ///
    /// # Arguments
    /// * `pc_id` - The player character receiving the item
    /// * `item_name` - Name of the item to create
    /// * `item_description` - Optional description of the item
    ///
    /// # Returns
    /// * `Ok(InventoryActionResult)` - Item given successfully
    /// * `Err(InventoryError)` - Failed to give item
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        item_name: String,
        item_description: Option<String>,
    ) -> Result<InventoryActionResult, InventoryError> {
        // Get the PC to verify they exist and get their world_id
        let pc = self
            .pc_repo
            .get(pc_id)
            .await?
            .ok_or(InventoryError::CharacterNotFound(pc_id))?;

        // Create a new item in the same world as the PC
        let validated_name = domain::ItemName::new(item_name.clone())?;
        let mut item = domain::Item::new(pc.world_id(), validated_name);
        if let Some(desc) = item_description {
            item = item.with_description(desc);
        }

        // Save the item
        self.item_repo.save(&item).await?;

        // Add to PC's inventory
        self.pc_repo.add_to_inventory(pc_id, item.id()).await?;

        tracing::info!(
            pc_id = %pc_id,
            item_id = %item.id(),
            item_name = %item_name,
            "Item given to player character"
        );

        Ok(InventoryActionResult {
            item_name,
            quantity: 1,
        })
    }
}
