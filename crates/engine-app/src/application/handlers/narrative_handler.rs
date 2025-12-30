//! Narrative domain request handlers
//!
//! Handles: NarrativeEvent and EventChain CRUD, triggering, favoriting, chain management

use std::sync::Arc;

use wrldbldr_domain::entities::{EventChain, NarrativeEvent};
use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_engine_ports::outbound::ClockPort;
use wrldbldr_protocol::{
    CreateEventChainData, CreateNarrativeEventData, ErrorCode, ResponseResult,
    UpdateEventChainData, UpdateNarrativeEventData,
};

use super::common::{parse_event_chain_id, parse_narrative_event_id, parse_world_id};
use crate::application::dto::{ChainStatusResponseDto, EventChainResponseDto, NarrativeEventResponseDto};
use crate::application::services::{EventChainService, NarrativeEventService};

// =============================================================================
// Narrative Event Handlers
// =============================================================================

/// Handle ListNarrativeEvents request
pub async fn list_narrative_events(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    world_id: &str,
) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match narrative_event_service.list_by_world(id).await {
        Ok(events) => {
            let dtos: Vec<NarrativeEventResponseDto> =
                events.into_iter().map(|e| e.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetNarrativeEvent request
pub async fn get_narrative_event(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    event_id: &str,
) -> ResponseResult {
    let id = match parse_narrative_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match narrative_event_service.get(id).await {
        Ok(Some(event)) => {
            let dto: NarrativeEventResponseDto = event.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Narrative event not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteNarrativeEvent request (DM only)
pub async fn delete_narrative_event(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    ctx: &RequestContext,
    event_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_narrative_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match narrative_event_service.delete(id).await {
        Ok(_) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetNarrativeEventActive request (DM only)
pub async fn set_narrative_event_active(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    ctx: &RequestContext,
    event_id: &str,
    active: bool,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_narrative_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match narrative_event_service.set_active(id, active).await {
        Ok(_) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetNarrativeEventFavorite request (DM only)
pub async fn set_narrative_event_favorite(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    ctx: &RequestContext,
    event_id: &str,
    _favorite: bool,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_narrative_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match narrative_event_service.toggle_favorite(id).await {
        Ok(is_favorite) => {
            ResponseResult::success(serde_json::json!({ "is_favorite": is_favorite }))
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle TriggerNarrativeEvent request (DM only)
pub async fn trigger_narrative_event(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    ctx: &RequestContext,
    event_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_narrative_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match narrative_event_service.mark_triggered(id, None).await {
        Ok(_) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle ResetNarrativeEvent request (DM only)
pub async fn reset_narrative_event(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    ctx: &RequestContext,
    event_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_narrative_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match narrative_event_service.reset_triggered(id).await {
        Ok(_) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateNarrativeEvent request (DM only)
pub async fn create_narrative_event(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    clock: &Arc<dyn ClockPort>,
    ctx: &RequestContext,
    world_id: &str,
    data: CreateNarrativeEventData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let wid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Create the narrative event entity
    let mut event = NarrativeEvent::new(wid, data.name, clock.now());
    event.description = data.description.unwrap_or_default();
    match narrative_event_service.create(event).await {
        Ok(created) => {
            let dto: NarrativeEventResponseDto = created.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateNarrativeEvent request (DM only)
pub async fn update_narrative_event(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    ctx: &RequestContext,
    event_id: &str,
    data: UpdateNarrativeEventData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_narrative_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Fetch existing event first
    let existing = match narrative_event_service.get(id).await {
        Ok(Some(e)) => e,
        Ok(None) => {
            return ResponseResult::error(ErrorCode::NotFound, "Narrative event not found")
        }
        Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    };
    // Apply updates
    let mut updated = existing;
    if let Some(name) = data.name {
        updated.name = name;
    }
    if let Some(description) = data.description {
        updated.description = description;
    }
    match narrative_event_service.update(updated).await {
        Ok(result) => {
            let dto: NarrativeEventResponseDto = result.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// Event Chain Handlers
// =============================================================================

/// Handle ListEventChains request
pub async fn list_event_chains(
    event_chain_service: &Arc<dyn EventChainService>,
    world_id: &str,
) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match event_chain_service.list_event_chains(id).await {
        Ok(chains) => {
            let dtos: Vec<EventChainResponseDto> = chains.into_iter().map(|c| c.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetEventChain request
pub async fn get_event_chain(
    event_chain_service: &Arc<dyn EventChainService>,
    chain_id: &str,
) -> ResponseResult {
    let id = match parse_event_chain_id(chain_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match event_chain_service.get_event_chain(id).await {
        Ok(Some(chain)) => {
            let dto: EventChainResponseDto = chain.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Event chain not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteEventChain request (DM only)
pub async fn delete_event_chain(
    event_chain_service: &Arc<dyn EventChainService>,
    ctx: &RequestContext,
    chain_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_event_chain_id(chain_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match event_chain_service.delete_event_chain(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateEventChain request (DM only)
pub async fn create_event_chain(
    event_chain_service: &Arc<dyn EventChainService>,
    clock: &Arc<dyn ClockPort>,
    ctx: &RequestContext,
    world_id: &str,
    data: CreateEventChainData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let wid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Create the event chain entity
    let mut chain = EventChain::new(wid, data.name, clock.now());
    chain.description = data.description.unwrap_or_default();
    match event_chain_service.create_event_chain(chain).await {
        Ok(created) => {
            let dto: EventChainResponseDto = created.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateEventChain request (DM only)
pub async fn update_event_chain(
    event_chain_service: &Arc<dyn EventChainService>,
    ctx: &RequestContext,
    chain_id: &str,
    data: UpdateEventChainData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_event_chain_id(chain_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Fetch existing chain first
    let existing = match event_chain_service.get_event_chain(id).await {
        Ok(Some(c)) => c,
        Ok(None) => return ResponseResult::error(ErrorCode::NotFound, "Event chain not found"),
        Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    };
    // Apply updates
    let mut updated = existing;
    if let Some(name) = data.name {
        updated.name = name;
    }
    if let Some(description) = data.description {
        updated.description = description;
    }
    match event_chain_service.update_event_chain(updated).await {
        Ok(result) => {
            let dto: EventChainResponseDto = result.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetEventChainActive request (DM only)
pub async fn set_event_chain_active(
    event_chain_service: &Arc<dyn EventChainService>,
    ctx: &RequestContext,
    chain_id: &str,
    active: bool,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_event_chain_id(chain_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match event_chain_service.set_active(id, active).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetEventChainFavorite request (DM only)
pub async fn set_event_chain_favorite(
    event_chain_service: &Arc<dyn EventChainService>,
    ctx: &RequestContext,
    chain_id: &str,
    _favorite: bool,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_event_chain_id(chain_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match event_chain_service.toggle_favorite(id).await {
        Ok(is_favorite) => {
            ResponseResult::success(serde_json::json!({ "is_favorite": is_favorite }))
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle AddEventToChain request (DM only)
pub async fn add_event_to_chain(
    event_chain_service: &Arc<dyn EventChainService>,
    ctx: &RequestContext,
    chain_id: &str,
    event_id: &str,
    _position: Option<i32>,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let cid = match parse_event_chain_id(chain_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let eid = match parse_narrative_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match event_chain_service.add_event_to_chain(cid, eid).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle RemoveEventFromChain request (DM only)
pub async fn remove_event_from_chain(
    event_chain_service: &Arc<dyn EventChainService>,
    ctx: &RequestContext,
    chain_id: &str,
    event_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let cid = match parse_event_chain_id(chain_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let eid = match parse_narrative_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match event_chain_service.remove_event_from_chain(cid, eid).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CompleteChainEvent request (DM only)
pub async fn complete_chain_event(
    event_chain_service: &Arc<dyn EventChainService>,
    ctx: &RequestContext,
    chain_id: &str,
    event_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let cid = match parse_event_chain_id(chain_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let eid = match parse_narrative_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match event_chain_service.complete_event(cid, eid).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle ResetEventChain request (DM only)
pub async fn reset_event_chain(
    event_chain_service: &Arc<dyn EventChainService>,
    ctx: &RequestContext,
    chain_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_event_chain_id(chain_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match event_chain_service.reset_chain(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetEventChainStatus request
pub async fn get_event_chain_status(
    event_chain_service: &Arc<dyn EventChainService>,
    chain_id: &str,
) -> ResponseResult {
    let id = match parse_event_chain_id(chain_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match event_chain_service.get_status(id).await {
        Ok(Some(status)) => {
            let dto: ChainStatusResponseDto = status.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Event chain not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
