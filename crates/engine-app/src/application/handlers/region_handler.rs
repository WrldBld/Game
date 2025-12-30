//! Region domain request handlers
//!
//! Handles: Region CRUD, RegionConnections, RegionExits, SpawnPoints

use std::sync::Arc;

use wrldbldr_domain::entities::{Region, RegionConnection, RegionExit};
use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_engine_ports::outbound::RegionCrudPort;
use wrldbldr_protocol::{
    CreateRegionConnectionData, CreateRegionData, ErrorCode, ResponseResult, UpdateRegionData,
};

use super::common::{parse_location_id, parse_region_id, parse_world_id};
use crate::application::services::{LocationService, RegionService};

/// Handle ListRegions request
pub async fn list_regions(
    location_service: &Arc<dyn LocationService>,
    location_id: &str,
) -> ResponseResult {
    let id = match parse_location_id(location_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Use location service to get location with regions
    match location_service.get_location_with_connections(id).await {
        Ok(Some(loc_with_conn)) => {
            let dtos: Vec<serde_json::Value> = loc_with_conn
                .regions
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id.to_string(),
                        "name": r.name,
                        "description": r.description,
                        "is_spawn_point": r.is_spawn_point,
                    })
                })
                .collect();
            ResponseResult::success(dtos)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Location not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetRegion request
pub async fn get_region(
    region_crud: &Arc<dyn RegionCrudPort>,
    region_id: &str,
) -> ResponseResult {
    let id = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match region_crud.get(id).await {
        Ok(Some(region)) => {
            let dto = serde_json::json!({
                "id": region.id.to_string(),
                "location_id": region.location_id.to_string(),
                "name": region.name,
                "description": region.description,
                "backdrop_asset": region.backdrop_asset,
                "atmosphere": region.atmosphere,
                "is_spawn_point": region.is_spawn_point,
                "order": region.order,
            });
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Region not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateRegion request (DM only)
pub async fn create_region(
    location_service: &Arc<dyn LocationService>,
    ctx: &RequestContext,
    location_id: &str,
    data: CreateRegionData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let lid = match parse_location_id(location_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Create region entity
    let mut region = Region::new(lid, data.name).with_description(data.description.unwrap_or_default());
    // Set spawn point if specified
    if data.is_spawn_point.unwrap_or(false) {
        region.is_spawn_point = true;
    }
    match location_service.add_region(lid, region.clone()).await {
        Ok(()) => ResponseResult::success(serde_json::json!({
            "id": region.id.to_string(),
            "name": region.name,
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateRegion request (DM only)
pub async fn update_region(
    region_service: &Arc<dyn RegionService>,
    ctx: &RequestContext,
    region_id: &str,
    data: UpdateRegionData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match region_service
        .update_region(id, data.name, data.description, data.is_spawn_point)
        .await
    {
        Ok(region) => ResponseResult::success(serde_json::json!({
            "id": region.id.to_string(),
            "name": region.name,
            "description": region.description,
            "is_spawn_point": region.is_spawn_point,
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteRegion request (DM only)
pub async fn delete_region(
    region_service: &Arc<dyn RegionService>,
    ctx: &RequestContext,
    region_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match region_service.delete_region(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetRegionConnections request
pub async fn get_region_connections(
    region_service: &Arc<dyn RegionService>,
    region_id: &str,
) -> ResponseResult {
    let id = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match region_service.get_connections(id).await {
        Ok(connections) => {
            let dtos: Vec<serde_json::Value> = connections
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "from_region": c.from_region.to_string(),
                        "to_region": c.to_region.to_string(),
                        "description": c.description,
                        "bidirectional": c.bidirectional,
                        "is_locked": c.is_locked,
                        "lock_description": c.lock_description,
                    })
                })
                .collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateRegionConnection request (DM only)
pub async fn create_region_connection(
    region_service: &Arc<dyn RegionService>,
    ctx: &RequestContext,
    from_id: &str,
    to_id: &str,
    data: CreateRegionConnectionData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let from = match parse_region_id(from_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let to = match parse_region_id(to_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let mut connection = RegionConnection::new(from, to);
    if let Some(desc) = data.description {
        connection = connection.with_description(desc);
    }
    if let Some(false) = data.bidirectional {
        connection = connection.one_way();
    }
    if let Some(true) = data.locked {
        connection.is_locked = true;
    }
    match region_service.create_connection(connection).await {
        Ok(()) => ResponseResult::success(serde_json::json!({
            "from_id": from_id,
            "to_id": to_id,
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteRegionConnection request (DM only)
pub async fn delete_region_connection(
    region_service: &Arc<dyn RegionService>,
    ctx: &RequestContext,
    from_id: &str,
    to_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let from = match parse_region_id(from_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let to = match parse_region_id(to_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match region_service.delete_connection(from, to).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UnlockRegionConnection request (DM only)
pub async fn unlock_region_connection(
    region_service: &Arc<dyn RegionService>,
    ctx: &RequestContext,
    from_id: &str,
    to_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let from = match parse_region_id(from_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let to = match parse_region_id(to_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match region_service.unlock_connection(from, to).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetRegionExits request
pub async fn get_region_exits(
    region_service: &Arc<dyn RegionService>,
    region_id: &str,
) -> ResponseResult {
    let id = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match region_service.get_exits(id).await {
        Ok(exits) => {
            let dtos: Vec<serde_json::Value> = exits
                .iter()
                .map(|exit| {
                    serde_json::json!({
                        "from_region": exit.from_region.to_string(),
                        "to_location": exit.to_location.to_string(),
                        "arrival_region_id": exit.arrival_region_id.to_string(),
                        "description": exit.description,
                        "bidirectional": exit.bidirectional,
                    })
                })
                .collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateRegionExit request (DM only)
pub async fn create_region_exit(
    region_service: &Arc<dyn RegionService>,
    ctx: &RequestContext,
    region_id: &str,
    location_id: &str,
    arrival_region_id: &str,
    description: Option<String>,
    bidirectional: Option<bool>,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let from_region = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let to_location = match parse_location_id(location_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let arrival = match parse_region_id(arrival_region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let mut exit = RegionExit::new(from_region, to_location, arrival);
    if let Some(desc) = description {
        exit = exit.with_description(desc);
    }
    if let Some(false) = bidirectional {
        exit = exit.one_way();
    }

    match region_service.create_exit(exit).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteRegionExit request (DM only)
pub async fn delete_region_exit(
    region_service: &Arc<dyn RegionService>,
    ctx: &RequestContext,
    region_id: &str,
    location_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let from = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let to = match parse_location_id(location_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match region_service.delete_exit(from, to).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle ListSpawnPoints request
pub async fn list_spawn_points(
    region_crud: &Arc<dyn RegionCrudPort>,
    world_id: &str,
) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match region_crud.list_spawn_points(id).await {
        Ok(regions) => {
            let dtos: Vec<serde_json::Value> = regions
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id.to_string(),
                        "location_id": r.location_id.to_string(),
                        "name": r.name,
                        "description": r.description,
                        "is_spawn_point": r.is_spawn_point,
                    })
                })
                .collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle ListRegionNpcs request
pub async fn list_region_npcs(
    region_service: &Arc<dyn RegionService>,
    region_id: &str,
) -> ResponseResult {
    let id = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match region_service.get_region_npcs(id).await {
        Ok(npcs) => {
            let dtos: Vec<serde_json::Value> = npcs
                .iter()
                .map(|(npc, rel_type)| {
                    serde_json::json!({
                        "id": npc.id.to_string(),
                        "name": npc.name,
                        "relationship_type": serde_json::to_value(rel_type).unwrap_or(serde_json::Value::Null),
                    })
                })
                .collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
