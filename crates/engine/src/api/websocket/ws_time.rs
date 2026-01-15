use super::*;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use wrldbldr_domain::{TimeAdvanceReason, TimeOfDay};

pub(super) async fn handle_set_game_time(
    state: &WsState,
    connection_id: Uuid,
    world_id: String,
    day: u32,
    hour: u8,
    notify_players: bool,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
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
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
            return Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                "TIME_ERROR",
                &sanitize_repo_error(&e, "setting game time"),
            ))
        }
    };

    if notify_players {
        let reason = TimeAdvanceReason::DmSetTime;
        let advance_data = crate::use_cases::time::build_time_advance_data(
            &outcome.previous_time,
            &outcome.new_time,
            outcome.minutes_advanced,
            &reason,
        );
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
    connection_id: Uuid,
    world_id: String,
    period: String,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
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
                "INVALID_PERIOD",
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
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
            return Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                "TIME_ERROR",
                &sanitize_repo_error(&e, "skipping to time period"),
            ))
        }
    };

    let reason = TimeAdvanceReason::DmSkipToPeriod {
        period: period_typed,
    };
    let advance_data = crate::use_cases::time::build_time_advance_data(
        &outcome.previous_time,
        &outcome.new_time,
        outcome.minutes_advanced,
        &reason,
    );
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
    connection_id: Uuid,
    world_id: String,
    paused: bool,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
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
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
            return Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                "TIME_ERROR",
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
    connection_id: Uuid,
    world_id: String,
    mode: wrldbldr_protocol::types::TimeMode,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
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
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
            return Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                "TIME_ERROR",
                &sanitize_repo_error(&e, "getting time config"),
            ))
        }
    };

    config.mode = mode;

    match state
        .app
        .use_cases
        .time
        .control
        .update_time_config(world_id_typed, config)
        .await
    {
        Ok(_) => {}
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
            return Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                "TIME_ERROR",
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
    connection_id: Uuid,
    world_id: String,
    costs: wrldbldr_protocol::types::TimeCostConfig,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
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
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
            return Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                "TIME_ERROR",
                &sanitize_repo_error(&e, "getting time config"),
            ))
        }
    };

    config.time_costs = costs;

    let updated = match state
        .app
        .use_cases
        .time
        .control
        .update_time_config(world_id_typed, config)
        .await
    {
        Ok(updated) => updated,
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
            return Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                "TIME_ERROR",
                &sanitize_repo_error(&e, "setting time costs"),
            ))
        }
    };

    let msg = ServerMessage::TimeConfigUpdated {
        world_id: world_id_typed.to_string(),
        config: updated.normalized_config,
    };
    state
        .connections
        .broadcast_to_dms(world_id_typed, msg)
        .await;

    None
}

pub(super) async fn handle_respond_to_time_suggestion(
    state: &WsState,
    connection_id: Uuid,
    suggestion_id: String,
    decision: wrldbldr_protocol::types::TimeSuggestionDecision,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let suggestion_uuid = match Uuid::parse_str(&suggestion_id) {
        Ok(id) => id,
        Err(_) => return Some(error_response("INVALID_ID", "Invalid time suggestion ID")),
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    // Convert protocol decision to domain decision at the API boundary
    let domain_decision: wrldbldr_domain::TimeSuggestionDecision = match decision.try_into() {
        Ok(d) => d,
        Err(_) => {
            return Some(error_response(
                "INVALID_DECISION",
                "Unknown time suggestion decision type",
            ))
        }
    };

    let time_suggestions = crate::repositories::TimeSuggestionStore::new(
        state.pending_time_suggestions.clone(),
    );
    match state
        .app
        .use_cases
        .time
        .suggestions
        .resolve(&time_suggestions, world_id, suggestion_uuid, domain_decision)
        .await
    {
        Ok(Some(resolution)) => {
            let msg = ServerMessage::GameTimeAdvanced {
                data: resolution.advance_data,
            };
            state.connections.broadcast_to_world(world_id, msg).await;
            None
        }
        Ok(None) => None,
        Err(crate::use_cases::time::TimeSuggestionError::NotFound) => Some(error_response(
            "TIME_SUGGESTION_NOT_FOUND",
            "Time suggestion not found",
        )),
        Err(crate::use_cases::time::TimeSuggestionError::WorldMismatch) => Some(error_response(
            "TIME_SUGGESTION_INVALID",
            "Time suggestion world mismatch",
        )),
        Err(e) => Some(error_response(
            "TIME_ERROR",
            &sanitize_repo_error(&e, "responding to time suggestion"),
        )),
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
