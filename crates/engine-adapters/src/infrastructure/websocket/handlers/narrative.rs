//! Narrative event handlers
//!
//! Handlers for narrative event approval and management.

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use wrldbldr_protocol::ServerMessage;

/// Handle NarrativeEventSuggestionDecision message
///
/// DM-only handler for approving or rejecting narrative event suggestions.
/// When a narrative event is suggested by the AI, the DM reviews it and can:
/// - Approve it to progress the story
/// - Reject it to discard the suggestion
/// - Select a specific outcome if multiple options are available
///
/// Returns None on success (use case layer should broadcast events).
/// Returns Some(error) on failure.
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

    match state
        .game
        .narrative_event_approval_service
        .handle_decision(world_id, request_id, event_id.clone(), approved, selected_outcome)
        .await
    {
        Ok(Some(result)) => {
            // Broadcast NarrativeEventTriggered to the world
            // TODO: Move to use case layer with BroadcastPort
            let message = ServerMessage::NarrativeEventTriggered {
                event_id,
                event_name: result.event_name,
                outcome_description: result.outcome_description,
                scene_direction: result.scene_direction.unwrap_or_default(),
            };
            let world_uuid: Uuid = world_id.into();
            state
                .world_connection_manager
                .broadcast_to_world(world_uuid, message)
                .await;
            None
        }
        Ok(None) => None, // Rejected - no broadcast needed
        Err(e) => Some(ServerMessage::Error {
            code: "NARRATIVE_EVENT_ERROR".to_string(),
            message: e.to_string(),
        }),
    }
}
