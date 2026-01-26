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
        if !items_in_region.iter().any(|i| i.id == item_id) {
            return Err(InventoryError::ItemNotInRegion);
        }

        // Remove IN_REGION edge (item is no longer on the ground)
        self.item_repo.remove_from_region(item_id).await?;

        // Add POSSESSES edge (add to inventory)
        self.pc_repo.add_to_inventory(pc_id, item_id).await?;

        Ok(InventoryActionResult {
            item_name: item.name.as_str().to_string(),
            quantity: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{MockItemRepo, MockPlayerCharacterRepo, RepoError};
    use chrono::Utc;
    use std::sync::Arc;
    use wrldbldr_domain::{
        CharacterName, Item, ItemId, ItemName, LocationId, PlayerCharacter, PlayerCharacterId,
        RegionId, UserId, WorldId,
    };

    fn test_item(world_id: WorldId) -> Item {
        Item::new(world_id, ItemName::new("Test Sword").unwrap())
    }

    fn test_pc(world_id: WorldId, region_id: Option<RegionId>) -> PlayerCharacter {
        let location_id = LocationId::new();
        let now = Utc::now();
        let pc = PlayerCharacter::new(
            UserId::new("user").unwrap(),
            world_id,
            CharacterName::new("Test PC").unwrap(),
            location_id,
            now,
        );
        if let Some(rid) = region_id {
            pc.with_current_region(Some(rid))
        } else {
            pc
        }
    }

    #[tokio::test]
    async fn when_item_not_found_returns_error() {
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();
        let item_id = ItemId::new();
        let region_id = RegionId::new();

        let mut item_repo = MockItemRepo::new();
        item_repo
            .expect_get()
            .withf(move |id| *id == item_id)
            .returning(|_| Ok(None));

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc = test_pc(world_id, Some(region_id)).with_id(pc_id);
        let pc_clone = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_clone.clone())));

        let use_case = PickupItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(matches!(result, Err(InventoryError::ItemNotFound(_))));
    }

    #[tokio::test]
    async fn when_pc_not_found_returns_error() {
        let pc_id = PlayerCharacterId::new();
        let item_id = ItemId::new();

        let item_repo = MockItemRepo::new();

        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(|_| Ok(None));

        let use_case = PickupItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(matches!(result, Err(InventoryError::CharacterNotFound(_))));
    }

    #[tokio::test]
    async fn when_pc_not_in_region_returns_error() {
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();
        let item_id = ItemId::new();

        let mut item_repo = MockItemRepo::new();
        let mut item = test_item(world_id);
        item.id = item_id;
        let item_clone = item.clone();
        item_repo
            .expect_get()
            .withf(move |id| *id == item_id)
            .returning(move |_| Ok(Some(item_clone.clone())));

        let mut pc_repo = MockPlayerCharacterRepo::new();
        // PC with no region
        let pc = test_pc(world_id, None).with_id(pc_id);
        let pc_clone = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_clone.clone())));

        let use_case = PickupItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(matches!(result, Err(InventoryError::NotInRegion)));
    }

    #[tokio::test]
    async fn when_item_not_in_region_returns_error() {
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();
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
        // Return empty list - item is not in the region
        item_repo
            .expect_list_in_region()
            .withf(move |rid| *rid == region_id)
            .returning(|_| Ok(vec![]));

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc = test_pc(world_id, Some(region_id)).with_id(pc_id);
        let pc_clone = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_clone.clone())));

        let use_case = PickupItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(matches!(result, Err(InventoryError::ItemNotInRegion)));
    }

    #[tokio::test]
    async fn when_valid_input_succeeds() {
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();
        let item_id = ItemId::new();
        let region_id = RegionId::new();

        let mut item_repo = MockItemRepo::new();
        let mut item = test_item(world_id);
        item.id = item_id;
        let item_for_get = item.clone();
        let item_for_list = item.clone();
        item_repo
            .expect_get()
            .withf(move |id| *id == item_id)
            .returning(move |_| Ok(Some(item_for_get.clone())));
        item_repo
            .expect_list_in_region()
            .withf(move |rid| *rid == region_id)
            .returning(move |_| Ok(vec![item_for_list.clone()]));
        item_repo
            .expect_remove_from_region()
            .withf(move |id| *id == item_id)
            .returning(|_| Ok(()));

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc = test_pc(world_id, Some(region_id)).with_id(pc_id);
        let pc_clone = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_clone.clone())));
        pc_repo
            .expect_add_to_inventory()
            .withf(move |pid, iid| *pid == pc_id && *iid == item_id)
            .returning(|_, _| Ok(()));

        let use_case = PickupItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(result.is_ok());
        let action_result = result.unwrap();
        assert_eq!(action_result.item_name, "Test Sword");
        assert_eq!(action_result.quantity, 1);
    }

    #[tokio::test]
    async fn when_repo_error_propagates() {
        let pc_id = PlayerCharacterId::new();
        let item_id = ItemId::new();

        let item_repo = MockItemRepo::new();

        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo.expect_get().returning(|_| {
            Err(RepoError::Database {
                operation: "get",
                message: "Database unavailable".to_string(),
            })
        });

        let use_case = PickupItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(matches!(result, Err(InventoryError::Repo(_))));
    }
}
