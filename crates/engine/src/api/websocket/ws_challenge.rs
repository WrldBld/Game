use super::*;
use crate::api::connections::ConnectionInfo;
use serde_json::json;
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
