//! Staging system handlers
//!
//! Thin handlers for DM staging operations.
//! All business logic is delegated to StagingApprovalUseCase.

use uuid::Uuid;

use crate::infrastructure::adapter_state::AdapterState;
use crate::infrastructure::websocket::IntoServerError;
use wrldbldr_domain::{CharacterId, RegionId};
use wrldbldr_engine_ports::outbound::{
    ApproveInput, ApprovedNpcInput, PreStageInput, RegenerateInput, StagingApprovalSource,
};
use wrldbldr_protocol::{ApprovedNpcInfo, ServerMessage, StagedNpcInfo};

use super::common::{error_msg, extract_dm_context_opt};

// =============================================================================
// Staging Approval Response Handler
// =============================================================================

/// Handles a staging approval response from the DM.
///
/// Delegates to `StagingApprovalUseCase::approve` which:
/// 1. Retrieves and validates the pending staging
/// 2. Persists the approved staging
/// 3. Notifies all waiting PCs with SceneChanged
pub async fn handle_staging_approval_response(
    state: &AdapterState,
    client_id: Uuid,
    request_id: String,
    approved_npcs: Vec<ApprovedNpcInfo>,
    ttl_hours: i32,
    source: String,
) -> Option<ServerMessage> {
    tracing::info!(
        request_id = %request_id,
        npc_count = approved_npcs.len(),
        ttl_hours = ttl_hours,
        source = %source,
        "Staging approval response received"
    );

    // Extract context (returns None if not DM)
    let ctx = extract_dm_context_opt(state, client_id).await?;

    // Parse source
    let staging_source = match source.as_str() {
        "rule" => StagingApprovalSource::RuleBased,
        "llm" => StagingApprovalSource::LlmBased,
        _ => StagingApprovalSource::DmCustomized,
    };

    // Parse approved NPCs
    let approved = approved_npcs
        .iter()
        .filter_map(|npc| {
            Uuid::parse_str(&npc.character_id)
                .ok()
                .map(|uuid| ApprovedNpcInput {
                    character_id: CharacterId::from_uuid(uuid),
                    is_present: npc.is_present,
                    is_hidden_from_players: npc.is_hidden_from_players,
                    reasoning: npc.reasoning.clone(),
                })
        })
        .collect();

    let input = ApproveInput {
        request_id,
        approved_npcs: approved,
        ttl_hours,
        source: staging_source,
    };

    match state.app.use_cases.staging.approve(ctx, input).await {
        Ok(_) => None, // No direct response to DM
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Staging Regenerate Request Handler
// =============================================================================

/// Handles a staging regeneration request from the DM.
///
/// Delegates to `StagingApprovalUseCase::regenerate` which:
/// 1. Retrieves the pending staging
/// 2. Regenerates LLM suggestions with guidance
/// 3. Returns new suggestions
pub async fn handle_staging_regenerate_request(
    state: &AdapterState,
    client_id: Uuid,
    request_id: String,
    guidance: String,
) -> Option<ServerMessage> {
    tracing::info!(
        request_id = %request_id,
        guidance = %guidance,
        "Staging regenerate request received"
    );

    // Extract context (returns None if not DM)
    let ctx = extract_dm_context_opt(state, client_id).await?;

    let input = RegenerateInput {
        request_id: request_id.clone(),
        guidance,
    };

    match state.app.use_cases.staging.regenerate(ctx, input).await {
        Ok(result) => {
            let llm_based_npcs: Vec<StagedNpcInfo> = result
                .llm_based_npcs
                .into_iter()
                .map(|npc| StagedNpcInfo {
                    character_id: npc.character_id,
                    name: npc.name,
                    sprite_asset: npc.sprite_asset,
                    portrait_asset: npc.portrait_asset,
                    is_present: npc.is_present,
                    reasoning: npc.reasoning,
                    is_hidden_from_players: npc.is_hidden_from_players,
                })
                .collect();

            Some(ServerMessage::StagingRegenerated {
                request_id,
                llm_based_npcs,
            })
        }
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Pre-Stage Region Handler
// =============================================================================

/// Handles a pre-stage region request from the DM.
///
/// Delegates to `StagingApprovalUseCase::pre_stage` which:
/// 1. Validates the region exists
/// 2. Pre-stages the region with provided NPCs
/// 3. Broadcasts StagingReady to DMs
pub async fn handle_pre_stage_region(
    state: &AdapterState,
    client_id: Uuid,
    region_id: String,
    npcs: Vec<ApprovedNpcInfo>,
    ttl_hours: i32,
) -> Option<ServerMessage> {
    tracing::info!(
        region_id = %region_id,
        npc_count = npcs.len(),
        ttl_hours = ttl_hours,
        "Pre-stage region request received"
    );

    // Extract context (returns None if not DM)
    let ctx = extract_dm_context_opt(state, client_id).await?;

    // Parse region ID
    let region_uuid = match Uuid::parse_str(&region_id) {
        Ok(uuid) => RegionId::from_uuid(uuid),
        Err(_) => {
            return Some(error_msg("INVALID_REGION_ID", "Invalid region ID format"));
        }
    };

    // Parse NPCs
    let approved_npcs = npcs
        .iter()
        .filter_map(|npc| {
            Uuid::parse_str(&npc.character_id)
                .ok()
                .map(|uuid| ApprovedNpcInput {
                    character_id: CharacterId::from_uuid(uuid),
                    is_present: npc.is_present,
                    is_hidden_from_players: npc.is_hidden_from_players,
                    reasoning: npc.reasoning.clone(),
                })
        })
        .collect();

    let input = PreStageInput {
        region_id: region_uuid,
        npcs: approved_npcs,
        ttl_hours,
    };

    match state.app.use_cases.staging.pre_stage(ctx, input).await {
        Ok(_) => None, // Success, no response needed
        Err(e) => Some(e.into_server_error()),
    }
}
