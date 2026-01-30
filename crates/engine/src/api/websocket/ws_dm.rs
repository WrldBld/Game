use wrldbldr_domain::ConnectionId;

use super::*;

use crate::api::websocket::error_sanitizer::sanitize_repo_error;

pub(super) async fn handle_directorial_update(
    state: &WsState,
    connection_id: ConnectionId,
    context: wrldbldr_shared::DirectorialContext,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can update directorial context
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Must join a world first",
            ))
        }
    };

    let context_store = crate::stores::DirectorialContextStore::new(state.connections.clone());
    let ctx = crate::use_cases::session::DirectorialUpdateContext {
        context_store: &context_store,
    };
    let input = crate::use_cases::session::DirectorialUpdateInput::from_protocol(world_id, context);

    // Store directorial context in per-world cache for LLM prompts.
    state
        .app
        .use_cases
        .session
        .directorial_update
        .execute(&ctx, input)
        .await;

    None
}

pub(super) async fn handle_trigger_approach_event(
    state: &WsState,
    connection_id: ConnectionId,
    npc_id: String,
    target_pc_id: String,
    description: String,
    reveal: bool,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can trigger approach events
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse target PC ID
    let pc_uuid = match parse_pc_id(&target_pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get NPC details
    let npc_uuid = match parse_character_id(&npc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get PC's current location for observation creation
    let pc = match state.app.use_cases.management.player_character.get(pc_uuid).await {
        Ok(Some(pc)) => pc,
        Ok(None) => {
            return Some(error_response(
                ErrorCode::NotFound,
                "Player character not found",
            ))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "get player character"),
            ))
        }
    };

    let location_id = pc.current_location_id();
    let region_id = match pc.current_region_id() {
        Some(id) => id,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Player must be in a region for approach event",
            ))
        }
    };

    let approach = match state
        .app
        .use_cases
        .npc
        .approach_events
        .build_event(npc_uuid, reveal, pc_uuid, location_id, region_id)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "build approach event"),
            ));
        }
    };

    if let Some(err) = approach.lookup_error.as_ref() {
        tracing::error!(error = %err, "Failed to load NPC for approach event");
    }

    if let Some(err) = approach.observation_error.as_ref() {
        tracing::error!(error = %err, "Failed to create observation for approach event");
    }

    // Send approach event to target PC
    let msg = ServerMessage::ApproachEvent {
        npc_id,
        npc_name: approach.npc_name,
        npc_sprite: approach.npc_sprite,
        description,
        reveal,
    };

    state.connections.send_to_pc(pc_uuid, msg).await;
    None
}

pub(super) async fn handle_trigger_location_event(
    state: &WsState,
    connection_id: ConnectionId,
    region_id: String,
    description: String,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can trigger location events
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let region_uuid = match parse_region_id(&region_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let event = match state
        .app
        .use_cases
        .location_events
        .trigger
        .execute(region_uuid, description)
        .await
    {
        Ok(event) => event,
        Err(crate::use_cases::location_events::LocationEventError::RegionNotFound) => {
            return Some(error_response(ErrorCode::NotFound, "Region not found"))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "trigger location event"),
            ));
        }
    };

    // Broadcast location event to all in the world
    if let Some(world_id) = conn_info.world_id {
        let msg = ServerMessage::LocationEvent {
            region_id: event.region_id.to_string(),
            description: event.description,
        };
        state.connections.broadcast_to_world(world_id, msg).await;
    }

    None
}

pub(super) async fn handle_share_npc_location(
    state: &WsState,
    connection_id: ConnectionId,
    pc_id: String,
    npc_id: String,
    location_id: String,
    region_id: String,
    notes: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can share NPC locations
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let pc_uuid = match parse_pc_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let npc_uuid = match parse_character_id(&npc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let location_uuid = match parse_location_id(&location_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let region_uuid = match parse_region_id(&region_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let share_result = match state
        .app
        .use_cases
        .npc
        .location_sharing
        .share_location(pc_uuid, npc_uuid, location_uuid, region_uuid, notes.clone())
        .await
    {
        Ok(result) => result,
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "share NPC location"),
            ));
        }
    };

    if let Some(err) = share_result.observation_error.as_ref() {
        tracing::error!(error = %err, "Failed to save NPC observation");
    }

    // Clone npc_id before moving into NpcLocationShared message
    let npc_id_for_lookup = npc_id.clone();

    // Send to target PC
    let msg = ServerMessage::NpcLocationShared {
        npc_id,
        npc_name: share_result.npc_name,
        region_name: share_result.region_name,
        notes: share_result.notes,
    };

    state.connections.send_to_pc(pc_uuid, msg).await;

    // Also send ObservationUpdated for the shared location
    match state
        .app
        .use_cases
        .management
        .observation
        .list_summaries(pc_uuid)
        .await
    {
        Ok(summaries) => {
            if let Some(summary) = summaries.iter().find(|s| s.npc_id == npc_id_for_lookup) {
                let obs_msg = ServerMessage::ObservationUpdated {
                    pc_id: pc_id.clone(),
                    observation: summary.clone(),
                };
                state.connections.send_to_pc(pc_uuid, obs_msg).await;
            }
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                pc_id = %pc_uuid,
                npc_id = %npc_uuid,
                "Failed to query observation for shared location update"
            );
        }
    }

    None
}
