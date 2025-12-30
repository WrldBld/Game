//! Challenge system handlers for WebSocket connections.
//!
//! Thin routing layer for challenge operations. All business logic is delegated
//! to ChallengeUseCase which orchestrates the domain services and broadcasts
//! events via BroadcastPort.

use uuid::Uuid;

use crate::infrastructure::websocket::IntoServerError;
use wrldbldr_domain::{CharacterId, PlayerCharacterId};
use wrldbldr_engine_ports::inbound::AppStatePort;
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_engine_ports::outbound::{
    ChallengeSuggestionDecisionInput as SuggestionDecisionInput, CreateAdHocInput,
    DiscardChallengeInput, OutcomeDecision, OutcomeDecisionInput, RegenerateOutcomeInput,
    RequestBranchesInput, SelectBranchInput, SubmitDiceInputInput, SubmitRollInput,
    TriggerChallengeInput,
};
use wrldbldr_protocol::ServerMessage;

use super::challenge_converters::{
    to_use_case_adhoc_outcomes, to_use_case_decision, to_use_case_dice_input,
};
use super::common::{error_msg, extract_dm_context, extract_player_context};

// --- Player Operations ---

/// Submit dice roll for active challenge. Returns None on success (use case broadcasts).
pub async fn handle_challenge_roll(
    state: &dyn AppStatePort,
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

    match state.challenge_use_case().submit_roll(ctx, input).await {
        Ok(_) => None, // Use case broadcasts to DM + players
        Err(e) => Some(e.into_server_error()),
    }
}

/// Submit dice input (formula or manual). Returns None on success (use case broadcasts).
pub async fn handle_challenge_roll_input(
    state: &dyn AppStatePort,
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
        .challenge_use_case()
        .submit_dice_input(ctx, input)
        .await
    {
        Ok(_) => None, // Use case broadcasts to DM + players
        Err(e) => Some(e.into_server_error()),
    }
}

// --- DM Operations ---

/// Trigger challenge against target. Returns None on success (use case broadcasts).
pub async fn handle_trigger_challenge(
    state: &dyn AppStatePort,
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
        .challenge_use_case()
        .trigger_challenge(ctx, input)
        .await
    {
        Ok(_) => None, // Use case broadcasts ChallengePrompt to world
        Err(e) => Some(e.into_server_error()),
    }
}

/// DM decision on AI-suggested challenge. Broadcasts ChallengePrompt if approved.
pub async fn handle_challenge_suggestion_decision(
    state: &dyn AppStatePort,
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
        .challenge_use_case()
        .suggestion_decision(ctx, input)
        .await
    {
        Ok(()) => None, // Use case handles broadcasting if approved
        Err(e) => Some(e.into_server_error()),
    }
}

/// Create ad-hoc challenge. Returns AdHocChallengeCreated, broadcasts ChallengePrompt.
pub async fn handle_create_adhoc_challenge(
    state: &dyn AppStatePort,
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

    match state.challenge_use_case().create_adhoc(ctx, input).await {
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

/// DM decision on challenge outcome.
pub async fn handle_challenge_outcome_decision(
    state: &dyn AppStatePort,
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

    match state
        .challenge_use_case()
        .outcome_decision(ctx, input)
        .await
    {
        Ok(_) => None, // Resolution broadcast handled by service
        Err(e) => Some(e.into_server_error()),
    }
}

/// Request AI-generated outcome suggestions.
pub async fn handle_request_outcome_suggestion(
    state: &dyn AppStatePort,
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

    match state
        .challenge_use_case()
        .outcome_decision(ctx, input)
        .await
    {
        Ok(_) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

/// Request branching outcome options.
pub async fn handle_request_outcome_branches(
    state: &dyn AppStatePort,
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

    match state
        .challenge_use_case()
        .request_branches(ctx, input)
        .await
    {
        Ok(()) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

/// Select specific outcome branch.
pub async fn handle_select_outcome_branch(
    state: &dyn AppStatePort,
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

    match state
        .challenge_use_case()
        .select_branch(ctx, input)
        .await
    {
        Ok(()) => None,
        Err(e) => Some(e.into_server_error()),
    }
}

/// Discard challenge from approval queue.
pub async fn handle_discard_challenge(
    state: &dyn AppStatePort,
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
        .challenge_use_case()
        .discard_challenge(ctx, input)
        .await
    {
        Ok(_) => Some(ServerMessage::ChallengeDiscarded { request_id }),
        Err(e) => Some(e.into_server_error()),
    }
}

/// Regenerate challenge outcome text.
pub async fn handle_regenerate_outcome(
    state: &dyn AppStatePort,
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
        .challenge_use_case()
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
