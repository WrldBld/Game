use super::*;

use crate::api::connections::ConnectionInfo;

use wrldbldr_protocol::StoryEventRequest;

pub(super) async fn handle_story_event_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: StoryEventRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        StoryEventRequest::GetStoryEvent { event_id } => {
            let event_uuid = match parse_uuid_for_request(&event_id, request_id, "Invalid event_id")
            {
                Ok(u) => wrldbldr_domain::StoryEventId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state.app.use_cases.story_events.ops.get(event_uuid).await {
                Ok(Some(event)) => Ok(ResponseResult::success(event)),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Story event not found",
                )),
                Err(crate::use_cases::story_events::StoryEventError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Story event not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        StoryEventRequest::UpdateStoryEvent { event_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let event_uuid = match parse_uuid_for_request(&event_id, request_id, "Invalid event_id")
            {
                Ok(u) => wrldbldr_domain::StoryEventId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .story_events
                .ops
                .update(event_uuid, data)
                .await
            {
                Ok(event) => Ok(ResponseResult::success(event)),
                Err(crate::use_cases::story_events::StoryEventError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Story event not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        StoryEventRequest::ListStoryEvents {
            world_id,
            page: _,
            page_size,
        } => {
            let world_uuid = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            // We don't support offset pagination in the repo yet; treat page_size as a limit.
            let limit = page_size.unwrap_or(100).min(500) as usize;

            match state
                .app
                .use_cases
                .story_events
                .ops
                .list(world_uuid, limit)
                .await
            {
                Ok(events) => Ok(ResponseResult::success(events)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        StoryEventRequest::CreateDmMarker { world_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_uuid = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .story_events
                .ops
                .create_dm_marker(world_uuid, data)
                .await
            {
                Ok(event_id) => Ok(ResponseResult::success(serde_json::json!({
                    "id": event_id.to_string(),
                }))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        StoryEventRequest::SetStoryEventVisibility { event_id, visible } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let event_uuid = match parse_uuid_for_request(&event_id, request_id, "Invalid event_id")
            {
                Ok(u) => wrldbldr_domain::StoryEventId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .story_events
                .ops
                .set_visibility(event_uuid, visible)
                .await
            {
                Ok(event) => Ok(ResponseResult::success(event)),
                Err(crate::use_cases::story_events::StoryEventError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Story event not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
    }
}
