//! Player action handlers
//!
//! Thin handler for player actions (travel, interact, speak, etc.).
//! All business logic is delegated to PlayerActionUseCase.

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket::converters::scene_changed_event_to_message;
use crate::infrastructure::websocket::IntoServerError;
use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_engine_app::application::use_cases::{ActionResult, PlayerActionInput};
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_protocol::ServerMessage;

/// Handles a PlayerAction message from a client.
///
/// Delegates to `PlayerActionUseCase::handle_action` which:
/// 1. Processes travel actions immediately (via MovementUseCase)
/// 2. Queues non-travel actions for LLM/DM processing
/// 3. Notifies DM of queued actions
pub async fn handle_player_action(
    state: &AppState,
    client_id: Uuid,
    action_type: String,
    target: Option<String>,
    dialogue: Option<String>,
    sender: mpsc::Sender<ServerMessage>,
) -> Option<ServerMessage> {
    tracing::debug!("Received player action: {} -> {:?}", action_type, target);

    // Extract context
    let ctx = match extract_context(state, client_id).await {
        Some(ctx) => ctx,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };

    let player_id = ctx.user_id.clone();

    let input = PlayerActionInput {
        action_type: action_type.clone(),
        target,
        dialogue,
    };

    match state
        .use_cases
        .player_action
        .handle_action(ctx, input)
        .await
    {
        Ok(ActionResult::TravelCompleted { action_id, scene }) => {
            // Send scene update to player
            let scene_msg = scene_changed_event_to_message(scene);
            let _ = sender.try_send(scene_msg);

            // Return acknowledgment
            Some(ServerMessage::ActionReceived {
                action_id,
                player_id,
                action_type,
            })
        }
        Ok(ActionResult::TravelPending {
            action_id,
            region_id,
            region_name,
        }) => {
            // Send staging pending
            let _ = sender.try_send(ServerMessage::StagingPending {
                region_id: region_id.to_string(),
                region_name,
            });

            // Return acknowledgment
            Some(ServerMessage::ActionReceived {
                action_id,
                player_id,
                action_type,
            })
        }
        Ok(ActionResult::Queued {
            action_id,
            queue_depth: _,
        }) => {
            // Return acknowledgment (DM is notified by the use case via DmNotificationPort)
            Some(ServerMessage::ActionReceived {
                action_id,
                player_id,
                action_type,
            })
        }
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Extract UseCaseContext from connection state
async fn extract_context(state: &AppState, client_id: Uuid) -> Option<UseCaseContext> {
    let conn = state
        .world_connection_manager
        .get_connection_by_client_id(&client_id.to_string())
        .await?;

    let world_id = conn.world_id?;

    Some(UseCaseContext {
        world_id: WorldId::from_uuid(world_id),
        user_id: conn.user_id.clone(),
        is_dm: conn.is_dm(),
        pc_id: conn.pc_id.map(PlayerCharacterId::from_uuid),
    })
}
