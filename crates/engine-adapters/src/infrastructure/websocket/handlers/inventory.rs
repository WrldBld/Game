//! Inventory handlers
//!
//! Thin handlers for item equip/unequip/drop/pickup operations.
//! All business logic is delegated to InventoryUseCase.

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket::IntoServerError;
use wrldbldr_domain::{ItemId, PlayerCharacterId};
use wrldbldr_engine_app::application::use_cases::InventoryUseCase;
use wrldbldr_engine_ports::outbound::{DropInput, EquipInput, PickupInput, UnequipInput};
use wrldbldr_protocol::ServerMessage;

use super::common::{error_msg, extract_context_opt};

// =============================================================================
// Equip Item Handler
// =============================================================================

/// Handle equipping an item from a player character's inventory.
pub async fn handle_equip_item(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    item_id: String,
) -> Option<ServerMessage> {
    tracing::info!(pc_id = %pc_id, item_id = %item_id, "Equip item request");

    // Extract context from connection (consistent with other handlers)
    let ctx = extract_context_opt(state, client_id).await?;

    // Parse IDs
    let pc_uuid = parse_pc_id(&pc_id)?;
    let item_uuid = parse_item_id(&item_id)?;

    let input = EquipInput {
        pc_id: pc_uuid,
        item_id: item_uuid,
    };

    match state.use_cases.inventory.equip(ctx, input).await {
        Ok(result) => Some(ServerMessage::ItemEquipped {
            pc_id,
            item_id,
            item_name: result.item_name,
        }),
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Unequip Item Handler
// =============================================================================

/// Handle unequipping an item from a player character.
pub async fn handle_unequip_item(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    item_id: String,
) -> Option<ServerMessage> {
    tracing::info!(pc_id = %pc_id, item_id = %item_id, "Unequip item request");

    // Extract context from connection
    let ctx = extract_context_opt(state, client_id).await?;

    // Parse IDs
    let pc_uuid = parse_pc_id(&pc_id)?;
    let item_uuid = parse_item_id(&item_id)?;

    let input = UnequipInput {
        pc_id: pc_uuid,
        item_id: item_uuid,
    };

    match state.use_cases.inventory.unequip(ctx, input).await {
        Ok(result) => Some(ServerMessage::ItemUnequipped {
            pc_id,
            item_id,
            item_name: result.item_name,
        }),
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Drop Item Handler
// =============================================================================

/// Handle dropping an item from a player character's inventory into the current region.
pub async fn handle_drop_item(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    item_id: String,
    quantity: u32,
) -> Option<ServerMessage> {
    tracing::info!(pc_id = %pc_id, item_id = %item_id, quantity = quantity, "Drop item request");

    // Extract context from connection
    let ctx = extract_context_opt(state, client_id).await?;

    // Parse IDs
    let pc_uuid = parse_pc_id(&pc_id)?;
    let item_uuid = parse_item_id(&item_id)?;

    let input = DropInput {
        pc_id: pc_uuid,
        item_id: item_uuid,
        quantity,
    };

    match InventoryUseCase::drop(&*state.use_cases.inventory, ctx, input).await {
        Ok(result) => Some(ServerMessage::ItemDropped {
            pc_id,
            item_id,
            item_name: result.item_name,
            quantity: result.quantity,
        }),
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Pickup Item Handler
// =============================================================================

/// Handle picking up an item from the current region into a player character's inventory.
pub async fn handle_pickup_item(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    item_id: String,
) -> Option<ServerMessage> {
    tracing::info!(pc_id = %pc_id, item_id = %item_id, "Pickup item request");

    // Validate input parameters
    if pc_id.trim().is_empty() {
        tracing::warn!("Empty PC ID provided for pickup request");
        return Some(error_msg("INVALID_PC_ID", "PC ID cannot be empty"));
    }

    if item_id.trim().is_empty() {
        tracing::warn!("Empty item ID provided for pickup request");
        return Some(error_msg("INVALID_ITEM_ID", "Item ID cannot be empty"));
    }

    // Extract context from connection
    let ctx = extract_context_opt(state, client_id).await?;

    // Parse IDs
    let pc_uuid = parse_pc_id(&pc_id)?;
    let item_uuid = parse_item_id(&item_id)?;

    let input = PickupInput {
        pc_id: pc_uuid,
        item_id: item_uuid,
    };

    match state.use_cases.inventory.pickup(ctx, input).await {
        Ok(result) => Some(ServerMessage::ItemPickedUp {
            pc_id,
            item_id,
            item_name: result.item_name,
        }),
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn parse_pc_id(id: &str) -> Option<PlayerCharacterId> {
    Uuid::parse_str(id).ok().map(PlayerCharacterId::from_uuid)
}

fn parse_item_id(id: &str) -> Option<ItemId> {
    Uuid::parse_str(id).ok().map(ItemId::from_uuid)
}
