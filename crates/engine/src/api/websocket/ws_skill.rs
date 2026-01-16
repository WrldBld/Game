use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use serde_json::json;
use wrldbldr_domain as domain;
use wrldbldr_protocol::{ErrorCode, ResponseResult, SkillRequest};

pub(super) async fn handle_skill_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: SkillRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        SkillRequest::ListSkills { world_id } => {
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .skill
                .list_in_world(world_id_typed)
                .await
            {
                Ok(skills) => {
                    let data: Vec<serde_json::Value> = skills.iter().map(skill_to_json).collect();
                    Ok(ResponseResult::success(json!(data)))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "listing skills"),
                )),
            }
        }
        SkillRequest::GetSkill { skill_id } => {
            let skill_id_typed = parse_skill_id_for_request(&skill_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .skill
                .get(skill_id_typed)
                .await
            {
                Ok(Some(skill)) => Ok(ResponseResult::success(skill_to_json(&skill))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Skill not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "getting skill"),
                )),
            }
        }
        SkillRequest::CreateSkill { world_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .skill
                .create(
                    world_id_typed,
                    data.name,
                    data.description,
                    data.category,
                    data.attribute,
                )
                .await
            {
                Ok(skill) => Ok(ResponseResult::success(skill_to_json(&skill))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(crate::use_cases::management::ManagementError::Domain(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "creating skill"),
                )),
            }
        }
        SkillRequest::UpdateSkill { skill_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let skill_id_typed = parse_skill_id_for_request(&skill_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .skill
                .update(
                    skill_id_typed,
                    data.name,
                    data.description,
                    data.category,
                    data.attribute,
                    data.is_hidden,
                )
                .await
            {
                Ok(skill) => Ok(ResponseResult::success(skill_to_json(&skill))),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Skill not found"),
                ),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(crate::use_cases::management::ManagementError::Domain(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "updating skill"),
                )),
            }
        }
        SkillRequest::DeleteSkill { skill_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let skill_id_typed = parse_skill_id_for_request(&skill_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .skill
                .delete(skill_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Skill not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "deleting skill"),
                )),
            }
        }
    }
}

fn skill_to_json(skill: &domain::Skill) -> serde_json::Value {
    json!({
        "id": skill.id().to_string(),
        "world_id": skill.world_id().to_string(),
        "name": skill.name(),
        "description": skill.description(),
        "category": skill.category().to_string(),
        "base_attribute": skill.base_attribute(),
        "is_custom": skill.is_custom(),
        "is_hidden": skill.is_hidden(),
        "order": skill.order(),
    })
}
