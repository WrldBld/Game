use super::*;
use crate::api::connections::ConnectionInfo;
use serde_json::json;
use wrldbldr_domain::{DiceRollInput, OutcomeType};
use wrldbldr_protocol::{ChallengeRequest, ErrorCode, ResponseResult};
use wrldbldr_protocol::types::ProposedToolInfo;

pub(super) async fn handle_challenge_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: ChallengeRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        ChallengeRequest::ListChallenges { world_id } => {
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            match state.app.use_cases.challenge.ops.list(world_id_typed).await {
                Ok(challenges) => Ok(ResponseResult::success(json!(challenges))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        ChallengeRequest::GetChallenge { challenge_id } => {
            let challenge_id_typed = parse_challenge_id_for_request(&challenge_id, request_id)?;
            match state.app.use_cases.challenge.ops.get(challenge_id_typed).await {
                Ok(Some(challenge)) => Ok(ResponseResult::success(json!(challenge))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Challenge not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        ChallengeRequest::CreateChallenge { world_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            match state
                .app
                .use_cases
                .challenge
                .ops
                .create(world_id_typed, data)
                .await
            {
                Ok(challenge) => Ok(ResponseResult::success(json!(challenge))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        ChallengeRequest::UpdateChallenge { challenge_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let challenge_id_typed = parse_challenge_id_for_request(&challenge_id, request_id)?;
            match state
                .app
                .use_cases
                .challenge
                .ops
                .update(challenge_id_typed, data)
                .await
            {
                Ok(challenge) => Ok(ResponseResult::success(json!(challenge))),
                Err(crate::use_cases::challenge::ChallengeCrudError::NotFound) => {
                    Ok(ResponseResult::error(ErrorCode::NotFound, "Challenge not found"))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        ChallengeRequest::DeleteChallenge { challenge_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let challenge_id_typed = parse_challenge_id_for_request(&challenge_id, request_id)?;
            match state
                .app
                .use_cases
                .challenge
                .ops
                .delete(challenge_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        ChallengeRequest::SetChallengeActive {
            challenge_id,
            active,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let challenge_id_typed = parse_challenge_id_for_request(&challenge_id, request_id)?;
            match state
                .app
                .use_cases
                .challenge
                .ops
                .set_active(challenge_id_typed, active)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::challenge::ChallengeCrudError::NotFound) => {
                    Ok(ResponseResult::error(ErrorCode::NotFound, "Challenge not found"))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        ChallengeRequest::SetChallengeFavorite {
            challenge_id,
            favorite,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let challenge_id_typed = parse_challenge_id_for_request(&challenge_id, request_id)?;
            match state
                .app
                .use_cases
                .challenge
                .ops
                .set_favorite(challenge_id_typed, favorite)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::challenge::ChallengeCrudError::NotFound) => {
                    Ok(ResponseResult::error(ErrorCode::NotFound, "Challenge not found"))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
    }
}

pub(super) async fn handle_challenge_roll(
    state: &WsState,
    connection_id: Uuid,
    challenge_id: String,
    roll: i32,
) -> Option<ServerMessage> {
    // Parse challenge ID
    let challenge_uuid = match parse_challenge_id(&challenge_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get connection info to verify authorization
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    let pc_id = match conn_info.pc_id {
        Some(id) => id,
        None => return Some(error_response("NO_PC", "Must have a PC to roll challenges")),
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    // Validate challenge belongs to this world and is active
    let challenge = match state.app.entities.challenge.get(challenge_uuid).await {
        Ok(Some(c)) => c,
        Ok(None) => return Some(error_response("NOT_FOUND", "Challenge not found")),
        Err(e) => return Some(error_response("CHALLENGE_ERROR", &e.to_string())),
    };

    if challenge.world_id != world_id {
        return Some(error_response(
            "INVALID_WORLD",
            "Challenge does not belong to this world",
        ));
    }

    if !challenge.active {
        return Some(error_response(
            "CHALLENGE_INACTIVE",
            "Challenge is not currently active",
        ));
    }

    // Calculate skill modifier from character stats if check_stat is specified
    // Uses get_numeric_value() which handles all field types:
    // - Number: direct modifier
    // - SkillEntry: bonus value
    // - DicePool: dice count (for Blades)
    // - Percentile: skill value (for CoC 7e)
    // - LadderRating: ladder position (for FATE)
    // - Resource: current value
    let skill_modifier = if let Some(ref stat_name) = challenge.check_stat {
        // Get the PC's sheet_data to look up stats
        match state.app.entities.player_character.get(pc_id).await {
            Ok(Some(pc)) => {
                // Look up the stat value from sheet_data using unified numeric extraction
                if let Some(ref sheet_data) = pc.sheet_data {
                    sheet_data.get_numeric_value(stat_name).unwrap_or(0)
                } else {
                    0
                }
            }
            _ => 0,
        }
    } else {
        0
    };

    tracing::debug!(
        challenge_id = %challenge_id,
        check_stat = ?challenge.check_stat,
        skill_modifier = skill_modifier,
        "Challenge roll with modifier"
    );

    match state
        .app
        .use_cases
        .challenge
        .roll
        .execute(world_id, challenge_uuid, pc_id, Some(roll), skill_modifier)
        .await
    {
        Ok(result) => {
            if result.requires_approval {
                let approval_id = match result.approval_queue_id {
                    Some(id) => id.to_string(),
                    None => {
                        return Some(error_response(
                            "APPROVAL_ERROR",
                            "Missing challenge approval request",
                        ))
                    }
                };

                let outcome_triggers = result
                    .outcome_triggers
                    .into_iter()
                    .map(|tool| ProposedToolInfo {
                        id: tool.id,
                        name: tool.name,
                        description: tool.description,
                        arguments: tool.arguments,
                    })
                    .collect();

                let pending = ServerMessage::ChallengeOutcomePending {
                    resolution_id: approval_id,
                    challenge_id: result.challenge_id.to_string(),
                    challenge_name: result.challenge_name.clone(),
                    character_id: result.character_id.to_string(),
                    character_name: result.character_name.clone(),
                    roll: result.roll,
                    modifier: result.modifier,
                    total: result.total,
                    outcome_type: outcome_type_to_str(result.outcome_type).to_string(),
                    outcome_description: result.outcome_description.clone(),
                    outcome_triggers,
                    roll_breakdown: result.roll_breakdown.clone(),
                };
                state.connections.broadcast_to_dms(world_id, pending).await;

                Some(ServerMessage::ChallengeRollSubmitted {
                    challenge_id,
                    challenge_name: result.challenge_name,
                    roll: result.roll,
                    modifier: result.modifier,
                    total: result.total,
                    outcome_type: outcome_type_to_str(result.outcome_type).to_string(),
                    status: "pending".to_string(),
                })
            } else {
                // Auto-resolve and broadcast to world
                let msg = ServerMessage::ChallengeResolved {
                    challenge_id: challenge_id.clone(),
                    challenge_name: result.challenge_name.clone(),
                    character_name: result.character_name.clone(),
                    roll: result.roll,
                    modifier: result.modifier,
                    total: result.total,
                    outcome: outcome_type_to_str(result.outcome_type).to_string(),
                    outcome_description: result.outcome_description,
                    roll_breakdown: result.roll_breakdown,
                    individual_rolls: None,
                };
                state.connections.broadcast_to_world(world_id, msg).await;
                None
            }
        }
        Err(crate::use_cases::challenge::ChallengeError::NotFound) => {
            Some(error_response("NOT_FOUND", "Challenge not found"))
        }
        Err(crate::use_cases::challenge::ChallengeError::PlayerCharacterNotFound) => {
            Some(error_response("NOT_FOUND", "Player character not found"))
        }
        Err(crate::use_cases::challenge::ChallengeError::DiceParse(_)) => {
            Some(error_response("INVALID_DICE_INPUT", "Invalid dice input"))
        }
        Err(e) => Some(error_response("ROLL_ERROR", &e.to_string())),
    }
}

pub(super) async fn handle_challenge_roll_input(
    state: &WsState,
    connection_id: Uuid,
    challenge_id: String,
    input_type: wrldbldr_protocol::DiceInputType,
) -> Option<ServerMessage> {
    // Parse challenge ID
    let challenge_uuid = match parse_challenge_id(&challenge_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get connection info to verify authorization
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    let pc_id = match conn_info.pc_id {
        Some(id) => id,
        None => return Some(error_response("NO_PC", "Must have a PC to roll challenges")),
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    let input = match input_type {
        wrldbldr_protocol::DiceInputType::Formula(formula) => DiceRollInput::Formula(formula),
        wrldbldr_protocol::DiceInputType::Manual(value) => DiceRollInput::ManualResult(value),
        wrldbldr_protocol::DiceInputType::Unknown => {
            return Some(error_response("INVALID_DICE_INPUT", "Unknown dice input type"))
        }
    };

    match state
        .app
        .use_cases
        .challenge
        .roll
        .execute_with_input(world_id, challenge_uuid, pc_id, input)
        .await
    {
        Ok(result) => {
            if result.requires_approval {
                let approval_id = match result.approval_queue_id {
                    Some(id) => id.to_string(),
                    None => {
                        return Some(error_response(
                            "APPROVAL_ERROR",
                            "Missing challenge approval request",
                        ))
                    }
                };

                let outcome_triggers = result
                    .outcome_triggers
                    .into_iter()
                    .map(|tool| ProposedToolInfo {
                        id: tool.id,
                        name: tool.name,
                        description: tool.description,
                        arguments: tool.arguments,
                    })
                    .collect();

                let pending = ServerMessage::ChallengeOutcomePending {
                    resolution_id: approval_id,
                    challenge_id: result.challenge_id.to_string(),
                    challenge_name: result.challenge_name.clone(),
                    character_id: result.character_id.to_string(),
                    character_name: result.character_name.clone(),
                    roll: result.roll,
                    modifier: result.modifier,
                    total: result.total,
                    outcome_type: outcome_type_to_str(result.outcome_type).to_string(),
                    outcome_description: result.outcome_description.clone(),
                    outcome_triggers,
                    roll_breakdown: result.roll_breakdown.clone(),
                };
                state.connections.broadcast_to_dms(world_id, pending).await;

                Some(ServerMessage::ChallengeRollSubmitted {
                    challenge_id,
                    challenge_name: result.challenge_name,
                    roll: result.roll,
                    modifier: result.modifier,
                    total: result.total,
                    outcome_type: outcome_type_to_str(result.outcome_type).to_string(),
                    status: "pending".to_string(),
                })
            } else {
                let msg = ServerMessage::ChallengeResolved {
                    challenge_id: challenge_id.clone(),
                    challenge_name: result.challenge_name.clone(),
                    character_name: result.character_name.clone(),
                    roll: result.roll,
                    modifier: result.modifier,
                    total: result.total,
                    outcome: outcome_type_to_str(result.outcome_type).to_string(),
                    outcome_description: result.outcome_description,
                    roll_breakdown: result.roll_breakdown,
                    individual_rolls: None,
                };
                state.connections.broadcast_to_world(world_id, msg).await;
                None
            }
        }
        Err(crate::use_cases::challenge::ChallengeError::NotFound) => {
            Some(error_response("NOT_FOUND", "Challenge not found"))
        }
        Err(crate::use_cases::challenge::ChallengeError::PlayerCharacterNotFound) => {
            Some(error_response("NOT_FOUND", "Player character not found"))
        }
        Err(crate::use_cases::challenge::ChallengeError::DiceParse(_)) => {
            Some(error_response("INVALID_DICE_INPUT", "Invalid dice input"))
        }
        Err(e) => Some(error_response("ROLL_ERROR", &e.to_string())),
    }
}

pub(super) async fn handle_trigger_challenge(
    state: &WsState,
    connection_id: Uuid,
    challenge_id: String,
    target_character_id: String,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can trigger challenges
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse IDs
    let challenge_uuid = match parse_challenge_id(&challenge_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let target_uuid = match parse_pc_id(&target_character_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    match state
        .app
        .use_cases
        .challenge
        .trigger_prompt
        .execute(challenge_uuid)
        .await
    {
        Ok(prompt) => {
            let msg = ServerMessage::ChallengePrompt {
                challenge_id,
                challenge_name: prompt.challenge_name,
                skill_name: prompt.skill_name,
                difficulty_display: prompt.difficulty_display,
                description: prompt.description,
                character_modifier: prompt.character_modifier,
                suggested_dice: prompt.suggested_dice,
                rule_system_hint: prompt.rule_system_hint,
            };
            state.connections.send_to_pc(target_uuid, msg).await;
            None
        }
        Err(crate::use_cases::challenge::ChallengeError::NotFound) => {
            Some(error_response("NOT_FOUND", "Challenge not found"))
        }
        Err(e) => Some(error_response("CHALLENGE_ERROR", &e.to_string())),
    }
}

pub(super) async fn handle_challenge_suggestion_decision(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    approved: bool,
    _modified_difficulty: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can make decisions
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse request ID as approval UUID
    let approval_id = match parse_id(&request_id, |u| u, "Invalid request ID") {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let decision = if approved {
        wrldbldr_domain::DmApprovalDecision::Accept
    } else {
        wrldbldr_domain::DmApprovalDecision::Reject {
            feedback: "Challenge rejected by DM".to_string(),
        }
    };

    match state
        .app
        .use_cases
        .approval
        .approve_suggestion
        .execute(approval_id, decision)
        .await
    {
        Ok(_) => {
            if !approved {
                Some(ServerMessage::ChallengeDiscarded { request_id })
            } else {
                None
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Challenge suggestion decision failed");
            Some(error_response("APPROVAL_ERROR", &e.to_string()))
        }
    }
}

pub(super) async fn handle_challenge_outcome_decision(
    state: &WsState,
    connection_id: Uuid,
    resolution_id: String,
    decision: wrldbldr_protocol::ChallengeOutcomeDecisionData,
) -> Option<ServerMessage> {
    // Only DMs can approve challenge outcomes
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    match state
        .app
        .use_cases
        .challenge
        .outcome_decision
        .execute(world_id, resolution_id.clone(), decision)
        .await
    {
        Ok(crate::use_cases::challenge::OutcomeDecisionResult::Resolved(payload)) => {
            let msg = ServerMessage::ChallengeResolved {
                challenge_id: payload.challenge_id,
                challenge_name: payload.challenge_name,
                character_name: payload.character_name,
                roll: payload.roll,
                modifier: payload.modifier,
                total: payload.total,
                outcome: payload.outcome,
                outcome_description: payload.outcome_description,
                roll_breakdown: payload.roll_breakdown,
                individual_rolls: None,
            };
            state.connections.broadcast_to_world(world_id, msg).await;
            None
        }
        Ok(crate::use_cases::challenge::OutcomeDecisionResult::Queued) => None,
        Err(crate::use_cases::challenge::OutcomeDecisionError::ApprovalNotFound) => {
            Some(error_response("NOT_FOUND", "Approval request not found"))
        }
        Err(crate::use_cases::challenge::OutcomeDecisionError::MissingOutcomeData) => {
            Some(error_response(
                "INVALID_DATA",
                "No challenge outcome data in approval request",
            ))
        }
        Err(crate::use_cases::challenge::OutcomeDecisionError::MissingPcId) => {
            Some(error_response(
                "MISSING_PC_ID",
                "Challenge outcome is missing target PC context",
            ))
        }
        Err(crate::use_cases::challenge::OutcomeDecisionError::InvalidResolutionId) => {
            Some(error_response("INVALID_ID", "Invalid resolution ID format"))
        }
        Err(e) => {
            tracing::error!(error = %e, "Challenge outcome decision failed");
            Some(error_response("RESOLVE_ERROR", &e.to_string()))
        }
    }
}

fn outcome_type_to_str(outcome_type: OutcomeType) -> &'static str {
    match outcome_type {
        OutcomeType::CriticalSuccess => "critical_success",
        OutcomeType::Success => "success",
        OutcomeType::Partial => "partial",
        OutcomeType::Failure => "failure",
        OutcomeType::CriticalFailure => "critical_failure",
    }
}
