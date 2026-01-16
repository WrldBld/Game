use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use wrldbldr_protocol::{LocationRequest, RegionRequest};

pub(super) async fn handle_location_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: LocationRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        LocationRequest::ListLocations { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .list_locations(world_id_typed)
                .await
            {
                Ok(locations) => {
                    let data: Vec<serde_json::Value> = locations
                        .into_iter()
                        .map(|l| {
                            serde_json::json!({
                                "id": l.id().to_string(),
                                "name": l.name().as_str(),
                                "location_type": format!("{:?}", l.location_type()),
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "list locations"),
                )),
            }
        }

        LocationRequest::GetLocation { location_id } => {
            let location_id_typed = match parse_location_id_for_request(&location_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .get_location(location_id_typed)
                .await
            {
                Ok(Some(location)) => Ok(ResponseResult::success(serde_json::json!({
                    "id": location.id().to_string(),
                    "name": location.name().as_str(),
                    "description": if location.description().is_empty() { None } else { Some(location.description().as_str()) },
                    "location_type": Some(format!("{:?}", location.location_type())),
                    "atmosphere": location.atmosphere(),
                    "backdrop_asset": location.backdrop_asset(),
                    "presence_cache_ttl_hours": location.presence_cache_ttl_hours(),
                }))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Location not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get location"),
                )),
            }
        }

        LocationRequest::CreateLocation { world_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .create_location(world_id_typed, data.name, data.description, data.setting)
                .await
            {
                Ok(location) => Ok(ResponseResult::success(serde_json::json!({
                    "id": location.id().to_string(),
                    "name": location.name().as_str(),
                    "description": if location.description().is_empty() { None } else { Some(location.description().as_str()) },
                    "location_type": Some(format!("{:?}", location.location_type())),
                    "atmosphere": location.atmosphere(),
                    "backdrop_asset": location.backdrop_asset(),
                    "presence_cache_ttl_hours": location.presence_cache_ttl_hours(),
                }))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create location"),
                )),
            }
        }

        LocationRequest::UpdateLocation { location_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let location_id_typed = match parse_location_id_for_request(&location_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .update_location(location_id_typed, data.name, data.description, data.setting)
                .await
            {
                Ok(location) => Ok(ResponseResult::success(serde_json::json!({
                    "id": location.id().to_string(),
                    "name": location.name().as_str(),
                    "description": if location.description().is_empty() { None } else { Some(location.description().as_str()) },
                    "location_type": Some(format!("{:?}", location.location_type())),
                    "atmosphere": location.atmosphere(),
                    "backdrop_asset": location.backdrop_asset(),
                    "presence_cache_ttl_hours": location.presence_cache_ttl_hours(),
                }))),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Location not found"),
                ),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "update location"),
                )),
            }
        }

        LocationRequest::DeleteLocation { location_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let location_id_typed = match parse_location_id_for_request(&location_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .delete_location(location_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Location not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete location"),
                )),
            }
        }

        LocationRequest::GetLocationConnections { location_id } => {
            let location_id_typed = match parse_location_id_for_request(&location_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .list_location_connections(location_id_typed)
                .await
            {
                Ok(connections) => {
                    let data: Vec<serde_json::Value> = connections
                        .into_iter()
                        .map(|c| {
                            serde_json::json!({
                                "from_location_id": c.from_location().to_string(),
                                "to_location_id": c.to_location().to_string(),
                                "connection_type": c.connection_type(),
                                "description": c.description().unwrap_or_default(),
                                "bidirectional": c.bidirectional(),
                                "travel_time": c.travel_time(),
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get location connections"),
                )),
            }
        }

        LocationRequest::CreateLocationConnection { data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let from_id = match parse_location_id_for_request(&data.from_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let to_id = match parse_location_id_for_request(&data.to_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .create_location_connection(from_id, to_id, data.bidirectional.unwrap_or(true))
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create location connection"),
                )),
            }
        }

        LocationRequest::DeleteLocationConnection { from_id, to_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let from_id = match parse_location_id_for_request(&from_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let to_id = match parse_location_id_for_request(&to_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .delete_location_connection(from_id, to_id)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete location connection"),
                )),
            }
        }
    }
}

pub(super) async fn handle_region_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: RegionRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        RegionRequest::ListRegions { location_id } => {
            let location_id_typed = match parse_location_id_for_request(&location_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .list_regions(location_id_typed)
                .await
            {
                Ok(regions) => {
                    let data: Vec<serde_json::Value> = regions
                        .into_iter()
                        .map(|r| {
                            let bounds = r.map_bounds().map(|b| {
                                serde_json::json!({
                                    "x": b.x(),
                                    "y": b.y(),
                                    "width": b.width(),
                                    "height": b.height(),
                                })
                            });
                            serde_json::json!({
                                "id": r.id().to_string(),
                                "location_id": r.location_id().to_string(),
                                "name": r.name(),
                                "description": r.description(),
                                "backdrop_asset": r.backdrop_asset(),
                                "atmosphere": r.atmosphere(),
                                "map_bounds": bounds,
                                "is_spawn_point": r.is_spawn_point(),
                                "order": r.order(),
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "list regions"),
                )),
            }
        }

        RegionRequest::GetRegion { region_id } => {
            let region_id_typed = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .get_region(region_id_typed)
                .await
            {
                Ok(Some(region)) => {
                    let bounds = region.map_bounds().map(|b| {
                        serde_json::json!({
                            "x": b.x(),
                            "y": b.y(),
                            "width": b.width(),
                            "height": b.height(),
                        })
                    });
                    Ok(ResponseResult::success(serde_json::json!({
                        "id": region.id().to_string(),
                        "location_id": region.location_id().to_string(),
                        "name": region.name(),
                        "description": region.description(),
                        "backdrop_asset": region.backdrop_asset(),
                        "atmosphere": region.atmosphere(),
                        "map_bounds": bounds,
                        "is_spawn_point": region.is_spawn_point(),
                        "order": region.order(),
                    })))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Region not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get region"),
                )),
            }
        }

        RegionRequest::CreateRegion { location_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let location_id_typed = match parse_location_id_for_request(&location_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .create_region(
                    location_id_typed,
                    data.name,
                    data.description,
                    data.is_spawn_point,
                )
                .await
            {
                Ok(region) => Ok(ResponseResult::success(serde_json::json!({
                    "id": region.id().to_string(),
                    "location_id": region.location_id().to_string(),
                    "name": region.name(),
                    "description": region.description(),
                    "backdrop_asset": region.backdrop_asset(),
                    "atmosphere": region.atmosphere(),
                    "map_bounds": region.map_bounds().map(|b| serde_json::json!({
                        "x": b.x(),
                        "y": b.y(),
                        "width": b.width(),
                        "height": b.height(),
                    })),
                    "is_spawn_point": region.is_spawn_point(),
                    "order": region.order(),
                }))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create region"),
                )),
            }
        }

        RegionRequest::UpdateRegion { region_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let region_id_typed = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .update_region(
                    region_id_typed,
                    data.name,
                    data.description,
                    data.is_spawn_point,
                )
                .await
            {
                Ok(region) => Ok(ResponseResult::success(serde_json::json!({
                    "id": region.id().to_string(),
                    "location_id": region.location_id().to_string(),
                    "name": region.name(),
                    "description": region.description(),
                    "backdrop_asset": region.backdrop_asset(),
                    "atmosphere": region.atmosphere(),
                    "map_bounds": region.map_bounds().map(|b| serde_json::json!({
                        "x": b.x(),
                        "y": b.y(),
                        "width": b.width(),
                        "height": b.height(),
                    })),
                    "is_spawn_point": region.is_spawn_point(),
                    "order": region.order(),
                }))),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Region not found"),
                ),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "update region"),
                )),
            }
        }

        RegionRequest::DeleteRegion { region_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let region_id_typed = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .delete_region(region_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Region not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete region"),
                )),
            }
        }

        RegionRequest::GetRegionConnections { region_id } => {
            let region_id_typed = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .list_region_connections(region_id_typed)
                .await
            {
                Ok(connections) => {
                    let data: Vec<serde_json::Value> = connections
                        .into_iter()
                        .map(|c| {
                            serde_json::json!({
                                "from_region_id": c.from_region().to_string(),
                                "to_region_id": c.to_region().to_string(),
                                "description": c.description(),
                                "bidirectional": c.bidirectional(),
                                "is_locked": c.is_locked(),
                                "lock_description": c.lock_description(),
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get region connections"),
                )),
            }
        }

        RegionRequest::CreateRegionConnection {
            from_id,
            to_id,
            data,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let from_id = match parse_region_id_for_request(&from_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let to_id = match parse_region_id_for_request(&to_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .create_region_connection(
                    from_id,
                    to_id,
                    data.description,
                    data.bidirectional,
                    data.locked,
                    None,
                )
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create region connection"),
                )),
            }
        }

        RegionRequest::DeleteRegionConnection { from_id, to_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let from_id = match parse_region_id_for_request(&from_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let to_id = match parse_region_id_for_request(&to_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .delete_region_connection(from_id, to_id)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete region connection"),
                )),
            }
        }

        RegionRequest::UnlockRegionConnection { from_id, to_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let from_id = match parse_region_id_for_request(&from_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let to_id = match parse_region_id_for_request(&to_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .unlock_region_connection(from_id, to_id)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Connection not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "unlock region connection"),
                )),
            }
        }

        RegionRequest::GetRegionExits { region_id } => {
            let region_id_typed = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .list_region_exits(region_id_typed)
                .await
            {
                Ok(exits) => {
                    let data: Vec<serde_json::Value> = exits
                        .into_iter()
                        .map(|e| {
                            serde_json::json!({
                                "region_id": e.from_region().to_string(),
                                "location_id": e.to_location().to_string(),
                                "arrival_region_id": e.arrival_region_id().to_string(),
                                "description": e.description(),
                                "bidirectional": e.bidirectional(),
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get region exits"),
                )),
            }
        }

        RegionRequest::CreateRegionExit {
            region_id,
            location_id,
            arrival_region_id,
            description,
            bidirectional,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let region_id_typed = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let location_id_typed = match parse_location_id_for_request(&location_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let arrival_region_id_typed =
                match parse_region_id_for_request(&arrival_region_id, request_id) {
                    Ok(id) => id,
                    Err(e) => return Err(e),
                };

            match state
                .app
                .use_cases
                .management
                .location
                .create_region_exit(
                    region_id_typed,
                    location_id_typed,
                    arrival_region_id_typed,
                    description,
                    bidirectional,
                )
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create region exit"),
                )),
            }
        }

        RegionRequest::DeleteRegionExit {
            region_id,
            location_id,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let region_id_typed = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let location_id_typed = match parse_location_id_for_request(&location_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .delete_region_exit(region_id_typed, location_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete region exit"),
                )),
            }
        }

        RegionRequest::ListSpawnPoints { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .location
                .list_spawn_points(world_id_typed)
                .await
            {
                Ok(spawn_points) => {
                    let data: Vec<serde_json::Value> = spawn_points
                        .into_iter()
                        .map(|r| {
                            serde_json::json!({
                                "id": r.id().to_string(),
                                "location_id": r.location_id().to_string(),
                                "name": r.name(),
                                "description": r.description(),
                                "backdrop_asset": r.backdrop_asset(),
                                "atmosphere": r.atmosphere(),
                                "map_bounds": r.map_bounds().map(|b| serde_json::json!({
                                    "x": b.x(),
                                    "y": b.y(),
                                    "width": b.width(),
                                    "height": b.height(),
                                })),
                                "is_spawn_point": r.is_spawn_point(),
                                "order": r.order(),
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "list spawn points"),
                )),
            }
        }
    }
}
