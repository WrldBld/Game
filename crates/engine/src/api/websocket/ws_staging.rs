use super::*;

pub(super) async fn handle_staging_approval(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    approved_npcs: Vec<wrldbldr_protocol::ApprovedNpcInfo>,
    ttl_hours: i32,
    source: String,
    location_state_id: Option<String>,
    region_state_id: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can approve staging
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // request_id is a correlation token; resolve it to a region_id.
    let pending = {
        let mut guard = state.pending_staging_requests.write().await;
        guard.remove(&request_id)
    };

    let (region_id, location_id) = if let Some(pending) = pending {
        (pending.region_id, Some(pending.location_id))
    } else {
        // Check if request_id looks like a UUID (staging request token)
        // If so, it was likely already processed by timeout or another handler
        if uuid::Uuid::parse_str(&request_id).is_ok() {
            return Some(error_response(
                "ALREADY_PROCESSED",
                "Staging request already processed or expired",
            ));
        }
        // Backward-compat: allow request_id to be the region_id.
        let region_id = match parse_region_id(&request_id) {
            Ok(id) => id,
            Err(e) => return Some(e),
        };
        (region_id, None)
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_CONNECTED", "World not joined")),
    };

    let input = crate::use_cases::staging::ApproveStagingInput {
        region_id,
        location_id,
        world_id,
        approved_by: conn_info.user_id.clone(),
        ttl_hours,
        source: parse_staging_source(&source),
        approved_npcs,
        location_state_id,
        region_state_id,
    };

    let payload = match state.app.use_cases.staging.approve.execute(input).await {
        Ok(result) => result,
        Err(crate::use_cases::staging::StagingError::RegionNotFound) => {
            return Some(error_response("NOT_FOUND", "Region not found"))
        }
        Err(crate::use_cases::staging::StagingError::WorldNotFound) => {
            return Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => return Some(error_response("REPO_ERROR", &e.to_string())),
    };

    state
        .connections
        .broadcast_to_world(
            world_id,
            ServerMessage::StagingReady {
                region_id: payload.region_id.to_string(),
                npcs_present: payload.npcs_present,
                visual_state: payload.visual_state,
            },
        )
        .await;

    None
}

pub(super) async fn handle_staging_regenerate(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    guidance: String,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can regenerate staging
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // request_id is a correlation token; resolve it to a region_id.
    let pending = {
        let guard = state.pending_staging_requests.read().await;
        guard.get(&request_id).cloned()
    };

    let region_id = if let Some(pending) = pending {
        pending.region_id
    } else {
        // Check if request_id looks like a UUID (staging request token)
        // If so, it was likely already processed by timeout or another handler
        if uuid::Uuid::parse_str(&request_id).is_ok() {
            return Some(error_response(
                "ALREADY_PROCESSED",
                "Staging request already processed or expired",
            ));
        }
        match parse_region_id(&request_id) {
            Ok(id) => id,
            Err(e) => return Some(e),
        }
    };

    let guidance_opt = if guidance.is_empty() {
        None
    } else {
        Some(guidance.as_str())
    };

    let llm_based_npcs = match state
        .app
        .use_cases
        .staging
        .regenerate
        .execute(region_id, guidance_opt)
        .await
    {
        Ok(npcs) => npcs,
        Err(crate::use_cases::staging::StagingError::RegionNotFound) => {
            return Some(error_response("NOT_FOUND", "Region not found"))
        }
        Err(e) => return Some(error_response("REPO_ERROR", &e.to_string())),
    };

    Some(ServerMessage::StagingRegenerated {
        request_id,
        llm_based_npcs,
    })
}

pub(super) async fn handle_pre_stage_region(
    state: &WsState,
    connection_id: Uuid,
    region_id: String,
    npcs: Vec<wrldbldr_protocol::ApprovedNpcInfo>,
    ttl_hours: i32,
    location_state_id: Option<String>,
    region_state_id: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can pre-stage
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse region ID
    let region_uuid = match parse_region_id(&region_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_CONNECTED", "World not joined")),
    };

    let input = crate::use_cases::staging::ApproveStagingInput {
        region_id: region_uuid,
        location_id: None,
        world_id,
        approved_by: conn_info.user_id.clone(),
        ttl_hours,
        source: StagingSource::PreStaged,
        approved_npcs: npcs,
        location_state_id,
        region_state_id,
    };

    if let Err(e) = state.app.use_cases.staging.approve.execute(input).await {
        return Some(match e {
            crate::use_cases::staging::StagingError::RegionNotFound => {
                error_response("NOT_FOUND", "Region not found")
            }
            crate::use_cases::staging::StagingError::WorldNotFound => {
                error_response("NOT_FOUND", "World not found")
            }
            _ => error_response("REPO_ERROR", &e.to_string()),
        });
    }

    None
}
