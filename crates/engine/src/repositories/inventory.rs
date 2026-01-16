//! Inventory entity CRUD operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, CharacterId, ItemId, PlayerCharacterId, RegionId, WorldId};

use crate::infrastructure::ports::{CharacterRepo, ItemRepo, PlayerCharacterRepo, RepoError};

/// Inventory entity operations.
///
/// Handles items in the game world and character inventories.
pub struct Inventory {
    item_repo: Arc<dyn ItemRepo>,
    character_repo: Arc<dyn CharacterRepo>,
    pc_repo: Arc<dyn PlayerCharacterRepo>,
}

/// Result of an inventory operation.
pub struct InventoryActionResult {
    pub item_name: String,
    pub quantity: u32,
}

impl Inventory {
    pub fn new(
        item_repo: Arc<dyn ItemRepo>,
        character_repo: Arc<dyn CharacterRepo>,
        pc_repo: Arc<dyn PlayerCharacterRepo>,
    ) -> Self {
        Self {
            item_repo,
            character_repo,
            pc_repo,
        }
    }

    // =========================================================================
    // Item CRUD
    // =========================================================================

    pub async fn get(&self, id: ItemId) -> Result<Option<domain::Item>, RepoError> {
        self.item_repo.get(id).await
    }

    pub async fn save(&self, item: &domain::Item) -> Result<(), RepoError> {
        self.item_repo.save(item).await
    }

    /// Delete an item by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete(&self, id: ItemId) -> Result<(), RepoError> {
        self.item_repo.delete(id).await
    }

    pub async fn list_in_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<domain::Item>, RepoError> {
        self.item_repo.list_in_region(region_id).await
    }

    pub async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<domain::Item>, RepoError> {
        self.item_repo.list_in_world(world_id).await
    }

    // =========================================================================
    // Character Inventory Operations
    // =========================================================================

    /// Get the inventory for a player character.
    pub async fn get_pc_inventory(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<domain::Item>, RepoError> {
        self.pc_repo.get_inventory(pc_id).await
    }

    /// Get the inventory for a character (NPC).
    pub async fn get_character_inventory(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<domain::Item>, RepoError> {
        self.character_repo.get_inventory(character_id).await
    }

    /// Equip an item (mark it as equipped in the character's inventory).
    ///
    /// Note: In the graph model, equipping creates an EQUIPPED_BY edge.
    /// For now, this is a simplified implementation.
    ///
    /// Returns the item name for UI feedback.
    pub async fn equip_item(
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

    /// Unequip an item.
    ///
    /// Returns the item name for UI feedback.
    pub async fn unequip_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<InventoryActionResult, InventoryError> {
        // Get the item
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

        // Remove EQUIPPED_BY edge in graph
        self.item_repo.set_unequipped(pc_id, item_id).await?;

        Ok(InventoryActionResult {
            item_name: item.name().to_string(),
            quantity: 1,
        })
    }

    /// Drop an item from inventory (place in current region or destroy).
    ///
    /// Returns the item name for UI feedback.
    pub async fn drop_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
        quantity: u32,
    ) -> Result<InventoryActionResult, InventoryError> {
        // Get the PC to verify they exist and get current region
        let pc = self
            .pc_repo
            .get(pc_id)
            .await?
            .ok_or(InventoryError::CharacterNotFound)?;

        // Get the item
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

        // Get the PC's current region for placing the dropped item
        let current_region = pc.current_region_id().ok_or(InventoryError::NotInRegion)?;

        // Remove POSSESSES edge (remove from inventory)
        self.pc_repo.remove_from_inventory(pc_id, item_id).await?;

        // Also remove EQUIPPED_BY edge if the item was equipped
        self.item_repo.set_unequipped(pc_id, item_id).await?;

        // Place item in the current region (create IN_REGION edge)
        self.item_repo
            .place_in_region(item_id, current_region)
            .await?;

        Ok(InventoryActionResult {
            item_name: item.name().to_string(),
            quantity,
        })
    }

    /// Give a new item to a player character (from challenge outcome).
    ///
    /// Creates a new item with the given name/description and adds it to the PC's inventory.
    /// This is used by the GiveItem trigger in challenge outcomes.
    pub async fn give_item_to_pc(
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
            .ok_or(InventoryError::CharacterNotFound)?;

        // Create a new item in the same world as the PC
        let mut item = domain::Item::new(pc.world_id(), item_name.clone());
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

    /// Pick up an item from the current region.
    ///
    /// Returns the item name for UI feedback.
    pub async fn pickup_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<InventoryActionResult, InventoryError> {
        // Get the PC
        let pc = self
            .pc_repo
            .get(pc_id)
            .await?
            .ok_or(InventoryError::CharacterNotFound)?;

        // Get the item
        let item = self
            .item_repo
            .get(item_id)
            .await?
            .ok_or(InventoryError::ItemNotFound)?;

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

    // =========================================================================
    // Item Placement (DM operations)
    // =========================================================================

    /// Place an existing item in a region (DM action).
    ///
    /// Removes the item from any character's inventory and places it in the region.
    pub async fn place_item_in_region(
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

    /// Create a new item and place it in a region (DM action).
    ///
    /// Returns the created item's ID.
    pub async fn create_and_place_in_region(
        &self,
        item: domain::Item,
        region_id: RegionId,
    ) -> Result<ItemId, InventoryError> {
        let item_id = item.id();
        let item_name = item.name().to_string();

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

/// Errors that can occur during inventory operations.
#[derive(Debug, thiserror::Error)]
pub enum InventoryError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Character not found")]
    CharacterNotFound,
    #[error("Item not in inventory")]
    ItemNotInInventory,
    #[error("Item not in current region")]
    ItemNotInRegion,
    #[error("Character not in a region")]
    NotInRegion,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
