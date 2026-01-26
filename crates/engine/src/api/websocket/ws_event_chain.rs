use super::*;
use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::use_cases::narrative::{CreateEventChainInput, UpdateEventChainInput};
use serde_json::json;
use wrldbldr_domain::{ActId, NarrativeEventId};
use wrldbldr_shared::{ErrorCode, EventChainRequest, ResponseResult};

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
                .use_cases
                .narrative
                .chains
                .list(world_id_typed)
                .await
            {
                Ok(chains) => Ok(ResponseResult::success(json!(chains))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "list event chains"),
                )),
            }
        }
        EventChainRequest::GetEventChain { chain_id } => {
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .chains
                .get(chain_id_typed)
                .await
            {
                Ok(Some(chain)) => Ok(ResponseResult::success(json!(chain))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Chain not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get event chain"),
                )),
            }
        }
        EventChainRequest::CreateEventChain { world_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            let act_id = parse_optional_act_id(data.act_id.clone(), request_id)?;
            let events = parse_event_ids(data.events.as_ref(), request_id)?;
            // Convert protocol data to domain input
            let input = CreateEventChainInput {
                name: data.name,
                description: data.description,
                tags: data.tags,
                color: data.color,
                is_active: data.is_active,
            };
            match state
                .app
                .use_cases
                .narrative
                .chains
                .create(world_id_typed, input, act_id, events)
                .await
            {
                Ok(chain) => Ok(ResponseResult::success(json!(chain))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create event chain"),
                )),
            }
        }
        EventChainRequest::UpdateEventChain { chain_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            let act_id = parse_optional_act_id(data.act_id.clone(), request_id)?;
            let events = parse_event_ids(data.events.as_ref(), request_id)?;
            // Convert protocol data to domain input
            let input = UpdateEventChainInput {
                name: data.name,
                description: data.description,
                tags: data.tags,
                color: data.color,
                is_active: data.is_active,
            };
            match state
                .app
                .use_cases
                .narrative
                .chains
                .update(chain_id_typed, input, act_id, events)
                .await
            {
                Ok(chain) => Ok(ResponseResult::success(json!(chain))),
                Err(crate::use_cases::narrative::EventChainError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Chain not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "update event chain"),
                )),
            }
        }
        EventChainRequest::DeleteEventChain { chain_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .chains
                .delete(chain_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete event chain"),
                )),
            }
        }
        EventChainRequest::SetEventChainActive { chain_id, active } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .chains
                .set_active(chain_id_typed, active)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::narrative::EventChainError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Chain not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "set event chain active"),
                )),
            }
        }
        EventChainRequest::SetEventChainFavorite { chain_id, favorite } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .chains
                .set_favorite(chain_id_typed, favorite)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::narrative::EventChainError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Chain not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "set event chain favorite"),
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
            let position = position.map(|pos| pos as usize);
            match state
                .app
                .use_cases
                .narrative
                .chains
                .add_event(chain_id_typed, event_id_typed, position)
                .await
            {
                Ok(chain) => Ok(ResponseResult::success(json!(chain))),
                Err(crate::use_cases::narrative::EventChainError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Chain not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "add event to chain"),
                )),
            }
        }
        EventChainRequest::RemoveEventFromChain { chain_id, event_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .chains
                .remove_event(chain_id_typed, event_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::narrative::EventChainError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Chain not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "remove event from chain"),
                )),
            }
        }
        EventChainRequest::CompleteChainEvent { chain_id, event_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            let event_id_typed = parse_narrative_event_id_for_request(&event_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .chains
                .complete_event(chain_id_typed, event_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::narrative::EventChainError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Chain not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "complete chain event"),
                )),
            }
        }
        EventChainRequest::ResetEventChain { chain_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .chains
                .reset(chain_id_typed)
                .await
            {
                Ok(chain) => Ok(ResponseResult::success(json!(chain))),
                Err(crate::use_cases::narrative::EventChainError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Chain not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "reset event chain"),
                )),
            }
        }
        EventChainRequest::GetEventChainStatus { chain_id } => {
            let chain_id_typed = parse_event_chain_id_for_request(&chain_id, request_id)?;
            match state
                .app
                .use_cases
                .narrative
                .chains
                .status(chain_id_typed)
                .await
            {
                Ok(status) => Ok(ResponseResult::success(json!(status))),
                Err(crate::use_cases::narrative::EventChainError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Chain not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get event chain status"),
                )),
            }
        }
    }
}

fn parse_event_ids(
    raw: Option<&Vec<String>>,
    request_id: &str,
) -> Result<Option<Vec<NarrativeEventId>>, ServerMessage> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    raw.iter()
        .map(|value| parse_narrative_event_id_from_str(value, request_id))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
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
