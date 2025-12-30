//! Location domain request handlers
//!
//! Handles: Location CRUD, Location Connections

use std::sync::Arc;

use wrldbldr_domain::entities::LocationType;
use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_protocol::{
    CreateLocationConnectionData, CreateLocationData, ErrorCode, ResponseResult, UpdateLocationData,
};

use super::common::{parse_location_id, parse_world_id};
use crate::application::dto::{ConnectionResponseDto, LocationResponseDto};
use crate::application::services::{
    CreateConnectionRequest, CreateLocationRequest, LocationService, UpdateLocationRequest,
};

/// Handle ListLocations request
pub async fn list_locations(
    location_service: &Arc<dyn LocationService>,
    world_id: &str,
) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match location_service.list_locations(id).await {
        Ok(locations) => {
            let dtos: Vec<LocationResponseDto> = locations.into_iter().map(|l| l.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetLocation request
pub async fn get_location(
    location_service: &Arc<dyn LocationService>,
    location_id: &str,
) -> ResponseResult {
    let id = match parse_location_id(location_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match location_service.get_location(id).await {
        Ok(Some(location)) => {
            let dto: LocationResponseDto = location.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Location not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteLocation request (DM only)
pub async fn delete_location(
    location_service: &Arc<dyn LocationService>,
    ctx: &RequestContext,
    location_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_location_id(location_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match location_service.delete_location(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateLocation request (DM only)
pub async fn create_location(
    location_service: &Arc<dyn LocationService>,
    ctx: &RequestContext,
    world_id: &str,
    data: CreateLocationData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let request = CreateLocationRequest {
        world_id: id,
        name: data.name,
        description: data.description,
        location_type: LocationType::Interior,
        parent_id: None,
        backdrop_asset: None,
        atmosphere: data.setting,
        presence_cache_ttl_hours: None,
        use_llm_presence: None,
    };
    match location_service.create_location(request).await {
        Ok(location) => {
            let dto: LocationResponseDto = location.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateLocation request (DM only)
pub async fn update_location(
    location_service: &Arc<dyn LocationService>,
    ctx: &RequestContext,
    location_id: &str,
    data: UpdateLocationData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_location_id(location_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let request = UpdateLocationRequest {
        name: data.name,
        description: data.description,
        location_type: None,
        backdrop_asset: None,
        atmosphere: data.setting.map(Some),
        presence_cache_ttl_hours: None,
        use_llm_presence: None,
    };
    match location_service.update_location(id, request).await {
        Ok(location) => {
            let dto: LocationResponseDto = location.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetLocationConnections request
pub async fn get_location_connections(
    location_service: &Arc<dyn LocationService>,
    location_id: &str,
) -> ResponseResult {
    let id = match parse_location_id(location_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match location_service.get_connections(id).await {
        Ok(connections) => {
            let dtos: Vec<ConnectionResponseDto> =
                connections.into_iter().map(|c| c.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateLocationConnection request (DM only)
pub async fn create_location_connection(
    location_service: &Arc<dyn LocationService>,
    ctx: &RequestContext,
    data: CreateLocationConnectionData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let from_id = match parse_location_id(&data.from_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let to_id = match parse_location_id(&data.to_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let request = CreateConnectionRequest {
        from_location: from_id,
        to_location: to_id,
        connection_type: "path".to_string(), // Default connection type
        description: None,
        bidirectional: data.bidirectional.unwrap_or(true),
        travel_time: 1,
        is_locked: false,
        lock_description: None,
    };
    match location_service.create_connection(request).await {
        Ok(()) => ResponseResult::success(serde_json::json!({
            "from_id": data.from_id,
            "to_id": data.to_id,
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteLocationConnection request (DM only)
pub async fn delete_location_connection(
    location_service: &Arc<dyn LocationService>,
    ctx: &RequestContext,
    from_id: &str,
    to_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let fid = match parse_location_id(from_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let tid = match parse_location_id(to_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match location_service.delete_connection(fid, tid).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
