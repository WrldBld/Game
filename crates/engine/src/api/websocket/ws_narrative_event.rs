use super::*;
use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::use_cases::narrative::decision::NarrativeDecisionError;
use serde_json::json;
use wrldbldr_domain::{self as domain, NarrativeTrigger};
use wrldbldr_shared::{ErrorCode, NarrativeEventRequest, ResponseResult, TriggerSchema};

pub(super) async fn handle_narrative_event_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: NarrativeEventRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        NarrativeEventRequest::ListNarrativeEvents { world_id } => {
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .events
                .list(world_id_typed)
                .await
            {
                Ok(events) => Ok(ResponseResult::success(json!(events))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "list narrative events"),
                )),
            }
        }
        NarrativeEventRequest::GetNarrativeEvent { event_id } => {
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .events
                .get(event_id_typed)
                .await
            {
                Ok(Some(event)) => Ok(ResponseResult::success(json!(event))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Event not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get narrative event"),
                )),
            }
        }
        NarrativeEventRequest::CreateNarrativeEvent { world_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            let trigger_conditions = parse_optional_triggers(data.trigger_conditions, request_id)?;
            let outcomes = parse_optional_outcomes(data.outcomes, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .events
                .create(
                    world_id_typed,
                    data.name,
                    data.description,
                    trigger_conditions,
                    outcomes,
                )
                .await
            {
                Ok(event) => Ok(ResponseResult::success(json!(event))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create narrative event"),
                )),
            }
        }
        NarrativeEventRequest::UpdateNarrativeEvent { event_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            let trigger_conditions = parse_optional_triggers(data.trigger_conditions, request_id)?;
            let outcomes = parse_optional_outcomes(data.outcomes, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .events
                .update(
                    event_id_typed,
                    data.name,
                    data.description,
                    trigger_conditions,
                    outcomes,
                )
                .await
            {
                Ok(event) => Ok(ResponseResult::success(json!(event))),
                Err(crate::use_cases::narrative::NarrativeEventError::NotFound(_)) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Event not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "update narrative event"),
                )),
            }
        }
        NarrativeEventRequest::DeleteNarrativeEvent { event_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .events
                .delete(event_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete narrative event"),
                )),
            }
        }
        NarrativeEventRequest::SetNarrativeEventActive { event_id, active } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .events
                .set_active(event_id_typed, active)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::narrative::NarrativeEventError::NotFound(_)) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Event not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "set event active"),
                )),
            }
        }
        NarrativeEventRequest::SetNarrativeEventFavorite { event_id, favorite } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .events
                .set_favorite(event_id_typed, favorite)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::narrative::NarrativeEventError::NotFound(_)) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Event not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "set event favorite"),
                )),
            }
        }
        NarrativeEventRequest::TriggerNarrativeEvent { event_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            let Some(world_id) = conn_info.world_id else {
                return Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    "Must join a world before triggering events",
                ));
            };
            match state
                .app
                .use_cases
                .narrative
                .events
                .trigger(event_id_typed, world_id, conn_info.pc_id)
                .await
            {
                Ok(result) => {
                    if let Some(summary) = result.effects_summary.as_ref() {
                        tracing::info!(
                            event_id = %result.event_id,
                            outcome = %result.outcome_name,
                            success_count = summary.success_count,
                            failure_count = summary.failure_count,
                            "Narrative event effects executed"
                        );
                    } else if result.effects_present {
                        tracing::warn!(
                            event_id = %result.event_id,
                            "No PC context provided for narrative event effects execution"
                        );
                    }

                    state
                        .connections
                        .broadcast_to_world(
                            world_id,
                            ServerMessage::NarrativeEventTriggered {
                                event_id: result.event_id.to_string(),
                                event_name: result.event_name.clone(),
                                outcome_description: result.outcome_description.clone(),
                                scene_direction: result.scene_direction.clone(),
                            },
                        )
                        .await;

                    Ok(ResponseResult::success(json!({
                        "event_id": result.event_id.to_string(),
                        "outcome": result.outcome_name,
                    })))
                }
                Err(crate::use_cases::narrative::NarrativeEventError::NotFound(_)) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Event not found"),
                ),
                Err(crate::use_cases::narrative::NarrativeEventError::WorldMismatch) => {
                    Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Event does not belong to current world",
                    ))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "trigger narrative event"),
                )),
            }
        }
        NarrativeEventRequest::ResetNarrativeEvent { event_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .events
                .reset(event_id_typed)
                .await
            {
                Ok(event) => Ok(ResponseResult::success(json!(event))),
                Err(crate::use_cases::narrative::NarrativeEventError::NotFound(_)) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Event not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "reset narrative event"),
                )),
            }
        }
        NarrativeEventRequest::GetTriggerSchema => {
            let schema = TriggerSchema::generate();
            Ok(ResponseResult::success(json!(schema)))
        }
    }
}

pub(super) async fn handle_narrative_event_decision(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    event_id: String,
    approved: bool,
    selected_outcome: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can make decisions
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

    let decision = if approved {
        crate::queue_types::DmApprovalDecision::Accept
    } else {
        crate::queue_types::DmApprovalDecision::Reject {
            feedback: "Narrative event rejected by DM".to_string(),
        }
    };

    let narrative_event_id =
        match parse_id(&event_id, NarrativeEventId::from_uuid, "Invalid event ID") {
            Ok(id) => id,
            Err(e) => return Some(e),
        };

    match state
        .app
        .use_cases
        .narrative
        .decision_flow
        .execute(approval_id.into(), decision, narrative_event_id, selected_outcome)
        .await
    {
        Ok(result) => {
            if let Some(triggered) = result.triggered {
                let msg = ServerMessage::NarrativeEventTriggered {
                    event_id: triggered.event_id,
                    event_name: triggered.event_name,
                    outcome_description: triggered.outcome_description,
                    scene_direction: triggered.scene_direction,
                };
                state
                    .connections
                    .broadcast_to_world(result.world_id, msg)
                    .await;
            }
            None
        }
        Err(NarrativeDecisionError::ApprovalNotFound) => Some(error_response(
            ErrorCode::NotFound,
            "Approval request not found",
        )),
        Err(e) => Some(error_response(
            ErrorCode::InternalError,
            &sanitize_repo_error(&e, "process narrative event decision"),
        )),
    }
}

fn parse_optional_triggers(
    value: Option<serde_json::Value>,
    request_id: &str,
) -> Result<Option<Vec<NarrativeTrigger>>, ServerMessage> {
    Ok(if let Some(value) = value {
        match serde_json::from_value::<Vec<NarrativeTrigger>>(value) {
            Ok(triggers) => Some(triggers),
            Err(e) => {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::BadRequest,
                        sanitize_repo_error(&e, "parse narrative triggers"),
                    ),
                })
            }
        }
    } else {
        None
    })
}

fn parse_optional_outcomes(
    value: Option<serde_json::Value>,
    request_id: &str,
) -> Result<Option<Vec<domain::EventOutcome>>, ServerMessage> {
    Ok(if let Some(value) = value {
        match serde_json::from_value::<Vec<domain::EventOutcome>>(value) {
            Ok(outcomes) => Some(outcomes),
            Err(e) => {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::BadRequest,
                        sanitize_repo_error(&e, "parse event outcomes"),
                    ),
                })
            }
        }
    } else {
        None
    })
}
