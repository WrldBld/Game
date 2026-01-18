use super::*;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use wrldbldr_shared::ErrorCode;

pub(super) async fn handle_player_action(
    state: &WsState,
    connection_id: Uuid,
    action_type: String,
    target: Option<String>,
    dialogue: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Must join a world first",
            ))
        }
    };

    let pc_id = match conn_info.pc_id {
        Some(id) => id,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Must have a PC to perform actions",
            ))
        }
    };

    let target_npc = if action_type == "talk" {
        match target.as_ref() {
            Some(target_str) => match parse_character_id(target_str) {
                Ok(id) => Some(id),
                Err(e) => return Some(e),
            },
            None => None,
        }
    } else {
        None
    };

    let processed = match state
        .app
        .use_cases
        .player_action
        .handle
        .execute(
            world_id,
            pc_id,
            conn_info.user_id.clone(),
            action_type.clone(),
            target_npc,
            dialogue.clone(),
        )
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::player_action::PlayerActionError::MissingTalkTarget) => {
            return Some(error_response(
                ErrorCode::ValidationError,
                "Talk action requires target NPC ID",
            ))
        }
        Err(crate::use_cases::player_action::PlayerActionError::Conversation(e)) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "starting conversation"),
            ));
        }
        Err(crate::use_cases::player_action::PlayerActionError::Queue(e)) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "enqueuing player action"),
            ));
        }
    };

    tracing::info!(
        connection_id = %connection_id,
        action_id = %processed.action_id,
        action_type = %processed.action_type,
        target = ?target,
        "Player action received"
    );

    // Acknowledge the action
    let ack = ServerMessage::ActionReceived {
        action_id: processed.action_id.to_string(),
        player_id: processed.player_id.clone(),
        action_type: processed.action_type.clone(),
    };

    // Notify DMs that action is queued
    let queue_msg = ServerMessage::ActionQueued {
        action_id: processed.action_id.to_string(),
        player_name: processed.player_id,
        action_type: processed.action_type,
        queue_depth: processed.queue_depth,
    };
    state
        .connections
        .broadcast_to_dms(world_id, queue_msg)
        .await;

    Some(ack)
}
