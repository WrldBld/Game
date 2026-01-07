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
        StoryEventRequest::GetStoryEvent { .. } | StoryEventRequest::UpdateStoryEvent { .. } => {
            let msg = "This request type is not yet implemented";
            Ok(ResponseResult::error(ErrorCode::BadRequest, msg))
        }

        StoryEventRequest::ListStoryEvents {
            world_id,
            page: _,
            page_size,
        } => {
            let world_uuid = match Uuid::parse_str(&world_id) {
                Ok(u) => wrldbldr_domain::WorldId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world_id"),
                    })
                }
            };

            // We don't support offset pagination in the repo yet; treat page_size as a limit.
            let limit = page_size.unwrap_or(100).min(500) as usize;

            let events = state
                .app
                .entities
                .narrative
                .list_story_events(world_uuid, limit)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            let data = events
                .into_iter()
                .map(|e| {
                    let event_type = match &e.event_type {
                        wrldbldr_domain::StoryEventType::LocationChange {
                            from_location,
                            to_location,
                            character_id,
                            travel_method,
                        } => serde_json::json!({
                            "type": "location_change",
                            "from_location": from_location.as_ref().map(|id| id.to_string()),
                            "to_location": to_location.to_string(),
                            "character_id": character_id.to_string(),
                            "travel_method": travel_method,
                        }),

                        wrldbldr_domain::StoryEventType::DialogueExchange {
                            npc_id,
                            npc_name,
                            player_dialogue,
                            npc_response,
                            topics_discussed,
                            tone,
                        } => serde_json::json!({
                            "type": "dialogue_exchange",
                            "npc_id": npc_id.to_string(),
                            "npc_name": npc_name,
                            "player_dialogue": player_dialogue,
                            "npc_response": npc_response,
                            "topics_discussed": topics_discussed,
                            "tone": tone,
                        }),

                        wrldbldr_domain::StoryEventType::DmMarker {
                            title,
                            note,
                            importance,
                            marker_type,
                        } => serde_json::json!({
                            "type": "dm_marker",
                            "title": title,
                            "note": note,
                            "importance": format!("{:?}", importance),
                            "marker_type": format!("{:?}", marker_type),
                        }),

                        wrldbldr_domain::StoryEventType::NarrativeEventTriggered {
                            narrative_event_id,
                            narrative_event_name,
                            outcome_branch,
                            effects_applied,
                        } => serde_json::json!({
                            "type": "narrative_event_triggered",
                            "narrative_event_id": narrative_event_id.to_string(),
                            "narrative_event_name": narrative_event_name,
                            "outcome_branch": outcome_branch,
                            "effects_applied": effects_applied,
                        }),

                        wrldbldr_domain::StoryEventType::SessionStarted {
                            session_number,
                            session_name,
                            players_present,
                        } => serde_json::json!({
                            "type": "session_started",
                            "session_number": session_number,
                            "session_name": session_name,
                            "players_present": players_present,
                        }),

                        wrldbldr_domain::StoryEventType::SessionEnded {
                            duration_minutes,
                            summary,
                        } => serde_json::json!({
                            "type": "session_ended",
                            "duration_minutes": duration_minutes,
                            "summary": summary,
                        }),

                        other => serde_json::json!({
                            "type": "custom",
                            "event_subtype": e.type_name(),
                            "title": e.type_name(),
                            "description": format!("{:?}", other),
                        }),
                    };

                    serde_json::json!({
                        "id": e.id.to_string(),
                        "world_id": e.world_id.to_string(),
                        "scene_id": serde_json::Value::Null,
                        "location_id": serde_json::Value::Null,
                        "event_type": event_type,
                        "timestamp": e.timestamp.to_rfc3339(),
                        "game_time": e.game_time,
                        "summary": e.summary,
                        "involved_characters": Vec::<String>::new(),
                        "is_hidden": e.is_hidden,
                        "tags": e.tags,
                        "triggered_by": serde_json::Value::Null,
                        "type_name": e.type_name(),
                    })
                })
                .collect::<Vec<_>>();

            Ok(ResponseResult::success(data))
        }

        StoryEventRequest::CreateDmMarker { world_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_uuid = match Uuid::parse_str(&world_id) {
                Ok(u) => wrldbldr_domain::WorldId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world_id"),
                    })
                }
            };

            let now = chrono::Utc::now();
            let event = wrldbldr_domain::StoryEvent {
                id: wrldbldr_domain::StoryEventId::new(),
                world_id: world_uuid,
                event_type: wrldbldr_domain::StoryEventType::DmMarker {
                    title: data.title.clone(),
                    note: data.content.clone().unwrap_or_default(),
                    importance: wrldbldr_domain::MarkerImportance::Notable,
                    marker_type: wrldbldr_domain::DmMarkerType::Note,
                },
                timestamp: now,
                game_time: None,
                summary: "DM Marker".to_string(),
                is_hidden: false,
                tags: Vec::new(),
            };

            state
                .app
                .entities
                .narrative
                .save_story_event(&event)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            Ok(ResponseResult::success(serde_json::json!({
                "id": event.id.to_string()
            })))
        }

        StoryEventRequest::SetStoryEventVisibility { event_id, visible } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let event_uuid = match Uuid::parse_str(&event_id) {
                Ok(u) => wrldbldr_domain::StoryEventId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid event_id"),
                    })
                }
            };

            let mut ev = match state
                .app
                .entities
                .narrative
                .get_story_event(event_uuid)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })? {
                Some(ev) => ev,
                None => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "Story event not found"),
                    })
                }
            };

            ev.is_hidden = !visible;

            state
                .app
                .entities
                .narrative
                .save_story_event(&ev)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            Ok(ResponseResult::success(serde_json::json!({
                "id": ev.id.to_string(),
                "is_hidden": ev.is_hidden,
            })))
        }
    }
}
