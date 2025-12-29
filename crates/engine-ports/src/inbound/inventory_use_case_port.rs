//! Inventory Use Case Port
//!
//! Defines the inbound port interface for inventory operations including
//! equipping, unequipping, dropping, and picking up items.
//!
//! # Responsibilities
//!
//! - Equip items from PC inventory
//! - Unequip currently equipped items
//! - Drop items from inventory into the current region
//! - Pick up items from the current region into inventory
//!
//! # Usage
//!
//! This port is implemented by the `InventoryUseCase` in the application layer
//! and consumed by adapters (e.g., WebSocket handlers) to process inventory
//! operations.

use async_trait::async_trait;
#[cfg(any(test, feature = "testing"))]
use mockall::automock;

use super::use_case_errors::InventoryError;
use super::UseCaseContext;
use crate::outbound::{DropInput, DropResult, EquipInput, EquipResult, PickupInput, PickupResult, UnequipInput, UnequipResult};

// =============================================================================
// Inventory Use Case Port
// =============================================================================

/// Port for inventory use case operations
///
/// Abstracts inventory management operations for hexagonal architecture.
/// Implementations coordinate item state changes between PCs and regions.
///
/// # Error Handling
///
/// All methods return `InventoryError` variants for domain-specific failures:
/// - `PcNotFound` - Player character doesn't exist
/// - `ItemNotFound` - Item doesn't exist in database
/// - `NotInInventory` - Item not in PC's inventory
/// - `AlreadyEquipped` / `NotEquipped` - Invalid equip state
/// - `InsufficientQuantity` - Not enough items to drop
/// - `NoCurrentRegion` - PC not in a region (for drop/pickup)
/// - `AlreadyOwned` - Item already in inventory (for pickup)
/// - `Database` - Persistence layer failure
#[cfg_attr(any(test, feature = "testing"), automock)]
#[async_trait]
pub trait InventoryUseCasePort: Send + Sync {
    /// Equip an item from the PC's inventory
    ///
    /// Marks an inventory item as equipped, making it active for use.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context containing user, world, and session info
    /// * `input` - Contains `pc_id` and `item_id` to equip
    ///
    /// # Returns
    ///
    /// * `Ok(EquipResult)` - Item name on success
    /// * `Err(InventoryError::NotInInventory)` - Item not in PC's inventory
    /// * `Err(InventoryError::AlreadyEquipped)` - Item is already equipped
    async fn equip(
        &self,
        ctx: UseCaseContext,
        input: EquipInput,
    ) -> Result<EquipResult, InventoryError>;

    /// Unequip an item from the PC
    ///
    /// Marks an equipped item as unequipped, returning it to inventory.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context containing user, world, and session info
    /// * `input` - Contains `pc_id` and `item_id` to unequip
    ///
    /// # Returns
    ///
    /// * `Ok(UnequipResult)` - Item name on success
    /// * `Err(InventoryError::NotInInventory)` - Item not in PC's inventory
    /// * `Err(InventoryError::NotEquipped)` - Item is not equipped
    async fn unequip(
        &self,
        ctx: UseCaseContext,
        input: UnequipInput,
    ) -> Result<UnequipResult, InventoryError>;

    /// Drop an item from the PC's inventory into the current region
    ///
    /// Transfers items from PC inventory to the region they're currently in.
    /// Supports partial drops when quantity is less than owned.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context containing user, world, and session info
    /// * `input` - Contains `pc_id`, `item_id`, and `quantity` to drop
    ///
    /// # Returns
    ///
    /// * `Ok(DropResult)` - Item name, quantity dropped, and region ID
    /// * `Err(InventoryError::NotInInventory)` - Item not in PC's inventory
    /// * `Err(InventoryError::InsufficientQuantity)` - Not enough items to drop
    /// * `Err(InventoryError::NoCurrentRegion)` - PC is not in a region
    async fn drop(
        &self,
        ctx: UseCaseContext,
        input: DropInput,
    ) -> Result<DropResult, InventoryError>;

    /// Pick up an item from the current region into the PC's inventory
    ///
    /// Transfers an item from the region to the PC's inventory.
    /// Items picked up are marked as "Found" acquisition method.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context containing user, world, and session info
    /// * `input` - Contains `pc_id` and `item_id` to pick up
    ///
    /// # Returns
    ///
    /// * `Ok(PickupResult)` - Item name on success
    /// * `Err(InventoryError::ItemNotFound)` - Item not in the region
    /// * `Err(InventoryError::AlreadyOwned)` - PC already has this item
    /// * `Err(InventoryError::NoCurrentRegion)` - PC is not in a region
    async fn pickup(
        &self,
        ctx: UseCaseContext,
        input: PickupInput,
    ) -> Result<PickupResult, InventoryError>;
}
