use super::*;
use chrono::Utc;
use wrldbldr_domain::{InteractionTarget, InteractionType};

use crate::queue_types::PlayerActionData;

use crate::api::websocket::error_sanitizer::sanitize_repo_error;

pub(super) async fn handle_start_conversation(
    state: &WsState,
    connection_id: Uuid,
    npc_id: String,
    message: String,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    let pc_id = match conn_info.pc_id {
        Some(id) => id,
        None => {
            return Some(error_response(
                "NO_PC",
                "Must have a PC to start conversation",
            ))
        }
    };

    let npc_uuid = match parse_character_id(&npc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let message = normalize_conversation_message(message);
    let conversation = match state
        .app
        .use_cases
        .conversation
        .start
        .execute(
            world_id,
            pc_id,
            npc_uuid,
            conn_info.user_id.clone(),
            message,
        )
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::conversation::ConversationError::PlayerCharacterNotFound) => {
            return Some(error_response("NOT_FOUND", "Player character not found"))
        }
        Err(crate::use_cases::conversation::ConversationError::NpcNotFound) => {
            return Some(error_response("NOT_FOUND", "NPC not found"))
        }
        Err(crate::use_cases::conversation::ConversationError::WorldNotFound) => {
            return Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(crate::use_cases::conversation::ConversationError::NpcNotInRegion) => {
            return Some(error_response(
                "INVALID_TARGET",
                "NPC is not in this region",
            ))
        }
        Err(e) => {
            return Some(error_response(
                "CONVERSATION_ERROR",
                &sanitize_repo_error(&e, "start conversation"),
            ))
        }
    };

    broadcast_action_queued(
        state,
        world_id,
        conversation.action_queue_id,
        conn_info.user_id.clone(),
        "talk",
        1,
    )
    .await;

    // Return ConversationStarted with the conversation_id for client tracking
    Some(ServerMessage::ConversationStarted {
        conversation_id: conversation.conversation_id.to_string(),
        npc_id,
        npc_name: conversation.npc_name,
        npc_disposition: conversation.npc_disposition,
    })
}

pub(super) async fn handle_continue_conversation(
    state: &WsState,
    connection_id: Uuid,
    npc_id: String,
    message: String,
    conversation_id: Option<String>,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    let pc_id = match conn_info.pc_id {
        Some(id) => id,
        None => {
            return Some(error_response(
                "NO_PC",
                "Must have a PC to continue conversation",
            ))
        }
    };

    let npc_uuid = match parse_character_id(&npc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Parse optional conversation_id from string to UUID
    let conversation_uuid = match conversation_id {
        Some(id_str) => match Uuid::parse_str(&id_str) {
            Ok(uuid) => Some(uuid),
            Err(_) => {
                return Some(error_response(
                    "INVALID_CONVERSATION_ID",
                    &format!("Invalid conversation_id format: {}", id_str),
                ))
            }
        },
        None => None,
    };

    let message = message.trim().to_string();
    if message.is_empty() {
        return Some(error_response(
            "INVALID_MESSAGE",
            "Conversation message cannot be empty",
        ));
    }

    let conversation = match state
        .app
        .use_cases
        .conversation
        .continue_conversation
        .execute(
            world_id,
            pc_id,
            npc_uuid,
            conn_info.user_id.clone(),
            message,
            conversation_uuid,
        )
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::conversation::ConversationError::NpcLeftRegion) => {
            return Some(error_response("CONVERSATION_ENDED", "NPC left the region"))
        }
        Err(crate::use_cases::conversation::ConversationError::NpcNotFound) => {
            return Some(error_response("NOT_FOUND", "NPC not found"))
        }
        Err(crate::use_cases::conversation::ConversationError::WorldNotFound) => {
            return Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                "CONVERSATION_ERROR",
                &sanitize_repo_error(&e, "continue conversation"),
            ))
        }
    };

    broadcast_action_queued(
        state,
        world_id,
        conversation.action_queue_id,
        conn_info.user_id.clone(),
        "talk",
        1,
    )
    .await;

    Some(ServerMessage::ActionReceived {
        action_id: conversation.action_queue_id.to_string(),
        player_id: conn_info.user_id,
        action_type: "talk".to_string(),
    })
}

pub(super) async fn handle_perform_interaction(
    state: &WsState,
    connection_id: Uuid,
    interaction_id: String,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    let pc_id = match conn_info.pc_id {
        Some(id) => id,
        None => return Some(error_response("NO_PC", "Must have a PC to act")),
    };

    let interaction_uuid = match parse_id(
        &interaction_id,
        wrldbldr_domain::InteractionId::from_uuid,
        "Invalid interaction ID format",
    ) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let interaction = match state
        .app
        .repositories
        .interaction
        .get(interaction_uuid)
        .await
    {
        Ok(Some(interaction)) => interaction,
        Ok(None) => return Some(error_response("NOT_FOUND", "Interaction not found")),
        Err(e) => {
            return Some(error_response(
                "REPO_ERROR",
                &sanitize_repo_error(&e, "fetch interaction"),
            ))
        }
    };

    if matches!(interaction.interaction_type(), InteractionType::Dialogue) {
        let npc_id = match interaction.target() {
            InteractionTarget::Character(id) => *id,
            _ => {
                return Some(error_response(
                    "INVALID_TARGET",
                    "Dialogue interaction missing NPC target",
                ))
            }
        };

        let conversation = match state
            .app
            .use_cases
            .conversation
            .start
            .execute(
                world_id,
                pc_id,
                npc_id,
                conn_info.user_id.clone(),
                "Hello".to_string(),
            )
            .await
        {
            Ok(result) => result,
            Err(e) => {
                return Some(error_response(
                    "CONVERSATION_ERROR",
                    &sanitize_repo_error(&e, "start conversation from interaction"),
                ))
            }
        };

        broadcast_action_queued(
            state,
            world_id,
            conversation.action_queue_id,
            conn_info.user_id.clone(),
            "talk",
            1,
        )
        .await;

        return Some(ServerMessage::ConversationStarted {
            conversation_id: conversation.conversation_id.to_string(),
            npc_id: npc_id.to_string(),
            npc_name: conversation.npc_name,
            npc_disposition: conversation.npc_disposition,
        });
    }

    let target = interaction_target_label(&interaction.target());
    let action_type = interaction_action_type(&interaction.interaction_type());

    let action_data = PlayerActionData {
        world_id,
        player_id: conn_info.user_id.clone(),
        pc_id: Some(pc_id),
        action_type: action_type.to_string(),
        target: target.clone(),
        dialogue: None,
        timestamp: Utc::now(),
        conversation_id: None,
    };

    let action_id = match state.app.queue.enqueue_player_action(&action_data).await {
        Ok(id) => id,
        Err(e) => {
            return Some(error_response(
                "QUEUE_ERROR",
                &sanitize_repo_error(&e, "enqueue player action"),
            ))
        }
    };

    let queue_depth = state
        .app
        .queue
        .get_pending_count("player_action")
        .await
        .unwrap_or(1);

    broadcast_action_queued(
        state,
        world_id,
        action_id,
        conn_info.user_id.clone(),
        action_type,
        queue_depth,
    )
    .await;

    Some(ServerMessage::ActionReceived {
        action_id: action_id.to_string(),
        player_id: conn_info.user_id,
        action_type: action_type.to_string(),
    })
}

fn normalize_conversation_message(message: String) -> String {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        "Hello".to_string()
    } else {
        trimmed.to_string()
    }
}

fn interaction_action_type(interaction_type: &InteractionType) -> &'static str {
    match interaction_type {
        InteractionType::Dialogue => "talk",
        InteractionType::Examine => "examine",
        InteractionType::UseItem => "use_item",
        InteractionType::PickUp => "pickup",
        InteractionType::GiveItem => "give_item",
        InteractionType::Attack => "attack",
        InteractionType::Travel => "travel",
        InteractionType::Custom(_) => "custom",
    }
}

fn interaction_target_label(target: &InteractionTarget) -> Option<String> {
    match target {
        InteractionTarget::Character(id) => Some(id.to_string()),
        InteractionTarget::Item(id) => Some(id.to_string()),
        InteractionTarget::Environment(label) => Some(label.clone()),
        InteractionTarget::None => None,
    }
}

async fn broadcast_action_queued(
    state: &WsState,
    world_id: WorldId,
    action_id: Uuid,
    player_id: String,
    action_type: &str,
    queue_depth: usize,
) {
    let queue_msg = ServerMessage::ActionQueued {
        action_id: action_id.to_string(),
        player_name: player_id,
        action_type: action_type.to_string(),
        queue_depth,
    };
    state
        .connections
        .broadcast_to_dms(world_id, queue_msg)
        .await;
}
