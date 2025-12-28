//! Challenge system handlers for WebSocket connections.
//!
//! Thin routing layer for challenge operations. Business logic is delegated
//! to ChallengeUseCase where adapters are wired, or to services directly
//! where the ChallengeResolutionPort placeholder is still in use.
//!
//! ## Use Case Delegation
//! - `outcome_decision`, `request_branches`, `select_branch` - fully via use case
//! - `discard_challenge`, `regenerate_outcome` - via use case
//!
//! ## Service Fallback (TODO: migrate when ChallengeResolutionPort is implemented)
//! - `handle_challenge_roll`, `handle_challenge_roll_input` - direct service call
//! - `handle_trigger_challenge`, `handle_challenge_suggestion_decision` - direct service call
//! - `handle_create_adhoc_challenge` - direct service call

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket::converters::{to_adhoc_outcomes_dto, to_service_dice_input};
use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_engine_app::application::services::ChallengeResolutionError;
use wrldbldr_engine_app::application::use_cases::{
    ChallengeOutcomeDecision, DiscardChallengeInput, ErrorCode, OutcomeDecisionInput,
    RegenerateOutcomeInput, RequestBranchesInput, SelectBranchInput,
};
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_protocol::ServerMessage;

// =============================================================================
// Context Extraction Helpers
// =============================================================================

/// Extract player context (world_id, pc_id) for player-facing operations
async fn extract_player_context(
    state: &AppState,
    client_id: Uuid,
) -> Result<(WorldId, PlayerCharacterId), ServerMessage> {
    let client_id_str = client_id.to_string();
    let connection = state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
        .ok_or_else(|| error_msg("NOT_CONNECTED", "Connection not found"))?;

    let world_id = connection
        .world_id
        .map(WorldId::from_uuid)
        .ok_or_else(|| error_msg("NO_WORLD", "Not connected to a world"))?;

    let pc_id = connection
        .pc_id
        .map(PlayerCharacterId::from_uuid)
        .ok_or_else(|| error_msg("NO_PC", "No player character selected"))?;

    Ok((world_id, pc_id))
}

/// Extract DM context for DM-only operations
async fn extract_dm_context(state: &AppState, client_id: Uuid) -> Result<UseCaseContext, ServerMessage> {
    let client_id_str = client_id.to_string();
    let connection = state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
        .ok_or_else(|| error_msg("NOT_CONNECTED", "Connection not found"))?;

    let world_id = connection
        .world_id
        .map(WorldId::from_uuid)
        .ok_or_else(|| error_msg("NO_WORLD", "Not connected to a world"))?;

    if !connection.is_dm() {
        return Err(error_msg("NOT_AUTHORIZED", "Only the DM can perform this action"));
    }

    Ok(UseCaseContext {
        world_id,
        user_id: connection.user_id.clone(),
        is_dm: true,
        pc_id: connection.pc_id.map(PlayerCharacterId::from_uuid),
    })
}

fn error_msg(code: &str, message: &str) -> ServerMessage {
    ServerMessage::Error {
        code: code.to_string(),
        message: message.to_string(),
    }
}

/// Convert ChallengeResolutionError to ServerMessage
fn resolution_error_to_msg(e: ChallengeResolutionError) -> ServerMessage {
    let (code, message) = match &e {
        ChallengeResolutionError::InvalidChallengeId(_) => ("INVALID_CHALLENGE_ID", e.to_string()),
        ChallengeResolutionError::ChallengeNotFound(_) => ("CHALLENGE_NOT_FOUND", e.to_string()),
        ChallengeResolutionError::ChallengeLoadFailed(_) => ("CHALLENGE_LOAD_ERROR", e.to_string()),
        ChallengeResolutionError::PlayerCharacterNotFound => ("PLAYER_CHARACTER_NOT_FOUND", e.to_string()),
        ChallengeResolutionError::PlayerCharacterLoadFailed(_) => ("PLAYER_CHARACTER_LOAD_ERROR", e.to_string()),
        ChallengeResolutionError::InvalidDiceFormula(_) => ("INVALID_DICE_FORMULA", e.to_string()),
        ChallengeResolutionError::ApprovalQueueFailed(_) => ("APPROVAL_QUEUE_ERROR", e.to_string()),
        ChallengeResolutionError::ApprovalLookupError(_) => ("APPROVAL_LOOKUP_ERROR", e.to_string()),
        ChallengeResolutionError::ChallengeSuggestionNotFound(_) => ("NO_CHALLENGE_SUGGESTION", e.to_string()),
    };
    ServerMessage::Error {
        code: code.to_string(),
        message,
    }
}

// =============================================================================
// Player Operations (Service Fallback - TODO: migrate to use case)
// =============================================================================

/// Handles a player submitting a dice roll result for an active challenge.
///
/// Returns None on success - the approval service broadcasts to DM and players.
pub async fn handle_challenge_roll(
    state: &AppState,
    client_id: Uuid,
    challenge_id: String,
    roll: i32,
) -> Option<ServerMessage> {
    let (world_id, pc_id) = match extract_player_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    // Service queues for DM approval and broadcasts to DM + player
    match state
        .game
        .challenge_resolution_service
        .handle_roll(&world_id, &pc_id, challenge_id, roll)
        .await
    {
        Ok(_) => None, // Approval service handles broadcasting
        Err(e) => Some(resolution_error_to_msg(e)),
    }
}

/// Handles a player submitting dice input (formula or manual) for a challenge.
///
/// Returns None on success - the approval service broadcasts to DM and players.
pub async fn handle_challenge_roll_input(
    state: &AppState,
    client_id: Uuid,
    challenge_id: String,
    input_type: wrldbldr_protocol::DiceInputType,
) -> Option<ServerMessage> {
    let (world_id, pc_id) = match extract_player_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    // Service queues for DM approval and broadcasts to DM + player
    match state
        .game
        .challenge_resolution_service
        .handle_roll_input(&world_id, &pc_id, challenge_id, to_service_dice_input(input_type))
        .await
    {
        Ok(_) => None, // Approval service handles broadcasting
        Err(e) => Some(resolution_error_to_msg(e)),
    }
}

// =============================================================================
// DM Operations (Service Fallback - TODO: migrate to use case)
// =============================================================================

/// Handles a DM triggering a challenge against a target character.
///
/// Returns the ChallengePrompt message for broadcasting to the world.
pub async fn handle_trigger_challenge(
    state: &AppState,
    client_id: Uuid,
    challenge_id: String,
    target_character_id: String,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    // Service returns trigger result, we convert to ServerMessage and broadcast
    match state
        .game
        .challenge_resolution_service
        .handle_trigger(&ctx.world_id, challenge_id, target_character_id)
        .await
    {
        Ok(result) => {
            // Convert result to ServerMessage and broadcast to world
            let message = ServerMessage::ChallengePrompt {
                challenge_id: result.challenge_id,
                challenge_name: result.challenge_name,
                skill_name: result.skill_name,
                difficulty_display: result.difficulty_display,
                description: result.description,
                character_modifier: result.character_modifier,
                suggested_dice: Some(result.suggested_dice),
                rule_system_hint: Some(result.rule_system_hint),
            };
            let world_uuid: Uuid = ctx.world_id.into();
            state.world_connection_manager.broadcast_to_world(world_uuid, message).await;
            None
        }
        Err(e) => Some(resolution_error_to_msg(e)),
    }
}

/// Handles a DM's decision on an AI-suggested challenge.
///
/// If approved, broadcasts the ChallengePrompt to the world.
pub async fn handle_challenge_suggestion_decision(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    approved: bool,
    modified_difficulty: Option<String>,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    // Service returns optional trigger result (None if rejected)
    match state
        .game
        .challenge_resolution_service
        .handle_suggestion_decision(&ctx.world_id, request_id, approved, modified_difficulty)
        .await
    {
        Ok(Some(result)) => {
            // Approved - broadcast challenge prompt to world
            let message = ServerMessage::ChallengePrompt {
                challenge_id: result.challenge_id,
                challenge_name: result.challenge_name,
                skill_name: result.skill_name,
                difficulty_display: result.difficulty_display,
                description: result.description,
                character_modifier: result.character_modifier,
                suggested_dice: Some(result.suggested_dice),
                rule_system_hint: Some(result.rule_system_hint),
            };
            let world_uuid: Uuid = ctx.world_id.into();
            state.world_connection_manager.broadcast_to_world(world_uuid, message).await;
            None
        }
        Ok(None) => None, // Rejected - nothing to broadcast
        Err(e) => Some(resolution_error_to_msg(e)),
    }
}

/// Handles a DM creating an ad-hoc challenge on the fly.
///
/// Broadcasts ChallengePrompt to world and returns AdHocChallengeCreated to DM.
pub async fn handle_create_adhoc_challenge(
    state: &AppState,
    client_id: Uuid,
    challenge_name: String,
    skill_name: String,
    difficulty: String,
    target_pc_id: String,
    outcomes: wrldbldr_protocol::AdHocOutcomes,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    // Service returns both adhoc result and trigger result
    match state
        .game
        .challenge_resolution_service
        .handle_adhoc_challenge(
            &ctx.world_id,
            challenge_name,
            skill_name,
            difficulty,
            target_pc_id,
            to_adhoc_outcomes_dto(outcomes),
        )
        .await
    {
        Ok((adhoc_result, trigger_result)) => {
            // Broadcast challenge prompt to world (target player will see it)
            let prompt_message = ServerMessage::ChallengePrompt {
                challenge_id: trigger_result.challenge_id,
                challenge_name: trigger_result.challenge_name,
                skill_name: trigger_result.skill_name,
                difficulty_display: trigger_result.difficulty_display,
                description: trigger_result.description,
                character_modifier: trigger_result.character_modifier,
                suggested_dice: Some(trigger_result.suggested_dice),
                rule_system_hint: Some(trigger_result.rule_system_hint),
            };
            let world_uuid: Uuid = ctx.world_id.into();
            state.world_connection_manager.broadcast_to_world(world_uuid, prompt_message).await;

            // Return AdHocChallengeCreated to DM for confirmation
            Some(ServerMessage::AdHocChallengeCreated {
                challenge_id: adhoc_result.challenge_id,
                challenge_name: adhoc_result.challenge_name,
                target_pc_id: adhoc_result.target_pc_id,
            })
        }
        Err(e) => Some(resolution_error_to_msg(e)),
    }
}

// =============================================================================
// DM Operations (Use Case - Properly Wired)
// =============================================================================

/// Handles a DM's decision on a challenge outcome.
pub async fn handle_challenge_outcome_decision(
    state: &AppState,
    client_id: Uuid,
    resolution_id: String,
    decision: wrldbldr_protocol::ChallengeOutcomeDecisionData,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    let input = OutcomeDecisionInput {
        resolution_id,
        decision: to_use_case_decision(decision),
    };

    match state.use_cases.challenge.outcome_decision(ctx, input).await {
        Ok(_) => None, // Resolution broadcast handled by service
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles a DM requesting AI-generated outcome suggestions.
pub async fn handle_request_outcome_suggestion(
    state: &AppState,
    client_id: Uuid,
    resolution_id: String,
    guidance: Option<String>,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    let input = OutcomeDecisionInput {
        resolution_id,
        decision: ChallengeOutcomeDecision::Suggest { guidance },
    };

    match state.use_cases.challenge.outcome_decision(ctx, input).await {
        Ok(_) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles a DM requesting branching outcome options.
pub async fn handle_request_outcome_branches(
    state: &AppState,
    client_id: Uuid,
    resolution_id: String,
    guidance: Option<String>,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    let input = RequestBranchesInput { resolution_id, guidance };

    match state.use_cases.challenge.request_branches(ctx, input).await {
        Ok(()) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles a DM selecting a specific outcome branch.
pub async fn handle_select_outcome_branch(
    state: &AppState,
    client_id: Uuid,
    resolution_id: String,
    branch_id: String,
    modified_description: Option<String>,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    let input = SelectBranchInput { resolution_id, branch_id, modified_description };

    match state.use_cases.challenge.select_branch(ctx, input).await {
        Ok(()) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles a DM discarding a challenge from the approval queue.
pub async fn handle_discard_challenge(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    feedback: Option<String>,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    let input = DiscardChallengeInput { request_id: request_id.clone(), feedback };

    match state.use_cases.challenge.discard_challenge(ctx, input).await {
        Ok(_) => Some(ServerMessage::ChallengeDiscarded { request_id }),
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles a DM request to regenerate challenge outcome text.
pub async fn handle_regenerate_outcome(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    outcome_type: Option<String>,
    guidance: Option<String>,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    let input = RegenerateOutcomeInput { request_id: request_id.clone(), outcome_type, guidance };

    match state.use_cases.challenge.regenerate_outcome(ctx, input).await {
        Ok(result) => Some(ServerMessage::OutcomeRegenerated {
            request_id,
            outcome_type: result.outcome_type,
            new_outcome: wrldbldr_protocol::OutcomeDetailData {
                flavor_text: result.new_outcome.flavor_text,
                scene_direction: result.new_outcome.scene_direction,
                proposed_tools: Vec::new(), // TODO: Enhance OutcomeDetail in use case
            },
        }),
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Conversion Helpers
// =============================================================================

fn to_use_case_decision(decision: wrldbldr_protocol::ChallengeOutcomeDecisionData) -> ChallengeOutcomeDecision {
    match decision {
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Accept => ChallengeOutcomeDecision::Accept,
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Edit { modified_description } => {
            ChallengeOutcomeDecision::Edit { modified_text: modified_description }
        }
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Suggest { guidance } => {
            ChallengeOutcomeDecision::Suggest { guidance }
        }
    }
}
