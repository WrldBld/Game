//! Place item in region use case.
//!
//! DM action to place an existing item in a region.

use std::sync::Arc;
use wrldbldr_domain::{ItemId, RegionId};

use crate::infrastructure::ports::ItemRepo;

use super::error::InventoryError;

/// Place item in region use case.
///
/// Orchestrates: Item validation, region placement.
pub struct PlaceItemInRegion {
    item_repo: Arc<dyn ItemRepo>,
}

impl PlaceItemInRegion {
    pub fn new(item_repo: Arc<dyn ItemRepo>) -> Self {
        Self { item_repo }
    }

    /// Execute the place item in region use case.
    ///
    /// Removes the item from any character's inventory and places it in the region.
    ///
    /// # Arguments
    /// * `item_id` - The item to place
    /// * `region_id` - The region to place the item in
    ///
    /// # Returns
    /// * `Ok(())` - Item placed successfully
    /// * `Err(InventoryError)` - Failed to place item
    pub async fn execute(
        &self,
        item_id: ItemId,
        region_id: RegionId,
    ) -> Result<(), InventoryError> {
        // Verify the item exists
        let _item = self
            .item_repo
            .get(item_id)
            .await?
            .ok_or(InventoryError::ItemNotFound)?;

        // Place item in the region (creates IN_REGION edge)
        self.item_repo.place_in_region(item_id, region_id).await?;

        tracing::info!(
            item_id = %item_id,
            region_id = %region_id,
            "Item placed in region"
        );

        Ok(())
    }
}
