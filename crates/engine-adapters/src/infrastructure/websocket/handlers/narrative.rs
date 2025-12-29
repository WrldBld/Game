//! Narrative event handlers
//!
//! Handlers for narrative event approval and management.

use uuid::Uuid;

use crate::infrastructure::adapter_state::AdapterState;
use crate::infrastructure::websocket::IntoServerError;
use wrldbldr_engine_ports::outbound::NarrativeEventSuggestionDecisionInput;
use wrldbldr_protocol::ServerMessage;

use super::common::extract_dm_context;

/// Handle NarrativeEventSuggestionDecision message
///
/// DM-only handler for approving or rejecting narrative event suggestions.
/// When a narrative event is suggested by the AI, the DM reviews it and can:
/// - Approve it to progress the story
/// - Reject it to discard the suggestion
/// - Select a specific outcome if multiple options are available
///
/// Returns None on success (use case broadcasts events via BroadcastPort).
/// Returns Some(error) on failure.
pub async fn handle_narrative_event_suggestion_decision(
    state: &AdapterState,
    client_id: Uuid,
    request_id: String,
    event_id: String,
    approved: bool,
    selected_outcome: Option<String>,
) -> Option<ServerMessage> {
    // Extract DM context (validates connection, world, and DM authorization)
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    // Build input for use case
    let input = NarrativeEventSuggestionDecisionInput {
        request_id,
        event_id,
        approved,
        selected_outcome,
    };

    // Delegate to use case (broadcasts are handled by the use case via BroadcastPort)
    match state
        .app
        .use_cases
        .narrative_event
        .handle_suggestion_decision(ctx, input)
        .await
    {
        Ok(_) => None, // Success - use case has already broadcast if approved
        Err(e) => Some(e.into_server_error()),
    }
}
