//! Inventory Use Case
//!
//! Handles equipping, unequipping, dropping, and picking up items
//! with proper validation and rollback on errors.
//!
//! # Responsibilities
//!
//! - Validate PC and item existence
//! - Handle inventory item state changes (equip/unequip)
//! - Coordinate item transfers between PC and region
//! - Rollback operations on failures
//! - Broadcast inventory change events

use std::sync::Arc;
use tracing::{info, warn};

use wrldbldr_domain::entities::AcquisitionMethod;
use wrldbldr_domain::{ItemId, PlayerCharacterId, RegionId, WorldId};
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, GameEvent, ItemInfo, PlayerCharacterRepositoryPort, RegionRepositoryPort,
};

use super::errors::InventoryError;

// =============================================================================
// Input/Output Types
// =============================================================================

/// Input for equipping an item
#[derive(Debug, Clone)]
pub struct EquipInput {
    pub pc_id: PlayerCharacterId,
    pub item_id: ItemId,
}

/// Input for unequipping an item
#[derive(Debug, Clone)]
pub struct UnequipInput {
    pub pc_id: PlayerCharacterId,
    pub item_id: ItemId,
}

/// Input for dropping an item
#[derive(Debug, Clone)]
pub struct DropInput {
    pub pc_id: PlayerCharacterId,
    pub item_id: ItemId,
    pub quantity: u32,
}

/// Input for picking up an item
#[derive(Debug, Clone)]
pub struct PickupInput {
    pub pc_id: PlayerCharacterId,
    pub item_id: ItemId,
}

/// Result of equipping an item
#[derive(Debug, Clone)]
pub struct EquipResult {
    pub item_name: String,
}

/// Result of unequipping an item
#[derive(Debug, Clone)]
pub struct UnequipResult {
    pub item_name: String,
}

/// Result of dropping an item
#[derive(Debug, Clone)]
pub struct DropResult {
    pub item_name: String,
    pub quantity: u32,
    pub region_id: RegionId,
}

/// Result of picking up an item
#[derive(Debug, Clone)]
pub struct PickupResult {
    pub item_name: String,
}

// =============================================================================
// Inventory Use Case
// =============================================================================

/// Use case for inventory operations
///
/// Coordinates item equip/unequip/drop/pickup with proper
/// validation and rollback on failures.
pub struct InventoryUseCase {
    pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
    region_repo: Arc<dyn RegionRepositoryPort>,
    broadcast: Arc<dyn BroadcastPort>,
}

impl InventoryUseCase {
    /// Create a new InventoryUseCase with all dependencies
    pub fn new(
        pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
        region_repo: Arc<dyn RegionRepositoryPort>,
        broadcast: Arc<dyn BroadcastPort>,
    ) -> Self {
        Self {
            pc_repo,
            region_repo,
            broadcast,
        }
    }

    /// Equip an item from the PC's inventory
    pub async fn equip(
        &self,
        ctx: UseCaseContext,
        input: EquipInput,
    ) -> Result<EquipResult, InventoryError> {
        // Get the item from inventory
        let item = self
            .pc_repo
            .get_inventory_item(input.pc_id, input.item_id)
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?
            .ok_or(InventoryError::NotInInventory)?;

        // Check if already equipped
        if item.equipped {
            return Err(InventoryError::AlreadyEquipped);
        }

        let item_name = item.item.name.clone();

        // Update the item to be equipped
        self.pc_repo
            .update_inventory_item(
                input.pc_id,
                input.item_id,
                item.quantity, // keep quantity unchanged
                true,          // is_equipped = true
            )
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?;

        // Broadcast equip event
        self.broadcast
            .broadcast(
                ctx.world_id,
                GameEvent::ItemEquipChanged {
                    user_id: ctx.user_id,
                    pc_id: input.pc_id,
                    item: ItemInfo {
                        item_id: input.item_id,
                        name: item_name.clone(),
                    },
                    equipped: true,
                },
            )
            .await;

        info!(
            pc_id = %input.pc_id,
            item_id = %input.item_id,
            item_name = %item_name,
            "Item equipped"
        );

        Ok(EquipResult { item_name })
    }

    /// Unequip an item from the PC
    pub async fn unequip(
        &self,
        ctx: UseCaseContext,
        input: UnequipInput,
    ) -> Result<UnequipResult, InventoryError> {
        // Get the item from inventory
        let item = self
            .pc_repo
            .get_inventory_item(input.pc_id, input.item_id)
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?
            .ok_or(InventoryError::NotInInventory)?;

        // Check if not equipped
        if !item.equipped {
            return Err(InventoryError::NotEquipped);
        }

        let item_name = item.item.name.clone();

        // Update the item to be unequipped
        self.pc_repo
            .update_inventory_item(
                input.pc_id,
                input.item_id,
                item.quantity, // keep quantity unchanged
                false,         // is_equipped = false
            )
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?;

        // Broadcast unequip event
        self.broadcast
            .broadcast(
                ctx.world_id,
                GameEvent::ItemEquipChanged {
                    user_id: ctx.user_id,
                    pc_id: input.pc_id,
                    item: ItemInfo {
                        item_id: input.item_id,
                        name: item_name.clone(),
                    },
                    equipped: false,
                },
            )
            .await;

        info!(
            pc_id = %input.pc_id,
            item_id = %input.item_id,
            item_name = %item_name,
            "Item unequipped"
        );

        Ok(UnequipResult { item_name })
    }

    /// Drop an item from the PC's inventory into the current region
    pub async fn drop(
        &self,
        ctx: UseCaseContext,
        input: DropInput,
    ) -> Result<DropResult, InventoryError> {
        // Get the item from inventory
        let item = self
            .pc_repo
            .get_inventory_item(input.pc_id, input.item_id)
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?
            .ok_or(InventoryError::NotInInventory)?;

        // Get PC's current region
        let pc = self
            .pc_repo
            .get(input.pc_id)
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?
            .ok_or(InventoryError::PcNotFound(input.pc_id))?;

        let current_region_id = pc.current_region_id.ok_or(InventoryError::NoCurrentRegion)?;

        let item_name = item.item.name.clone();
        let dropped_quantity = input.quantity.min(item.quantity);

        // Check if trying to drop more than available
        if input.quantity > item.quantity {
            return Err(InventoryError::InsufficientQuantity {
                needed: input.quantity,
                available: item.quantity,
            });
        }

        // Place the item in the region
        self.region_repo
            .add_item_to_region(current_region_id, input.item_id)
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?;

        // Update or remove from PC inventory
        let remove_result = if dropped_quantity >= item.quantity {
            // Remove the item entirely from inventory
            self.pc_repo
                .remove_inventory_item(input.pc_id, input.item_id)
                .await
        } else {
            // Reduce quantity in inventory
            let new_quantity = item.quantity - dropped_quantity;
            self.pc_repo
                .update_inventory_item(input.pc_id, input.item_id, new_quantity, item.equipped)
                .await
        };

        // Handle failure with rollback
        if let Err(e) = remove_result {
            // Try to undo the region placement
            let _ = self
                .region_repo
                .remove_item_from_region(current_region_id, input.item_id)
                .await;
            return Err(InventoryError::Database(e.to_string()));
        }

        // Broadcast drop event
        self.broadcast
            .broadcast(
                ctx.world_id,
                GameEvent::ItemDropped {
                    user_id: ctx.user_id,
                    pc_id: input.pc_id,
                    item: ItemInfo {
                        item_id: input.item_id,
                        name: item_name.clone(),
                    },
                    quantity: dropped_quantity,
                    region_id: current_region_id,
                },
            )
            .await;

        info!(
            pc_id = %input.pc_id,
            item_id = %input.item_id,
            item_name = %item_name,
            region_id = %current_region_id,
            quantity = dropped_quantity,
            "Item dropped in region"
        );

        Ok(DropResult {
            item_name,
            quantity: dropped_quantity,
            region_id: current_region_id,
        })
    }

    /// Pick up an item from the current region into the PC's inventory
    pub async fn pickup(
        &self,
        ctx: UseCaseContext,
        input: PickupInput,
    ) -> Result<PickupResult, InventoryError> {
        // Get PC's current region
        let pc = self
            .pc_repo
            .get(input.pc_id)
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?
            .ok_or(InventoryError::PcNotFound(input.pc_id))?;

        let current_region_id = pc.current_region_id.ok_or(InventoryError::NoCurrentRegion)?;

        // Get region items to verify item is present
        let region_items = self
            .region_repo
            .get_region_items(current_region_id)
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?;

        let item = region_items
            .iter()
            .find(|i| i.id == input.item_id)
            .ok_or(InventoryError::ItemNotFound(input.item_id))?;

        let item_name = item.name.clone();

        // Check if PC already has this item
        if self
            .pc_repo
            .get_inventory_item(input.pc_id, input.item_id)
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?
            .is_some()
        {
            warn!(
                pc_id = %input.pc_id,
                item_id = %input.item_id,
                "PC already has this item in inventory"
            );
            return Err(InventoryError::AlreadyOwned);
        }

        // Remove from region first
        self.region_repo
            .remove_item_from_region(current_region_id, input.item_id)
            .await
            .map_err(|e| InventoryError::Database(e.to_string()))?;

        // Add to PC inventory
        let add_result = self
            .pc_repo
            .add_inventory_item(
                input.pc_id,
                input.item_id,
                1,     // quantity - items in regions are single instances
                false, // not equipped by default
                Some(AcquisitionMethod::Found),
            )
            .await;

        // Handle failure with rollback
        if let Err(e) = add_result {
            // Rollback: put item back in region
            let _ = self
                .region_repo
                .add_item_to_region(current_region_id, input.item_id)
                .await;
            return Err(InventoryError::Database(e.to_string()));
        }

        // Broadcast pickup event
        self.broadcast
            .broadcast(
                ctx.world_id,
                GameEvent::ItemPickedUp {
                    user_id: ctx.user_id,
                    pc_id: input.pc_id,
                    item: ItemInfo {
                        item_id: input.item_id,
                        name: item_name.clone(),
                    },
                    quantity: 1,
                },
            )
            .await;

        info!(
            pc_id = %input.pc_id,
            item_id = %input.item_id,
            item_name = %item_name,
            region_id = %current_region_id,
            "Item picked up from region"
        );

        Ok(PickupResult { item_name })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_insufficient_quantity_error_message() {
        let err = InventoryError::InsufficientQuantity {
            needed: 5,
            available: 2,
        };
        assert!(err.to_string().contains("need 5"));
        assert!(err.to_string().contains("have 2"));
    }

    #[test]
    fn test_input_types() {
        let pc_id = PlayerCharacterId::from_uuid(Uuid::new_v4());
        let item_id = ItemId::from_uuid(Uuid::new_v4());

        let _ = EquipInput {
            pc_id,
            item_id,
        };

        let _ = DropInput {
            pc_id,
            item_id,
            quantity: 1,
        };
    }
}
