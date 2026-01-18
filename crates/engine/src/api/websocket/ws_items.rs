use super::*;

use crate::api::connections::ConnectionInfo;

use wrldbldr_shared::ItemsRequest;

pub(super) async fn handle_items_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: ItemsRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        ItemsRequest::PlaceItemInRegion { region_id, item_id } => {
            require_dm_for_request(conn_info, request_id)?;

            let region_uuid = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let item_uuid = match parse_item_id_for_request(&item_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let place_item = crate::use_cases::inventory::PlaceItemInRegion::new(
                state.app.repositories.item.clone(),
            );
            match place_item.execute(item_uuid, region_uuid).await {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        ItemsRequest::CreateAndPlaceItem {
            world_id,
            region_id,
            data,
        } => {
            require_dm_for_request(conn_info, request_id)?;

            let world_uuid = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let region_uuid = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            // Create the item using the domain builder pattern
            let item_name = match wrldbldr_domain::ItemName::new(data.name) {
                Ok(name) => name,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::ValidationError,
                        format!("Invalid item name: {}", e),
                    ))
                }
            };
            let mut item = wrldbldr_domain::Item::new(world_uuid, item_name);
            if let Some(desc) = data.description {
                item = item.with_description(desc);
            }
            if let Some(item_type) = data.item_type {
                item = item.with_type(item_type);
            }
            if let Some(props) = data.properties {
                item = item.with_properties(serde_json::to_string(&props).unwrap_or_default());
            }

            let create_and_place = crate::use_cases::inventory::CreateAndPlaceItem::new(
                state.app.repositories.item.clone(),
            );
            match create_and_place.execute(item, region_uuid).await {
                Ok(item_id) => Ok(ResponseResult::success(serde_json::json!({
                    "success": true,
                    "item_id": item_id.to_string(),
                }))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
    }
}
