use super::*;

use chrono::Utc;
use uuid::Uuid;
use wrldbldr_domain::{
    ConnectionId, ConversationId, InteractionTarget, InteractionType, QueueItemId, WorldId,
};

use crate::queue_types::PlayerActionData;

use crate::api::websocket::error_sanitizer::sanitize_repo_error;

/// Maximum conversation message length to prevent unbounded text processing.
const MAX_MESSAGE_LENGTH: usize = 2000;

pub(super) async fn handle_start_conversation(
    state: &WsState,
    connection_id: ConnectionId,
    npc_id: String,
    message: String,
) -> Option<ServerMessage> {
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
            conn_info.user_id.to_string(),
            message,
        )
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::conversation::ConversationError::PlayerCharacterNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("Player character not found: {}", id),
            ))
        }
        Err(crate::use_cases::conversation::ConversationError::NpcNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("NPC not found: {}", id),
            ))
        }
        Err(crate::use_cases::conversation::ConversationError::WorldNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("World not found: {}", id),
            ))
        }
        Err(crate::use_cases::conversation::ConversationError::NpcNotInRegion) => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "NPC is not in this region",
            ))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "start conversation"),
            ))
        }
    };

    broadcast_action_queued(
        state,
        world_id,
        conversation.action_queue_id,
        conn_info.user_id.to_string(),
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

pub(super) async fn handle_end_conversation(
    state: &WsState,
    connection_id: ConnectionId,
    npc_id: String,
    summary: Option<String>,
) -> Option<ServerMessage> {
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
                "Must have a PC to end conversation",
            ))
        }
    };

    let npc_uuid = match parse_character_id(&npc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Authorization check: only DM can end any conversation, or player ending their own PC's conversation
    if !conn_info.is_dm() {
        // For non-DM, ensure the PC being used matches the connection's PC
        // (i.e., only the player controlling this PC can end their conversation)
        // This is already enforced by using conn_info.pc_id as the pc_id
    }

    // Execute end conversation use case
    let result = match state
        .app
        .use_cases
        .conversation
        .end
        .execute(pc_id, npc_uuid, summary)
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::conversation::EndConversationError::PlayerCharacterNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("Player character not found: {}", id),
            ))
        }
        Err(crate::use_cases::conversation::EndConversationError::NpcNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("NPC not found: {}", id),
            ))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "end conversation"),
            ))
        }
    };

    tracing::info!(
        conversation_id = ?result.conversation_id,
        pc_id = %pc_id,
        pc_name = %result.pc_name,
        npc_id = %npc_uuid,
        npc_name = %result.npc_name,
        "Conversation ended"
    );

    // Broadcast ConversationEnded to all participants and DMs
    // For player-initiated ends, ended_by and reason are None
    let broadcast_msg = ServerMessage::ConversationEnded {
        npc_id: npc_id.clone(),
        npc_name: result.npc_name.clone(),
        pc_id: pc_id.to_string(),
        summary: result.summary.clone(),
        conversation_id: result.conversation_id.map(|id| id.to_string()),
        ended_by: None,
        reason: None,
    };

    state
        .connections
        .broadcast_to_world(world_id, broadcast_msg)
        .await;

    // Return success response to caller
    Some(ServerMessage::ConversationEnded {
        npc_id: npc_id,
        npc_name: result.npc_name,
        pc_id: pc_id.to_string(),
        summary: result.summary,
        conversation_id: result.conversation_id.map(|id| id.to_string()),
        ended_by: None,
        reason: None,
    })
}

pub(super) async fn handle_continue_conversation(
    state: &WsState,
    connection_id: ConnectionId,
    npc_id: String,
    message: String,
    conversation_id: Option<String>,
) -> Option<ServerMessage> {
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
                "Must have a PC to continue conversation",
            ))
        }
    };

    let npc_uuid = match parse_character_id(&npc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Parse optional conversation_id from string to ConversationId
    let conversation_uuid = match conversation_id {
        Some(id_str) => match Uuid::parse_str(&id_str) {
            Ok(uuid) => Some(ConversationId::from(uuid)),
            Err(_) => {
                return Some(error_response(
                    ErrorCode::ValidationError,
                    &format!("Invalid conversation_id format: {}", id_str),
                ))
            }
        },
        None => None,
    };

    let message = message.trim().to_string();

    // Validate message length
    if message.is_empty() {
        return Some(error_response(
            ErrorCode::ValidationError,
            "Conversation message cannot be empty",
        ));
    }

    if message.len() > MAX_MESSAGE_LENGTH {
        return Some(error_response(
            ErrorCode::BadRequest,
            &format!("Message too long (max {} chars)", MAX_MESSAGE_LENGTH),
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
            conn_info.user_id.to_string(),
            message,
            conversation_uuid,
        )
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::conversation::ConversationError::NpcLeftRegion) => {
            return Some(error_response(ErrorCode::BadRequest, "NPC left the region"))
        }
        Err(crate::use_cases::conversation::ConversationError::NpcNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("NPC not found: {}", id),
            ))
        }
        Err(crate::use_cases::conversation::ConversationError::WorldNotFound(id)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                &format!("World not found: {}", id),
            ))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "continue conversation"),
            ))
        }
    };

    broadcast_action_queued(
        state,
        world_id,
        conversation.action_queue_id,
        conn_info.user_id.to_string(),
        "talk",
        1,
    )
    .await;

    Some(ServerMessage::ActionReceived {
        action_id: conversation.action_queue_id.to_string(),
        player_id: conn_info.user_id.to_string(),
        action_type: "talk".to_string(),
    })
}

pub(super) async fn handle_perform_interaction(
    state: &WsState,
    connection_id: ConnectionId,
    interaction_id: String,
) -> Option<ServerMessage> {
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
                "Must have a PC to act",
            ))
        }
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
        Ok(None) => return Some(error_response(ErrorCode::NotFound, "Interaction not found")),
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "fetch interaction"),
            ))
        }
    };

    if matches!(interaction.interaction_type(), InteractionType::Dialogue) {
        let npc_id = match interaction.target() {
            InteractionTarget::Character(id) => *id,
            _ => {
                return Some(error_response(
                    ErrorCode::BadRequest,
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
                conn_info.user_id.to_string(),
                "Hello".to_string(),
            )
            .await
        {
            Ok(result) => result,
            Err(e) => {
                return Some(error_response(
                    ErrorCode::InternalError,
                    &sanitize_repo_error(&e, "start conversation from interaction"),
                ))
            }
        };

        broadcast_action_queued(
            state,
            world_id,
            conversation.action_queue_id,
            conn_info.user_id.to_string(),
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

    let target = interaction_target_label(interaction.target());
    let action_type = interaction_action_type(interaction.interaction_type());

    let action_data = PlayerActionData {
        world_id,
        player_id: conn_info.user_id.to_string(),
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
                ErrorCode::InternalError,
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
        conn_info.user_id.to_string(),
        action_type,
        queue_depth,
    )
    .await;

    Some(ServerMessage::ActionReceived {
        action_id: action_id.to_string(),
        player_id: conn_info.user_id.to_string(),
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
    action_id: QueueItemId,
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

/// Handle ListActiveConversations request (DM only).
///
/// DM requests list of all active conversations in the current world.
/// Returns ActiveConversationsList response with conversation info.
pub(super) async fn handle_list_active_conversations(
    state: &WsState,
    connection_id: ConnectionId,
    world_id: Uuid,
    _include_ended: bool,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    // DM only
    if !conn_info.is_dm() {
        return Some(error_response(
            ErrorCode::Unauthorized,
            "Only DMs can list active conversations",
        ));
    }

    // Authorization: Use conn_info.world_id to prevent cross-world access
    let world_uuid = match conn_info.world_id {
        Some(id) => id,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Must join a world first",
            ))
        }
    };

    // Verify the client-provided world_id matches the connection's world_id
    if world_uuid != wrldbldr_domain::WorldId::from(world_id) {
        return Some(error_response(
            ErrorCode::Unauthorized,
            "Cannot access other worlds",
        ));
    }

    // Call use case to get active conversations
    let result = match state
        .app
        .use_cases
        .conversation
        .list_active
        .execute(world_uuid)
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::conversation::ListActiveConversationsError::WorldNotFound(_)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                "World not found",
            ))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "list active conversations"),
            ))
        }
    };

    // Convert domain types to protocol
    let conversations = result
        .conversations
        .into_iter()
        .map(|c| c.to_protocol())
        .collect();

    Some(ServerMessage::ActiveConversationsList { conversations })
}

/// Handle EndConversationById request (DM only).
///
/// DM ends a specific conversation by conversation ID.
/// Broadcasts ConversationEnded to all participants.
pub(super) async fn handle_end_conversation_by_id(
    state: &WsState,
    connection_id: ConnectionId,
    conversation_id: Uuid,
    reason: Option<String>,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    // DM only
    if !conn_info.is_dm() {
        return Some(error_response(
            ErrorCode::Unauthorized,
            "Only DMs can end conversations by ID",
        ));
    }

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Must join a world first",
            ))
        }
    };

    let conversation_uuid = wrldbldr_domain::ConversationId::from(conversation_id);

    // Track who ended it (optional - could be DM's character ID if DM has a PC)
    let ended_by = None; // For now, DM actions don't track which character

    // Call use case to end conversation by ID
    let result = match state
        .app
        .use_cases
        .conversation
        .end_by_id
        .execute(conversation_uuid, ended_by, reason)
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::conversation::EndConversationByIdError::ConversationNotFound(_)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                "Conversation not found",
            ))
        }
        Err(crate::use_cases::conversation::EndConversationByIdError::ConversationAlreadyEnded(_)) => {
            return Some(error_response(
                ErrorCode::Conflict,
                "Conversation already ended",
            ))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "end conversation by ID"),
            ))
        }
    };

    tracing::info!(
        conversation_id = %result.conversation_id,
        ended_by = ?result.ended_by,
        reason = ?result.reason,
        pc_id = %result.pc_id,
        npc_id = %result.npc_id,
        "Conversation ended by ID"
    );

    // Broadcast ConversationEnded to all in world
    let broadcast_msg = ServerMessage::ConversationEnded {
        npc_id: result.npc_id.to_string(),
        npc_name: result.npc_name.clone(),
        pc_id: result.pc_id.to_string(),
        summary: result.summary.clone(),
        conversation_id: Some(result.conversation_id.to_string()),
        ended_by: result.ended_by.map(|id| id.to_string()),
        reason: result.reason.clone(),
    };

    state
        .connections
        .broadcast_to_world(world_id, broadcast_msg)
        .await;

    // Also return to caller for confirmation
    Some(ServerMessage::ConversationEnded {
        npc_id: result.npc_id.to_string(),
        npc_name: result.npc_name,
        pc_id: result.pc_id.to_string(),
        summary: result.summary,
        conversation_id: Some(result.conversation_id.to_string()),
        ended_by: result.ended_by.map(|id| id.to_string()),
        reason: result.reason,
    })
}

/// Handle GetConversationDetails request (DM only).
///
/// DM requests full details for a specific conversation.
/// Returns ConversationDetails response with participants and recent turns.
pub(super) async fn handle_get_conversation_details(
    state: &WsState,
    connection_id: ConnectionId,
    conversation_id: Uuid,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    // DM only
    if !conn_info.is_dm() {
        return Some(error_response(
            ErrorCode::Unauthorized,
            "Only DMs can view conversation details",
        ));
    }

    let conversation_uuid = wrldbldr_domain::ConversationId::from(conversation_id);

    // Call use case to get conversation details
    let result = match state
        .app
        .use_cases
        .conversation
        .get_details
        .execute(crate::use_cases::conversation::GetConversationDetailsInput {
            conversation_id: conversation_uuid,
        })
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::conversation::GetConversationDetailsError::ConversationNotFound(_)) => {
            return Some(error_response(
                ErrorCode::NotFound,
                "Conversation not found",
            ))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "get conversation details"),
            ))
        }
    };

    Some(ServerMessage::ConversationDetails {
        details: result.to_protocol(),
    })
}
