use super::*;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::use_cases::time::TimeAdvanceResultData;
use wrldbldr_domain::{GameTime, TimeAdvanceReason, TimeOfDay};
use wrldbldr_shared::types as protocol;
use wrldbldr_shared::ErrorCode;

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
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
            return Some(error_response(ErrorCode::NotFound, "World not found"))
        }
        Err(e) => {
            return Some(error_response(
                ErrorCode::InternalError,
                &sanitize_repo_error(&e, "setting game time"),
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
    connection_id: Uuid,
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
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
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
    connection_id: Uuid,
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
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
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
    connection_id: Uuid,
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
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
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
    config.mode = protocol_time_mode_to_domain(mode);

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
    connection_id: Uuid,
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
        Err(crate::use_cases::time::TimeControlError::WorldNotFound) => {
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
    config.time_costs = protocol_time_costs_to_domain(&costs);

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
        config: time_config_to_protocol(&updated.normalized_config),
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
            suggestion_uuid,
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
        Err(crate::use_cases::time::TimeSuggestionError::NotFound) => Some(error_response(
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
    protocol::GameTime {
        day: gt.day(),
        hour: gt.hour(),
        minute: gt.minute(),
        is_paused: gt.is_paused(),
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
        mode: match config.mode {
            wrldbldr_domain::TimeMode::Manual => protocol::TimeMode::Manual,
            wrldbldr_domain::TimeMode::Suggested => protocol::TimeMode::Suggested,
            wrldbldr_domain::TimeMode::Auto => protocol::TimeMode::Suggested,
        },
        time_costs: protocol::TimeCostConfig {
            travel_location: config.time_costs.travel_location,
            travel_region: config.time_costs.travel_region,
            rest_short: config.time_costs.rest_short,
            rest_long: config.time_costs.rest_long,
            conversation: config.time_costs.conversation,
            challenge: config.time_costs.challenge,
            scene_transition: config.time_costs.scene_transition,
        },
        show_time_to_players: config.show_time_to_players,
        time_format: protocol::TimeFormat::TwelveHour,
    }
}

/// Convert protocol TimeMode to domain TimeMode.
fn protocol_time_mode_to_domain(mode: protocol::TimeMode) -> wrldbldr_domain::TimeMode {
    match mode {
        protocol::TimeMode::Manual => wrldbldr_domain::TimeMode::Manual,
        protocol::TimeMode::Suggested => wrldbldr_domain::TimeMode::Suggested,
        protocol::TimeMode::Auto => wrldbldr_domain::TimeMode::Suggested, // Auto normalized to Suggested
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
    wrldbldr_domain::GameTimeConfig {
        mode: protocol_time_mode_to_domain(config.mode),
        time_costs: protocol_time_costs_to_domain(&config.time_costs),
        show_time_to_players: config.show_time_to_players,
        time_format: wrldbldr_domain::TimeFormat::TwelveHour,
    }
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
