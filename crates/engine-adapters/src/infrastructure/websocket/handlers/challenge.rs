//! Challenge system handlers for WebSocket connections.
//!
//! This module contains handlers for all challenge-related operations including:
//!
//! ## Player Operations
//! - [`handle_challenge_roll`] - Submit a dice roll result for an active challenge
//! - [`handle_challenge_roll_input`] - Submit dice input (formula or manual) for rolling
//!
//! ## DM Operations
//! - [`handle_trigger_challenge`] - DM triggers a challenge against a target character
//! - [`handle_challenge_suggestion_decision`] - DM approves/modifies AI-suggested challenges
//! - [`handle_regenerate_outcome`] - DM requests regeneration of challenge outcome text
//! - [`handle_discard_challenge`] - DM discards a challenge from the approval queue
//! - [`handle_create_adhoc_challenge`] - DM creates a custom challenge on the fly
//! - [`handle_challenge_outcome_decision`] - DM accepts/edits challenge outcome
//! - [`handle_request_outcome_suggestion`] - DM requests AI-generated outcome suggestions
//! - [`handle_request_outcome_branches`] - DM requests branching outcome options
//! - [`handle_select_outcome_branch`] - DM selects a specific outcome branch
//!
//! All handlers follow the pattern of taking `&AppState`, `client_id`, and
//! message-specific parameters, returning `Option<ServerMessage>`.

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket::converters::{
    to_adhoc_outcomes_dto, to_challenge_outcome_decision, to_service_dice_input,
    value_to_server_message,
};
use wrldbldr_engine_app::application::dto::ChallengeOutcomeDecision;
use wrldbldr_protocol::ServerMessage;

/// Handles a player submitting a dice roll result for an active challenge.
///
/// This is called after the player has rolled dice (either manually or via formula)
/// and submits the final roll value for challenge resolution.
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client
/// * `challenge_id` - The UUID of the challenge being resolved
/// * `roll` - The dice roll result value
///
/// # Returns
/// A `ServerMessage` with the challenge resolution result, or an error if the
/// connection context is invalid.
pub async fn handle_challenge_roll(
    state: &AppState,
    client_id: Uuid,
    challenge_id: String,
    roll: i32,
) -> Option<ServerMessage> {
    tracing::debug!(
        "Received challenge roll: {} for challenge {}",
        roll,
        challenge_id
    );

    // Get connection context for world_id and pc_id
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    let pc_id = match connection.pc_id {
        Some(id) => wrldbldr_domain::PlayerCharacterId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_PC".to_string(),
                message: "No player character selected".to_string(),
            })
        }
    };

    state
        .game
        .challenge_resolution_service
        .handle_roll(&world_id, &pc_id, challenge_id, roll)
        .await
        .and_then(value_to_server_message)
}

/// Handles a player submitting dice input for a challenge roll.
///
/// This allows players to specify how they want to roll dice:
/// - `Formula`: A dice formula like "1d20+5" that the server will evaluate
/// - `Manual`: A pre-rolled value the player enters manually
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client
/// * `challenge_id` - The UUID of the challenge being resolved
/// * `input_type` - The type of dice input (formula string or manual value)
///
/// # Returns
/// A `ServerMessage` with the roll result, or an error if the connection context is invalid.
pub async fn handle_challenge_roll_input(
    state: &AppState,
    client_id: Uuid,
    challenge_id: String,
    input_type: wrldbldr_protocol::DiceInputType,
) -> Option<ServerMessage> {
    tracing::debug!(
        "Received challenge roll input: {:?} for challenge {}",
        input_type,
        challenge_id
    );

    // Get connection context for world_id and pc_id
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    let pc_id = match connection.pc_id {
        Some(id) => wrldbldr_domain::PlayerCharacterId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_PC".to_string(),
                message: "No player character selected".to_string(),
            })
        }
    };

    state
        .game
        .challenge_resolution_service
        .handle_roll_input(
            &world_id,
            &pc_id,
            challenge_id,
            to_service_dice_input(input_type),
        )
        .await
        .and_then(value_to_server_message)
}

/// Handles a DM triggering a challenge against a target character.
///
/// This is a DM-only operation that initiates a challenge against a specific
/// player character. The challenge must already exist in the system.
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client (must be DM)
/// * `challenge_id` - The UUID of the challenge to trigger
/// * `target_character_id` - The UUID of the target player character
///
/// # Returns
/// A `ServerMessage` with challenge trigger confirmation, or an error if not authorized.
pub async fn handle_trigger_challenge(
    state: &AppState,
    client_id: Uuid,
    challenge_id: String,
    target_character_id: String,
) -> Option<ServerMessage> {
    // Get connection context for world_id (DM operation)
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can trigger challenges".to_string(),
        });
    }

    state
        .game
        .challenge_resolution_service
        .handle_trigger(&world_id, challenge_id, target_character_id)
        .await
        .and_then(value_to_server_message)
}

/// Handles a DM's decision on an AI-suggested challenge.
///
/// When the AI suggests a challenge, the DM can approve it, reject it, or
/// modify the difficulty before it's presented to the player.
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client (must be DM)
/// * `request_id` - The UUID of the challenge suggestion request
/// * `approved` - Whether the DM approves the challenge
/// * `modified_difficulty` - Optional modified difficulty if DM wants to adjust it
///
/// # Returns
/// A `ServerMessage` confirming the decision, or an error if not authorized.
pub async fn handle_challenge_suggestion_decision(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    approved: bool,
    modified_difficulty: Option<String>,
) -> Option<ServerMessage> {
    // Get connection context for world_id (DM operation)
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can approve challenge suggestions".to_string(),
        });
    }

    state
        .game
        .challenge_resolution_service
        .handle_suggestion_decision(&world_id, request_id, approved, modified_difficulty)
        .await
        .and_then(value_to_server_message)
}

/// Handles a DM request to regenerate challenge outcome text.
///
/// This allows the DM to request new AI-generated outcome text if they're
/// not satisfied with the current suggestion. Optional guidance can be
/// provided to steer the regeneration.
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client
/// * `request_id` - The UUID of the approval request
/// * `outcome_type` - Optional type of outcome to regenerate (e.g., "success", "failure")
/// * `guidance` - Optional guidance text to influence regeneration
///
/// # Returns
/// A `ServerMessage::OutcomeRegenerated` with the new outcome text.
pub async fn handle_regenerate_outcome(
    state: &AppState,
    _client_id: Uuid,
    request_id: String,
    outcome_type: Option<String>,
    guidance: Option<String>,
) -> Option<ServerMessage> {
    tracing::debug!(
        "DM requested outcome regeneration for request {} outcome {:?}",
        request_id,
        outcome_type
    );

    // Best-effort: look up the approval item for context
    let maybe_approval = state
        .queues
        .dm_approval_queue_service
        .get_by_id(&request_id)
        .await
        .ok()
        .flatten();

    let base_flavor = if let Some(item) = maybe_approval {
        format!("{} (regenerated)", item.payload.proposed_dialogue.trim())
    } else {
        "Regenerated outcome (no approval context found)".to_string()
    };

    let flavor_text = if let Some(g) = guidance {
        if g.trim().is_empty() {
            base_flavor
        } else {
            format!("{} — Guidance: {}", base_flavor, g.trim())
        }
    } else {
        base_flavor
    };

    let outcome_type_str = outcome_type.unwrap_or_else(|| "all".to_string());

    Some(ServerMessage::OutcomeRegenerated {
        request_id,
        outcome_type: outcome_type_str,
        new_outcome: wrldbldr_protocol::OutcomeDetailData {
            flavor_text,
            scene_direction: "DM: narrate this regenerated outcome to the table.".to_string(),
            proposed_tools: Vec::new(),
        },
    })
}

/// Handles a DM discarding a challenge from the approval queue.
///
/// This removes a challenge suggestion from the DM's approval queue,
/// typically because the DM wants to handle the situation differently
/// or the challenge is no longer relevant.
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client
/// * `request_id` - The UUID of the challenge request to discard
/// * `feedback` - Optional feedback explaining why the challenge was discarded
///
/// # Returns
/// A `ServerMessage::ChallengeDiscarded` confirming the discard.
pub async fn handle_discard_challenge(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    feedback: Option<String>,
) -> Option<ServerMessage> {
    tracing::info!(
        "DM discarded challenge for request {}, feedback: {:?}",
        request_id,
        feedback
    );

    // Remove the challenge suggestion from the approval queue
    // The approval will be re-queued for a non-challenge response
    state
        .queues
        .dm_approval_queue_service
        .discard_challenge(&client_id.to_string(), &request_id)
        .await;

    Some(ServerMessage::ChallengeDiscarded { request_id })
}

/// Handles a DM creating an ad-hoc challenge on the fly.
///
/// This allows the DM to create a custom challenge that wasn't pre-defined
/// in the world data. The DM specifies all parameters including custom
/// outcomes for success/failure states.
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client (must be DM)
/// * `challenge_name` - Display name for the challenge
/// * `skill_name` - The skill being tested (e.g., "Athletics", "Persuasion")
/// * `difficulty` - The difficulty class (DC) for the challenge
/// * `target_pc_id` - The UUID of the target player character
/// * `outcomes` - Custom outcome descriptions for different result tiers
///
/// # Returns
/// A `ServerMessage` with the created challenge, or an error if not authorized.
pub async fn handle_create_adhoc_challenge(
    state: &AppState,
    client_id: Uuid,
    challenge_name: String,
    skill_name: String,
    difficulty: String,
    target_pc_id: String,
    outcomes: wrldbldr_protocol::AdHocOutcomes,
) -> Option<ServerMessage> {
    tracing::info!(
        "DM creating ad-hoc challenge '{}' for PC {}",
        challenge_name,
        target_pc_id
    );

    // Get connection context for world_id (DM operation)
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can create ad-hoc challenges".to_string(),
        });
    }

    state
        .game
        .challenge_resolution_service
        .handle_adhoc_challenge(
            &world_id,
            challenge_name,
            skill_name,
            difficulty,
            target_pc_id,
            to_adhoc_outcomes_dto(outcomes),
        )
        .await
        .and_then(value_to_server_message)
}

/// Handles a DM's decision on a challenge outcome.
///
/// After a challenge is resolved (dice rolled, success/failure determined),
/// the DM reviews the proposed outcome and can:
/// - Accept it as-is
/// - Edit the outcome description
/// - Request AI suggestions for alternative outcomes
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client (must be DM)
/// * `resolution_id` - The UUID of the challenge resolution
/// * `decision` - The DM's decision (Accept, Edit, or Suggest)
///
/// # Returns
/// `None` on success (resolution is broadcast by the service), or an error message.
pub async fn handle_challenge_outcome_decision(
    state: &AppState,
    client_id: Uuid,
    resolution_id: String,
    decision: wrldbldr_protocol::ChallengeOutcomeDecisionData,
) -> Option<ServerMessage> {
    tracing::info!(
        "DM decision on challenge outcome {}: {:?}",
        resolution_id,
        decision
    );

    // Get connection context for world_id (DM operation)
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can approve challenge outcomes".to_string(),
        });
    }

    // Convert wire decision to service decision
    let svc_decision = to_challenge_outcome_decision(decision);

    // Process the decision via the approval service
    match state
        .game
        .challenge_outcome_approval_service
        .process_decision(&world_id, &resolution_id, svc_decision)
        .await
    {
        Ok(()) => {
            // Success - resolution broadcast is handled by the service
            None
        }
        Err(e) => {
            tracing::error!("Failed to process challenge outcome decision: {}", e);
            Some(ServerMessage::Error {
                code: "APPROVAL_ERROR".to_string(),
                message: format!("Failed to process decision: {}", e),
            })
        }
    }
}

/// Handles a DM requesting AI-generated outcome suggestions.
///
/// The DM can request the AI to generate alternative outcome descriptions
/// for a challenge resolution. Optional guidance can be provided to steer
/// the generation toward a particular narrative direction.
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client (must be DM)
/// * `resolution_id` - The UUID of the challenge resolution
/// * `guidance` - Optional guidance text to influence AI generation
///
/// # Returns
/// `None` on success (suggestions sent via `OutcomeSuggestionReady`), or an error message.
pub async fn handle_request_outcome_suggestion(
    state: &AppState,
    client_id: Uuid,
    resolution_id: String,
    guidance: Option<String>,
) -> Option<ServerMessage> {
    tracing::info!(
        "DM requesting outcome suggestion for {}: {:?}",
        resolution_id,
        guidance
    );

    // Get connection context for world_id (DM operation)
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can request outcome suggestions".to_string(),
        });
    }

    // Process as a Suggest decision - the service will handle LLM generation
    let svc_decision = ChallengeOutcomeDecision::Suggest { guidance };

    match state
        .game
        .challenge_outcome_approval_service
        .process_decision(&world_id, &resolution_id, svc_decision)
        .await
    {
        Ok(()) => {
            // Success - the service will send OutcomeSuggestionReady when LLM completes
            None
        }
        Err(e) => {
            tracing::error!("Failed to request outcome suggestions: {}", e);
            Some(ServerMessage::Error {
                code: "SUGGESTION_ERROR".to_string(),
                message: format!("Failed to request suggestions: {}", e),
            })
        }
    }
}

/// Handles a DM requesting branching outcome options.
///
/// This generates multiple possible outcome branches that the DM can choose
/// from, allowing for more nuanced narrative outcomes than a simple
/// success/failure dichotomy.
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client (must be DM)
/// * `resolution_id` - The UUID of the challenge resolution
/// * `guidance` - Optional guidance text to influence branch generation
///
/// # Returns
/// `None` on success (branches sent via `OutcomeBranchesReady`), or an error message.
pub async fn handle_request_outcome_branches(
    state: &AppState,
    client_id: Uuid,
    resolution_id: String,
    guidance: Option<String>,
) -> Option<ServerMessage> {
    tracing::info!(
        "DM requesting outcome branches for {}: {:?}",
        resolution_id,
        guidance
    );

    // Get connection context for world_id (DM operation)
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can request outcome branches".to_string(),
        });
    }

    // Request branches via the approval service
    match state
        .game
        .challenge_outcome_approval_service
        .request_branches(&world_id, &resolution_id, guidance)
        .await
    {
        Ok(()) => {
            // Success - the service will send OutcomeBranchesReady when LLM completes
            None
        }
        Err(e) => {
            tracing::error!("Failed to request outcome branches: {}", e);
            Some(ServerMessage::Error {
                code: "BRANCH_ERROR".to_string(),
                message: format!("Failed to request branches: {}", e),
            })
        }
    }
}

/// Handles a DM selecting a specific outcome branch.
///
/// After receiving branching options via `OutcomeBranchesReady`, the DM
/// selects one of the branches to apply as the final outcome. The DM
/// can optionally modify the description before finalizing.
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client (must be DM)
/// * `resolution_id` - The UUID of the challenge resolution
/// * `branch_id` - The UUID of the selected branch
/// * `modified_description` - Optional modified description text
///
/// # Returns
/// `None` on success (challenge is resolved), or an error message.
pub async fn handle_select_outcome_branch(
    state: &AppState,
    client_id: Uuid,
    resolution_id: String,
    branch_id: String,
    modified_description: Option<String>,
) -> Option<ServerMessage> {
    tracing::info!(
        "DM selecting branch {} for resolution {}",
        branch_id,
        resolution_id
    );

    // Only DMs should select branches - check via world connection manager
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can select outcome branches".to_string(),
        });
    }

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Process branch selection via the approval service
    match state
        .game
        .challenge_outcome_approval_service
        .select_branch(&world_id, &resolution_id, &branch_id, modified_description)
        .await
    {
        Ok(()) => {
            // Success - challenge is resolved
            None
        }
        Err(e) => {
            tracing::error!("Failed to select outcome branch: {}", e);
            Some(ServerMessage::Error {
                code: "BRANCH_SELECT_ERROR".to_string(),
                message: format!("Failed to select branch: {}", e),
            })
        }
    }
}
