use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::use_cases::time::TimeAdvanceResultData;
use wrldbldr_domain::{ConnectionId, GameTime, TimeAdvanceReason, TimeOfDay};
use wrldbldr_shared::types as protocol;
use wrldbldr_shared::{ErrorCode, TimeRequest};

pub(super) async fn handle_set_game_time(
    state: &WsState,
    connection_id: ConnectionId,
    world_id: String,
    day: u32,
    hour: u8,
    notify_players: bool,
) -> Option<ServerMessage> {
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

    let world_id_typed = match parse_world_id(&world_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let outcome = match state
        .app
        .use_cases
        .time
        .control
        .set_game_time(world_id_typed, day, hour)
        .await
    {
        Ok(outcome) => outcome,
        Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
            return Some(error_response(ErrorCode::NotFound, "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "getting time config"),
            ))
        }
    };

    if notify_players {
        let reason = TimeAdvanceReason::DmSetTime;
        let domain_data = crate::use_cases::time::build_time_advance_data(
            &outcome.previous_time,
            &outcome.new_time,
            outcome.minutes_advanced,
            &reason,
        );
        let advance_data = time_advance_data_to_protocol(&domain_data);
        let msg = ServerMessage::GameTimeAdvanced { data: advance_data };
        state
            .connections
            .broadcast_to_world(world_id_typed, msg)
            .await;
    }

    tracing::info!(world_id = %world_id_typed, day = day, hour = hour, "Game time set");
    None
}

pub(super) async fn handle_skip_to_period(
    state: &WsState,
    connection_id: ConnectionId,
    world_id: String,
    period: String,
) -> Option<ServerMessage> {
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

    let world_id_typed = match parse_world_id(&world_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let period_typed = match parse_time_of_day(&period) {
        Some(value) => value,
        None => {
            return Some(error_response(
                ErrorCode::ValidationError,
                "Invalid time period value",
            ))
        }
    };

    let outcome = match state
        .app
        .use_cases
        .time
        .control
        .skip_to_period(world_id_typed, period_typed)
        .await
    {
        Ok(outcome) => outcome,
        Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
            return Some(error_response(ErrorCode::NotFound, "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "skipping to time period"),
            ))
        }
    };

    let reason = TimeAdvanceReason::DmSkipToPeriod {
        period: period_typed,
    };
    let domain_data = crate::use_cases::time::build_time_advance_data(
        &outcome.previous_time,
        &outcome.new_time,
        outcome.minutes_advanced,
        &reason,
    );
    let advance_data = time_advance_data_to_protocol(&domain_data);
    let msg = ServerMessage::GameTimeAdvanced { data: advance_data };
    state
        .connections
        .broadcast_to_world(world_id_typed, msg)
        .await;

    tracing::info!(world_id = %world_id_typed, period = %period, "Game time skipped to period");
    None
}

pub(super) async fn handle_pause_game_time(
    state: &WsState,
    connection_id: ConnectionId,
    world_id: String,
    paused: bool,
) -> Option<ServerMessage> {
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

    let world_id_typed = match parse_world_id(&world_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    match state
        .app
        .use_cases
        .time
        .control
        .set_paused(world_id_typed, paused)
        .await
    {
        Ok(_time) => {}
        Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
            return Some(error_response(ErrorCode::NotFound, "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "pausing game time"),
            ))
        }
    }

    let msg = ServerMessage::GameTimePaused {
        world_id: world_id_typed.to_string(),
        paused,
    };
    state
        .connections
        .broadcast_to_world(world_id_typed, msg)
        .await;

    tracing::info!(world_id = %world_id_typed, paused = paused, "Game time pause state changed");
    None
}

pub(super) async fn handle_set_time_mode(
    state: &WsState,
    connection_id: ConnectionId,
    world_id: String,
    mode: protocol::TimeMode,
) -> Option<ServerMessage> {
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

    let world_id_typed = match parse_world_id(&world_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let mut config = match state
        .app
        .use_cases
        .time
        .control
        .get_time_config(world_id_typed)
        .await
    {
        Ok(config) => config,
        Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
            return Some(error_response(ErrorCode::NotFound, "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "getting time config"),
            ))
        }
    };

    // Convert protocol mode to domain mode at API boundary
    config.set_mode(protocol_time_mode_to_domain(mode));

    match state
        .app
        .use_cases
        .time
        .control
        .update_time_config(world_id_typed, config)
        .await
    {
        Ok(_) => {}
        Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
            return Some(error_response(ErrorCode::NotFound, "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "setting time mode"),
            ))
        }
    }

    let msg = ServerMessage::TimeModeChanged {
        world_id: world_id_typed.to_string(),
        mode,
    };
    state
        .connections
        .broadcast_to_world(world_id_typed, msg)
        .await;

    None
}

pub(super) async fn handle_set_time_costs(
    state: &WsState,
    connection_id: ConnectionId,
    world_id: String,
    costs: protocol::TimeCostConfig,
) -> Option<ServerMessage> {
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

    let world_id_typed = match parse_world_id(&world_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let mut config = match state
        .app
        .use_cases
        .time
        .control
        .get_time_config(world_id_typed)
        .await
    {
        Ok(config) => config,
        Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
            return Some(error_response(ErrorCode::NotFound, "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "getting time config"),
            ))
        }
    };

    // Convert protocol time costs to domain at API boundary
    config.set_time_costs(protocol_time_costs_to_domain(&costs));

    let updated = match state
        .app
        .use_cases
        .time
        .control
        .update_time_config(world_id_typed, config)
        .await
    {
        Ok(updated) => updated,
        Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
            return Some(error_response(ErrorCode::NotFound, "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "setting time costs"),
            ))
        }
    };

    // Convert domain config back to protocol for the response
    let msg = ServerMessage::TimeConfigUpdated {
        world_id: world_id_typed.to_string(),
        config: time_config_to_protocol(&updated.config),
    };
    state
        .connections
        .broadcast_to_dms(world_id_typed, msg)
        .await;

    None
}

pub(super) async fn handle_respond_to_time_suggestion(
    state: &WsState,
    connection_id: ConnectionId,
    suggestion_id: String,
    decision: protocol::TimeSuggestionDecision,
) -> Option<ServerMessage> {
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

    let suggestion_uuid = match Uuid::parse_str(&suggestion_id) {
        Ok(id) => id,
        Err(_) => {
            return Some(error_response(
                ErrorCode::ValidationError,
                "Invalid time suggestion ID",
            ))
        }
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Must join a world first",
            ))
        }
    };

    // Convert protocol decision to domain decision at the API boundary
    let domain_decision: wrldbldr_domain::TimeSuggestionDecision = match decision.try_into() {
        Ok(d) => d,
        Err(_) => {
            return Some(error_response(
                ErrorCode::ValidationError,
                "Unknown time suggestion decision type",
            ))
        }
    };

    let time_suggestions =
        crate::stores::TimeSuggestionStore::new(state.pending_time_suggestions.clone());
    match state
        .app
        .use_cases
        .time
        .suggestions
        .resolve(
            &time_suggestions,
            world_id,
            suggestion_uuid.into(),
            domain_decision,
        )
        .await
    {
        Ok(Some(resolution)) => {
            // Convert domain data to protocol at API boundary
            let advance_data = time_advance_data_to_protocol(&resolution.advance_data);
            let msg = ServerMessage::GameTimeAdvanced { data: advance_data };
            state.connections.broadcast_to_world(world_id, msg).await;
            None
        }
        Ok(None) => None,
        Err(crate::use_cases::time::TimeSuggestionError::NotFound(_)) => Some(error_response(
            ErrorCode::NotFound,
            "Time suggestion not found",
        )),
        Err(crate::use_cases::time::TimeSuggestionError::WorldMismatch) => Some(error_response(
            ErrorCode::ValidationError,
            "Time suggestion world mismatch",
        )),
        Err(e) => Some(error_response(
            ErrorCode::InternalError,
            &sanitize_repo_error(&e, "responding to time suggestion"),
        )),
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
                    "game_time": game_time_to_protocol(&game_time),
                }))),
                Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "World not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        TimeRequest::AdvanceGameTime { world_id, hours } => {
            require_dm_for_request(conn_info, request_id)?;

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
                Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::InternalError,
                            sanitize_repo_error(&e, "advance game time"),
                        ),
                    });
                }
            };

            let protocol_game_time = game_time_to_protocol(&outcome.new_time);
            let update_msg = ServerMessage::GameTimeUpdated {
                game_time: protocol_game_time.clone(),
            };
            state
                .connections
                .broadcast_to_world(world_id_typed, update_msg)
                .await;

            tracing::info!(
                world_id = %world_id_typed,
                hours_advanced = hours,
                new_day = outcome.new_time.day_ordinal(),
                new_hour = outcome.new_time.hour(),
                "Game time advanced"
            );

            Ok(ResponseResult::success(serde_json::json!({
                "game_time": protocol_game_time,
                "hours_advanced": hours,
            })))
        }

        TimeRequest::AdvanceGameTimeMinutes {
            world_id,
            minutes,
            reason: _reason,
        } => {
            require_dm_for_request(conn_info, request_id)?;

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
                Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::InternalError,
                            sanitize_repo_error(&e, "advance game minutes"),
                        ),
                    });
                }
            };

            let domain_data = crate::use_cases::time::build_time_advance_data(
                &outcome.previous_time,
                &outcome.new_time,
                minutes,
                &advance_reason,
            );
            let advance_data = time_advance_data_to_protocol(&domain_data);
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

            let protocol_game_time = game_time_to_protocol(&outcome.new_time);
            Ok(ResponseResult::success(serde_json::json!({
                "game_time": protocol_game_time,
                "minutes_advanced": minutes,
            })))
        }

        TimeRequest::SetGameTime {
            world_id,
            day,
            hour,
            notify_players,
        } => {
            require_dm_for_request(conn_info, request_id)?;

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
                Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::InternalError,
                            sanitize_repo_error(&e, "set game time"),
                        ),
                    });
                }
            };

            if notify_players {
                let reason = wrldbldr_domain::TimeAdvanceReason::DmSetTime;
                let domain_data = crate::use_cases::time::build_time_advance_data(
                    &outcome.previous_time,
                    &outcome.new_time,
                    0,
                    &reason,
                );
                let advance_data = time_advance_data_to_protocol(&domain_data);
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

            let protocol_game_time = game_time_to_protocol(&outcome.new_time);
            Ok(ResponseResult::success(serde_json::json!({
                "game_time": protocol_game_time,
            })))
        }

        TimeRequest::SkipToPeriod { world_id, period } => {
            require_dm_for_request(conn_info, request_id)?;

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
                Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::InternalError,
                            sanitize_repo_error(&e, "skip to period"),
                        ),
                    });
                }
            };

            let reason = wrldbldr_domain::TimeAdvanceReason::DmSkipToPeriod {
                period: target_period,
            };
            let domain_data = crate::use_cases::time::build_time_advance_data(
                &outcome.previous_time,
                &outcome.new_time,
                outcome.minutes_advanced,
                &reason,
            );
            let advance_data = time_advance_data_to_protocol(&domain_data);
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

            let protocol_game_time = game_time_to_protocol(&outcome.new_time);
            Ok(ResponseResult::success(serde_json::json!({
                "game_time": protocol_game_time,
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
                    "mode": format!("{:?}", config.mode()).to_lowercase(),
                    "time_costs": {
                        "travel_location": config.time_costs().travel_location,
                        "travel_region": config.time_costs().travel_region,
                        "rest_short": config.time_costs().rest_short,
                        "rest_long": config.time_costs().rest_long,
                        "conversation": config.time_costs().conversation,
                        "challenge": config.time_costs().challenge,
                        "scene_transition": config.time_costs().scene_transition,
                    },
                    "show_time_to_players": config.show_time_to_players(),
                }))),
                Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "World not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "pause game"),
                )),
            }
        }

        TimeRequest::UpdateTimeConfig { world_id, config } => {
            require_dm_for_request(conn_info, request_id)?;

            let world_id_typed =
                match parse_uuid_for_request(&world_id, request_id, "Invalid world ID") {
                    Ok(uuid) => WorldId::from_uuid(uuid),
                    Err(e) => return Err(e),
                };

            // Convert protocol config to domain config at API boundary
            let domain_config = protocol_time_config_to_domain(&config);

            let update = match state
                .app
                .use_cases
                .time
                .control
                .update_time_config(world_id_typed, domain_config)
                .await
            {
                Ok(result) => result,
                Err(crate::use_cases::time::TimeControlError::WorldNotFound(_)) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::InternalError,
                            sanitize_repo_error(&e, "update time config"),
                        ),
                    });
                }
            };

            // Convert domain config back to protocol for broadcasting
            let update_msg = ServerMessage::TimeConfigUpdated {
                world_id: update.world_id.to_string(),
                config: time_config_to_protocol(&update.config),
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

fn parse_time_of_day(period: &str) -> Option<TimeOfDay> {
    match period.trim().to_ascii_lowercase().as_str() {
        "morning" => Some(TimeOfDay::Morning),
        "afternoon" => Some(TimeOfDay::Afternoon),
        "evening" => Some(TimeOfDay::Evening),
        "night" => Some(TimeOfDay::Night),
        _ => None,
    }
}

// =============================================================================
// Protocol Conversion Functions
// =============================================================================

/// Convert domain GameTime to protocol GameTime.
pub(super) fn game_time_to_protocol(gt: &GameTime) -> protocol::GameTime {
    let hour = gt.hour();
    let period = match hour {
        5..=11 => "Morning",
        12..=17 => "Afternoon",
        18..=21 => "Evening",
        _ => "Night",
    }
    .to_string();

    // Format time as 12-hour display
    let am_pm = if hour >= 12 { "PM" } else { "AM" };
    let display_hour = if hour == 0 {
        12
    } else if hour > 12 {
        hour - 12
    } else {
        hour
    };
    let formatted_time = Some(format!("{}:{:02} {}", display_hour, gt.minute(), am_pm));

    protocol::GameTime {
        total_minutes: gt.total_minutes(),
        day: gt.day(),
        hour,
        minute: gt.minute(),
        is_paused: gt.is_paused(),
        formatted_date: None, // Calendar formatting handled elsewhere
        formatted_time,
        period,
    }
}

/// Convert domain TimeAdvanceResultData to protocol TimeAdvanceData.
pub(super) fn time_advance_data_to_protocol(
    data: &TimeAdvanceResultData,
) -> protocol::TimeAdvanceData {
    protocol::TimeAdvanceData {
        previous_time: game_time_to_protocol(&data.previous_time),
        new_time: game_time_to_protocol(&data.new_time),
        minutes_advanced: data.minutes_advanced,
        reason: data.reason.clone(),
        period_changed: data.period_changed,
        new_period: data.new_period.clone(),
    }
}

/// Convert domain GameTimeConfig to protocol GameTimeConfig.
pub(super) fn time_config_to_protocol(
    config: &wrldbldr_domain::GameTimeConfig,
) -> protocol::GameTimeConfig {
    protocol::GameTimeConfig {
        mode: match config.mode() {
            wrldbldr_domain::TimeMode::Manual => protocol::TimeMode::Manual,
            wrldbldr_domain::TimeMode::Suggested => protocol::TimeMode::Suggested,
        },
        time_costs: protocol::TimeCostConfig {
            travel_location: config.time_costs().travel_location,
            travel_region: config.time_costs().travel_region,
            rest_short: config.time_costs().rest_short,
            rest_long: config.time_costs().rest_long,
            conversation: config.time_costs().conversation,
            challenge: config.time_costs().challenge,
            scene_transition: config.time_costs().scene_transition,
        },
        show_time_to_players: config.show_time_to_players(),
        time_format: protocol::TimeFormat::TwelveHour,
        calendar_id: None, // Calendar configuration not yet exposed from domain
        epoch_year: None,
    }
}

/// Convert protocol TimeMode to domain TimeMode.
fn protocol_time_mode_to_domain(mode: protocol::TimeMode) -> wrldbldr_domain::TimeMode {
    match mode {
        protocol::TimeMode::Manual => wrldbldr_domain::TimeMode::Manual,
        protocol::TimeMode::Suggested => wrldbldr_domain::TimeMode::Suggested,
    }
}

/// Convert protocol TimeCostConfig to domain TimeCostConfig.
fn protocol_time_costs_to_domain(
    costs: &protocol::TimeCostConfig,
) -> wrldbldr_domain::TimeCostConfig {
    wrldbldr_domain::TimeCostConfig {
        travel_location: costs.travel_location,
        travel_region: costs.travel_region,
        rest_short: costs.rest_short,
        rest_long: costs.rest_long,
        conversation: costs.conversation,
        challenge: costs.challenge,
        scene_transition: costs.scene_transition,
    }
}

/// Convert protocol GameTimeConfig to domain GameTimeConfig.
pub(super) fn protocol_time_config_to_domain(
    config: &protocol::GameTimeConfig,
) -> wrldbldr_domain::GameTimeConfig {
    // Calendar settings use defaults when not provided in protocol
    // (protocol will be extended to include these in a future update)
    wrldbldr_domain::GameTimeConfig::new(
        protocol_time_mode_to_domain(config.mode),
        protocol_time_costs_to_domain(&config.time_costs),
        config.show_time_to_players,
        wrldbldr_domain::TimeFormat::TwelveHour,
        wrldbldr_domain::CalendarId::new("gregorian").expect("gregorian is a valid calendar ID"),
        wrldbldr_domain::EpochConfig::default(),
    )
}

/// Convert domain TimeSuggestion to protocol TimeSuggestionData.
pub(super) fn time_suggestion_to_protocol(
    suggestion: &crate::infrastructure::ports::TimeSuggestion,
) -> protocol::TimeSuggestionData {
    protocol::TimeSuggestionData {
        suggestion_id: suggestion.id.to_string(),
        pc_id: suggestion.pc_id.to_string(),
        pc_name: suggestion.pc_name.clone(),
        action_type: suggestion.action_type.clone(),
        action_description: suggestion.action_description.clone(),
        suggested_minutes: suggestion.suggested_minutes,
        current_time: game_time_to_protocol(&suggestion.current_time),
        resulting_time: game_time_to_protocol(&suggestion.resulting_time),
        period_change: suggestion.period_change.as_ref().map(|(from, to)| {
            (
                from.display_name().to_string(),
                to.display_name().to_string(),
            )
        }),
    }
}
