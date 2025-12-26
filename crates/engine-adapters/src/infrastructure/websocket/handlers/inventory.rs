//! Inventory action handlers for WebSocket messages.
//!
//! Handles equipping, unequipping, dropping, and picking up items
//! with proper validation and rollback on errors.

use crate::infrastructure::state::AppState;
use wrldbldr_engine_ports::outbound::{PlayerCharacterRepositoryPort, RegionRepositoryPort};
use wrldbldr_protocol::ServerMessage;

/// Handle equipping an item from a player character's inventory.
pub async fn handle_equip_item(
    state: &AppState,
    pc_id: String,
    item_id: String,
) -> Option<ServerMessage> {
    tracing::info!(pc_id = %pc_id, item_id = %item_id, "Equip item request");

    let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
        Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_PC_ID".to_string(),
                message: "Invalid PC ID format".to_string(),
            });
        }
    };

    let item_uuid = match uuid::Uuid::parse_str(&item_id) {
        Ok(uuid) => wrldbldr_domain::ItemId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_ITEM_ID".to_string(),
                message: "Invalid item ID format".to_string(),
            });
        }
    };

    // Get the item to find its name and verify ownership
    let item = match state
        .repository
        .player_characters()
        .get_inventory_item(pc_uuid, item_uuid)
        .await
    {
        Ok(Some(item)) => item,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "ITEM_NOT_FOUND".to_string(),
                message: "Item not found in inventory".to_string(),
            });
        }
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to fetch item: {}", e),
            });
        }
    };

    // Update the item to be equipped
    if let Err(e) = state
        .repository
        .player_characters()
        .update_inventory_item(
            pc_uuid,
            item_uuid,
            item.quantity, // keep quantity unchanged
            true,          // is_equipped = true
        )
        .await
    {
        return Some(ServerMessage::Error {
            code: "UPDATE_ERROR".to_string(),
            message: format!("Failed to equip item: {}", e),
        });
    }

    Some(ServerMessage::ItemEquipped {
        pc_id,
        item_id,
        item_name: item.item.name,
    })
}

/// Handle unequipping an item from a player character.
pub async fn handle_unequip_item(
    state: &AppState,
    pc_id: String,
    item_id: String,
) -> Option<ServerMessage> {
    tracing::info!(pc_id = %pc_id, item_id = %item_id, "Unequip item request");

    let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
        Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_PC_ID".to_string(),
                message: "Invalid PC ID format".to_string(),
            });
        }
    };

    let item_uuid = match uuid::Uuid::parse_str(&item_id) {
        Ok(uuid) => wrldbldr_domain::ItemId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_ITEM_ID".to_string(),
                message: "Invalid item ID format".to_string(),
            });
        }
    };

    // Get the item to find its name
    let item = match state
        .repository
        .player_characters()
        .get_inventory_item(pc_uuid, item_uuid)
        .await
    {
        Ok(Some(item)) => item,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "ITEM_NOT_FOUND".to_string(),
                message: "Item not found in inventory".to_string(),
            });
        }
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to fetch item: {}", e),
            });
        }
    };

    // Update the item to be unequipped
    if let Err(e) = state
        .repository
        .player_characters()
        .update_inventory_item(
            pc_uuid,
            item_uuid,
            item.quantity, // keep quantity unchanged
            false,         // is_equipped = false
        )
        .await
    {
        return Some(ServerMessage::Error {
            code: "UPDATE_ERROR".to_string(),
            message: format!("Failed to unequip item: {}", e),
        });
    }

    Some(ServerMessage::ItemUnequipped {
        pc_id,
        item_id,
        item_name: item.item.name,
    })
}

/// Handle dropping an item from a player character's inventory into the current region.
pub async fn handle_drop_item(
    state: &AppState,
    pc_id: String,
    item_id: String,
    quantity: u32,
) -> Option<ServerMessage> {
    tracing::info!(pc_id = %pc_id, item_id = %item_id, quantity = quantity, "Drop item request");

    let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
        Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_PC_ID".to_string(),
                message: "Invalid PC ID format".to_string(),
            });
        }
    };

    let item_uuid = match uuid::Uuid::parse_str(&item_id) {
        Ok(uuid) => wrldbldr_domain::ItemId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_ITEM_ID".to_string(),
                message: "Invalid item ID format".to_string(),
            });
        }
    };

    // Get the item to find its name and current quantity
    let item = match state
        .repository
        .player_characters()
        .get_inventory_item(pc_uuid, item_uuid)
        .await
    {
        Ok(Some(item)) => item,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "ITEM_NOT_FOUND".to_string(),
                message: "Item not found in inventory".to_string(),
            });
        }
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to fetch item: {}", e),
            });
        }
    };

    let item_name = item.item.name.clone();
    let dropped_quantity = quantity.min(item.quantity);

    // Get PC's current region to place the dropped item
    let pc = match state.repository.player_characters().get(pc_uuid).await {
        Ok(Some(pc)) => pc,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "PC_NOT_FOUND".to_string(),
                message: "Player character not found".to_string(),
            });
        }
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to fetch PC: {}", e),
            });
        }
    };

    let current_region_id = match pc.current_region_id {
        Some(region_id) => region_id,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_REGION".to_string(),
                message: "PC is not in a region, cannot drop item".to_string(),
            });
        }
    };

    // Place the item in the region
    if let Err(e) = state
        .repository
        .regions()
        .add_item_to_region(current_region_id, item_uuid)
        .await
    {
        return Some(ServerMessage::Error {
            code: "DROP_ERROR".to_string(),
            message: format!("Failed to place item in region: {}", e),
        });
    }

    // Remove from PC inventory (or reduce quantity)
    if dropped_quantity >= item.quantity {
        // Remove the item entirely from inventory
        if let Err(e) = state
            .repository
            .player_characters()
            .remove_inventory_item(pc_uuid, item_uuid)
            .await
        {
            // Try to undo the region placement
            let _ = state
                .repository
                .regions()
                .remove_item_from_region(current_region_id, item_uuid)
                .await;
            return Some(ServerMessage::Error {
                code: "DELETE_ERROR".to_string(),
                message: format!("Failed to drop item: {}", e),
            });
        }
    } else {
        // Reduce quantity in inventory
        let new_quantity = item.quantity - dropped_quantity;
        if let Err(e) = state
            .repository
            .player_characters()
            .update_inventory_item(
                pc_uuid,
                item_uuid,
                new_quantity,
                item.equipped, // keep equipped status unchanged
            )
            .await
        {
            // Try to undo the region placement
            let _ = state
                .repository
                .regions()
                .remove_item_from_region(current_region_id, item_uuid)
                .await;
            return Some(ServerMessage::Error {
                code: "UPDATE_ERROR".to_string(),
                message: format!("Failed to update item quantity: {}", e),
            });
        }
    }

    tracing::info!(
        pc_id = %pc_id,
        item_id = %item_id,
        item_name = %item_name,
        region_id = %current_region_id,
        quantity = dropped_quantity,
        "Item dropped in region"
    );

    Some(ServerMessage::ItemDropped {
        pc_id,
        item_id,
        item_name,
        quantity: dropped_quantity,
    })
}

/// Handle picking up an item from the current region into a player character's inventory.
pub async fn handle_pickup_item(
    state: &AppState,
    pc_id: String,
    item_id: String,
) -> Option<ServerMessage> {
    tracing::info!(pc_id = %pc_id, item_id = %item_id, "Pickup item request");

    // Validate input parameters
    if pc_id.trim().is_empty() {
        tracing::warn!("Empty PC ID provided for pickup request");
        return Some(ServerMessage::Error {
            code: "INVALID_PC_ID".to_string(),
            message: "PC ID cannot be empty".to_string(),
        });
    }

    if item_id.trim().is_empty() {
        tracing::warn!("Empty item ID provided for pickup request");
        return Some(ServerMessage::Error {
            code: "INVALID_ITEM_ID".to_string(),
            message: "Item ID cannot be empty".to_string(),
        });
    }

    // Parse UUIDs
    let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
        Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
        Err(e) => {
            tracing::warn!(pc_id = %pc_id, error = %e, "Invalid PC ID format for pickup");
            return Some(ServerMessage::Error {
                code: "INVALID_PC_ID".to_string(),
                message: "Invalid PC ID format".to_string(),
            });
        }
    };

    let item_uuid = match uuid::Uuid::parse_str(&item_id) {
        Ok(uuid) => wrldbldr_domain::ItemId::from_uuid(uuid),
        Err(e) => {
            tracing::warn!(item_id = %item_id, error = %e, "Invalid item ID format for pickup");
            return Some(ServerMessage::Error {
                code: "INVALID_ITEM_ID".to_string(),
                message: "Invalid item ID format".to_string(),
            });
        }
    };

    // Get PC's current region
    let pc = match state.repository.player_characters().get(pc_uuid).await {
        Ok(Some(pc)) => pc,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "PC_NOT_FOUND".to_string(),
                message: "Player character not found".to_string(),
            });
        }
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to fetch PC: {}", e),
            });
        }
    };

    let current_region_id = match pc.current_region_id {
        Some(region_id) => region_id,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_REGION".to_string(),
                message: "PC is not in a region, cannot pick up items".to_string(),
            });
        }
    };

    // Get region items to verify item is present and get item details
    let region_items = match state
        .repository
        .regions()
        .get_region_items(current_region_id)
        .await
    {
        Ok(items) => items,
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to fetch region items: {}", e),
            });
        }
    };

    let item = match region_items.iter().find(|i| i.id == item_uuid) {
        Some(item) => item.clone(),
        None => {
            tracing::warn!(
                pc_id = %pc_id,
                item_id = %item_id,
                region_id = %current_region_id,
                available_items = region_items.len(),
                "Attempted to pick up item not in region"
            );
            return Some(ServerMessage::Error {
                code: "ITEM_NOT_IN_REGION".to_string(),
                message: "Item is not in this region".to_string(),
            });
        }
    };

    // Additional validation: Check if PC already has this specific item
    // This prevents edge cases where client and server state are out of sync
    match state
        .repository
        .player_characters()
        .get_inventory_item(pc_uuid, item_uuid)
        .await
    {
        Ok(Some(_existing_item)) => {
            tracing::warn!(
                pc_id = %pc_id,
                item_id = %item_id,
                item_name = %item.name,
                "PC already has this item in inventory, refusing pickup"
            );
            return Some(ServerMessage::Error {
                code: "ITEM_ALREADY_OWNED".to_string(),
                message: "You already have this item in your inventory".to_string(),
            });
        }
        Ok(None) => {
            // Good, PC doesn't have this item
            tracing::debug!(pc_id = %pc_id, item_id = %item_id, "Validated PC doesn't already have item");
        }
        Err(e) => {
            tracing::error!(pc_id = %pc_id, item_id = %item_id, error = %e, "Failed to check PC inventory for duplicate item");
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to validate inventory state: {}", e),
            });
        }
    }

    // Remove from region first (atomic operation)
    if let Err(e) = state
        .repository
        .regions()
        .remove_item_from_region(current_region_id, item_uuid)
        .await
    {
        return Some(ServerMessage::Error {
            code: "PICKUP_ERROR".to_string(),
            message: format!("Failed to remove item from region: {}", e),
        });
    }

    // Add to PC inventory
    if let Err(e) = state
        .repository
        .player_characters()
        .add_inventory_item(
            pc_uuid,
            item_uuid,
            1,     // quantity - items in regions are single instances
            false, // not equipped by default
            Some(wrldbldr_domain::entities::AcquisitionMethod::Found),
        )
        .await
    {
        // Rollback: put item back in region
        let rollback_result = state
            .repository
            .regions()
            .add_item_to_region(current_region_id, item_uuid)
            .await;
        if let Err(rollback_error) = rollback_result {
            tracing::error!(
                original_error = %e,
                rollback_error = %rollback_error,
                "Failed to rollback region placement after inventory error"
            );
        }
        return Some(ServerMessage::Error {
            code: "INVENTORY_ERROR".to_string(),
            message: format!("Failed to add item to inventory: {}", e),
        });
    }

    tracing::info!(
        pc_id = %pc_id,
        item_id = %item_id,
        item_name = %item.name,
        region_id = %current_region_id,
        "Item picked up from region"
    );

    Some(ServerMessage::ItemPickedUp {
        pc_id,
        item_id,
        item_name: item.name,
    })
}
