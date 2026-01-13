use super::*;

/// Maximum allowed TTL in hours (1 year).
const MAX_TTL_HOURS: i32 = 8760;

/// Validates TTL hours value.
/// Returns an error response if TTL is invalid (negative, zero, or unreasonably large).
fn validate_ttl_hours(ttl_hours: i32) -> Result<(), ServerMessage> {
    if ttl_hours <= 0 {
        return Err(error_response(
            "INVALID_TTL",
            "TTL hours must be a positive value",
        ));
    }
    if ttl_hours > MAX_TTL_HOURS {
        return Err(error_response(
            "INVALID_TTL",
            &format!(
                "TTL hours cannot exceed {} (1 year)",
                MAX_TTL_HOURS
            ),
        ));
    }
    Ok(())
}

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

    // Validate TTL hours
    if let Err(e) = validate_ttl_hours(ttl_hours) {
        return Some(e);
    }

    // request_id is a correlation token; resolve it to a region_id.
    let pending = state.pending_staging_requests.remove(&request_id).await;

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

    // Convert protocol types to domain types
    let domain_approved_npcs: Vec<crate::use_cases::staging::ApprovedNpc> = match approved_npcs
        .iter()
        .map(crate::use_cases::staging::ApprovedNpc::from_protocol)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(npcs) => npcs,
        Err(e) => return Some(error_response("VALIDATION_ERROR", &e.to_string())),
    };

    let input = crate::use_cases::staging::ApproveStagingInput {
        region_id,
        location_id,
        world_id,
        approved_by: conn_info.user_id.clone(),
        ttl_hours,
        source: parse_staging_source(&source),
        approved_npcs: domain_approved_npcs,
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

    // Convert domain types to protocol types for the response
    let npcs_present_proto: Vec<wrldbldr_protocol::NpcPresentInfo> =
        payload.npcs_present.iter().map(|n| n.to_protocol()).collect();

    state
        .connections
        .broadcast_to_world(
            world_id,
            ServerMessage::StagingReady {
                region_id: payload.region_id.to_string(),
                npcs_present: npcs_present_proto,
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
    let pending = state.pending_staging_requests.get(&request_id).await;

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

    // Convert domain types to protocol types for the response
    let llm_based_npcs_proto: Vec<wrldbldr_protocol::StagedNpcInfo> =
        llm_based_npcs.iter().map(|n| n.to_protocol()).collect();

    Some(ServerMessage::StagingRegenerated {
        request_id,
        llm_based_npcs: llm_based_npcs_proto,
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

    // Validate TTL hours
    if let Err(e) = validate_ttl_hours(ttl_hours) {
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

    // Convert protocol types to domain types
    let domain_approved_npcs: Vec<crate::use_cases::staging::ApprovedNpc> = match npcs
        .iter()
        .map(crate::use_cases::staging::ApprovedNpc::from_protocol)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(npcs) => npcs,
        Err(e) => return Some(error_response("VALIDATION_ERROR", &e.to_string())),
    };

    let input = crate::use_cases::staging::ApproveStagingInput {
        region_id: region_uuid,
        location_id: None,
        world_id,
        approved_by: conn_info.user_id.clone(),
        ttl_hours,
        source: StagingSource::PreStaged,
        approved_npcs: domain_approved_npcs,
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
