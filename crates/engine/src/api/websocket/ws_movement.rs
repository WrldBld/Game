use super::*;
use crate::use_cases::movement::{EnterRegionError, StagingStatus};

pub(super) async fn handle_move_to_region(
    state: &WsState,
    connection_id: Uuid,
    pc_id: String,
    region_id: String,
) -> Option<ServerMessage> {
    // Parse IDs
    let pc_uuid = match parse_pc_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let region_uuid = match parse_region_id(&region_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get connection info to verify authorization
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Verify the PC belongs to this connection (or is DM)
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response("UNAUTHORIZED", "Cannot control this PC"));
    }

    // Execute movement use case
    match state
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_uuid, region_uuid)
        .await
    {
        Ok(result) => {
            let world_id = result.pc.world_id;

            match result.staging_status {
                StagingStatus::Pending { previous_staging } => {
                    let ctx = crate::use_cases::staging::StagingApprovalContext {
                        connections: &state.connections,
                        pending_time_suggestions: &state.pending_time_suggestions,
                        pending_staging_requests: &state.pending_staging_requests,
                    };
                    let input = crate::use_cases::staging::StagingApprovalInput {
                        world_id,
                        region: result.region.clone(),
                        pc: result.pc.clone(),
                        previous_staging,
                        time_suggestion: result.time_suggestion.clone(),
                        guidance: None,
                    };

                    match state
                        .app
                        .use_cases
                        .staging
                        .request_approval
                        .execute(&ctx, input)
                        .await
                    {
                        Ok(msg) => {
                            state.connections.broadcast_to_dms(world_id, msg).await;

                            maybe_broadcast_time_suggestion(state, world_id, &result.time_suggestion)
                                .await;

                            let pending = ServerMessage::StagingPending {
                                region_id: region_id.clone(),
                                region_name: result.region.name.clone(),
                            };
                            state.connections.send_to_pc(pc_uuid, pending).await;

                            None
                        }
                        Err(e) => Some(error_response("STAGING_ERROR", &e.to_string())),
                    }
                }
                StagingStatus::Ready => {
                    maybe_broadcast_time_suggestion(state, world_id, &result.time_suggestion).await;

                    let scene_change = state
                        .app
                        .use_cases
                        .scene_change
                        .build_scene_change(&result.region, result.npcs, conn_info.is_dm())
                        .await;

                    Some(ServerMessage::SceneChanged {
                        pc_id: pc_id.clone(),
                        region: scene_change.region,
                        npcs_present: scene_change.npcs_present,
                        navigation: scene_change.navigation,
                        region_items: scene_change.region_items,
                    })
                }
            }
        }
        Err(EnterRegionError::RegionNotFound) => Some(error_response("NOT_FOUND", "Region not found")),
        Err(EnterRegionError::PlayerCharacterNotFound) => {
            Some(error_response("NOT_FOUND", "Player character not found"))
        }
        Err(EnterRegionError::WorldNotFound) => Some(error_response("NOT_FOUND", "World not found")),
        Err(EnterRegionError::RegionNotInCurrentLocation) => {
            Some(error_response("INVALID_MOVE", "Region not in current location"))
        }
        Err(EnterRegionError::NoPathToRegion) => Some(ServerMessage::MovementBlocked {
            pc_id,
            reason: "No path to region".to_string(),
        }),
        Err(EnterRegionError::MovementBlocked(reason)) => {
            Some(ServerMessage::MovementBlocked { pc_id, reason })
        }
        Err(e) => Some(error_response("MOVE_ERROR", &e.to_string())),
    }
}

pub(super) async fn handle_exit_to_location(
    state: &WsState,
    connection_id: Uuid,
    pc_id: String,
    location_id: String,
    arrival_region_id: Option<String>,
) -> Option<ServerMessage> {
    // Parse IDs
    let pc_uuid = match parse_pc_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let location_uuid = match parse_location_id(&location_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let arrival_region_uuid = match arrival_region_id {
        Some(ref id) => match parse_region_id(id) {
            Ok(r) => Some(r),
            Err(e) => return Some(e),
        },
        None => None,
    };

    // Get connection info to verify authorization
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Verify the PC belongs to this connection (or is DM)
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response("UNAUTHORIZED", "Cannot control this PC"));
    }

    match state
        .app
        .use_cases
        .movement
        .exit_location
        .execute(pc_uuid, location_uuid, arrival_region_uuid)
        .await
    {
        Ok(result) => {
            let world_id = result.pc.world_id;

            match result.staging_status {
                StagingStatus::Pending { previous_staging } => {
                    let ctx = crate::use_cases::staging::StagingApprovalContext {
                        connections: &state.connections,
                        pending_time_suggestions: &state.pending_time_suggestions,
                        pending_staging_requests: &state.pending_staging_requests,
                    };
                    let input = crate::use_cases::staging::StagingApprovalInput {
                        world_id,
                        region: result.region.clone(),
                        pc: result.pc.clone(),
                        previous_staging,
                        time_suggestion: result.time_suggestion.clone(),
                        guidance: None,
                    };

                    match state
                        .app
                        .use_cases
                        .staging
                        .request_approval
                        .execute(&ctx, input)
                        .await
                    {
                        Ok(msg) => {
                            state.connections.broadcast_to_dms(world_id, msg).await;

                            maybe_broadcast_time_suggestion(state, world_id, &result.time_suggestion)
                                .await;

                            let pending = ServerMessage::StagingPending {
                                region_id: result.region.id.to_string(),
                                region_name: result.region.name.clone(),
                            };
                            state.connections.send_to_pc(pc_uuid, pending).await;

                            None
                        }
                        Err(e) => Some(error_response("STAGING_ERROR", &e.to_string())),
                    }
                }
                StagingStatus::Ready => {
                    maybe_broadcast_time_suggestion(state, world_id, &result.time_suggestion).await;

                    let scene_change = state
                        .app
                        .use_cases
                        .scene_change
                        .build_scene_change(&result.region, result.npcs, conn_info.is_dm())
                        .await;

                    Some(ServerMessage::SceneChanged {
                        pc_id: pc_id.clone(),
                        region: scene_change.region,
                        npcs_present: scene_change.npcs_present,
                        navigation: scene_change.navigation,
                        region_items: scene_change.region_items,
                    })
                }
            }
        }
        Err(crate::use_cases::movement::ExitLocationError::LocationNotFound) => {
            Some(error_response("NOT_FOUND", "Location not found"))
        }
        Err(crate::use_cases::movement::ExitLocationError::RegionNotFound) => {
            Some(error_response("NOT_FOUND", "Region not found"))
        }
        Err(crate::use_cases::movement::ExitLocationError::PlayerCharacterNotFound) => {
            Some(error_response("NOT_FOUND", "Player character not found"))
        }
        Err(crate::use_cases::movement::ExitLocationError::RegionLocationMismatch) => {
            Some(error_response("INVALID_MOVE", "Region is not in target location"))
        }
        Err(crate::use_cases::movement::ExitLocationError::WorldNotFound) => {
            Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => Some(error_response("MOVE_ERROR", &e.to_string())),
    }
}

async fn maybe_broadcast_time_suggestion(
    state: &WsState,
    world_id: WorldId,
    time_suggestion: &Option<crate::use_cases::time::TimeSuggestion>,
) {
    if let Some(suggestion) = time_suggestion {
        let msg = ServerMessage::TimeSuggestion {
            data: suggestion.to_protocol(),
        };
        {
            let mut guard = state.pending_time_suggestions.write().await;
            guard.insert(suggestion.id, suggestion.clone());
        }
        state.connections.broadcast_to_dms(world_id, msg).await;
    }
}
