//! Challenge system handlers for WebSocket connections.
//!
//! Thin routing layer for challenge operations. All business logic is delegated
//! to ChallengeUseCase which orchestrates the domain services and broadcasts
//! events via BroadcastPort.
//!
//! ## Handler Pattern
//!
//! All handlers return `Option<ServerMessage>`:
//! - `None` on success: Use case broadcasts events to appropriate recipients
//! - `Some(error)` on failure: Returns error message to the requesting client
//!
//! ## Operations
//!
//! ### Player Operations
//! - `handle_challenge_roll` - Submit a numeric roll result
//! - `handle_challenge_roll_input` - Submit dice formula or manual roll
//!
//! ### DM Operations
//! - `handle_trigger_challenge` - Trigger a challenge against a target
//! - `handle_challenge_suggestion_decision` - Accept/reject AI challenge suggestion
//! - `handle_create_adhoc_challenge` - Create ad-hoc challenge on the fly
//! - `handle_challenge_outcome_decision` - Accept/edit/suggest outcome
//! - `handle_request_outcome_suggestion` - Request AI outcome suggestions
//! - `handle_request_outcome_branches` - Request branching outcome options
//! - `handle_select_outcome_branch` - Select a specific outcome branch
//! - `handle_discard_challenge` - Discard a pending challenge
//! - `handle_regenerate_outcome` - Request outcome text regeneration

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket::IntoServerError;
use wrldbldr_domain::{CharacterId, PlayerCharacterId};
use wrldbldr_engine_ports::inbound::{AdHocOutcomes, DiceInputType, UseCaseContext};
use wrldbldr_engine_ports::outbound::{
    CreateAdHocInput, DiscardChallengeInput, OutcomeDecision, OutcomeDecisionInput,
    RegenerateOutcomeInput, RequestBranchesInput, SelectBranchInput, SubmitDiceInputInput,
    SubmitRollInput, ChallengeSuggestionDecisionInput as SuggestionDecisionInput,
    TriggerChallengeInput,
};
use wrldbldr_protocol::ServerMessage;

use super::common::{error_msg, extract_dm_context, extract_player_context};

// =============================================================================
// Player Operations (Use Case - Properly Wired)
// =============================================================================

/// Handles a player submitting a dice roll result for an active challenge.
///
/// Returns None on success - the use case broadcasts to DM and players.
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

    let ctx = UseCaseContext {
        world_id,
        user_id: client_id.to_string(),
        is_dm: false,
        pc_id: Some(pc_id),
    };

    let input = SubmitRollInput { challenge_id, roll };

    match state.use_cases.challenge.submit_roll(ctx, input).await {
        Ok(_) => None, // Use case broadcasts to DM + players
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles a player submitting dice input (formula or manual) for a challenge.
///
/// Returns None on success - the use case broadcasts to DM and players.
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

    let ctx = UseCaseContext {
        world_id,
        user_id: client_id.to_string(),
        is_dm: false,
        pc_id: Some(pc_id),
    };

    let input = SubmitDiceInputInput {
        challenge_id,
        input_type: to_use_case_dice_input(input_type),
    };

    match state
        .use_cases
        .challenge
        .submit_dice_input(ctx, input)
        .await
    {
        Ok(_) => None, // Use case broadcasts to DM + players
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// DM Operations (Use Case - Properly Wired)
// =============================================================================

/// Handles a DM triggering a challenge against a target character.
///
/// Returns None on success - the use case broadcasts ChallengePrompt to world.
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

    // Parse target_character_id to CharacterId
    let target_id = match uuid::Uuid::parse_str(&target_character_id) {
        Ok(uuid) => CharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(error_msg(
                "INVALID_CHARACTER_ID",
                "Invalid target character ID",
            ))
        }
    };

    let input = TriggerChallengeInput {
        challenge_id,
        target_character_id: target_id,
    };

    match state
        .use_cases
        .challenge
        .trigger_challenge(ctx, input)
        .await
    {
        Ok(_) => None, // Use case broadcasts ChallengePrompt to world
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles a DM's decision on an AI-suggested challenge.
///
/// If approved, the use case broadcasts the ChallengePrompt to the world.
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

    let input = SuggestionDecisionInput {
        request_id,
        approved,
        modified_difficulty,
    };

    match state
        .use_cases
        .challenge
        .suggestion_decision(ctx, input)
        .await
    {
        Ok(()) => None, // Use case handles broadcasting if approved
        Err(e) => Some(e.into_server_error()),
    }
}

/// Handles a DM creating an ad-hoc challenge on the fly.
///
/// Returns AdHocChallengeCreated to DM. The use case broadcasts ChallengePrompt to world.
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

    // Parse target_pc_id to PlayerCharacterId
    let pc_id = match uuid::Uuid::parse_str(&target_pc_id) {
        Ok(uuid) => PlayerCharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(error_msg(
                "INVALID_PC_ID",
                "Invalid target player character ID",
            ))
        }
    };

    let input = CreateAdHocInput {
        challenge_name: challenge_name.clone(),
        skill_name,
        difficulty,
        target_pc_id: pc_id,
        outcomes: to_use_case_adhoc_outcomes(outcomes),
    };

    match state.use_cases.challenge.create_adhoc(ctx, input).await {
        Ok(result) => {
            // Use case broadcasts ChallengePrompt to world
            // Return AdHocChallengeCreated to DM for confirmation
            Some(ServerMessage::AdHocChallengeCreated {
                challenge_id: result.challenge_id,
                challenge_name,
                target_pc_id,
            })
        }
        Err(e) => Some(e.into_server_error()),
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
        decision: OutcomeDecision::Suggest { guidance },
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

    let input = RequestBranchesInput {
        resolution_id,
        guidance,
    };

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

    let input = SelectBranchInput {
        resolution_id,
        branch_id,
        modified_description,
    };

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

    let input = DiscardChallengeInput {
        request_id: request_id.clone(),
        feedback,
    };

    match state
        .use_cases
        .challenge
        .discard_challenge(ctx, input)
        .await
    {
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

    let input = RegenerateOutcomeInput {
        request_id: request_id.clone(),
        outcome_type,
        guidance,
    };

    match state
        .use_cases
        .challenge
        .regenerate_outcome(ctx, input)
        .await
    {
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

fn to_use_case_decision(
    decision: wrldbldr_protocol::ChallengeOutcomeDecisionData,
) -> OutcomeDecision {
    match decision {
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Accept => OutcomeDecision::Accept,
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Edit {
            modified_description,
        } => OutcomeDecision::Edit {
            modified_text: modified_description,
        },
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Suggest { guidance } => {
            OutcomeDecision::Suggest { guidance }
        }
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Unknown => {
            OutcomeDecision::Accept // Default unknown to Accept
        }
    }
}

fn to_use_case_dice_input(input: wrldbldr_protocol::DiceInputType) -> DiceInputType {
    match input {
        wrldbldr_protocol::DiceInputType::Formula(formula) => DiceInputType::Formula(formula),
        wrldbldr_protocol::DiceInputType::Manual(value) => DiceInputType::Manual(value),
        wrldbldr_protocol::DiceInputType::Unknown => DiceInputType::Manual(0), // Default unknown to Manual(0)
    }
}

fn to_use_case_adhoc_outcomes(outcomes: wrldbldr_protocol::AdHocOutcomes) -> AdHocOutcomes {
    AdHocOutcomes {
        critical_success: outcomes.critical_success,
        success: Some(outcomes.success),
        failure: Some(outcomes.failure),
        critical_failure: outcomes.critical_failure,
    }
}
