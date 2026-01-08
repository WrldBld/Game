use super::*;

use std::collections::{HashMap, HashSet};

use crate::api::connections::ConnectionInfo;
use wrldbldr_domain::{LlmRequestType, WorldId};

use wrldbldr_protocol::{AiRequest, ExpressionRequest, GenerationRequest};

#[derive(Debug, Default, Clone)]
pub struct GenerationReadState {
    pub read_batches: HashSet<String>,
    pub read_suggestions: HashSet<String>,
}

fn map_queue_status_to_batch_status(
    status: crate::infrastructure::ports::QueueItemStatus,
) -> &'static str {
    match status {
        crate::infrastructure::ports::QueueItemStatus::Pending => "queued",
        crate::infrastructure::ports::QueueItemStatus::Processing => "generating",
        crate::infrastructure::ports::QueueItemStatus::Completed => "ready",
        crate::infrastructure::ports::QueueItemStatus::Failed => "failed",
    }
}

fn map_queue_status_to_suggestion_status(
    status: crate::infrastructure::ports::QueueItemStatus,
) -> &'static str {
    match status {
        crate::infrastructure::ports::QueueItemStatus::Pending => "queued",
        crate::infrastructure::ports::QueueItemStatus::Processing => "processing",
        crate::infrastructure::ports::QueueItemStatus::Completed => "ready",
        crate::infrastructure::ports::QueueItemStatus::Failed => "failed",
    }
}

pub(super) async fn handle_generation_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: GenerationRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        GenerationRequest::GetGenerationQueue { world_id, user_id } => {
            let world_uuid = match Uuid::parse_str(&world_id) {
                Ok(u) => WorldId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world_id"),
                    })
                }
            };

            // Prefer explicit user_id (for forward compatibility), fallback to connection user_id.
            let effective_user_id = user_id
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| conn_info.user_id.clone());

            let read_key = format!("{}:{}", effective_user_id, world_uuid);

            // Read-through cache: check memory first, then fall back to persisted state.
            let mut read_state = {
                let read_map = state.generation_read_state.read().await;
                read_map.get(&read_key).cloned()
            };

            if read_state.is_none() {
                let persisted = state
                    .app
                    .queue
                    .get_generation_read_state(effective_user_id.as_str(), world_uuid)
                    .await
                    .map_err(|e| ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    })?;

                if let Some((read_batches, read_suggestions)) = persisted {
                    read_state = Some(GenerationReadState {
                        read_batches: read_batches.into_iter().collect(),
                        read_suggestions: read_suggestions.into_iter().collect(),
                    });
                }

                // Populate cache even if empty, to avoid repeated DB reads.
                let mut map = state.generation_read_state.write().await;
                map.insert(read_key.clone(), read_state.clone().unwrap_or_default());
            }

            let read_state = read_state.unwrap_or_default();

            // Compute queue position for pending asset_generation items in this world.
            let asset_items = state
                .app
                .queue
                .list_by_type("asset_generation", 500)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                })?;

            let mut pending_asset_ids_in_order: Vec<String> = asset_items
                .iter()
                .filter(|item| {
                    item.status == crate::infrastructure::ports::QueueItemStatus::Pending
                })
                .filter_map(|item| match &item.data {
                    crate::infrastructure::ports::QueueItemData::AssetGeneration(d) => {
                        if d.world_id == Some(world_uuid) {
                            Some(item.id.to_string())
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .collect();

            // Items returned newest-first; for position we want oldest-first.
            pending_asset_ids_in_order.reverse();
            let pending_positions: HashMap<String, u32> = pending_asset_ids_in_order
                .into_iter()
                .enumerate()
                .map(|(idx, id)| (id, (idx as u32) + 1))
                .collect();

            let batches: Vec<serde_json::Value> = asset_items
                .into_iter()
                .filter_map(|item| {
                    let crate::infrastructure::ports::QueueItemData::AssetGeneration(d) = item.data
                    else {
                        return None;
                    };

                    if d.world_id != Some(world_uuid) {
                        return None;
                    }

                    let batch_id = item.id.to_string();
                    let position = pending_positions.get(&batch_id).copied();

                    Some(serde_json::json!({
                        "batch_id": batch_id,
                        "entity_type": d.entity_type,
                        "entity_id": d.entity_id,
                        // The protocol expects asset_type; the queue stores workflow_id.
                        "asset_type": d.workflow_id,
                        "status": map_queue_status_to_batch_status(item.status),
                        "position": position,
                        "progress": serde_json::Value::Null,
                        "asset_count": serde_json::Value::Null,
                        "error": item.error_message,
                        "is_read": read_state.read_batches.contains(&batch_id),
                    }))
                })
                .collect();

            let llm_items = state
                .app
                .queue
                .list_by_type("llm_request", 500)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                })?;

            let suggestions: Vec<serde_json::Value> = llm_items
                .into_iter()
                .filter_map(|item| {
                    let crate::infrastructure::ports::QueueItemData::LlmRequest(d) = item.data
                    else {
                        return None;
                    };

                    if d.world_id != world_uuid {
                        return None;
                    }

                    let LlmRequestType::Suggestion {
                        field_type,
                        entity_id,
                    } = d.request_type
                    else {
                        return None;
                    };

                    let request_key = d.callback_id.clone();
                    let mut suggestions: Option<Vec<String>> = None;

                    if let Some(result_json) = &item.result_json {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(result_json) {
                            suggestions =
                                v.get("suggestions").and_then(|s| s.as_array()).map(|arr| {
                                    arr.iter()
                                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                        .collect()
                                });
                        }
                    }

                    Some(serde_json::json!({
                        "request_id": request_key,
                        "field_type": field_type,
                        "entity_id": entity_id,
                        "status": map_queue_status_to_suggestion_status(item.status),
                        "suggestions": suggestions,
                        "error": item.error_message,
                        "is_read": read_state.read_suggestions.contains(&request_key),
                    }))
                })
                .collect();

            Ok(ResponseResult::success(serde_json::json!({
                "batches": batches,
                "suggestions": suggestions,
            })))
        }

        GenerationRequest::SyncGenerationReadState {
            world_id,
            read_batches,
            read_suggestions,
        } => {
            let world_uuid = match Uuid::parse_str(&world_id) {
                Ok(u) => WorldId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world_id"),
                    })
                }
            };

            // Persist first so state survives restarts.
            state
                .app
                .queue
                .upsert_generation_read_state(
                    &conn_info.user_id,
                    world_uuid,
                    &read_batches,
                    &read_suggestions,
                )
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                })?;

            let read_key = format!("{}:{}", conn_info.user_id, world_uuid);
            let mut map = state.generation_read_state.write().await;
            let entry = map.entry(read_key).or_default();

            entry.read_batches = read_batches.iter().cloned().collect();
            entry.read_suggestions = read_suggestions.iter().cloned().collect();

            Ok(ResponseResult::success_empty())
        }
    }
}

pub(super) async fn handle_ai_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: AiRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        AiRequest::EnqueueContentSuggestion {
            world_id,
            suggestion_type,
            context,
        } => {
            let world_uuid = match Uuid::parse_str(&world_id) {
                Ok(u) => WorldId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world_id"),
                    })
                }
            };

            let result = state
                .app
                .use_cases
                .ai
                .suggestions
                .enqueue_content_suggestion(world_uuid, suggestion_type.to_string(), context)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                })?;

            // Best-effort broadcast to world so the queue UI can update.
            state
                .connections
                .broadcast_to_world(
                    result.world_id,
                    ServerMessage::SuggestionQueued {
                        request_id: result.request_id.clone(),
                        field_type: result.field_type.clone(),
                        entity_id: result.entity_id.clone(),
                    },
                )
                .await;

            Ok(ResponseResult::success(serde_json::json!({
                "request_id": result.request_id,
                "status": "queued",
            })))
        }

        AiRequest::CancelContentSuggestion { request_id: rid } => {
            let cancelled = state
                .app
                .use_cases
                .ai
                .suggestions
                .cancel_content_suggestion(rid.as_str())
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                })?;

            Ok(ResponseResult::success(serde_json::json!({
                "cancelled": cancelled,
            })))
        }

        AiRequest::SuggestWantDescription { .. }
        | AiRequest::SuggestActantialReason { .. }
        | AiRequest::SuggestDeflectionBehavior { .. }
        | AiRequest::SuggestBehavioralTells { .. } => {
            // These are legacy/creator utilities; gate behind DM for now.
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let Some(world_uuid) = conn_info.world_id else {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Must join a world before requesting suggestions",
                    ),
                });
            };

            let result = match request {
                AiRequest::SuggestWantDescription { npc_id, context } => {
                    let npc_id_typed = parse_character_id(&npc_id)?;
                    state
                        .app
                        .use_cases
                        .ai
                        .suggestions
                        .suggest_want_description(world_uuid, npc_id_typed, context)
                        .await
                }
                AiRequest::SuggestDeflectionBehavior {
                    npc_id,
                    want_id,
                    want_description,
                } => {
                    let npc_id_typed = parse_character_id(&npc_id)?;
                    state
                        .app
                        .use_cases
                        .ai
                        .suggestions
                        .suggest_deflection_behavior(
                            world_uuid,
                            npc_id_typed,
                            want_id,
                            want_description,
                        )
                        .await
                }
                AiRequest::SuggestBehavioralTells {
                    npc_id,
                    want_id,
                    want_description,
                } => {
                    let npc_id_typed = parse_character_id(&npc_id)?;
                    state
                        .app
                        .use_cases
                        .ai
                        .suggestions
                        .suggest_behavioral_tells(
                            world_uuid,
                            npc_id_typed,
                            want_id,
                            want_description,
                        )
                        .await
                }
                AiRequest::SuggestActantialReason {
                    npc_id,
                    want_id,
                    target_id,
                    role,
                } => {
                    let npc_id_typed = parse_character_id(&npc_id)?;
                    state
                        .app
                        .use_cases
                        .ai
                        .suggestions
                        .suggest_actantial_reason(
                            world_uuid,
                            npc_id_typed,
                            want_id,
                            target_id,
                            role,
                        )
                        .await
                }
                _ => unreachable!(),
            }
            .map_err(|e| ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            })?;

            // Best-effort broadcast to world so the queue UI can update.
            state
                .connections
                .broadcast_to_world(
                    world_uuid,
                    ServerMessage::SuggestionQueued {
                        request_id: result.request_id.clone(),
                        field_type: result.field_type.clone(),
                        entity_id: result.entity_id.clone(),
                    },
                )
                .await;

            Ok(ResponseResult::success(serde_json::json!({
                "request_id": result.request_id,
                "status": "queued",
            })))
        }

        other => {
            let msg = format!("This request type is not yet implemented: {:?}", other);
            Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
        }
    }
}

pub(super) async fn handle_expression_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: ExpressionRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        ExpressionRequest::GenerateExpressionSheet {
            character_id,
            workflow,
            expressions,
            grid_layout,
            style_prompt,
        } => {
            // DM-only for now.
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let character_uuid = match Uuid::parse_str(&character_id) {
                Ok(u) => wrldbldr_domain::CharacterId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            "Invalid character_id",
                        ),
                    })
                }
            };

            let (cols, rows) = match grid_layout {
                Some(s) if !s.trim().is_empty() => {
                    let parts: Vec<&str> = s.split('x').collect();
                    if parts.len() == 2 {
                        let c = parts[0].trim().parse::<u32>().unwrap_or(4);
                        let r = parts[1].trim().parse::<u32>().unwrap_or(4);
                        (c.max(1), r.max(1))
                    } else {
                        (4, 4)
                    }
                }
                _ => (4, 4),
            };

            let exprs: Vec<String> = expressions.map(|xs| xs.to_vec()).unwrap_or_else(|| {
                crate::use_cases::assets::expression_sheet::STANDARD_EXPRESSION_ORDER
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });

            let req = crate::use_cases::assets::expression_sheet::ExpressionSheetRequest {
                character_id: character_uuid,
                source_asset_id: None,
                expressions: exprs,
                grid_layout: (cols, rows),
                workflow: workflow.to_string(),
                style_prompt: style_prompt.map(|s| s.to_string()),
            };

            let result = state
                .app
                .use_cases
                .assets
                .expression_sheet
                .queue(req)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                })?;

            Ok(ResponseResult::success(serde_json::json!({
                "batch_id": result.batch_id.to_string(),
                "character_id": result.character_id.to_string(),
                "expressions": result.expressions,
            })))
        }
    }
}
