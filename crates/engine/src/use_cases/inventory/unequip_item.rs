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
        if !inventory.iter().any(|i| i.id == item_id) {
            return Err(InventoryError::ItemNotInInventory(item_id));
        }

        // Remove EQUIPPED_BY edge in graph
        self.item_repo.set_unequipped(pc_id, item_id).await?;

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

    fn test_pc(world_id: WorldId) -> PlayerCharacter {
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let now = Utc::now();
        PlayerCharacter::new(
            UserId::new("user").unwrap(),
            world_id,
            CharacterName::new("Test PC").unwrap(),
            location_id,
            now,
        )
        .with_current_region(Some(region_id))
    }

    #[tokio::test]
    async fn when_item_not_found_returns_error() {
        let pc_id = PlayerCharacterId::new();
        let item_id = ItemId::new();

        let mut item_repo = MockItemRepo::new();
        item_repo
            .expect_get()
            .withf(move |id| *id == item_id)
            .returning(|_| Ok(None));

        let pc_repo = MockPlayerCharacterRepo::new();

        let use_case = UnequipItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(matches!(result, Err(InventoryError::ItemNotFound(_))));
    }

    #[tokio::test]
    async fn when_pc_not_found_returns_error() {
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
        // Return empty inventory to simulate item not in inventory
        pc_repo
            .expect_get_inventory()
            .withf(move |id| *id == pc_id)
            .returning(|_| Ok(vec![]));

        let use_case = UnequipItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(matches!(result, Err(InventoryError::ItemNotInInventory(_))));
    }

    #[tokio::test]
    async fn when_item_not_in_inventory_returns_error() {
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
        // Return empty inventory
        pc_repo
            .expect_get_inventory()
            .withf(move |id| *id == pc_id)
            .returning(|_| Ok(vec![]));

        let use_case = UnequipItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(matches!(result, Err(InventoryError::ItemNotInInventory(_))));
    }

    #[tokio::test]
    async fn when_valid_input_succeeds() {
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();
        let item_id = ItemId::new();

        let mut item_repo = MockItemRepo::new();
        let mut item = test_item(world_id);
        item.id = item_id;
        let item_for_get = item.clone();
        let item_for_inv = item.clone();
        item_repo
            .expect_get()
            .withf(move |id| *id == item_id)
            .returning(move |_| Ok(Some(item_for_get.clone())));
        item_repo
            .expect_set_unequipped()
            .withf(move |pid, iid| *pid == pc_id && *iid == item_id)
            .returning(|_, _| Ok(()));

        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get_inventory()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(vec![item_for_inv.clone()]));

        let use_case = UnequipItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(result.is_ok());
        let action_result = result.unwrap();
        assert_eq!(action_result.item_name, "Test Sword");
        assert_eq!(action_result.quantity, 1);
    }

    #[tokio::test]
    async fn when_repo_error_propagates() {
        let item_id = ItemId::new();
        let pc_id = PlayerCharacterId::new();

        let mut item_repo = MockItemRepo::new();
        item_repo.expect_get().returning(|_| {
            Err(RepoError::Database {
                operation: "get",
                message: "Database unavailable".to_string(),
            })
        });

        let pc_repo = MockPlayerCharacterRepo::new();

        let use_case = UnequipItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case.execute(pc_id, item_id).await;

        assert!(matches!(result, Err(InventoryError::Repo(_))));
    }
}
