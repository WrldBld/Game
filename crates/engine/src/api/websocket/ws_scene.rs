use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use serde_json::json;
use wrldbldr_domain::{self as domain, InteractionTarget, InteractionType};
use wrldbldr_shared::{ActRequest, ErrorCode, InteractionRequest, ResponseResult, SceneRequest};

pub(super) async fn handle_act_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: ActRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        ActRequest::ListActs { world_id } => {
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .act
                .list_in_world(world_id_typed)
                .await
            {
                Ok(acts) => {
                    let data: Vec<serde_json::Value> = acts.iter().map(act_to_json).collect();
                    Ok(ResponseResult::success(json!(data)))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "listing acts"),
                )),
            }
        }
        ActRequest::CreateAct { world_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .act
                .create(world_id_typed, data.name, data.description, data.order)
                .await
            {
                Ok(act) => Ok(ResponseResult::success(act_to_json(&act))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "creating act"),
                )),
            }
        }
    }
}

pub(super) async fn handle_scene_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: SceneRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        SceneRequest::ListScenes { act_id } => {
            let act_id_typed = parse_act_id_for_request(&act_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .scene
                .list_for_act(act_id_typed)
                .await
            {
                Ok(scenes) => {
                    let data: Vec<serde_json::Value> = scenes.iter().map(scene_to_json).collect();
                    Ok(ResponseResult::success(json!(data)))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "listing scenes"),
                )),
            }
        }
        SceneRequest::GetScene { scene_id } => {
            let scene_id_typed = parse_scene_id_for_request(&scene_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .scene
                .get(scene_id_typed)
                .await
            {
                Ok(Some(scene)) => Ok(ResponseResult::success(scene_to_json(&scene))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Scene not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "getting scene"),
                )),
            }
        }
        SceneRequest::CreateScene { act_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let act_id_typed = parse_act_id_for_request(&act_id, request_id)?;
            let location_id = match data.location_id {
                Some(id) => Some(parse_location_id_for_request(&id, request_id)?),
                None => None,
            };
            match state
                .app
                .use_cases
                .management
                .scene
                .create(act_id_typed, data.name, data.description, location_id)
                .await
            {
                Ok(scene) => Ok(ResponseResult::success(scene_to_json(&scene))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "creating scene"),
                )),
            }
        }
        SceneRequest::UpdateScene { scene_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let scene_id_typed = parse_scene_id_for_request(&scene_id, request_id)?;
            let location_id = match data.location_id {
                Some(id) => Some(parse_location_id_for_request(&id, request_id)?),
                None => None,
            };
            match state
                .app
                .use_cases
                .management
                .scene
                .update(scene_id_typed, data.name, data.description, location_id)
                .await
            {
                Ok(scene) => Ok(ResponseResult::success(scene_to_json(&scene))),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Scene not found"),
                ),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "updating scene"),
                )),
            }
        }
        SceneRequest::DeleteScene { scene_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let scene_id_typed = parse_scene_id_for_request(&scene_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .scene
                .delete(scene_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Scene not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "deleting scene"),
                )),
            }
        }
    }
}

pub(super) async fn handle_interaction_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: InteractionRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        InteractionRequest::ListInteractions { scene_id } => {
            let scene_id_typed = parse_scene_id_for_request(&scene_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .interaction
                .list_for_scene(scene_id_typed)
                .await
            {
                Ok(interactions) => {
                    let data: Vec<serde_json::Value> =
                        interactions.iter().map(interaction_to_json).collect();
                    Ok(ResponseResult::success(json!(data)))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "listing interactions"),
                )),
            }
        }
        InteractionRequest::GetInteraction { interaction_id } => {
            let interaction_id_typed =
                parse_interaction_id_for_request(&interaction_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .interaction
                .get(interaction_id_typed)
                .await
            {
                Ok(Some(interaction)) => {
                    Ok(ResponseResult::success(interaction_to_json(&interaction)))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Interaction not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "getting interaction"),
                )),
            }
        }
        InteractionRequest::CreateInteraction { scene_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let scene_id_typed = parse_scene_id_for_request(&scene_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .interaction
                .create(
                    scene_id_typed,
                    data.name,
                    data.description,
                    data.trigger,
                    data.available,
                )
                .await
            {
                Ok(interaction) => Ok(ResponseResult::success(interaction_to_json(&interaction))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "creating interaction"),
                )),
            }
        }
        InteractionRequest::UpdateInteraction {
            interaction_id,
            data,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let interaction_id_typed =
                parse_interaction_id_for_request(&interaction_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .interaction
                .update(
                    interaction_id_typed,
                    data.name,
                    data.description,
                    data.trigger,
                    data.available,
                )
                .await
            {
                Ok(interaction) => Ok(ResponseResult::success(interaction_to_json(&interaction))),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Interaction not found"),
                ),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "updating interaction"),
                )),
            }
        }
        InteractionRequest::DeleteInteraction { interaction_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let interaction_id_typed =
                parse_interaction_id_for_request(&interaction_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .interaction
                .delete(interaction_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Interaction not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "deleting interaction"),
                )),
            }
        }
        InteractionRequest::SetInteractionAvailability {
            interaction_id,
            available,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let interaction_id_typed =
                parse_interaction_id_for_request(&interaction_id, request_id)?;
            match state
                .app
                .use_cases
                .management
                .interaction
                .update(interaction_id_typed, None, None, None, Some(available))
                .await
            {
                Ok(interaction) => Ok(ResponseResult::success(interaction_to_json(&interaction))),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Interaction not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "setting interaction availability"),
                )),
            }
        }
    }
}

fn act_to_json(act: &domain::Act) -> serde_json::Value {
    json!({
        "id": act.id().to_string(),
        "world_id": act.world_id().to_string(),
        "name": act.name(),
        "stage": act.stage().to_string(),
        "description": act.description(),
        "order": act.order(),
    })
}

fn scene_to_json(scene: &domain::Scene) -> serde_json::Value {
    let entry_conditions = scene
        .entry_conditions()
        .iter()
        .map(|condition| format!("{:?}", condition))
        .collect::<Vec<_>>();

    json!({
        "id": scene.id().to_string(),
        "act_id": scene.act_id().to_string(),
        "name": scene.name(),
        "location_id": scene.location_id().to_string(),
        "time_context": format!("{:?}", scene.time_context()),
        "backdrop_override": scene.backdrop_override(),
        "featured_characters": scene.featured_characters().iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        "directorial_notes": scene.directorial_notes(),
        "entry_conditions": entry_conditions,
        "order": scene.order(),
    })
}

fn interaction_to_json(interaction: &domain::InteractionTemplate) -> serde_json::Value {
    let interaction_type = interaction_type_to_string(interaction.interaction_type());
    let target_name = interaction_target_name(interaction.target());
    let conditions = interaction
        .conditions()
        .iter()
        .map(|condition| format!("{:?}", condition))
        .collect::<Vec<_>>();

    json!({
        "id": interaction.id().to_string(),
        "scene_id": interaction.scene_id().to_string(),
        "name": interaction.name(),
        "interaction_type": interaction_type,
        "target_name": target_name,
        "is_available": interaction.is_available(),
        "prompt_hints": if interaction.prompt_hints().is_empty() { None } else { Some(interaction.prompt_hints().to_string()) },
        "conditions": conditions,
        "order": interaction.order(),
    })
}

fn interaction_type_to_string(interaction_type: &InteractionType) -> String {
    match interaction_type {
        InteractionType::Dialogue => "Dialogue".to_string(),
        InteractionType::Examine => "Examine".to_string(),
        InteractionType::UseItem => "UseItem".to_string(),
        InteractionType::PickUp => "PickUp".to_string(),
        InteractionType::GiveItem => "GiveItem".to_string(),
        InteractionType::Attack => "Attack".to_string(),
        InteractionType::Travel => "Travel".to_string(),
        InteractionType::Custom(custom) => custom.clone(),
    }
}

fn interaction_target_name(target: &InteractionTarget) -> Option<String> {
    match target {
        InteractionTarget::Environment(description) => Some(description.clone()),
        InteractionTarget::Character(_) | InteractionTarget::Item(_) | InteractionTarget::None => {
            None
        }
    }
}
