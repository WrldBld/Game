use super::*;
use crate::api::connections::ConnectionInfo;
use chrono::Utc;
use serde_json::json;
use wrldbldr_domain::{self as domain, ActId, EventChain, NarrativeEventId};
use wrldbldr_protocol::{ErrorCode, EventChainRequest, ResponseResult};

pub(super) async fn handle_event_chain_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: EventChainRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        EventChainRequest::ListEventChains { world_id } => {
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            match state
                .app
                .entities
                .narrative
                .list_chains_for_world(world_id_typed)
                .await
            {
                Ok(chains) => {
                    let data: Vec<serde_json::Value> =
                        chains.iter().map(event_chain_to_json).collect();
                    Ok(ResponseResult::success(json!(data)))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::GetEventChain { chain_id } => {
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state.app.entities.narrative.get_chain(chain_id_typed).await {
                Ok(Some(chain)) => Ok(ResponseResult::success(json!(event_chain_to_json(&chain)))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Chain not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::CreateEventChain { world_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            let now = Utc::now();
            let mut chain = EventChain::new(world_id_typed, &data.name, now);
            if let Some(description) = data.description {
                chain.description = description;
            }
            if let Some(tags) = data.tags {
                chain.tags = tags;
            }
            if let Some(color) = data.color {
                chain.color = Some(color);
            }
            if let Some(active) = data.is_active {
                chain.is_active = active;
            }
            if let Some(act_id_str) = data.act_id {
                let act_id = parse_optional_act_id(Some(act_id_str), request_id)?;
                chain.act_id = act_id;
            }
            if let Some(events) = data.events {
                for event_id in events {
                    let parsed = parse_narrative_event_id_from_str(&event_id, request_id)?;
                    chain.add_event(parsed, now);
                }
            }
            match state.app.entities.narrative.save_chain(&chain).await {
                Ok(()) => Ok(ResponseResult::success(json!(event_chain_to_json(&chain)))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::UpdateEventChain { chain_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state.app.entities.narrative.get_chain(chain_id_typed).await {
                Ok(Some(mut chain)) => {
                    if let Some(name) = data.name {
                        chain.name = name;
                    }
                    if let Some(description) = data.description {
                        chain.description = description;
                    }
                    if let Some(tags) = data.tags {
                        chain.tags = tags;
                    }
                    if let Some(color) = data.color {
                        chain.color = Some(color);
                    }
                    if let Some(active) = data.is_active {
                        chain.is_active = active;
                    }
                    if let Some(act_id_str) = data.act_id {
                        chain.act_id = parse_optional_act_id(Some(act_id_str), request_id)?;
                    }
                    if let Some(events) = data.events {
                        let parsed_events = parse_event_ids(&events, request_id)?;
                        chain.reorder_events(parsed_events, Utc::now());
                    }
                    match state.app.entities.narrative.save_chain(&chain).await {
                        Ok(()) => Ok(ResponseResult::success(json!(event_chain_to_json(&chain)))),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Chain not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::DeleteEventChain { chain_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state
                .app
                .entities
                .narrative
                .delete_chain(chain_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::SetEventChainActive { chain_id, active } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state.app.entities.narrative.get_chain(chain_id_typed).await {
                Ok(Some(mut chain)) => {
                    let now = Utc::now();
                    if active {
                        chain.activate(now);
                    } else {
                        chain.deactivate(now);
                    }
                    match state.app.entities.narrative.save_chain(&chain).await {
                        Ok(()) => Ok(ResponseResult::success_empty()),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Chain not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::SetEventChainFavorite { chain_id, favorite } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state.app.entities.narrative.get_chain(chain_id_typed).await {
                Ok(Some(mut chain)) => {
                    chain.is_favorite = favorite;
                    match state.app.entities.narrative.save_chain(&chain).await {
                        Ok(()) => Ok(ResponseResult::success_empty()),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Chain not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::AddEventToChain {
            chain_id,
            event_id,
            position,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state.app.entities.narrative.get_chain(chain_id_typed).await {
                Ok(Some(mut chain)) => {
                    let now = Utc::now();
                    if let Some(pos) = position {
                        let insert_position = pos as usize;
                        chain.insert_event(insert_position, event_id_typed, now);
                    } else {
                        chain.add_event(event_id_typed, now);
                    }
                    match state.app.entities.narrative.save_chain(&chain).await {
                        Ok(()) => Ok(ResponseResult::success(json!(event_chain_to_json(&chain)))),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Chain not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::RemoveEventFromChain { chain_id, event_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state.app.entities.narrative.get_chain(chain_id_typed).await {
                Ok(Some(mut chain)) => {
                    let now = Utc::now();
                    chain.remove_event(&event_id_typed, now);
                    match state.app.entities.narrative.save_chain(&chain).await {
                        Ok(()) => Ok(ResponseResult::success_empty()),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Chain not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::CompleteChainEvent { chain_id, event_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state.app.entities.narrative.get_chain(chain_id_typed).await {
                Ok(Some(mut chain)) => {
                    let now = Utc::now();
                    chain.complete_event(event_id_typed, now);
                    match state.app.entities.narrative.save_chain(&chain).await {
                        Ok(()) => Ok(ResponseResult::success_empty()),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Chain not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::ResetEventChain { chain_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state.app.entities.narrative.get_chain(chain_id_typed).await {
                Ok(Some(mut chain)) => {
                    chain.reset(Utc::now());
                    match state.app.entities.narrative.save_chain(&chain).await {
                        Ok(()) => Ok(ResponseResult::success(json!(event_chain_to_json(&chain)))),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Chain not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        EventChainRequest::GetEventChainStatus { chain_id } => {
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state.app.entities.narrative.get_chain(chain_id_typed).await {
                Ok(Some(chain)) => {
                    let status: domain::ChainStatus = (&chain).into();
                    Ok(ResponseResult::success(json!(chain_status_to_json(
                        &status
                    ))))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Chain not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
    }
}

fn event_chain_to_json(chain: &EventChain) -> serde_json::Value {
    json!({
        "id": chain.id.to_string(),
        "world_id": chain.world_id.to_string(),
        "name": chain.name,
        "description": chain.description,
        "events": chain
            .events
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>(),
        "is_active": chain.is_active,
        "current_position": chain.current_position,
        "completed_events": chain
            .completed_events
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>(),
        "act_id": chain.act_id.map(|id| id.to_string()),
        "tags": chain.tags,
        "color": chain.color,
        "is_favorite": chain.is_favorite,
        "progress_percent": (chain.progress() * 100.0) as u32,
        "is_complete": chain.is_complete(),
        "remaining_events": chain.remaining_events(),
        "created_at": chain.created_at.to_rfc3339(),
        "updated_at": chain.updated_at.to_rfc3339(),
    })
}

fn chain_status_to_json(status: &domain::ChainStatus) -> serde_json::Value {
    json!({
        "chain_id": status.chain_id.to_string(),
        "chain_name": status.chain_name,
        "is_active": status.is_active,
        "is_complete": status.is_complete,
        "total_events": status.total_events,
        "completed_events": status.completed_events,
        "progress_percent": status.progress_percent,
        "current_event_id": status.current_event_id.map(|id| id.to_string()),
    })
}

fn parse_event_ids(
    raw: &[String],
    request_id: &str,
) -> Result<Vec<NarrativeEventId>, ServerMessage> {
    raw.iter()
        .map(|value| parse_narrative_event_id_from_str(value, request_id))
        .collect()
}

fn parse_narrative_event_id_from_str(
    value: &str,
    request_id: &str,
) -> Result<NarrativeEventId, ServerMessage> {
    parse_uuid_for_request(value, request_id, "Invalid event ID format").map(NarrativeEventId::from)
}

fn parse_optional_act_id(
    value: Option<String>,
    request_id: &str,
) -> Result<Option<ActId>, ServerMessage> {
    Ok(match value {
        Some(id) => Some(
            parse_uuid_for_request(&id, request_id, "Invalid act ID format").map(ActId::from)?,
        ),
        None => None,
    })
}
