use super::*;

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
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Verify authorization
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response("UNAUTHORIZED", "Cannot control this PC"));
    }

    // Execute the inventory action
    let result = match action {
        InventoryAction::Equip => state
            .app
            .use_cases
            .inventory
            .actions
            .equip(pc_uuid, item_uuid)
            .await,
        InventoryAction::Unequip => state
            .app
            .use_cases
            .inventory
            .actions
            .unequip(pc_uuid, item_uuid)
            .await,
        InventoryAction::Drop => state
            .app
            .use_cases
            .inventory
            .actions
            .drop_item(pc_uuid, item_uuid, quantity)
            .await,
        InventoryAction::Pickup => state
            .app
            .use_cases
            .inventory
            .actions
            .pickup(pc_uuid, item_uuid)
            .await,
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
            tracing::error!(error = %e, action = ?action, "Inventory action failed");
            Some(error_response("INVENTORY_ERROR", &e.to_string()))
        }
    }
}
