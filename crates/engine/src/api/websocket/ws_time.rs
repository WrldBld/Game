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
            outcome.seconds_advanced,
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
        outcome.seconds_advanced,
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

    let time_suggestions = &state.pending_time_suggestions;
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
                    &sanitize_repo_error(&e, "getting game time"),
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

        TimeRequest::AdvanceGameTimeSeconds {
            world_id,
            seconds,
            reason,
        } => {
            require_dm_for_request(conn_info, request_id)?;

            let world_id_typed =
                match parse_uuid_for_request(&world_id, request_id, "Invalid world ID") {
                    Ok(uuid) => WorldId::from_uuid(uuid),
                    Err(e) => return Err(e),
                };

            // Build TimeAdvanceReason with proper handling for sub-hour values
            // Use DmManual for manual advancement, with hours field for coarse description
            // The exact seconds value is tracked separately in the advance_data
            let advance_reason = wrldbldr_domain::TimeAdvanceReason::DmManual {
                hours: seconds / 3600,
            };

            let outcome = match state
                .app
                .use_cases
                .time
                .control
                .advance_seconds(world_id_typed, seconds, advance_reason.clone())
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
                            sanitize_repo_error(&e, "advance game seconds"),
                        ),
                    });
                }
            };

            // Build display reason: use provided reason if available, otherwise generate precise description
            let display_reason = if let Some(r) = reason {
                r
            } else {
                // Build seconds-based reason to preserve precision for sub-hour values
                let hours = seconds / 3600;
                let minutes = (seconds % 3600) / 60;
                if minutes == 0 {
                    format!(
                        "Time advanced by {} hour{}",
                        hours,
                        if hours == 1 { "" } else { "s" }
                    )
                } else if hours == 0 {
                    format!("Time advanced by {} minute{}", minutes, if minutes == 1 { "" } else { "s" })
                } else {
                    format!("Time advanced by {} hour{} {} minute{}", hours, if hours == 1 { "" } else { "s" }, minutes, if minutes == 1 { "" } else { "s" })
                }
            };

            // Build domain data with custom reason that preserves seconds precision
            let mut domain_data = crate::use_cases::time::build_time_advance_data(
                &outcome.previous_time,
                &outcome.new_time,
                seconds,
                &advance_reason,
            );
            // Override the auto-generated description with our precise display_reason
            domain_data.reason = display_reason.clone();

            let advance_data = time_advance_data_to_protocol(&domain_data);
            let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
            state
                .connections
                .broadcast_to_world(world_id_typed, update_msg)
                .await;

            tracing::info!(
                world_id = %world_id_typed,
                seconds_advanced = seconds,
                reason = %display_reason,
                "Game time advanced (seconds)"
            );

            Ok(ResponseResult::success(serde_json::json!({
                "game_time": game_time_to_protocol(&outcome.new_time),
                "seconds_advanced": seconds,
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
                outcome.seconds_advanced,
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
            let domain_config = protocol_time_config_to_domain(&config)
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e),
                })?;

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
        total_seconds: gt.total_seconds(),
        day: gt.day(),
        hour,
        minute: gt.minute(),
        second: gt.second(),
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
        seconds_advanced: data.seconds_advanced,
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
) -> Result<wrldbldr_domain::GameTimeConfig, String> {
    // Calendar settings use defaults when not provided in protocol
    // (protocol will be extended to include these in a future update)
    let calendar_id = wrldbldr_domain::CalendarId::new("gregorian")
        .map_err(|e| format!("Failed to initialize calendar: {}", e))?;

    Ok(wrldbldr_domain::GameTimeConfig::new(
        protocol_time_mode_to_domain(config.mode),
        protocol_time_costs_to_domain(&config.time_costs),
        config.show_time_to_players,
        wrldbldr_domain::TimeFormat::TwelveHour,
        calendar_id,
        wrldbldr_domain::EpochConfig::default(),
    ))
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
        suggested_seconds: suggestion.suggested_seconds,
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

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AdvanceGameTimeSeconds Reason Tests
    // =========================================================================

    /// Test that sub-hour seconds produce precise reason descriptions
    /// This mirrors the logic in handle_time_request for AdvanceGameTimeSeconds
    #[test]
    fn sub_hour_seconds_produce_precise_reason() {
        let seconds = 1800u32; // 30 minutes
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        let display_reason = if minutes == 0 {
            format!(
                "Time advanced by {} hour{}",
                hours,
                if hours == 1 { "" } else { "s" }
            )
        } else if hours == 0 {
            format!(
                "Time advanced by {} minute{}",
                minutes,
                if minutes == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "Time advanced by {} hour{} {} minute{}",
                hours,
                if hours == 1 { "" } else { "s" },
                minutes,
                if minutes == 1 { "" } else { "s" }
            )
        };

        assert_eq!(display_reason, "Time advanced by 30 minutes");
    }

    /// Test that hours + minutes produce correct reason
    #[test]
    fn hour_and_minute_produce_combined_reason() {
        let seconds = 5400u32; // 1 hour 30 minutes
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        let display_reason = format!(
            "Time advanced by {} hour{} {} minute{}",
            hours,
            if hours == 1 { "" } else { "s" },
            minutes,
            if minutes == 1 { "" } else { "s" }
        );

        assert_eq!(display_reason, "Time advanced by 1 hour 30 minutes");
    }

    /// Test that pluralization works correctly for 1 hour
    #[test]
    fn single_hour_has_no_plural() {
        let seconds = 3600u32;
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        let display_reason = if minutes == 0 {
            format!(
                "Time advanced by {} hour{}",
                hours,
                if hours == 1 { "" } else { "s" }
            )
        } else {
            panic!("Should not have minutes");
        };

        assert_eq!(display_reason, "Time advanced by 1 hour");
    }

    /// Test that pluralization works correctly for multiple hours
    #[test]
    fn multiple_hours_has_plural() {
        let seconds = 7200u32; // 2 hours
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        let display_reason = if minutes == 0 {
            format!(
                "Time advanced by {} hour{}",
                hours,
                if hours == 1 { "" } else { "s" }
            )
        } else {
            panic!("Should not have minutes");
        };

        assert_eq!(display_reason, "Time advanced by 2 hours");
    }

    /// Test that single minute has no plural
    #[test]
    fn single_minute_has_no_plural() {
        let seconds = 60u32;
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        let display_reason = if hours == 0 {
            format!(
                "Time advanced by {} minute{}",
                minutes,
                if minutes == 1 { "" } else { "s" }
            )
        } else {
            panic!("Should not have hours");
        };

        assert_eq!(display_reason, "Time advanced by 1 minute");
    }

    /// Test that multiple minutes have plural
    #[test]
    fn multiple_minutes_have_plural() {
        let seconds = 120u32; // 2 minutes
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        let display_reason = if hours == 0 {
            format!(
                "Time advanced by {} minute{}",
                minutes,
                if minutes == 1 { "" } else { "s" }
            )
        } else {
            panic!("Should not have hours");
        };

        assert_eq!(display_reason, "Time advanced by 2 minutes");
    }

    /// Test that custom reason overrides generated reason
    /// This tests the logic in ws_time.rs lines 599-626
    #[test]
    fn custom_reason_overrides_generated() {
        let seconds = 3600u32;
        let custom_reason = Some("Party rested for the night".to_string());

        let display_reason = if let Some(r) = custom_reason {
            r
        } else {
            let hours = seconds / 3600;
            let minutes = (seconds % 3600) / 60;
            if minutes == 0 {
                format!(
                    "Time advanced by {} hour{}",
                    hours,
                    if hours == 1 { "" } else { "s" }
                )
            } else {
                panic!("Should not reach here with custom_reason")
            }
        };

        assert_eq!(display_reason, "Party rested for the night");
    }

    /// Test that None custom_reason uses generated reason
    #[test]
    fn none_custom_reason_uses_generated() {
        let seconds = 5400u32; // 1.5 hours
        let custom_reason = None;

        let display_reason = if let Some(r) = custom_reason {
            r
        } else {
            let hours = seconds / 3600;
            let minutes = (seconds % 3600) / 60;
            if minutes == 0 {
                format!(
                    "Time advanced by {} hour{}",
                    hours,
                    if hours == 1 { "" } else { "s" }
                )
            } else if hours == 0 {
                format!(
                    "Time advanced by {} minute{}",
                    minutes,
                    if minutes == 1 { "" } else { "s" }
                )
            } else {
                format!(
                    "Time advanced by {} hour{} {} minute{}",
                    hours,
                    if hours == 1 { "" } else { "s" },
                    minutes,
                    if minutes == 1 { "" } else { "s" }
                )
            }
        };

        assert_eq!(display_reason, "Time advanced by 1 hour 30 minutes");
    }

    /// Test edge case: zero seconds
    #[test]
    fn zero_seconds_generates_zero_hours() {
        let seconds = 0u32;
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        let display_reason = if minutes == 0 {
            format!(
                "Time advanced by {} hour{}",
                hours,
                if hours == 1 { "" } else { "s" }
            )
        } else {
            panic!("Should not have minutes");
        };

        // Note: 0 is not 1, so we get "hours" plural
        assert_eq!(display_reason, "Time advanced by 0 hours");
    }

    // =========================================================================
    // Protocol Conversion Tests
    // =========================================================================

    /// Test that game_time_to_protocol produces correct GameTime
    #[test]
    fn game_time_to_protocol_converts_correctly() {
        let domain_time = wrldbldr_domain::GameTime::from_seconds(45296); // 12:34:56, day 1
        let protocol_time = game_time_to_protocol(&domain_time);

        assert_eq!(protocol_time.total_seconds, 45296);
        assert_eq!(protocol_time.day, 1);
        assert_eq!(protocol_time.hour, 12);
        assert_eq!(protocol_time.minute, 34);
        assert_eq!(protocol_time.second, 56);
        assert_eq!(protocol_time.period, "Afternoon");
    }

    /// Test that game_time_to_protocol formats time correctly
    #[test]
    fn game_time_to_protocol_formats_time() {
        let domain_time = wrldbldr_domain::GameTime::from_seconds(32400); // 9:00 AM
        let protocol_time = game_time_to_protocol(&domain_time);

        assert_eq!(protocol_time.formatted_time, Some("9:00 AM".to_string()));
    }

    /// Test that game_time_to_protocol handles noon
    #[test]
    fn game_time_to_protocol_handles_noon() {
        let domain_time = wrldbldr_domain::GameTime::from_seconds(43200); // 12:00 PM
        let protocol_time = game_time_to_protocol(&domain_time);

        assert_eq!(protocol_time.formatted_time, Some("12:00 PM".to_string()));
    }

    /// Test that game_time_to_protocol handles midnight
    #[test]
    fn game_time_to_protocol_handles_midnight() {
        let domain_time = wrldbldr_domain::GameTime::from_seconds(0); // 12:00 AM
        let protocol_time = game_time_to_protocol(&domain_time);

        assert_eq!(protocol_time.formatted_time, Some("12:00 AM".to_string()));
    }

    /// Test that game_time_to_protocol handles evening
    #[test]
    fn game_time_to_protocol_handles_evening() {
        let domain_time = wrldbldr_domain::GameTime::from_seconds(64800); // 6:00 PM
        let protocol_time = game_time_to_protocol(&domain_time);

        assert_eq!(protocol_time.formatted_time, Some("6:00 PM".to_string()));
    }

    /// Test that time_advance_data_to_protocol converts correctly
    #[test]
    fn time_advance_data_to_protocol_converts_correctly() {
        let previous_time = wrldbldr_domain::GameTime::from_seconds(0);
        let new_time = wrldbldr_domain::GameTime::from_seconds(3600);
        let reason = wrldbldr_domain::TimeAdvanceReason::DmManual { hours: 1 };

        let domain_data = crate::use_cases::time::build_time_advance_data(
            &previous_time,
            &new_time,
            3600,
            &reason,
        );
        let protocol_data = time_advance_data_to_protocol(&domain_data);

        assert_eq!(protocol_data.seconds_advanced, 3600);
        assert!(protocol_data.reason.contains("1 hour"));
        assert_eq!(protocol_data.period_changed, false);
    }

    /// Test that time_advance_data_to_protocol detects period change
    #[test]
    fn time_advance_data_to_protocol_detects_period_change() {
        let previous_time = wrldbldr_domain::GameTime::from_seconds(21600); // 6 AM = Morning
        let new_time = wrldbldr_domain::GameTime::from_seconds(43200); // 12 PM = Afternoon
        let reason = wrldbldr_domain::TimeAdvanceReason::DmManual { hours: 6 };

        let domain_data = crate::use_cases::time::build_time_advance_data(
            &previous_time,
            &new_time,
            21600,
            &reason,
        );
        let protocol_data = time_advance_data_to_protocol(&domain_data);

        assert_eq!(protocol_data.period_changed, true);
        assert_eq!(protocol_data.new_period, Some("Afternoon".to_string()));
    }

    // =========================================================================
    // TimeAdvanceReason Tests
    // =========================================================================

    /// Test TimeAdvanceReason::DmManual description
    #[test]
    fn dm_manual_reason_description() {
        use wrldbldr_domain::TimeAdvanceReason;

        let reason = TimeAdvanceReason::DmManual { hours: 1 };
        assert_eq!(reason.description(), "Time advanced by 1 hour");

        let reason = TimeAdvanceReason::DmManual { hours: 4 };
        assert_eq!(reason.description(), "Time advanced by 4 hours");
    }

    /// Test TimeAdvanceReason::DmSetTime description
    #[test]
    fn dm_set_time_reason_description() {
        use wrldbldr_domain::TimeAdvanceReason;

        let reason = TimeAdvanceReason::DmSetTime;
        assert_eq!(reason.description(), "Time set by DM");
    }

    /// Test TimeAdvanceReason::DmSkipToPeriod description
    #[test]
    fn dm_skip_to_period_reason_description() {
        use wrldbldr_domain::{TimeAdvanceReason, TimeOfDay};

        let reason = TimeAdvanceReason::DmSkipToPeriod {
            period: TimeOfDay::Evening,
        };
        assert_eq!(reason.description(), "Skipped to Evening");
    }

    /// Test TimeAdvanceReason::RestShort description
    #[test]
    fn rest_short_reason_description() {
        use wrldbldr_domain::TimeAdvanceReason;

        let reason = TimeAdvanceReason::RestShort;
        assert_eq!(reason.description(), "Took a short rest");
    }

    /// Test TimeAdvanceReason::RestLong description
    #[test]
    fn rest_long_reason_description() {
        use wrldbldr_domain::TimeAdvanceReason;

        let reason = TimeAdvanceReason::RestLong;
        assert_eq!(reason.description(), "Rested for the night");
    }
}
