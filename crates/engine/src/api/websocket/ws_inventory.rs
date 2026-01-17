use super::*;
use wrldbldr_shared::ErrorCode;

use crate::api::websocket::error_sanitizer::sanitize_repo_error;

#[derive(Debug)]
pub(super) enum InventoryAction {
    Equip,
    Unequip,
    Drop,
    Pickup,
}

pub(super) async fn handle_inventory_action(
    state: &WsState,
    connection_id: Uuid,
    action: InventoryAction,
    pc_id: &str,
    item_id: &str,
    quantity: u32,
) -> Option<ServerMessage> {
    // Parse IDs
    let pc_uuid = match parse_pc_id(pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let item_uuid = match parse_item_id(item_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get connection info
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    // Verify authorization
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response(
            ErrorCode::Unauthorized,
            "Cannot control this PC",
        ));
    }

    // Execute the inventory action via use cases (ADR-009)
    let item_repo = state.app.repositories.item.clone();
    let pc_repo = state.app.repositories.player_character.clone();

    let result = match action {
        InventoryAction::Equip => {
            let equip = crate::use_cases::inventory::EquipItem::new(item_repo, pc_repo);
            equip.execute(pc_uuid, item_uuid).await
        }
        InventoryAction::Unequip => {
            let unequip = crate::use_cases::inventory::UnequipItem::new(item_repo, pc_repo);
            unequip.execute(pc_uuid, item_uuid).await
        }
        InventoryAction::Drop => {
            let drop_item = crate::use_cases::inventory::DropItem::new(item_repo, pc_repo);
            drop_item.execute(pc_uuid, item_uuid, quantity).await
        }
        InventoryAction::Pickup => {
            let pickup = crate::use_cases::inventory::PickupItem::new(item_repo, pc_repo);
            pickup.execute(pc_uuid, item_uuid).await
        }
    };

    match result {
        Ok(action_result) => match action {
            InventoryAction::Equip => Some(ServerMessage::ItemEquipped {
                pc_id: pc_id.to_string(),
                item_id: item_id.to_string(),
                item_name: action_result.item_name,
            }),
            InventoryAction::Unequip => Some(ServerMessage::ItemUnequipped {
                pc_id: pc_id.to_string(),
                item_id: item_id.to_string(),
                item_name: action_result.item_name,
            }),
            InventoryAction::Drop => Some(ServerMessage::ItemDropped {
                pc_id: pc_id.to_string(),
                item_id: item_id.to_string(),
                item_name: action_result.item_name,
                quantity: action_result.quantity,
            }),
            InventoryAction::Pickup => Some(ServerMessage::ItemPickedUp {
                pc_id: pc_id.to_string(),
                item_id: item_id.to_string(),
                item_name: action_result.item_name,
            }),
        },
        Err(e) => {
            let action_desc = match action {
                InventoryAction::Equip => "equip item",
                InventoryAction::Unequip => "unequip item",
                InventoryAction::Drop => "drop item",
                InventoryAction::Pickup => "pickup item",
            };
            Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, action_desc),
            ))
        }
    }
}
