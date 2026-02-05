use wrldbldr_domain::{ConnectionId, PlayerCharacterId};

use super::*;
use crate::api::websocket::ws_time::time_suggestion_to_protocol;

use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use wrldbldr_shared::ErrorCode;

/// Convert protocol item_recipients (String PC IDs) to domain type (PlayerCharacterId)
fn convert_item_recipients(
    recipients: std::collections::HashMap<String, Vec<String>>,
) -> std::collections::HashMap<String, Vec<PlayerCharacterId>> {
    recipients
        .into_iter()
        .map(|(tool_id, pc_ids)| {
            let typed_ids: Vec<PlayerCharacterId> = pc_ids
                .into_iter()
                .filter_map(|s| uuid::Uuid::parse_str(&s).ok())
                .map(PlayerCharacterId::from_uuid)
                .collect();
            (tool_id, typed_ids)
        })
        .collect()
}

pub(super) async fn handle_approval_decision(
    state: &WsState,
    connection_id: ConnectionId,
    request_id: String,
    decision: wrldbldr_shared::ApprovalDecision,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can make approval decisions
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse request ID as approval UUID
    let approval_id = match parse_id(&request_id, |u| u, "Invalid request ID") {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Validate approval decision
    if let Err(e) = decision.validate() {
        return Some(error_response(
            ErrorCode::BadRequest,
            &e,
        ));
    }

    // Convert protocol decision to domain decision
    let domain_decision = match decision {
        wrldbldr_shared::ApprovalDecision::Accept => crate::queue_types::DmApprovalDecision::Accept,
        wrldbldr_shared::ApprovalDecision::AcceptWithRecipients { item_recipients } => {
            crate::queue_types::DmApprovalDecision::AcceptWithRecipients {
                item_recipients: convert_item_recipients(item_recipients)
            }
        }
        wrldbldr_shared::ApprovalDecision::Reject { feedback } => {
            crate::queue_types::DmApprovalDecision::Reject { feedback }
        }
        wrldbldr_shared::ApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => crate::queue_types::DmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients: convert_item_recipients(item_recipients),
        },
        wrldbldr_shared::ApprovalDecision::TakeOver { dm_response } => {
            crate::queue_types::DmApprovalDecision::TakeOver { dm_response }
        }
        wrldbldr_shared::ApprovalDecision::Unknown => {
            return Some(error_response(
                ErrorCode::ValidationError,
                "Unknown approval decision type",
            ));
        }
    };

    match state
        .app
        .use_cases
        .approval
        .decision_flow
        .execute(approval_id.into(), domain_decision)
        .await
    {
        Ok(result) => {
            if result.approved {
                let dialogue = result.final_dialogue.clone().unwrap_or_default();
                let world_id = result.world_id;
                let conversation_id = result.conversation_id;

                // Send ResponseApproved to DMs (shows what tools were executed)
                let dm_msg = ServerMessage::ResponseApproved {
                    npc_dialogue: dialogue.clone(),
                    executed_tools: result.approved_tools.clone(),
                };
                state.connections.broadcast_to_dms(world_id, dm_msg).await;

                // Send DialogueResponse to all players (for visual novel display)
                let npc_id = result.npc_id;
                if !dialogue.is_empty() {
                    if let Some(ref speaker_id) = npc_id {
                        let dialogue_msg = ServerMessage::DialogueResponse {
                            speaker_id: speaker_id.to_string(),
                            speaker_name: result.npc_name.clone().unwrap_or_else(|| "Unknown".to_string()),
                            text: dialogue,
                            choices: vec![], // Free-form input mode
                            conversation_id: conversation_id.map(|id| id.to_string()),
                        };
                        state
                            .connections
                            .broadcast_to_world(world_id, dialogue_msg)
                            .await;
                    } else {
                        tracing::warn!(
                            "Approved dialogue has no speaker ID, skipping DialogueResponse broadcast"
                        );
                    }
                }

                // Send time suggestion to DMs (US-TIME-013)
                if let Some(ref time_suggestion) = result.time_suggestion {
                    // Remove any existing suggestion for the same PC to prevent unbounded growth
                    state
                        .pending_time_suggestions
                        .remove_for_pc(time_suggestion.pc_id)
                        .await;
                    state
                        .pending_time_suggestions
                        .insert(time_suggestion.id, time_suggestion.clone())
                        .await;

                    let suggestion_msg = ServerMessage::TimeSuggestion {
                        data: time_suggestion_to_protocol(time_suggestion),
                    };
                    state
                        .connections
                        .broadcast_to_dms(world_id, suggestion_msg)
                        .await;

                    tracing::info!(
                        approval_id = %approval_id,
                        npc_id = ?npc_id,
                        suggested_seconds = time_suggestion.suggested_seconds,
                        "Emitted time suggestion for dialogue approval"
                    );
                }

                // Broadcast stat updates from tool execution
                for stat_update in &result.stat_updates {
                    let stat_msg = ServerMessage::CharacterStatUpdated {
                        character_id: stat_update.character_id.to_string(),
                        character_name: stat_update.character_name.clone(),
                        stat_name: stat_update.stat_name.clone(),
                        old_value: stat_update.old_value,
                        new_value: stat_update.new_value,
                        delta: stat_update.delta,
                        source: "tool_call".to_string(),
                    };
                    state.connections.broadcast_to_world(world_id, stat_msg).await;

                    tracing::info!(
                        approval_id = %approval_id,
                        character_id = %stat_update.character_id,
                        character_name = %stat_update.character_name,
                        stat_name = %stat_update.stat_name,
                        old_value = stat_update.old_value,
                        new_value = stat_update.new_value,
                        delta = stat_update.delta,
                        "Emitted CharacterStatUpdated for tool execution"
                    );
                }
            }
            None
        }
        Err(crate::use_cases::approval::ApprovalDecisionError::ApprovalNotFound(id)) => {
            Some(error_response(
                ErrorCode::NotFound,
                &format!("Approval request not found: {id}"),
            ))
        }
        Err(e) => Some(error_response(
            ErrorCode::InternalError,
            &sanitize_repo_error(&e, "process approval decision"),
        )),
    }
}
