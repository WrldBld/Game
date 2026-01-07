use super::*;
use crate::api::connections::ConnectionInfo;
use crate::use_cases::narrative::EffectExecutionContext;
use chrono::Utc;
use serde_json::json;
use wrldbldr_domain::{self as domain, NarrativeEvent, NarrativeTrigger};
use wrldbldr_protocol::{ErrorCode, NarrativeEventRequest, ResponseResult, TriggerSchema};

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
                .entities
                .narrative
                .list_events(world_id_typed)
                .await
            {
                Ok(events) => {
                    let data: Vec<serde_json::Value> =
                        events.iter().map(narrative_event_to_json).collect();
                    Ok(ResponseResult::success(json!(data)))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        NarrativeEventRequest::GetNarrativeEvent { event_id } => {
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state.app.entities.narrative.get_event(event_id_typed).await {
                Ok(Some(event)) => Ok(ResponseResult::success(json!(narrative_event_to_json(
                    &event
                )))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Event not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        NarrativeEventRequest::CreateNarrativeEvent { world_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            let now = Utc::now();
            let mut event = NarrativeEvent::new(world_id_typed, &data.name, now);
            if let Some(description) = data.description {
                event.description = description;
            }
            if let Some(triggers) = parse_optional_triggers(data.trigger_conditions, request_id)? {
                event.trigger_conditions = triggers;
            }
            if let Some(outcomes) = parse_optional_outcomes(data.outcomes, request_id)? {
                event.outcomes = outcomes;
            }
            match state.app.entities.narrative.save_event(&event).await {
                Ok(()) => Ok(ResponseResult::success(json!(narrative_event_to_json(
                    &event
                )))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        NarrativeEventRequest::UpdateNarrativeEvent { event_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state.app.entities.narrative.get_event(event_id_typed).await {
                Ok(Some(mut event)) => {
                    if let Some(name) = data.name {
                        event.name = name;
                    }
                    if let Some(description) = data.description {
                        event.description = description;
                    }
                    if let Some(triggers) =
                        parse_optional_triggers(data.trigger_conditions, request_id)?
                    {
                        event.trigger_conditions = triggers;
                    }
                    if let Some(outcomes) = parse_optional_outcomes(data.outcomes, request_id)? {
                        event.outcomes = outcomes;
                    }
                    match state.app.entities.narrative.save_event(&event).await {
                        Ok(()) => Ok(ResponseResult::success(json!(narrative_event_to_json(
                            &event
                        )))),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Event not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        NarrativeEventRequest::DeleteNarrativeEvent { event_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state
                .app
                .entities
                .narrative
                .delete_event(event_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        NarrativeEventRequest::SetNarrativeEventActive { event_id, active } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state.app.entities.narrative.get_event(event_id_typed).await {
                Ok(Some(mut event)) => {
                    event.is_active = active;
                    match state.app.entities.narrative.save_event(&event).await {
                        Ok(()) => Ok(ResponseResult::success_empty()),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Event not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        NarrativeEventRequest::SetNarrativeEventFavorite { event_id, favorite } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state.app.entities.narrative.get_event(event_id_typed).await {
                Ok(Some(mut event)) => {
                    event.is_favorite = favorite;
                    match state.app.entities.narrative.save_event(&event).await {
                        Ok(()) => Ok(ResponseResult::success_empty()),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Event not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        NarrativeEventRequest::TriggerNarrativeEvent { event_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            if conn_info.world_id.is_none() {
                return Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    "Must join a world before triggering events",
                ));
            }
            let world_id = conn_info.world_id.unwrap();
            match state.app.entities.narrative.get_event(event_id_typed).await {
                Ok(Some(mut event)) => {
                    let outcome_name = event
                        .selected_outcome
                        .clone()
                        .or_else(|| event.default_outcome.clone())
                        .or_else(|| event.outcomes.first().map(|o| o.name.clone()))
                        .unwrap_or_default();

                    event.is_triggered = true;
                    event.selected_outcome = Some(outcome_name.clone());
                    event.triggered_at = Some(Utc::now());
                    event.trigger_count = event.trigger_count.saturating_add(1);
                    let maybe_outcome = event.outcomes.iter().find(|o| o.name == outcome_name);
                    match state.app.entities.narrative.save_event(&event).await {
                        Ok(()) => { /* continue */ }
                        Err(e) => {
                            return Ok(ResponseResult::error(
                                ErrorCode::InternalError,
                                e.to_string(),
                            ))
                        }
                    }

                    if let Some(outcome) = maybe_outcome {
                        if !outcome.effects.is_empty() {
                            if let Some(pc_id) = conn_info.pc_id {
                                let context = EffectExecutionContext {
                                    pc_id,
                                    world_id,
                                    current_scene_id: None,
                                };

                                let summary = state
                                    .app
                                    .use_cases
                                    .narrative
                                    .execute_effects
                                    .execute(
                                        event.id,
                                        outcome.name.clone(),
                                        &outcome.effects,
                                        &context,
                                    )
                                    .await;

                                tracing::info!(
                                    event_id = %event.id,
                                    outcome = %outcome.name,
                                    success_count = summary.success_count,
                                    failure_count = summary.failure_count,
                                    "Narrative event effects executed"
                                );
                            } else {
                                tracing::warn!(
                                    "No PC context provided for narrative event effects execution"
                                );
                            }
                        }
                    }

                    let event_name = event.name.clone();
                    let outcome_description = maybe_outcome
                        .map(|o| o.description.clone())
                        .unwrap_or_default();
                    state
                        .connections
                        .broadcast_to_world(
                            world_id,
                            ServerMessage::NarrativeEventTriggered {
                                event_id: event.id.to_string(),
                                event_name,
                                outcome_description,
                                scene_direction: event.scene_direction.clone(),
                            },
                        )
                        .await;

                    Ok(ResponseResult::success(json!({
                        "event_id": event.id.to_string(),
                        "outcome": outcome_name,
                    })))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Event not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        NarrativeEventRequest::ResetNarrativeEvent { event_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state.app.entities.narrative.get_event(event_id_typed).await {
                Ok(Some(mut event)) => {
                    event.is_triggered = false;
                    event.selected_outcome = None;
                    event.triggered_at = None;
                    event.trigger_count = 0;
                    match state.app.entities.narrative.save_event(&event).await {
                        Ok(()) => Ok(ResponseResult::success(json!(narrative_event_to_json(
                            &event
                        )))),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Event not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        NarrativeEventRequest::GetTriggerSchema => {
            let schema = TriggerSchema::generate();
            Ok(ResponseResult::success(json!(schema)))
        }
    }
}

fn narrative_event_to_json(event: &NarrativeEvent) -> serde_json::Value {
    json!({
        "id": event.id.to_string(),
        "world_id": event.world_id.to_string(),
        "name": event.name,
        "description": event.description,
        "scene_direction": event.scene_direction,
        "suggested_opening": event.suggested_opening,
        "trigger_count": event.trigger_count,
        "is_active": event.is_active,
        "is_triggered": event.is_triggered,
        "triggered_at": event.triggered_at.map(|dt| dt.to_rfc3339()),
        "selected_outcome": event.selected_outcome,
        "is_repeatable": event.is_repeatable,
        "delay_turns": event.delay_turns,
        "expires_after_turns": event.expires_after_turns,
        "priority": event.priority,
        "is_favorite": event.is_favorite,
        "tags": event.tags,
        "scene_id": Option::<String>::None,
        "location_id": Option::<String>::None,
        "act_id": Option::<String>::None,
        "chain_id": Option::<String>::None,
        "chain_position": Option::<u32>::None,
        "outcome_count": event.outcomes.len(),
        "trigger_condition_count": event.trigger_conditions.len(),
        "created_at": event.created_at.to_rfc3339(),
        "updated_at": event.updated_at.to_rfc3339(),
    })
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
                    result: ResponseResult::error(ErrorCode::BadRequest, e.to_string()),
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
                    result: ResponseResult::error(ErrorCode::BadRequest, e.to_string()),
                })
            }
        }
    } else {
        None
    })
}
