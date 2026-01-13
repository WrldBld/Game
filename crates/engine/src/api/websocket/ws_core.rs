use super::*;

use chrono::Timelike;

use crate::api::connections::ConnectionInfo;

use wrldbldr_protocol::{CharacterRequest, ItemsRequest, NpcRequest, TimeRequest, WorldRequest};

pub(super) async fn handle_world_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: WorldRequest,
) -> Result<ResponseResult, ServerMessage> {
    let _ = conn_info;
    match request {
        WorldRequest::ListWorlds => match state.app.use_cases.management.world.list().await {
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

            match state
                .app
                .use_cases
                .management
                .world
                .get(world_id_typed)
                .await
            {
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

        WorldRequest::CreateWorld { data } => {
            // Note: CreateWorld does NOT require DM auth - anyone can create a world.
            // The creator becomes the DM when they join the world.
            match state
                .app
                .use_cases
                .management
                .world
                .create(data.name, data.description, data.setting)
                .await
            {
                Ok(world) => Ok(ResponseResult::success(serde_json::json!({
                    "id": world.id.to_string(),
                    "name": world.name,
                    "description": world.description,
                }))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        WorldRequest::UpdateWorld { world_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .world
                .update(world_id_typed, data.name, data.description, data.setting)
                .await
            {
                Ok(world) => Ok(ResponseResult::success(serde_json::json!({
                    "id": world.id.to_string(),
                    "name": world.name,
                    "description": world.description,
                }))),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "World not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        WorldRequest::DeleteWorld { world_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .world
                .delete(world_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "World not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        WorldRequest::ExportWorld { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .world
                .export
                .execute(world_id_typed)
                .await
            {
                Ok(export) => Ok(ResponseResult::success(serde_json::json!(export))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        WorldRequest::GetSheetTemplate { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            // Get the world to determine its rule system
            let world = match state
                .app
                .use_cases
                .management
                .world
                .get(world_id_typed)
                .await
            {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "World not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        e.to_string(),
                    ));
                }
            };

            // Get the schema based on the world's rule system
            use wrldbldr_domain::game_systems::{
                BladesSystem, Coc7eSystem, Dnd5eSystem, FateCoreSystem, PbtaSystem, Pf2eSystem,
            };
            use wrldbldr_domain::{CharacterSheetProvider, RuleSystemVariant};

            let schema = match &world.rule_system.variant {
                RuleSystemVariant::Dnd5e => Some(Dnd5eSystem::new().character_sheet_schema()),
                RuleSystemVariant::Pathfinder2e => Some(Pf2eSystem::new().character_sheet_schema()),
                RuleSystemVariant::CallOfCthulhu7e => {
                    Some(Coc7eSystem::new().character_sheet_schema())
                }
                RuleSystemVariant::FateCore => {
                    Some(FateCoreSystem::new().character_sheet_schema())
                }
                RuleSystemVariant::BladesInTheDark => {
                    Some(BladesSystem::new().character_sheet_schema())
                }
                RuleSystemVariant::PoweredByApocalypse => {
                    Some(PbtaSystem::generic().character_sheet_schema())
                }
                RuleSystemVariant::KidsOnBikes => {
                    Some(PbtaSystem::generic().character_sheet_schema())
                }
                RuleSystemVariant::RuneQuest => Some(Coc7eSystem::new().character_sheet_schema()),
                RuleSystemVariant::GenericD20 | RuleSystemVariant::Custom(_) => {
                    Some(Dnd5eSystem::new().character_sheet_schema())
                }
                RuleSystemVariant::GenericD100 => {
                    Some(Coc7eSystem::new().character_sheet_schema())
                }
                RuleSystemVariant::Unknown => Some(Dnd5eSystem::new().character_sheet_schema()),
            };

            match schema {
                Some(schema) => Ok(ResponseResult::success(
                    serde_json::to_value(&schema).unwrap_or_else(|e| {
                        serde_json::json!({"error": format!("Failed to serialize schema: {}", e)})
                    }),
                )),
                None => Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    "No character sheet schema available for this game system",
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
                .use_cases
                .management
                .character
                .list_in_world(world_id_typed)
                .await
            {
                Ok(chars) => {
                    let data: Vec<serde_json::Value> = chars
                        .into_iter()
                        .map(|c| {
                            serde_json::json!({
                                "id": c.id.to_string(),
                                "name": c.name,
                                "archetype": Some(c.current_archetype.to_string()),
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

        CharacterRequest::GetCharacter { character_id } => {
            let char_id = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state.app.use_cases.management.character.get(char_id).await {
                Ok(Some(character)) => Ok(ResponseResult::success(serde_json::json!({
                    "id": character.id.to_string(),
                    "name": character.name,
                    "description": if character.description.is_empty() { None } else { Some(character.description) },
                    "archetype": Some(character.current_archetype.to_string()),
                    "sprite_asset": character.sprite_asset,
                    "portrait_asset": character.portrait_asset,
                    "sheet_data": serde_json::Value::Null,
                }))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        CharacterRequest::CreateCharacter { world_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .character
                .create(
                    world_id_typed,
                    data.name,
                    data.description,
                    data.archetype,
                    data.sprite_asset,
                    data.portrait_asset,
                )
                .await
            {
                Ok(character) => Ok(ResponseResult::success(serde_json::json!({
                    "id": character.id.to_string(),
                    "name": character.name,
                    "description": if character.description.is_empty() { None } else { Some(character.description) },
                    "archetype": Some(character.current_archetype.to_string()),
                    "sprite_asset": character.sprite_asset,
                    "portrait_asset": character.portrait_asset,
                    "sheet_data": serde_json::Value::Null,
                }))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        CharacterRequest::UpdateCharacter { character_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let char_id = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .character
                .update(
                    char_id,
                    data.name,
                    data.description,
                    data.sprite_asset,
                    data.portrait_asset,
                    data.is_alive,
                    data.is_active,
                )
                .await
            {
                Ok(character) => Ok(ResponseResult::success(serde_json::json!({
                    "id": character.id.to_string(),
                    "name": character.name,
                    "description": if character.description.is_empty() { None } else { Some(character.description) },
                    "archetype": Some(character.current_archetype.to_string()),
                    "sprite_asset": character.sprite_asset,
                    "portrait_asset": character.portrait_asset,
                    "sheet_data": serde_json::Value::Null,
                }))),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Character not found"),
                ),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        CharacterRequest::DeleteCharacter { character_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let char_id = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .character
                .delete(char_id)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Character not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        CharacterRequest::ChangeArchetype { character_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let char_id = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .character
                .change_archetype(char_id, data.new_archetype, data.reason)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Character not found"),
                ),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
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
            let items = match state
                .app
                .entities
                .inventory
                .get_pc_inventory(pc_id)
                .await
            {
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

            match state
                .app
                .use_cases
                .time
                .control
                .get_game_time(world_id_typed)
                .await
            {
                Ok(game_time) => Ok(ResponseResult::success(serde_json::json!({
                    "game_time": crate::use_cases::time::game_time_to_protocol(&game_time),
                }))),
                Err(crate::use_cases::time::TimeControlError::WorldNotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "World not found"),
                ),
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

            let outcome = match state
                .app
                .use_cases
                .time
                .control
                .advance_hours(world_id_typed, hours)
                .await
            {
                Ok(result) => result,
                Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
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

            let game_time = crate::use_cases::time::game_time_to_protocol(&outcome.new_time);
            let update_msg = ServerMessage::GameTimeUpdated { game_time };
            state
                .connections
                .broadcast_to_world(world_id_typed, update_msg)
                .await;

            tracing::info!(
                world_id = %world_id_typed,
                hours_advanced = hours,
                new_day = outcome.new_time.day_ordinal(),
                new_hour = outcome.new_time.current().hour(),
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

            let advance_reason = wrldbldr_domain::TimeAdvanceReason::DmManual {
                hours: minutes / 60,
            };
            let outcome = match state
                .app
                .use_cases
                .time
                .control
                .advance_minutes(world_id_typed, minutes, advance_reason.clone())
                .await
            {
                Ok(result) => result,
                Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
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

            let advance_data = crate::use_cases::time::build_time_advance_data(
                &outcome.previous_time,
                &outcome.new_time,
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

            let game_time = crate::use_cases::time::game_time_to_protocol(&outcome.new_time);
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

            let outcome = match state
                .app
                .use_cases
                .time
                .control
                .set_game_time(world_id_typed, day, hour)
                .await
            {
                Ok(result) => result,
                Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
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

            if notify_players {
                let reason = wrldbldr_domain::TimeAdvanceReason::DmSetTime;
                let advance_data = crate::use_cases::time::build_time_advance_data(
                    &outcome.previous_time,
                    &outcome.new_time,
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

            let game_time = crate::use_cases::time::game_time_to_protocol(&outcome.new_time);
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

            let outcome = match state
                .app
                .use_cases
                .time
                .control
                .skip_to_period(world_id_typed, target_period)
                .await
            {
                Ok(result) => result,
                Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
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

            let reason = wrldbldr_domain::TimeAdvanceReason::DmSkipToPeriod {
                period: target_period,
            };
            let advance_data = crate::use_cases::time::build_time_advance_data(
                &outcome.previous_time,
                &outcome.new_time,
                outcome.minutes_advanced,
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

            let game_time = crate::use_cases::time::game_time_to_protocol(&outcome.new_time);
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

            match state
                .app
                .use_cases
                .time
                .control
                .get_time_config(world_id_typed)
                .await
            {
                Ok(config) => Ok(ResponseResult::success(serde_json::json!({
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
                }))),
                Err(crate::use_cases::time::TimeControlError::WorldNotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "World not found"),
                ),
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

            let update = match state
                .app
                .use_cases
                .time
                .control
                .update_time_config(world_id_typed, config)
                .await
            {
                Ok(result) => result,
                Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
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

            let update_msg = ServerMessage::TimeConfigUpdated {
                world_id: update.world_id.to_string(),
                config: update.normalized_config,
            };
            state
                .connections
                .broadcast_to_dms(world_id_typed, update_msg)
                .await;

            tracing::info!(world_id = %world_id_typed, "Time config updated");

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
        NpcRequest::SetNpcDisposition {
            npc_id,
            pc_id,
            disposition,
            reason,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let npc_id_typed = match parse_character_id_for_request(&npc_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let pc_uuid = match parse_uuid_for_request(&pc_id, request_id, "Invalid PC ID") {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let pc_id_typed = PlayerCharacterId::from_uuid(pc_uuid);

            let disposition_level: wrldbldr_domain::DispositionLevel =
                disposition.parse().map_err(|_| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Invalid disposition value",
                    ),
                })?;

            if disposition_level == wrldbldr_domain::DispositionLevel::Unknown {
                return Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    "Invalid disposition value",
                ));
            }

            let update = match state
                .app
                .use_cases
                .npc
                .disposition
                .set_disposition(npc_id_typed, pc_id_typed, disposition_level, reason.clone())
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        e.to_string(),
                    ))
                }
            };

            if let Some(world_id) = conn_info.world_id {
                let msg = ServerMessage::NpcDispositionChanged {
                    npc_id: update.npc_id.to_string(),
                    npc_name: update.npc_name,
                    pc_id: update.pc_id.to_string(),
                    disposition: update.disposition.to_string(),
                    relationship: update.relationship.to_string(),
                    reason: update.reason,
                };
                state.connections.broadcast_to_dms(world_id, msg).await;
            }

            Ok(ResponseResult::success_empty())
        }

        NpcRequest::SetNpcRelationship {
            npc_id,
            pc_id,
            relationship,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let npc_id_typed = match parse_character_id_for_request(&npc_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let pc_uuid = match parse_uuid_for_request(&pc_id, request_id, "Invalid PC ID") {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let pc_id_typed = PlayerCharacterId::from_uuid(pc_uuid);

            let relationship_level: wrldbldr_domain::RelationshipLevel =
                relationship.parse().map_err(|_| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Invalid relationship value",
                    ),
                })?;

            if relationship_level == wrldbldr_domain::RelationshipLevel::Unknown {
                return Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    "Invalid relationship value",
                ));
            }

            let update = match state
                .app
                .use_cases
                .npc
                .disposition
                .set_relationship(npc_id_typed, pc_id_typed, relationship_level)
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        e.to_string(),
                    ))
                }
            };

            if let Some(world_id) = conn_info.world_id {
                let msg = ServerMessage::NpcDispositionChanged {
                    npc_id: update.npc_id.to_string(),
                    npc_name: update.npc_name,
                    pc_id: update.pc_id.to_string(),
                    disposition: update.disposition.to_string(),
                    relationship: update.relationship.to_string(),
                    reason: update.reason,
                };
                state.connections.broadcast_to_dms(world_id, msg).await;
            }

            Ok(ResponseResult::success_empty())
        }

        NpcRequest::GetNpcDispositions { pc_id } => {
            let pc_uuid = match parse_uuid_for_request(&pc_id, request_id, "Invalid PC ID") {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let pc_id_typed = PlayerCharacterId::from_uuid(pc_uuid);

            let domain_data = match state
                .app
                .use_cases
                .npc
                .disposition
                .list_for_pc(pc_id_typed)
                .await
            {
                Ok(list) => list,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        e.to_string(),
                    ))
                }
            };

            // Convert domain types to protocol types
            let dispositions = domain_data
                .into_iter()
                .map(|d| wrldbldr_protocol::NpcDispositionData {
                    npc_id: d.npc_id,
                    npc_name: d.npc_name,
                    disposition: d.disposition,
                    relationship: d.relationship,
                    sentiment: d.sentiment,
                    last_reason: d.last_reason,
                })
                .collect();

            let msg = ServerMessage::NpcDispositionsResponse {
                pc_id: pc_id_typed.to_string(),
                dispositions,
            };

            match state
                .connections
                .send_critical(conn_info.connection_id, msg)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(_) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    "Failed to send dispositions response",
                )),
            }
        }

        NpcRequest::ListCharacterRegionRelationships { character_id } => {
            let char_id_typed = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .npc
                .region_relationships
                .list_for_character(char_id_typed)
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
                Err(crate::use_cases::npc::NpcError::NotFound) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
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
                .use_cases
                .npc
                .region_relationships
                .set_home_region(char_uuid, region_uuid)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(crate::use_cases::npc::NpcError::NotFound) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
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
                .use_cases
                .npc
                .region_relationships
                .set_work_region(char_uuid, region_uuid)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(crate::use_cases::npc::NpcError::NotFound) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
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
                .use_cases
                .npc
                .region_relationships
                .remove_relationship(char_uuid, region_uuid, &relationship_type)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(crate::use_cases::npc::NpcError::NotFound) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
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
                .use_cases
                .npc
                .region_relationships
                .list_region_npcs(region_id_typed)
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
                Err(crate::use_cases::npc::NpcError::NotFound) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Region not found",
                )),
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

            match state
                .app
                .use_cases
                .npc
                .mood
                .set_mood(region_uuid, npc_uuid, mood_state)
                .await
            {
                Ok(change) => {
                    let mood_changed_msg = ServerMessage::NpcMoodChanged {
                        npc_id: change.npc_id.to_string(),
                        npc_name: change.npc_name.clone(),
                        old_mood: change.old_mood.to_string(),
                        new_mood: change.new_mood.to_string(),
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
                Err(crate::use_cases::npc::NpcError::NotFound) => {
                    Ok(ResponseResult::error(ErrorCode::NotFound, "NPC not found"))
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
                .use_cases
                .npc
                .mood
                .get_mood(region_uuid, npc_uuid)
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

            // Create the item using the domain builder pattern
            let mut item = wrldbldr_domain::Item::new(world_uuid, data.name);
            if let Some(desc) = data.description {
                item = item.with_description(desc);
            }
            if let Some(item_type) = data.item_type {
                item = item.with_type(item_type);
            }
            if let Some(props) = data.properties {
                item = item.with_properties(serde_json::to_string(&props).unwrap_or_default());
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
