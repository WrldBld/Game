//! Inventory management operations for Character entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{AcquisitionMethod, CharacterId, InventoryItem, ItemId};

/// Inventory management operations for Character entities.
///
/// This trait covers:
/// - Managing POSSESSES edges to Item nodes
/// - Tracking item quantity, equipped status, and acquisition
///
/// # Used By
/// - `InventoryServiceImpl` - For inventory operations
/// - `CharacterServiceImpl` - For character inventory management
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait CharacterInventoryPort: Send + Sync {
    /// Add an item to character's inventory
    async fn add_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
        acquisition_method: Option<AcquisitionMethod>,
    ) -> Result<()>;

    /// Get character's inventory
    async fn get_inventory(&self, character_id: CharacterId) -> Result<Vec<InventoryItem>>;

    /// Get a single inventory item by ID
    async fn get_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<Option<InventoryItem>>;

    /// Update inventory item (quantity, equipped status)
    async fn update_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
        quantity: u32,
        equipped: bool,
    ) -> Result<()>;

    /// Remove an item from inventory
    async fn remove_inventory_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<()>;
}
