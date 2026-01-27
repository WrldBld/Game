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

    // Parse and validate location_state_id as typed ID if provided
    let validated_location_state_id = match &location_state_id {
        Some(id_str) => match uuid::Uuid::parse_str(id_str) {
            Ok(uuid) => Some(wrldbldr_domain::LocationStateId::from_uuid(uuid)),
            Err(e) => {
                return Some(error_response(
                    ErrorCode::ValidationError,
                    &format!("Invalid location_state_id UUID: {}", e),
                ))
            }
        },
        None => None,
    };

    // Parse and validate region_state_id as typed ID if provided
    let validated_region_state_id = match &region_state_id {
        Some(id_str) => match uuid::Uuid::parse_str(id_str) {
            Ok(uuid) => Some(wrldbldr_domain::RegionStateId::from_uuid(uuid)),
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
        .map(approved_npc_from_protocol)
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
        .map(npc_present_to_protocol)
        .collect();

    let region_id_str = payload.region_id.to_string();
    let visual_state_proto = payload.visual_state.as_ref().map(resolved_visual_state_to_protocol);

    state
        .connections
        .broadcast_to_world(
            world_id,
            ServerMessage::StagingReady {
                region_id: region_id_str.clone(),
                npcs_present: npcs_present_proto,
                visual_state: visual_state_proto.clone(),
            },
        )
        .await;

    // Broadcast VisualStateChanged after staging approval
    // This ensures players already in region get the updated visual state
    // without requiring a scene change
    state
        .connections
        .broadcast_to_world(
            world_id,
            ServerMessage::VisualStateChanged {
                region_id: Some(region_id_str),
                visual_state: visual_state_proto,
            },
        )
        .await;

    None
}

// =============================================================================
// Domain -> Protocol Conversions for Staging
// =============================================================================

fn staged_npc_to_protocol(npc: &crate::use_cases::staging::StagedNpc) -> wrldbldr_shared::StagedNpcInfo {
    wrldbldr_shared::StagedNpcInfo {
        character_id: npc.character_id.to_string(),
        name: npc.name.clone(),
        sprite_asset: npc.sprite_asset.clone(),
        portrait_asset: npc.portrait_asset.clone(),
        is_present: npc.is_present,
        reasoning: npc.reasoning.clone(),
        is_hidden_from_players: npc.is_hidden_from_players,
        mood: npc.mood.clone(),
    }
}

fn approved_npc_from_protocol(
    info: &wrldbldr_shared::ApprovedNpcInfo,
) -> Result<crate::use_cases::staging::ApprovedNpc, String> {
    let character_id = info
        .character_id
        .parse::<uuid::Uuid>()
        .map(wrldbldr_domain::CharacterId::from)
        .map_err(|e| format!("Invalid character ID: {}", e))?;
    Ok(crate::use_cases::staging::ApprovedNpc {
        character_id,
        is_present: info.is_present,
        reasoning: info.reasoning.clone(),
        is_hidden_from_players: info.is_hidden_from_players,
        mood: info.mood.clone(),
    })
}

fn waiting_pc_to_protocol(pc: &crate::use_cases::staging::WaitingPc) -> wrldbldr_shared::WaitingPcInfo {
    wrldbldr_shared::WaitingPcInfo {
        pc_id: pc.pc_id.to_string(),
        pc_name: pc.pc_name.clone(),
        player_id: pc.player_id.clone(),
    }
}

fn npc_present_to_protocol(npc: &crate::use_cases::staging::NpcPresent) -> wrldbldr_shared::NpcPresentInfo {
    wrldbldr_shared::NpcPresentInfo {
        character_id: npc.character_id.to_string(),
        name: npc.name.clone(),
        sprite_asset: npc.sprite_asset.clone(),
        portrait_asset: npc.portrait_asset.clone(),
        is_hidden_from_players: npc.is_hidden_from_players,
        mood: npc.mood.clone(),
    }
}

fn previous_staging_to_protocol(
    data: &crate::use_cases::staging::PreviousStagingData,
) -> wrldbldr_shared::PreviousStagingInfo {
    wrldbldr_shared::PreviousStagingInfo {
        staging_id: data.staging_id.to_string(),
        approved_at: data.approved_at.to_rfc3339(),
        npcs: data.npcs.iter().map(staged_npc_to_protocol).collect(),
    }
}

fn game_time_to_protocol(gt: &crate::use_cases::staging::GameTimeData) -> wrldbldr_shared::types::GameTime {
    let period = match gt.hour {
        5..=11 => "Morning",
        12..=17 => "Afternoon",
        18..=21 => "Evening",
        _ => "Night",
    }
    .to_string();

    // Format time as 12-hour display
    let am_pm = if gt.hour >= 12 { "PM" } else { "AM" };
    let display_hour = if gt.hour == 0 {
        12
    } else if gt.hour > 12 {
        gt.hour - 12
    } else {
        gt.hour
    };
    let formatted_time = Some(format!("{}:{:02} {}", display_hour, gt.minute, am_pm));

    // Calculate total_seconds from day/hour/minute/second
    let total_seconds = (gt.day as i64 - 1) * 24 * 60 * 60
        + (gt.hour as i64) * 60 * 60
        + (gt.minute as i64) * 60
        + (gt.second as i64);

    wrldbldr_shared::types::GameTime {
        total_seconds,
        day: gt.day,
        hour: gt.hour,
        minute: gt.minute,
        second: gt.second,
        is_paused: gt.is_paused,
        formatted_date: None, // Calendar formatting handled elsewhere
        formatted_time,
        period,
    }
}

fn resolved_state_info_to_protocol(
    info: &crate::use_cases::staging::ResolvedStateInfo,
) -> wrldbldr_shared::types::ResolvedStateInfoData {
    wrldbldr_shared::types::ResolvedStateInfoData {
        id: info.id.clone(),
        name: info.name.clone(),
        backdrop_override: info.backdrop_override.clone(),
        atmosphere_override: info.atmosphere_override.clone(),
        ambient_sound: info.ambient_sound.clone(),
    }
}

fn resolved_visual_state_to_protocol(
    vs: &crate::use_cases::staging::ResolvedVisualState,
) -> wrldbldr_shared::types::ResolvedVisualStateData {
    wrldbldr_shared::types::ResolvedVisualStateData {
        location_state: vs.location_state.as_ref().map(resolved_state_info_to_protocol),
        region_state: vs.region_state.as_ref().map(resolved_state_info_to_protocol),
    }
}

fn state_option_to_protocol(option: &crate::use_cases::staging::StateOption) -> wrldbldr_shared::types::StateOptionData {
    wrldbldr_shared::types::StateOptionData {
        id: option.id.clone(),
        name: option.name.clone(),
        priority: option.priority,
        is_default: option.is_default,
        match_reason: option.match_reason.clone(),
    }
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
        llm_based_npcs.iter().map(staged_npc_to_protocol).collect();

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

    // Parse and validate location_state_id as typed ID if provided
    let validated_location_state_id = match &location_state_id {
        Some(id_str) => match uuid::Uuid::parse_str(id_str) {
            Ok(uuid) => Some(wrldbldr_domain::LocationStateId::from_uuid(uuid)),
            Err(e) => {
                return Some(error_response(
                    ErrorCode::ValidationError,
                    &format!("Invalid location_state_id UUID: {}", e),
                ))
            }
        },
        None => None,
    };

    // Parse and validate region_state_id as typed ID if provided
    let validated_region_state_id = match &region_state_id {
        Some(id_str) => match uuid::Uuid::parse_str(id_str) {
            Ok(uuid) => Some(wrldbldr_domain::RegionStateId::from_uuid(uuid)),
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
        .map(approved_npc_from_protocol)
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
