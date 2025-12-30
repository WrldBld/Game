//! Challenge domain request handlers
//!
//! Handles: Challenge CRUD, active/favorite status management

use std::sync::Arc;

use wrldbldr_domain::entities::{Challenge, ChallengeOutcomes};
use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_protocol::{CreateChallengeData, ErrorCode, ResponseResult, UpdateChallengeData};

use super::common::{parse_challenge_id, parse_difficulty, parse_skill_id, parse_world_id};
use crate::application::dto::ChallengeResponseDto;
use crate::application::services::ChallengeService;

/// Handle ListChallenges request
pub async fn list_challenges(
    challenge_service: &Arc<dyn ChallengeService>,
    world_id: &str,
) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match challenge_service.list_challenges(id).await {
        Ok(challenges) => {
            let dtos: Vec<ChallengeResponseDto> = challenges
                .into_iter()
                .map(ChallengeResponseDto::from_challenge_minimal)
                .collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetChallenge request
pub async fn get_challenge(
    challenge_service: &Arc<dyn ChallengeService>,
    challenge_id: &str,
) -> ResponseResult {
    let id = match parse_challenge_id(challenge_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match challenge_service.get_challenge(id).await {
        Ok(Some(challenge)) => {
            let dto = ChallengeResponseDto::from_challenge_minimal(challenge);
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Challenge not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteChallenge request (DM only)
pub async fn delete_challenge(
    challenge_service: &Arc<dyn ChallengeService>,
    ctx: &RequestContext,
    challenge_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_challenge_id(challenge_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match challenge_service.delete_challenge(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetChallengeActive request (DM only)
pub async fn set_challenge_active(
    challenge_service: &Arc<dyn ChallengeService>,
    ctx: &RequestContext,
    challenge_id: &str,
    active: bool,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_challenge_id(challenge_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match challenge_service.set_active(id, active).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetChallengeFavorite request (DM only)
///
/// Note: The `favorite` parameter is not used; this handler toggles the current state.
pub async fn set_challenge_favorite(
    challenge_service: &Arc<dyn ChallengeService>,
    ctx: &RequestContext,
    challenge_id: &str,
    _favorite: bool,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_challenge_id(challenge_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match challenge_service.toggle_favorite(id).await {
        Ok(is_favorite) => ResponseResult::success(serde_json::json!({ "is_favorite": is_favorite })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateChallenge request (DM only)
pub async fn create_challenge(
    challenge_service: &Arc<dyn ChallengeService>,
    ctx: &RequestContext,
    world_id: &str,
    data: CreateChallengeData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let wid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Parse difficulty from string
    let difficulty = parse_difficulty(&data.difficulty);

    // Create the challenge entity
    let challenge = Challenge::new(wid, data.name, difficulty)
        .with_description(data.description.unwrap_or_default());

    // Set outcomes if provided
    let challenge = if data.success_outcome.is_some() || data.failure_outcome.is_some() {
        let outcomes = ChallengeOutcomes::simple(
            data.success_outcome.unwrap_or_default(),
            data.failure_outcome.unwrap_or_default(),
        );
        challenge.with_outcomes(outcomes)
    } else {
        challenge
    };

    match challenge_service.create_challenge(challenge.clone()).await {
        Ok(created) => {
            // If skill_id was provided, set the required skill relationship
            if !data.skill_id.is_empty() {
                if let Ok(skill_id) = parse_skill_id(&data.skill_id) {
                    let _ = challenge_service
                        .set_required_skill(created.id, skill_id)
                        .await;
                }
            }
            let dto = ChallengeResponseDto::from_challenge_minimal(created);
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateChallenge request (DM only)
pub async fn update_challenge(
    challenge_service: &Arc<dyn ChallengeService>,
    ctx: &RequestContext,
    challenge_id: &str,
    data: UpdateChallengeData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_challenge_id(challenge_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Fetch existing challenge first
    let existing = match challenge_service.get_challenge(id).await {
        Ok(Some(c)) => c,
        Ok(None) => return ResponseResult::error(ErrorCode::NotFound, "Challenge not found"),
        Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    };

    // Apply updates
    let mut updated = existing;
    if let Some(name) = data.name {
        updated.name = name;
    }
    if let Some(description) = data.description {
        updated.description = description;
    }
    if let Some(ref difficulty_str) = data.difficulty {
        updated.difficulty = parse_difficulty(difficulty_str);
    }
    if data.success_outcome.is_some() || data.failure_outcome.is_some() {
        let outcomes = ChallengeOutcomes::simple(
            data.success_outcome
                .unwrap_or_else(|| updated.outcomes.success.description.clone()),
            data.failure_outcome
                .unwrap_or_else(|| updated.outcomes.failure.description.clone()),
        );
        updated.outcomes = outcomes;
    }

    match challenge_service.update_challenge(updated.clone()).await {
        Ok(result) => {
            // Update skill relationship if provided
            if let Some(ref skill_id_str) = data.skill_id {
                if let Ok(skill_id) = parse_skill_id(skill_id_str) {
                    let _ = challenge_service
                        .set_required_skill(result.id, skill_id)
                        .await;
                }
            }
            let dto = ChallengeResponseDto::from_challenge_minimal(result);
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
