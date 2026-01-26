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
            .ok_or(InventoryError::ItemNotFound(item_id))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{MockItemRepo, RepoError};
    use std::sync::Arc;
    use wrldbldr_domain::{Item, ItemId, ItemName, RegionId, WorldId};

    fn test_item(world_id: WorldId) -> Item {
        Item::new(world_id, ItemName::new("Test Sword").unwrap())
    }

    #[tokio::test]
    async fn when_item_not_found_returns_error() {
        let item_id = ItemId::new();
        let region_id = RegionId::new();

        let mut item_repo = MockItemRepo::new();
        item_repo
            .expect_get()
            .withf(move |id| *id == item_id)
            .returning(|_| Ok(None));

        let use_case = PlaceItemInRegion::new(Arc::new(item_repo));
        let result = use_case.execute(item_id, region_id).await;

        assert!(matches!(result, Err(InventoryError::ItemNotFound(_))));
    }

    #[tokio::test]
    async fn when_valid_input_succeeds() {
        let world_id = WorldId::new();
        let item_id = ItemId::new();
        let region_id = RegionId::new();

        let mut item_repo = MockItemRepo::new();
        let mut item = test_item(world_id);
        item.id = item_id;
        let item_clone = item.clone();
        item_repo
            .expect_get()
            .withf(move |id| *id == item_id)
            .returning(move |_| Ok(Some(item_clone.clone())));
        item_repo
            .expect_place_in_region()
            .withf(move |iid, rid| *iid == item_id && *rid == region_id)
            .returning(|_, _| Ok(()));

        let use_case = PlaceItemInRegion::new(Arc::new(item_repo));
        let result = use_case.execute(item_id, region_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn when_repo_error_propagates() {
        let item_id = ItemId::new();
        let region_id = RegionId::new();

        let mut item_repo = MockItemRepo::new();
        item_repo.expect_get().returning(|_| {
            Err(RepoError::Database {
                operation: "get",
                message: "Database unavailable".to_string(),
            })
        });

        let use_case = PlaceItemInRegion::new(Arc::new(item_repo));
        let result = use_case.execute(item_id, region_id).await;

        assert!(matches!(result, Err(InventoryError::Repo(_))));
    }
}
