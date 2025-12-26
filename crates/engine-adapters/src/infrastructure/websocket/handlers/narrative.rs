//! Narrative event handlers
//!
//! Handlers for narrative event approval and management.

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket::converters::value_to_server_message;
use wrldbldr_protocol::ServerMessage;

/// Handle NarrativeEventSuggestionDecision message
///
/// DM-only handler for approving or rejecting narrative event suggestions.
/// When a narrative event is suggested by the AI, the DM reviews it and can:
/// - Approve it to progress the story
/// - Reject it to discard the suggestion
/// - Select a specific outcome if multiple options are available
///
/// # Arguments
/// * `state` - Application state containing services and connection manager
/// * `client_id` - The UUID of the connected client (must be DM)
/// * `request_id` - The ID of the narrative event suggestion request
/// * `event_id` - The ID of the narrative event being decided upon
/// * `approved` - Whether the DM approves the narrative event
/// * `selected_outcome` - Optional selected outcome if multiple outcomes are available
///
/// # Returns
/// A `ServerMessage` confirming the decision, or an error if not authorized.
pub async fn handle_narrative_event_suggestion_decision(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    event_id: String,
    approved: bool,
    selected_outcome: Option<String>,
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
            message: "Only the DM can approve narrative event suggestions".to_string(),
        });
    }

    state
        .game
        .narrative_event_approval_service
        .handle_decision(
            world_id,
            request_id,
            event_id,
            approved,
            selected_outcome,
        )
        .await
        .and_then(value_to_server_message)
}
