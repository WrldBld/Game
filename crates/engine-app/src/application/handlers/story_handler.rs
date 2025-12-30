//! Story Event request handlers
//!
//! Handles: ListStoryEvents, GetStoryEvent, UpdateStoryEvent,
//! SetStoryEventVisibility, CreateDmMarker

use std::sync::Arc;

use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_protocol::{CreateDmMarkerData, ErrorCode, ResponseResult, UpdateStoryEventData};

use super::common::{parse_story_event_id, parse_world_id};
use crate::application::services::StoryEventService;

/// Handle ListStoryEvents request
pub async fn list_story_events(
    story_event_service: &Arc<dyn StoryEventService>,
    world_id: &str,
    page: Option<u32>,
    page_size: Option<u32>,
) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let page = page.unwrap_or(0);
    let page_size = page_size.unwrap_or(50);
    match story_event_service
        .list_by_world_paginated(id, page, page_size)
        .await
    {
        Ok(events) => {
            let dtos: Vec<serde_json::Value> = events
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "id": e.id.to_string(),
                        "world_id": e.world_id.to_string(),
                        "event_type": format!("{:?}", e.event_type),
                        "summary": e.summary,
                        "timestamp": e.timestamp.to_rfc3339(),
                        "game_time": e.game_time,
                        "is_hidden": e.is_hidden,
                    })
                })
                .collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetStoryEvent request
pub async fn get_story_event(
    story_event_service: &Arc<dyn StoryEventService>,
    event_id: &str,
) -> ResponseResult {
    let id = match parse_story_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match story_event_service.get_event(id).await {
        Ok(Some(event)) => {
            let dto = serde_json::json!({
                "id": event.id.to_string(),
                "world_id": event.world_id.to_string(),
                "event_type": format!("{:?}", event.event_type),
                "summary": event.summary,
                "timestamp": event.timestamp.to_rfc3339(),
                "game_time": event.game_time,
                "is_hidden": event.is_hidden,
            });
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Story event not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateStoryEvent request (DM only)
pub async fn update_story_event(
    story_event_service: &Arc<dyn StoryEventService>,
    ctx: &RequestContext,
    event_id: &str,
    data: UpdateStoryEventData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_story_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Update summary if provided
    if let Some(summary) = data.summary {
        if let Err(e) = story_event_service.update_summary(id, &summary).await {
            return ResponseResult::error(ErrorCode::InternalError, e.to_string());
        }
    }
    // Update tags if provided
    if let Some(tags) = data.tags {
        if let Err(e) = story_event_service.update_tags(id, tags).await {
            return ResponseResult::error(ErrorCode::InternalError, e.to_string());
        }
    }
    ResponseResult::success_empty()
}

/// Handle SetStoryEventVisibility request (DM only)
pub async fn set_story_event_visibility(
    story_event_service: &Arc<dyn StoryEventService>,
    ctx: &RequestContext,
    event_id: &str,
    visible: bool,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_story_event_id(event_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match story_event_service.set_visibility(id, visible).await {
        Ok(_) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateDmMarker request (DM only)
pub async fn create_dm_marker(
    story_event_service: &Arc<dyn StoryEventService>,
    ctx: &RequestContext,
    world_id: &str,
    data: CreateDmMarkerData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let wid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match story_event_service
        .create_dm_marker(wid, data.title, data.content)
        .await
    {
        Ok(event_id) => ResponseResult::success(serde_json::json!({
            "id": event_id.to_string(),
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
