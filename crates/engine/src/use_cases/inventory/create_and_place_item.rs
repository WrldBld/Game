//! Create and place item use case.
//!
//! DM action to create a new item and place it in a region.

use std::sync::Arc;
use wrldbldr_domain::{Item, ItemId, RegionId};

use crate::infrastructure::ports::ItemRepo;

use super::error::InventoryError;

/// Create and place item use case.
///
/// Orchestrates: Item creation, region placement.
pub struct CreateAndPlaceItem {
    item_repo: Arc<dyn ItemRepo>,
}

impl CreateAndPlaceItem {
    pub fn new(item_repo: Arc<dyn ItemRepo>) -> Self {
        Self { item_repo }
    }

    /// Execute the create and place item use case.
    ///
    /// Creates a new item and places it in a region.
    ///
    /// # Arguments
    /// * `item` - The item to create
    /// * `region_id` - The region to place the item in
    ///
    /// # Returns
    /// * `Ok(ItemId)` - ID of the created item
    /// * `Err(InventoryError)` - Failed to create/place item
    pub async fn execute(&self, item: Item, region_id: RegionId) -> Result<ItemId, InventoryError> {
        let item_id = item.id;
        let item_name = item.name.as_str();

        // Save the item
        self.item_repo.save(&item).await?;

        // Place in the region
        self.item_repo.place_in_region(item_id, region_id).await?;

        tracing::info!(
            item_id = %item_id,
            item_name = %item_name,
            region_id = %region_id,
            "Item created and placed in region"
        );

        Ok(item_id)
    }
}
