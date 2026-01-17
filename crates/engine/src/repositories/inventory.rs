// Inventory repository - methods for future item management
#![allow(dead_code)]

//! Inventory entity CRUD operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, CharacterId, ItemId, PlayerCharacterId, RegionId, WorldId};

use crate::infrastructure::ports::{CharacterRepo, ItemRepo, PlayerCharacterRepo, RepoError};

// Re-export from use_cases for backward compatibility during migration
pub use crate::use_cases::inventory::{InventoryActionResult, InventoryError};

/// Inventory entity operations.
///
/// Provides CRUD operations for items and inventory queries.
/// Business logic has been moved to use_cases/inventory/.
pub struct InventoryRepository {
    item_repo: Arc<dyn ItemRepo>,
    character_repo: Arc<dyn CharacterRepo>,
    pc_repo: Arc<dyn PlayerCharacterRepo>,
}

impl InventoryRepository {
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

    /// Get the underlying item repository port.
    pub fn item_port(&self) -> Arc<dyn ItemRepo> {
        self.item_repo.clone()
    }

    /// Get the underlying player character repository port.
    pub fn pc_port(&self) -> Arc<dyn PlayerCharacterRepo> {
        self.pc_repo.clone()
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
    // Inventory Queries
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

    // =========================================================================
    // Deprecated Business Logic Methods
    // These methods are kept for backward compatibility during migration.
    // New code should use the use cases in use_cases/inventory/ directly.
    // =========================================================================

    /// Drop an item from inventory (place in current region or destroy).
    ///
    /// # Deprecated
    /// Use `use_cases::inventory::DropItem` instead.
    #[deprecated(note = "Use use_cases::inventory::DropItem instead")]
    pub async fn drop_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
        quantity: u32,
    ) -> Result<InventoryActionResult, InventoryError> {
        let drop_item = crate::use_cases::inventory::DropItem::new(
            self.item_repo.clone(),
            self.pc_repo.clone(),
        );
        drop_item.execute(pc_id, item_id, quantity).await
    }

    /// Give a new item to a player character (from challenge outcome).
    ///
    /// # Deprecated
    /// Use `use_cases::inventory::GiveItem` instead.
    #[deprecated(note = "Use use_cases::inventory::GiveItem instead")]
    pub async fn give_item_to_pc(
        &self,
        pc_id: PlayerCharacterId,
        item_name: String,
        item_description: Option<String>,
    ) -> Result<InventoryActionResult, InventoryError> {
        let give_item = crate::use_cases::inventory::GiveItem::new(
            self.item_repo.clone(),
            self.pc_repo.clone(),
        );
        give_item.execute(pc_id, item_name, item_description).await
    }

    /// Equip an item (mark it as equipped in the character's inventory).
    ///
    /// # Deprecated
    /// Use `use_cases::inventory::EquipItem` instead.
    #[deprecated(note = "Use use_cases::inventory::EquipItem instead")]
    pub async fn equip_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<InventoryActionResult, InventoryError> {
        let equip_item = crate::use_cases::inventory::EquipItem::new(
            self.item_repo.clone(),
            self.pc_repo.clone(),
        );
        equip_item.execute(pc_id, item_id).await
    }

    /// Unequip an item.
    ///
    /// # Deprecated
    /// Use `use_cases::inventory::UnequipItem` instead.
    #[deprecated(note = "Use use_cases::inventory::UnequipItem instead")]
    pub async fn unequip_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<InventoryActionResult, InventoryError> {
        let unequip_item = crate::use_cases::inventory::UnequipItem::new(
            self.item_repo.clone(),
            self.pc_repo.clone(),
        );
        unequip_item.execute(pc_id, item_id).await
    }

    /// Pick up an item from the current region.
    ///
    /// # Deprecated
    /// Use `use_cases::inventory::PickupItem` instead.
    #[deprecated(note = "Use use_cases::inventory::PickupItem instead")]
    pub async fn pickup_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<InventoryActionResult, InventoryError> {
        let pickup_item = crate::use_cases::inventory::PickupItem::new(
            self.item_repo.clone(),
            self.pc_repo.clone(),
        );
        pickup_item.execute(pc_id, item_id).await
    }

    /// Place an existing item in a region (DM action).
    ///
    /// # Deprecated
    /// Use `use_cases::inventory::PlaceItemInRegion` instead.
    #[deprecated(note = "Use use_cases::inventory::PlaceItemInRegion instead")]
    pub async fn place_item_in_region(
        &self,
        item_id: ItemId,
        region_id: RegionId,
    ) -> Result<(), InventoryError> {
        let place_item =
            crate::use_cases::inventory::PlaceItemInRegion::new(self.item_repo.clone());
        place_item.execute(item_id, region_id).await
    }

    /// Create a new item and place it in a region (DM action).
    ///
    /// # Deprecated
    /// Use `use_cases::inventory::CreateAndPlaceItem` instead.
    #[deprecated(note = "Use use_cases::inventory::CreateAndPlaceItem instead")]
    pub async fn create_and_place_in_region(
        &self,
        item: domain::Item,
        region_id: RegionId,
    ) -> Result<ItemId, InventoryError> {
        let create_and_place =
            crate::use_cases::inventory::CreateAndPlaceItem::new(self.item_repo.clone());
        create_and_place.execute(item, region_id).await
    }
}
