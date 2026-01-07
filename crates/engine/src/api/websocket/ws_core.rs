use super::*;

use crate::api::connections::ConnectionInfo;

use wrldbldr_protocol::{
    CharacterRequest, ItemsRequest, LocationRequest, NpcRequest, TimeRequest, WorldRequest,
};

pub(super) async fn handle_world_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: WorldRequest,
) -> Result<ResponseResult, ServerMessage> {
    let _ = conn_info;
    match request {
        WorldRequest::ListWorlds => match state.app.entities.world.list_all().await {
            Ok(worlds) => {
                let data: Vec<serde_json::Value> = worlds
                    .into_iter()
                    .map(|w| {
                        serde_json::json!({
                            "id": w.id,
                            "name": w.name,
                            "description": w.description,
                        })
                    })
                    .collect();
                Ok(ResponseResult::success(data))
            }
            Err(e) => Ok(ResponseResult::error(
                ErrorCode::InternalError,
                e.to_string(),
            )),
        },

        WorldRequest::GetWorld { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(world)) => Ok(ResponseResult::success(serde_json::json!({
                    "id": world.id,
                    "name": world.name,
                    "description": world.description,
                }))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "World not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        other => {
            let msg = format!("This request type is not yet implemented: {:?}", other);
            Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
        }
    }
}

pub(super) async fn handle_character_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: CharacterRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        CharacterRequest::ListCharacters { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .entities
                .character
                .list_in_world(world_id_typed)
                .await
            {
                Ok(chars) => {
                    let data: Vec<serde_json::Value> = chars
                        .into_iter()
                        .map(|c| {
                            serde_json::json!({
                                "id": c.id,
                                "name": c.name,
                                "description": c.description,
                                "is_active": c.is_active,
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        CharacterRequest::GetCharacterInventory { character_id } => {
            let _ = conn_info;
            let char_uuid =
                match parse_uuid_for_request(&character_id, request_id, "Invalid character ID") {
                    Ok(id) => id,
                    Err(e) => return Err(e),
                };

            let pc_id = PlayerCharacterId::from_uuid(char_uuid);
            let items = match state.app.entities.inventory.get_pc_inventory(pc_id).await {
                Ok(items) => items,
                Err(_) => {
                    let npc_id = CharacterId::from_uuid(char_uuid);
                    state
                        .app
                        .entities
                        .inventory
                        .get_character_inventory(npc_id)
                        .await
                        .map_err(|e| ServerMessage::Response {
                            request_id: request_id.to_string(),
                            result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                        })?
                }
            };

            let data: Vec<serde_json::Value> = items
                .into_iter()
                .map(|item| {
                    serde_json::json!({
                        "id": item.id,
                        "name": item.name,
                        "description": item.description,
                        "item_type": item.item_type,
                        "is_unique": item.is_unique,
                        "properties": item.properties,
                    })
                })
                .collect();
            Ok(ResponseResult::success(data))
        }

        other => {
            let msg = format!("This request type is not yet implemented: {:?}", other);
            Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
        }
    }
}

pub(super) async fn handle_location_request(
    state: &WsState,
    request_id: &str,
    _conn_info: &ConnectionInfo,
    request: LocationRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        LocationRequest::ListLocations { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .entities
                .location
                .list_in_world(world_id_typed)
                .await
            {
                Ok(locations) => {
                    let data: Vec<serde_json::Value> = locations
                        .into_iter()
                        .map(|l| {
                            serde_json::json!({
                                "id": l.id,
                                "name": l.name,
                                "description": l.description,
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        other => {
            let msg = format!("This request type is not yet implemented: {:?}", other);
            Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
        }
    }
}

pub(super) async fn handle_time_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: TimeRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        TimeRequest::GetGameTime { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(world)) => {
                    let gt = &world.game_time;
                    let game_time = wrldbldr_protocol::types::GameTime {
                        day: gt.day_ordinal(),
                        hour: gt.current().hour() as u8,
                        minute: gt.current().minute() as u8,
                        is_paused: gt.is_paused(),
                    };
                    Ok(ResponseResult::success(serde_json::json!({
                        "game_time": game_time,
                    })))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "World not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        TimeRequest::AdvanceGameTime { world_id, hours } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed =
                match parse_uuid_for_request(&world_id, request_id, "Invalid world ID") {
                    Ok(uuid) => WorldId::from_uuid(uuid),
                    Err(e) => return Err(e),
                };

            let mut world = match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    });
                }
            };

            world.game_time.advance_hours(hours);
            world.updated_at = chrono::Utc::now();

            if let Err(e) = state.app.entities.world.save(&world).await {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                });
            }

            let gt = &world.game_time;
            let game_time = wrldbldr_protocol::types::GameTime {
                day: gt.day_ordinal(),
                hour: gt.current().hour() as u8,
                minute: gt.current().minute() as u8,
                is_paused: gt.is_paused(),
            };

            let update_msg = ServerMessage::GameTimeUpdated { game_time };
            state
                .connections
                .broadcast_to_world(world_id_typed, update_msg)
                .await;

            tracing::info!(
                world_id = %world_id_typed,
                hours_advanced = hours,
                new_day = gt.day_ordinal(),
                new_hour = gt.current().hour(),
                "Game time advanced"
            );

            Ok(ResponseResult::success(serde_json::json!({
                "game_time": game_time,
                "hours_advanced": hours,
            })))
        }

        TimeRequest::AdvanceGameTimeMinutes {
            world_id,
            minutes,
            reason: _reason,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed =
                match parse_uuid_for_request(&world_id, request_id, "Invalid world ID") {
                    Ok(uuid) => WorldId::from_uuid(uuid),
                    Err(e) => return Err(e),
                };

            let mut world = match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    });
                }
            };

            let previous_time = world.game_time.clone();
            let advance_reason = wrldbldr_domain::TimeAdvanceReason::DmManual {
                hours: minutes / 60,
            };
            let result = world.advance_time(minutes, advance_reason.clone(), chrono::Utc::now());

            if let Err(e) = state.app.entities.world.save(&world).await {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                });
            }

            let advance_data = crate::use_cases::time::build_time_advance_data(
                &previous_time,
                &result.new_time,
                minutes,
                &advance_reason,
            );
            let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
            state
                .connections
                .broadcast_to_world(world_id_typed, update_msg)
                .await;

            tracing::info!(
                world_id = %world_id_typed,
                minutes_advanced = minutes,
                "Game time advanced (minutes)"
            );

            let game_time = crate::use_cases::time::game_time_to_protocol(&world.game_time);
            Ok(ResponseResult::success(serde_json::json!({
                "game_time": game_time,
                "minutes_advanced": minutes,
            })))
        }

        TimeRequest::SetGameTime {
            world_id,
            day,
            hour,
            notify_players,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed =
                match parse_uuid_for_request(&world_id, request_id, "Invalid world ID") {
                    Ok(uuid) => WorldId::from_uuid(uuid),
                    Err(e) => return Err(e),
                };

            let mut world = match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    });
                }
            };

            let previous_time = world.game_time.clone();
            world.game_time.set_day_and_hour(day, hour as u32);
            world.updated_at = chrono::Utc::now();

            if let Err(e) = state.app.entities.world.save(&world).await {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                });
            }

            if notify_players {
                let reason = wrldbldr_domain::TimeAdvanceReason::DmSetTime;
                let advance_data = crate::use_cases::time::build_time_advance_data(
                    &previous_time,
                    &world.game_time,
                    0,
                    &reason,
                );
                let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
                state
                    .connections
                    .broadcast_to_world(world_id_typed, update_msg)
                    .await;
            }

            tracing::info!(
                world_id = %world_id_typed,
                new_day = day,
                new_hour = hour,
                "Game time set"
            );

            let game_time = crate::use_cases::time::game_time_to_protocol(&world.game_time);
            Ok(ResponseResult::success(serde_json::json!({
                "game_time": game_time,
            })))
        }

        TimeRequest::SkipToPeriod { world_id, period } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed =
                match parse_uuid_for_request(&world_id, request_id, "Invalid world ID") {
                    Ok(uuid) => WorldId::from_uuid(uuid),
                    Err(e) => return Err(e),
                };

            let target_period = match period.to_lowercase().as_str() {
                "morning" => wrldbldr_domain::TimeOfDay::Morning,
                "afternoon" => wrldbldr_domain::TimeOfDay::Afternoon,
                "evening" => wrldbldr_domain::TimeOfDay::Evening,
                "night" => wrldbldr_domain::TimeOfDay::Night,
                _ => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            "Invalid period. Use: morning, afternoon, evening, night",
                        ),
                    });
                }
            };

            let mut world = match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    });
                }
            };

            let previous_time = world.game_time.clone();
            let minutes_until = world.game_time.minutes_until_period(target_period);
            world.game_time.skip_to_period(target_period);
            world.updated_at = chrono::Utc::now();

            if let Err(e) = state.app.entities.world.save(&world).await {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                });
            }

            let reason = wrldbldr_domain::TimeAdvanceReason::DmSkipToPeriod {
                period: target_period,
            };
            let advance_data = crate::use_cases::time::build_time_advance_data(
                &previous_time,
                &world.game_time,
                minutes_until,
                &reason,
            );
            let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
            state
                .connections
                .broadcast_to_world(world_id_typed, update_msg)
                .await;

            tracing::info!(
                world_id = %world_id_typed,
                target_period = %target_period,
                "Skipped to time period"
            );

            let game_time = crate::use_cases::time::game_time_to_protocol(&world.game_time);
            Ok(ResponseResult::success(serde_json::json!({
                "game_time": game_time,
                "skipped_to": period,
            })))
        }

        TimeRequest::GetTimeConfig { world_id } => {
            let world_id_typed =
                match parse_uuid_for_request(&world_id, request_id, "Invalid world ID") {
                    Ok(uuid) => WorldId::from_uuid(uuid),
                    Err(e) => return Err(e),
                };

            match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(world)) => {
                    let config = &world.time_config;
                    Ok(ResponseResult::success(serde_json::json!({
                        "mode": format!("{:?}", config.mode).to_lowercase(),
                        "time_costs": {
                            "travel_location": config.time_costs.travel_location,
                            "travel_region": config.time_costs.travel_region,
                            "rest_short": config.time_costs.rest_short,
                            "rest_long": config.time_costs.rest_long,
                            "conversation": config.time_costs.conversation,
                            "challenge": config.time_costs.challenge,
                            "scene_transition": config.time_costs.scene_transition,
                        },
                        "show_time_to_players": config.show_time_to_players,
                    })))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "World not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        TimeRequest::UpdateTimeConfig { world_id, config } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed =
                match parse_uuid_for_request(&world_id, request_id, "Invalid world ID") {
                    Ok(uuid) => WorldId::from_uuid(uuid),
                    Err(e) => return Err(e),
                };

            let mut world = match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    });
                }
            };

            let mut normalized_config = config.clone();
            if matches!(
                normalized_config.mode,
                wrldbldr_protocol::types::TimeMode::Auto
            ) {
                normalized_config.mode = wrldbldr_protocol::types::TimeMode::Suggested;
            }

            let mode = match normalized_config.mode {
                wrldbldr_protocol::types::TimeMode::Manual => wrldbldr_domain::TimeMode::Manual,
                wrldbldr_protocol::types::TimeMode::Suggested => {
                    wrldbldr_domain::TimeMode::Suggested
                }
                wrldbldr_protocol::types::TimeMode::Auto => wrldbldr_domain::TimeMode::Suggested,
            };

            let time_costs = wrldbldr_domain::TimeCostConfig {
                travel_location: normalized_config.time_costs.travel_location,
                travel_region: normalized_config.time_costs.travel_region,
                rest_short: normalized_config.time_costs.rest_short,
                rest_long: normalized_config.time_costs.rest_long,
                conversation: normalized_config.time_costs.conversation,
                challenge: normalized_config.time_costs.challenge,
                scene_transition: normalized_config.time_costs.scene_transition,
            };

            world.time_config = wrldbldr_domain::GameTimeConfig {
                mode,
                time_costs,
                show_time_to_players: normalized_config.show_time_to_players,
                time_format: wrldbldr_domain::TimeFormat::TwelveHour,
            };
            world.updated_at = chrono::Utc::now();

            if let Err(e) = state.app.entities.world.save(&world).await {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                });
            }

            let update_msg = ServerMessage::TimeConfigUpdated {
                world_id: world_id_typed.to_string(),
                config: normalized_config,
            };
            state
                .connections
                .broadcast_to_dms(world_id_typed, update_msg)
                .await;

            tracing::info!(world_id = %world_id_typed, mode = ?mode, "Time config updated");

            Ok(ResponseResult::success_empty())
        }
    }
}

pub(super) async fn handle_npc_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: NpcRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        NpcRequest::ListCharacterRegionRelationships { character_id } => {
            let char_id_typed = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .entities
                .character
                .get_region_relationships(char_id_typed)
                .await
            {
                Ok(relationships) => {
                    let data: Vec<serde_json::Value> = relationships
                        .into_iter()
                        .map(|r| {
                            serde_json::json!({
                                "region_id": r.region_id.to_string(),
                                "relationship_type": format!("{}", r.relationship_type),
                                "shift": r.shift,
                                "frequency": r.frequency,
                                "time_of_day": r.time_of_day,
                                "reason": r.reason,
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::SetCharacterHomeRegion {
            character_id,
            region_id,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let char_uuid = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let region_uuid = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .entities
                .character
                .set_home_region(char_uuid, region_uuid)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::SetCharacterWorkRegion {
            character_id,
            region_id,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let char_uuid = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let region_uuid = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .entities
                .character
                .set_work_region(char_uuid, region_uuid, None)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::RemoveCharacterRegionRelationship {
            character_id,
            region_id,
            relationship_type,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let char_uuid = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let region_uuid = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .entities
                .character
                .remove_region_relationship(char_uuid, region_uuid, &relationship_type)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::ListRegionNpcs { region_id } => {
            let region_id_typed = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .entities
                .character
                .get_npcs_for_region(region_id_typed)
                .await
            {
                Ok(npcs) => {
                    let data: Vec<serde_json::Value> = npcs
                        .into_iter()
                        .map(|n| {
                            serde_json::json!({
                                "character_id": n.character_id.to_string(),
                                "name": n.name,
                                "sprite_asset": n.sprite_asset,
                                "portrait_asset": n.portrait_asset,
                                "relationship_type": format!("{}", n.relationship_type),
                                "shift": n.shift,
                                "frequency": n.frequency,
                                "time_of_day": n.time_of_day,
                                "reason": n.reason,
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::SetNpcMood {
            npc_id,
            region_id,
            mood,
            reason,
        } => {
            let npc_uuid = match Uuid::parse_str(&npc_id) {
                Ok(u) => CharacterId::from(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid NPC ID"),
                    })
                }
            };

            let region_uuid = match Uuid::parse_str(&region_id) {
                Ok(u) => RegionId::from(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid region ID"),
                    })
                }
            };

            let mood_state: MoodState = mood.parse().unwrap_or(MoodState::Calm);

            if !conn_info.is_dm() {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::Forbidden, "Only DM can set NPC mood"),
                });
            }

            let world_id = match conn_info.world_id {
                Some(wid) => wid,
                None => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            "Not connected to a world",
                        ),
                    })
                }
            };

            let npc = match state.app.entities.character.get(npc_uuid).await {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "NPC not found"),
                    })
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                    })
                }
            };

            let old_mood = state
                .app
                .entities
                .staging
                .get_npc_mood(region_uuid, npc_uuid)
                .await
                .unwrap_or(npc.default_mood);

            match state
                .app
                .entities
                .staging
                .set_npc_mood(region_uuid, npc_uuid, mood_state)
                .await
            {
                Ok(_) => {
                    let mood_changed_msg = ServerMessage::NpcMoodChanged {
                        npc_id: npc_id.to_string(),
                        npc_name: npc.name.clone(),
                        old_mood: old_mood.to_string(),
                        new_mood: mood_state.to_string(),
                        reason: reason.map(|s| s.to_string()),
                        region_id: Some(region_id.to_string()),
                    };

                    state
                        .connections
                        .broadcast_to_world(world_id, mood_changed_msg)
                        .await;

                    Ok(ResponseResult::success(serde_json::json!({
                        "npc_id": npc_id,
                        "region_id": region_id,
                        "mood": mood_state.to_string(),
                    })))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    &e.to_string(),
                )),
            }
        }

        NpcRequest::GetNpcMood { npc_id, region_id } => {
            let npc_uuid = match Uuid::parse_str(&npc_id) {
                Ok(u) => CharacterId::from(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid NPC ID"),
                    })
                }
            };

            let region_uuid = match Uuid::parse_str(&region_id) {
                Ok(u) => RegionId::from(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid region ID"),
                    })
                }
            };

            match state
                .app
                .entities
                .staging
                .get_npc_mood(region_uuid, npc_uuid)
                .await
            {
                Ok(mood) => Ok(ResponseResult::success(serde_json::json!({
                    "npc_id": npc_id,
                    "region_id": region_id,
                    "mood": mood.to_string(),
                    "default_expression": mood.default_expression(),
                }))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    &e.to_string(),
                )),
            }
        }

        other => {
            let msg = format!("This request type is not yet implemented: {:?}", other);
            Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
        }
    }
}

pub(super) async fn handle_items_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: ItemsRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        ItemsRequest::PlaceItemInRegion { region_id, item_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let region_uuid = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let item_uuid = match parse_item_id_for_request(&item_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .entities
                .inventory
                .place_item_in_region(item_uuid, region_uuid)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        ItemsRequest::CreateAndPlaceItem {
            world_id,
            region_id,
            data,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_uuid = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let region_uuid = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let mut item = wrldbldr_domain::Item::new(world_uuid, data.name.clone());
            if let Some(desc) = &data.description {
                item = item.with_description(desc.clone());
            }
            if let Some(item_type) = &data.item_type {
                item = item.with_type(item_type.clone());
            }
            if let Some(props) = &data.properties {
                item = item.with_properties(props.to_string());
            }

            match state
                .app
                .entities
                .inventory
                .create_and_place_in_region(item, region_uuid)
                .await
            {
                Ok(item_id) => Ok(ResponseResult::success(serde_json::json!({
                    "success": true,
                    "item_id": item_id.to_string(),
                }))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
    }
}
