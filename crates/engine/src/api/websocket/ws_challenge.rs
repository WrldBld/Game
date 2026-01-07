use super::*;
use crate::api::connections::ConnectionInfo;
use serde_json::json;
use wrldbldr_domain::{self as domain, Difficulty};
use wrldbldr_protocol::{ChallengeRequest, ErrorCode, ResponseResult};

pub(super) async fn handle_challenge_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: ChallengeRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        ChallengeRequest::ListChallenges { world_id } => {
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            match state
                .app
                .entities
                .challenge
                .list_for_world(world_id_typed)
                .await
            {
                Ok(challenges) => {
                    let data: Vec<serde_json::Value> =
                        challenges.iter().map(challenge_to_json).collect();
                    Ok(ResponseResult::success(json!(data)))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        ChallengeRequest::GetChallenge { challenge_id } => {
            let challenge_id_typed = parse_challenge_id_for_request(&challenge_id, request_id)?;
            match state.app.entities.challenge.get(challenge_id_typed).await {
                Ok(Some(challenge)) => Ok(ResponseResult::success(json!(challenge_to_json(
                    &challenge
                )))),
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
            let mut challenge = domain::Challenge::new(
                world_id_typed,
                &data.name,
                Difficulty::parse(&data.difficulty),
            );
            challenge.description = data.description.unwrap_or_default();
            challenge.outcomes.success.description = data.success_outcome.unwrap_or_default();
            challenge.outcomes.failure.description = data.failure_outcome.unwrap_or_default();
            challenge.order = 0;

            match state.app.entities.challenge.save(&challenge).await {
                Ok(()) => Ok(ResponseResult::success(json!(challenge_to_json(
                    &challenge
                )))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
        ChallengeRequest::UpdateChallenge { challenge_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let challenge_id_typed = parse_challenge_id_for_request(&challenge_id, request_id)?;
            match state.app.entities.challenge.get(challenge_id_typed).await {
                Ok(Some(mut challenge)) => {
                    if let Some(name) = data.name {
                        challenge.name = name;
                    }
                    if let Some(description) = data.description {
                        challenge.description = description;
                    }
                    if let Some(difficulty) = data.difficulty {
                        challenge.difficulty = Difficulty::parse(&difficulty);
                    }
                    if let Some(success) = data.success_outcome {
                        challenge.outcomes.success.description = success;
                    }
                    if let Some(failure) = data.failure_outcome {
                        challenge.outcomes.failure.description = failure;
                    }
                    match state.app.entities.challenge.save(&challenge).await {
                        Ok(()) => Ok(ResponseResult::success(json!(challenge_to_json(
                            &challenge
                        )))),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
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
        ChallengeRequest::DeleteChallenge { challenge_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let challenge_id_typed = parse_challenge_id_for_request(&challenge_id, request_id)?;
            match state
                .app
                .entities
                .challenge
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
            match state.app.entities.challenge.get(challenge_id_typed).await {
                Ok(Some(mut challenge)) => {
                    challenge.active = active;
                    match state.app.entities.challenge.save(&challenge).await {
                        Ok(()) => Ok(ResponseResult::success_empty()),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
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
        ChallengeRequest::SetChallengeFavorite {
            challenge_id,
            favorite,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let challenge_id_typed = parse_challenge_id_for_request(&challenge_id, request_id)?;
            match state.app.entities.challenge.get(challenge_id_typed).await {
                Ok(Some(mut challenge)) => {
                    challenge.is_favorite = favorite;
                    match state.app.entities.challenge.save(&challenge).await {
                        Ok(()) => Ok(ResponseResult::success_empty()),
                        Err(e) => Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        )),
                    }
                }
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
    }
}

fn challenge_to_json(challenge: &domain::Challenge) -> serde_json::Value {
    json!({
        "id": challenge.id.to_string(),
        "world_id": challenge.world_id.to_string(),
        "scene_id": serde_json::Value::Null,
        "name": challenge.name,
        "description": challenge.description,
        "challenge_type": challenge_type_to_str(&challenge.challenge_type),
        "skill_id": "",
        "difficulty": difficulty_to_json(&challenge.difficulty),
        "outcomes": {
            "success": outcome_to_json(&challenge.outcomes.success),
            "failure": outcome_to_json(&challenge.outcomes.failure),
            "partial": challenge
                .outcomes
                .partial
                .as_ref()
                .map(outcome_to_json),
            "critical_success": challenge
                .outcomes
                .critical_success
                .as_ref()
                .map(outcome_to_json),
            "critical_failure": challenge
                .outcomes
                .critical_failure
                .as_ref()
                .map(outcome_to_json),
        },
        "trigger_conditions": challenge
            .trigger_conditions
            .iter()
            .map(trigger_condition_to_json)
            .collect::<Vec<_>>(),
        "prerequisite_challenges": Vec::<String>::new(),
        "active": challenge.active,
        "order": challenge.order,
        "is_favorite": challenge.is_favorite,
        "tags": challenge.tags,
    })
}

fn outcome_to_json(outcome: &domain::Outcome) -> serde_json::Value {
    json!({
        "description": outcome.description,
        "triggers": outcome
            .triggers
            .iter()
            .map(outcome_trigger_to_json)
            .collect::<Vec<_>>(),
    })
}

fn outcome_trigger_to_json(trigger: &domain::OutcomeTrigger) -> serde_json::Value {
    match trigger {
        domain::OutcomeTrigger::RevealInformation { info, persist } => json!({
            "type": "reveal_information",
            "info": info,
            "persist": persist,
        }),
        domain::OutcomeTrigger::EnableChallenge { challenge_id } => json!({
            "type": "enable_challenge",
            "challenge_id": challenge_id.to_string(),
        }),
        domain::OutcomeTrigger::DisableChallenge { challenge_id } => json!({
            "type": "disable_challenge",
            "challenge_id": challenge_id.to_string(),
        }),
        domain::OutcomeTrigger::ModifyCharacterStat { stat, modifier } => json!({
            "type": "modify_character_stat",
            "stat": stat,
            "modifier": modifier,
        }),
        domain::OutcomeTrigger::TriggerScene { scene_id } => json!({
            "type": "trigger_scene",
            "scene_id": scene_id.to_string(),
        }),
        domain::OutcomeTrigger::GiveItem {
            item_name,
            item_description,
        } => json!({
            "type": "give_item",
            "item_name": item_name,
            "item_description": item_description,
        }),
        domain::OutcomeTrigger::Custom { description } => json!({
            "type": "custom",
            "description": description,
        }),
    }
}

fn trigger_condition_to_json(condition: &domain::TriggerCondition) -> serde_json::Value {
    json!({
        "condition_type": trigger_type_to_json(&condition.condition_type),
        "description": condition.description,
        "required": condition.required,
    })
}

fn trigger_type_to_json(trigger_type: &domain::TriggerType) -> serde_json::Value {
    match trigger_type {
        domain::TriggerType::ObjectInteraction { keywords } => json!({
            "type": "object_interaction",
            "keywords": keywords,
        }),
        domain::TriggerType::EnterArea { area_keywords } => json!({
            "type": "enter_area",
            "area_keywords": area_keywords,
        }),
        domain::TriggerType::DialogueTopic { topic_keywords } => json!({
            "type": "dialogue_topic",
            "topic_keywords": topic_keywords,
        }),
        domain::TriggerType::ChallengeComplete {
            challenge_id,
            requires_success,
        } => json!({
            "type": "challenge_complete",
            "challenge_id": challenge_id.to_string(),
            "requires_success": requires_success,
        }),
        domain::TriggerType::TimeBased { turns } => json!({
            "type": "time_based",
            "turns": turns,
        }),
        domain::TriggerType::NpcPresent { npc_keywords } => json!({
            "type": "npc_present",
            "npc_keywords": npc_keywords,
        }),
        domain::TriggerType::Custom { description } => json!({
            "type": "custom",
            "description": description,
        }),
    }
}

fn challenge_type_to_str(challenge_type: &domain::ChallengeType) -> &'static str {
    match challenge_type {
        domain::ChallengeType::SkillCheck => "skill_check",
        domain::ChallengeType::AbilityCheck => "ability_check",
        domain::ChallengeType::SavingThrow => "saving_throw",
        domain::ChallengeType::OpposedCheck => "opposed_check",
        domain::ChallengeType::ComplexChallenge => "complex_challenge",
        domain::ChallengeType::Unknown => "unknown",
    }
}

fn difficulty_to_json(difficulty: &domain::Difficulty) -> serde_json::Value {
    match difficulty {
        domain::Difficulty::DC(value) => json!({
            "type": "dc",
            "value": value,
        }),
        domain::Difficulty::Percentage(value) => json!({
            "type": "percentage",
            "value": value,
        }),
        domain::Difficulty::Descriptor(descriptor) => json!({
            "type": "descriptor",
            "value": format!("{descriptor:?}"),
        }),
        domain::Difficulty::Opposed => json!({
            "type": "opposed",
        }),
        domain::Difficulty::Custom(custom) => json!({
            "type": "custom",
            "value": custom,
        }),
    }
}
