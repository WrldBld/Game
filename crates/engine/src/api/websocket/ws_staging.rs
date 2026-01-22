use wrldbldr_domain::ConnectionId;

use super::*;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use wrldbldr_shared::ErrorCode;

/// Maximum allowed TTL in hours (1 year).
const MAX_TTL_HOURS: i32 = 8760;

/// Maximum guidance length to prevent unbounded text processing.
const MAX_GUIDANCE_LENGTH: usize = 2000;

/// Maximum number of NPCs that can be approved in a single request.
/// Prevents DoS via oversized payloads.
const MAX_APPROVED_NPCS: usize = 100;

/// Validates TTL hours value.
/// Returns an error response if TTL is invalid (negative, zero, or unreasonably large).
fn validate_ttl_hours(ttl_hours: i32) -> Result<(), ServerMessage> {
    if ttl_hours <= 0 {
        return Err(error_response(
            ErrorCode::ValidationError,
            "TTL hours must be a positive value",
        ));
    }
    if ttl_hours > MAX_TTL_HOURS {
        return Err(error_response(
            ErrorCode::ValidationError,
            &format!("TTL hours cannot exceed {} (1 year)", MAX_TTL_HOURS),
        ));
    }
    Ok(())
}

pub(super) async fn handle_staging_approval(
    state: &WsState,
    connection_id: ConnectionId,
    request_id: String,
    approved_npcs: Vec<wrldbldr_shared::ApprovedNpcInfo>,
    ttl_hours: i32,
    source: String,
    location_state_id: Option<String>,
    region_state_id: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can approve staging
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

    // Validate TTL hours
    if let Err(e) = validate_ttl_hours(ttl_hours) {
        return Some(e);
    }

    // Validate input sizes to prevent DoS via oversized payloads
    if approved_npcs.len() > MAX_APPROVED_NPCS {
        return Some(error_response(
            ErrorCode::BadRequest,
            &format!("Too many NPCs (max {})", MAX_APPROVED_NPCS),
        ));
    }

    // request_id is a correlation token; resolve it to a region_id.
    // Check idempotency first to prevent double-approval race condition.
    if state.pending_staging_requests.contains_processed(&request_id) {
        return Some(error_response(
            ErrorCode::Conflict,
            "Staging request already processed",
        ));
    }

    let pending = state.pending_staging_requests.remove_and_mark_processed(&request_id).await;

    let (region_id, location_id) = match pending {
        Some(pending) => (pending.region_id, Some(pending.location_id)),
        None => {
            return Some(error_response(
                ErrorCode::Conflict,
                "Staging request not found or already processed",
            ))
        }
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response(ErrorCode::BadRequest, "World not joined")),
    };

    // Validate location_state_id as UUID if provided
    let validated_location_state_id = match &location_state_id {
        Some(id_str) => match uuid::Uuid::parse_str(id_str) {
            Ok(_) => Some(id_str.clone()),
            Err(e) => {
                return Some(error_response(
                    ErrorCode::ValidationError,
                    &format!("Invalid location_state_id UUID: {}", e),
                ))
            }
        },
        None => None,
    };

    // Validate region_state_id as UUID if provided
    let validated_region_state_id = match &region_state_id {
        Some(id_str) => match uuid::Uuid::parse_str(id_str) {
            Ok(_) => Some(id_str.clone()),
            Err(e) => {
                return Some(error_response(
                    ErrorCode::ValidationError,
                    &format!("Invalid region_state_id UUID: {}", e),
                ))
            }
        },
        None => None,
    };

    // Convert protocol types to domain types
    let domain_approved_npcs: Vec<crate::use_cases::staging::ApprovedNpc> = match approved_npcs
        .iter()
        .map(crate::use_cases::staging::ApprovedNpc::from_protocol)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(npcs) => npcs,
        Err(e) => {
            return Some(error_response(
                ErrorCode::ValidationError,
                &sanitize_repo_error(&e, "validating approved NPCs"),
            ))
        }
    };

    let input = crate::use_cases::staging::ApproveStagingInput {
        region_id,
        location_id,
        world_id,
        approved_by: conn_info.user_id.to_string(),
        ttl_hours,
        source: parse_staging_source(&source),
        approved_npcs: domain_approved_npcs,
        location_state_id: validated_location_state_id,
        region_state_id: validated_region_state_id,
    };

    let payload = match state.app.use_cases.staging.approve.execute(input).await {
        Ok(result) => result,
        Err(crate::use_cases::staging::StagingError::RegionNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("Region not found: {}", id),
            ))
        }
        Err(crate::use_cases::staging::StagingError::CharacterNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("Character not found: {}", id),
            ))
        }
        Err(crate::use_cases::staging::StagingError::WorldNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("World not found: {}", id),
            ))
        }
        Err(crate::use_cases::staging::StagingError::Validation(message)) => {
            return Some(error_response(ErrorCode::ValidationError, &message))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "approving staging"),
            ))
        }
    };

    // Convert domain types to protocol types for the response
    let npcs_present_proto: Vec<wrldbldr_shared::NpcPresentInfo> = payload
        .npcs_present
        .iter()
        .map(|n| n.to_protocol())
        .collect();

    state
        .connections
        .broadcast_to_world(
            world_id,
            ServerMessage::StagingReady {
                region_id: payload.region_id.to_string(),
                npcs_present: npcs_present_proto,
                visual_state: payload.visual_state.map(|vs| vs.to_protocol()),
            },
        )
        .await;

    None
}

pub(super) async fn handle_staging_regenerate(
    state: &WsState,
    connection_id: ConnectionId,
    request_id: String,
    guidance: String,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can regenerate staging
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

    // Validate guidance length
    if guidance.len() > MAX_GUIDANCE_LENGTH {
        return Some(error_response(
            ErrorCode::BadRequest,
            &format!("Guidance too long (max {} chars)", MAX_GUIDANCE_LENGTH),
        ));
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
                ErrorCode::Conflict,
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
        Err(crate::use_cases::staging::StagingError::RegionNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("Region not found: {}", id),
            ))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "regenerating staging"),
            ))
        }
    };

    // Convert domain types to protocol types for the response
    let llm_based_npcs_proto: Vec<wrldbldr_shared::StagedNpcInfo> =
        llm_based_npcs.iter().map(|n| n.to_protocol()).collect();

    Some(ServerMessage::StagingRegenerated {
        request_id,
        llm_based_npcs: llm_based_npcs_proto,
    })
}

pub(super) async fn handle_pre_stage_region(
    state: &WsState,
    connection_id: ConnectionId,
    region_id: String,
    npcs: Vec<wrldbldr_shared::ApprovedNpcInfo>,
    ttl_hours: i32,
    location_state_id: Option<String>,
    region_state_id: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can pre-stage
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

    // Validate TTL hours
    if let Err(e) = validate_ttl_hours(ttl_hours) {
        return Some(e);
    }

    // Validate input sizes to prevent DoS via oversized payloads
    if npcs.len() > MAX_APPROVED_NPCS {
        return Some(error_response(
            ErrorCode::BadRequest,
            &format!("Too many NPCs (max {})", MAX_APPROVED_NPCS),
        ));
    }

    // Parse region ID
    let region_uuid = match parse_region_id(&region_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response(ErrorCode::BadRequest, "World not joined")),
    };

    // Validate location_state_id as UUID if provided
    let validated_location_state_id = match &location_state_id {
        Some(id_str) => match uuid::Uuid::parse_str(id_str) {
            Ok(_) => Some(id_str.clone()),
            Err(e) => {
                return Some(error_response(
                    ErrorCode::ValidationError,
                    &format!("Invalid location_state_id UUID: {}", e),
                ))
            }
        },
        None => None,
    };

    // Validate region_state_id as UUID if provided
    let validated_region_state_id = match &region_state_id {
        Some(id_str) => match uuid::Uuid::parse_str(id_str) {
            Ok(_) => Some(id_str.clone()),
            Err(e) => {
                return Some(error_response(
                    ErrorCode::ValidationError,
                    &format!("Invalid region_state_id UUID: {}", e),
                ))
            }
        },
        None => None,
    };

    // Convert protocol types to domain types
    let domain_approved_npcs: Vec<crate::use_cases::staging::ApprovedNpc> = match npcs
        .iter()
        .map(crate::use_cases::staging::ApprovedNpc::from_protocol)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(npcs) => npcs,
        Err(e) => {
            return Some(error_response(
                ErrorCode::ValidationError,
                &sanitize_repo_error(&e, "validating pre-staged NPCs"),
            ))
        }
    };

    let input = crate::use_cases::staging::ApproveStagingInput {
        region_id: region_uuid,
        location_id: None,
        world_id,
        approved_by: conn_info.user_id.to_string(),
        ttl_hours,
        source: StagingSource::PreStaged,
        approved_npcs: domain_approved_npcs,
        location_state_id: validated_location_state_id,
        region_state_id: validated_region_state_id,
    };

    if let Err(e) = state.app.use_cases.staging.approve.execute(input).await {
        return Some(match e {
            crate::use_cases::staging::StagingError::RegionNotFound(id) => {
                error_response(ErrorCode::NotFound, &format!("Region not found: {}", id))
            }
            crate::use_cases::staging::StagingError::CharacterNotFound(id) => {
                error_response(ErrorCode::NotFound, &format!("Character not found: {}", id))
            }
            crate::use_cases::staging::StagingError::WorldNotFound(id) => {
                error_response(ErrorCode::NotFound, &format!("World not found: {}", id))
            }
            crate::use_cases::staging::StagingError::Validation(message) => {
                error_response(ErrorCode::ValidationError, &message)
            }
            _ => error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "pre-staging region"),
            ),
        });
    }

    None
}
