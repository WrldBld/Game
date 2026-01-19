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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{MockItemRepo, MockPlayerCharacterRepo, RepoError};
    use chrono::Utc;
    use std::sync::Arc;
    use wrldbldr_domain::{
        CharacterName, LocationId, PlayerCharacter, PlayerCharacterId, RegionId, UserId, WorldId,
    };

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
    async fn when_pc_not_found_returns_error() {
        let pc_id = PlayerCharacterId::new();

        let item_repo = MockItemRepo::new();

        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(|_| Ok(None));

        let use_case = GiveItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case
            .execute(pc_id, "Magic Sword".to_string(), None)
            .await;

        assert!(matches!(result, Err(InventoryError::CharacterNotFound(_))));
    }

    #[tokio::test]
    async fn when_invalid_item_name_returns_error() {
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();

        let item_repo = MockItemRepo::new();

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc = test_pc(world_id).with_id(pc_id);
        let pc_clone = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_clone.clone())));

        let use_case = GiveItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        // Empty name should fail validation
        let result = use_case.execute(pc_id, "".to_string(), None).await;

        assert!(matches!(result, Err(InventoryError::Validation(_))));
    }

    #[tokio::test]
    async fn when_valid_input_succeeds() {
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();

        let mut item_repo = MockItemRepo::new();
        item_repo.expect_save().returning(|_| Ok(()));

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc = test_pc(world_id).with_id(pc_id);
        let pc_clone = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_clone.clone())));
        pc_repo
            .expect_add_to_inventory()
            .withf(move |pid, _| *pid == pc_id)
            .returning(|_, _| Ok(()));

        let use_case = GiveItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case
            .execute(
                pc_id,
                "Magic Sword".to_string(),
                Some("A glowing blade".to_string()),
            )
            .await;

        assert!(result.is_ok());
        let action_result = result.unwrap();
        assert_eq!(action_result.item_name, "Magic Sword");
        assert_eq!(action_result.quantity, 1);
    }

    #[tokio::test]
    async fn when_valid_input_without_description_succeeds() {
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();

        let mut item_repo = MockItemRepo::new();
        item_repo.expect_save().returning(|_| Ok(()));

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc = test_pc(world_id).with_id(pc_id);
        let pc_clone = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_clone.clone())));
        pc_repo
            .expect_add_to_inventory()
            .withf(move |pid, _| *pid == pc_id)
            .returning(|_, _| Ok(()));

        let use_case = GiveItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case
            .execute(pc_id, "Simple Dagger".to_string(), None)
            .await;

        assert!(result.is_ok());
        let action_result = result.unwrap();
        assert_eq!(action_result.item_name, "Simple Dagger");
        assert_eq!(action_result.quantity, 1);
    }

    #[tokio::test]
    async fn when_repo_error_propagates() {
        let pc_id = PlayerCharacterId::new();

        let item_repo = MockItemRepo::new();

        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo.expect_get().returning(|_| {
            Err(RepoError::Database {
                operation: "get",
                message: "Database unavailable".to_string(),
            })
        });

        let use_case = GiveItem::new(Arc::new(item_repo), Arc::new(pc_repo));
        let result = use_case
            .execute(pc_id, "Magic Sword".to_string(), None)
            .await;

        assert!(matches!(result, Err(InventoryError::Repo(_))));
    }
}
