use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use wrldbldr_domain::{
    ActantialActor, ActantialContext, ActantialRole, ActantialTarget, CharacterId, GoalId, WantId,
    WantTarget, WantVisibility,
};
use wrldbldr_protocol::{
    messages::{
        ActantialActorData, ActantialRoleData, ActorTypeData, GoalData, NpcActantialContextData,
        SocialRelationData, SocialViewsData, WantData, WantTargetData, WantTargetTypeData,
        WantVisibilityData,
    },
    ActantialRequest, GoalRequest, WantRequest,
};

#[derive(Debug, serde::Serialize)]
struct WantResponse {
    id: String,
    description: String,
    intensity: f32,
    priority: u32,
    visibility: WantVisibilityData,
    target: Option<WantTargetData>,
    deflection_behavior: Option<String>,
    tells: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct GoalResponse {
    id: String,
    name: String,
    description: Option<String>,
}

pub(super) async fn handle_goal_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: GoalRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        GoalRequest::ListGoals { world_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .actantial
                .goals
                .list(world_id_typed)
                .await
            {
                Ok(goals) => {
                    let data: Vec<GoalResponse> = goals
                        .into_iter()
                        .map(|details| GoalResponse {
                            id: details.goal.id.to_string(),
                            name: details.goal.name,
                            description: details.goal.description,
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "list goals"),
                )),
            }
        }

        GoalRequest::GetGoal { goal_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let goal_id_typed = match parse_goal_id_for_request(&goal_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state.app.use_cases.actantial.goals.get(goal_id_typed).await {
                Ok(Some(details)) => Ok(ResponseResult::success(GoalResponse {
                    id: details.goal.id.to_string(),
                    name: details.goal.name,
                    description: details.goal.description,
                })),
                Ok(None) => Ok(ResponseResult::error(ErrorCode::NotFound, "Goal not found")),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get goal"),
                )),
            }
        }

        GoalRequest::CreateGoal { world_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .actantial
                .goals
                .create(world_id_typed, data.name, data.description)
                .await
            {
                Ok(details) => {
                    if let Some(world_id) = conn_info.world_id {
                        let msg = ServerMessage::GoalCreated {
                            world_id: world_id.to_string(),
                            goal: goal_details_to_data(&details),
                        };
                        state.connections.broadcast_to_world(world_id, msg).await;
                    }

                    Ok(ResponseResult::success(GoalResponse {
                        id: details.goal.id.to_string(),
                        name: details.goal.name,
                        description: details.goal.description,
                    }))
                }
                Err(crate::use_cases::actantial::ActantialError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create goal"),
                )),
            }
        }

        GoalRequest::UpdateGoal { goal_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let goal_id_typed = match parse_goal_id_for_request(&goal_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .actantial
                .goals
                .update(goal_id_typed, data.name, data.description)
                .await
            {
                Ok(details) => {
                    if let Some(world_id) = conn_info.world_id {
                        let msg = ServerMessage::GoalUpdated {
                            goal: goal_details_to_data(&details),
                        };
                        state.connections.broadcast_to_world(world_id, msg).await;
                    }

                    Ok(ResponseResult::success(GoalResponse {
                        id: details.goal.id.to_string(),
                        name: details.goal.name,
                        description: details.goal.description,
                    }))
                }
                Err(crate::use_cases::actantial::ActantialError::NotFound) => {
                    Ok(ResponseResult::error(ErrorCode::NotFound, "Goal not found"))
                }
                Err(crate::use_cases::actantial::ActantialError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "update goal"),
                )),
            }
        }

        GoalRequest::DeleteGoal { goal_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let goal_id_typed = match parse_goal_id_for_request(&goal_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .actantial
                .goals
                .delete(goal_id_typed)
                .await
            {
                Ok(()) => {
                    if let Some(world_id) = conn_info.world_id {
                        let msg = ServerMessage::GoalDeleted {
                            goal_id: goal_id_typed.to_string(),
                        };
                        state.connections.broadcast_to_world(world_id, msg).await;
                    }
                    Ok(ResponseResult::success_empty())
                }
                Err(crate::use_cases::actantial::ActantialError::NotFound) => {
                    Ok(ResponseResult::error(ErrorCode::NotFound, "Goal not found"))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete goal"),
                )),
            }
        }
    }
}

pub(super) async fn handle_want_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: WantRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        WantRequest::ListWants { character_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let character_id_typed = match parse_character_id_for_request(&character_id, request_id)
            {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .actantial
                .context
                .get_context(character_id_typed)
                .await
            {
                Ok(Some(context)) => {
                    let data: Vec<WantResponse> = context
                        .wants()
                        .iter()
                        .map(want_context_to_response)
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "list wants"),
                )),
            }
        }

        WantRequest::GetWant { want_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let want_id_typed = match parse_want_id_for_request(&want_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let details = match state.app.use_cases.actantial.wants.get(want_id_typed).await {
                Ok(Some(details)) => details,
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "Want not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get want"),
                    ));
                }
            };

            let response = match state
                .app
                .use_cases
                .actantial
                .context
                .get_context(details.character_id)
                .await
            {
                Ok(Some(context)) => context
                    .wants()
                    .iter()
                    .find(|want| want.want_id() == WantId::from_uuid(details.want.id.into()))
                    .map(want_context_to_response)
                    .unwrap_or_else(|| want_details_to_response(&details)),
                _ => want_details_to_response(&details),
            };

            Ok(ResponseResult::success(response))
        }

        WantRequest::CreateWant { character_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let character_id_typed = match parse_character_id_for_request(&character_id, request_id)
            {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let visibility = map_visibility_from_data(data.visibility);

            let mut details = match state
                .app
                .use_cases
                .actantial
                .wants
                .create(
                    character_id_typed,
                    data.description,
                    data.intensity,
                    data.priority,
                    visibility,
                    data.deflection_behavior,
                    data.tells,
                )
                .await
            {
                Ok(details) => details,
                Err(crate::use_cases::actantial::ActantialError::InvalidInput(msg)) => {
                    return Ok(ResponseResult::error(ErrorCode::BadRequest, msg));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "create want"),
                    ));
                }
            };

            if let (Some(target_id), Some(target_type)) = (data.target_id, data.target_type) {
                match map_want_target_ref(&target_id, target_type, request_id) {
                    Ok(target_ref) => match state
                        .app
                        .use_cases
                        .actantial
                        .wants
                        .set_target(details.want.id, target_ref)
                        .await
                    {
                        Ok(target) => details.target = Some(target),
                        Err(crate::use_cases::actantial::ActantialError::NotFound) => {
                            return Ok(ResponseResult::error(
                                ErrorCode::NotFound,
                                "Target not found",
                            ));
                        }
                        Err(e) => {
                            return Ok(ResponseResult::error(
                                ErrorCode::InternalError,
                                sanitize_repo_error(&e, "set want target"),
                            ));
                        }
                    },
                    Err(e) => return Err(e),
                }
            }

            if let Some(world_id) = conn_info.world_id {
                let want_data = resolve_want_data(state, details.character_id, details.want.id)
                    .await
                    .unwrap_or_else(|| want_details_to_data(&details));
                let msg = ServerMessage::NpcWantCreated {
                    npc_id: details.character_id.to_string(),
                    want: want_data,
                };
                state.connections.broadcast_to_dms(world_id, msg).await;
            }

            Ok(ResponseResult::success(want_details_to_response(&details)))
        }

        WantRequest::UpdateWant { want_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let want_id_typed = match parse_want_id_for_request(&want_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let visibility = data.visibility.map(map_visibility_from_data);

            let details = match state
                .app
                .use_cases
                .actantial
                .wants
                .update(
                    want_id_typed,
                    data.description,
                    data.intensity,
                    data.priority,
                    visibility,
                    data.deflection_behavior,
                    data.tells,
                )
                .await
            {
                Ok(details) => details,
                Err(crate::use_cases::actantial::ActantialError::NotFound) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "Want not found"));
                }
                Err(crate::use_cases::actantial::ActantialError::InvalidInput(msg)) => {
                    return Ok(ResponseResult::error(ErrorCode::BadRequest, msg));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "update want"),
                    ));
                }
            };

            if let Some(world_id) = conn_info.world_id {
                let want_data = resolve_want_data(state, details.character_id, details.want.id)
                    .await
                    .unwrap_or_else(|| want_details_to_data(&details));
                let msg = ServerMessage::NpcWantUpdated {
                    npc_id: details.character_id.to_string(),
                    want: want_data,
                };
                state.connections.broadcast_to_dms(world_id, msg).await;
            }

            Ok(ResponseResult::success(want_details_to_response(&details)))
        }

        WantRequest::DeleteWant { want_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let want_id_typed = match parse_want_id_for_request(&want_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let details = match state.app.use_cases.actantial.wants.get(want_id_typed).await {
                Ok(Some(details)) => details,
                Ok(None) => {
                    return Ok(ResponseResult::error(ErrorCode::NotFound, "Want not found"));
                }
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "get want"),
                    ));
                }
            };

            match state
                .app
                .use_cases
                .actantial
                .wants
                .delete(want_id_typed)
                .await
            {
                Ok(()) => {
                    if let Some(world_id) = conn_info.world_id {
                        let msg = ServerMessage::NpcWantDeleted {
                            npc_id: details.character_id.to_string(),
                            want_id: want_id_typed.to_string(),
                        };
                        state.connections.broadcast_to_dms(world_id, msg).await;
                    }
                    Ok(ResponseResult::success_empty())
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete want"),
                )),
            }
        }

        WantRequest::SetWantTarget {
            want_id,
            target_id,
            target_type,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let want_id_typed = match parse_want_id_for_request(&want_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let target_ref = match map_want_target_ref(&target_id, target_type, request_id) {
                Ok(target_ref) => target_ref,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .actantial
                .wants
                .set_target(want_id_typed, target_ref)
                .await
            {
                Ok(target) => {
                    if let Some(world_id) = conn_info.world_id {
                        let msg = ServerMessage::WantTargetSet {
                            want_id: want_id_typed.to_string(),
                            target: want_target_to_data(&target),
                        };
                        state.connections.broadcast_to_dms(world_id, msg).await;
                    }
                    Ok(ResponseResult::success_empty())
                }
                Err(crate::use_cases::actantial::ActantialError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Target not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "set want target"),
                )),
            }
        }

        WantRequest::RemoveWantTarget { want_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let want_id_typed = match parse_want_id_for_request(&want_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .actantial
                .wants
                .remove_target(want_id_typed)
                .await
            {
                Ok(()) => {
                    if let Some(world_id) = conn_info.world_id {
                        let msg = ServerMessage::WantTargetRemoved {
                            want_id: want_id_typed.to_string(),
                        };
                        state.connections.broadcast_to_dms(world_id, msg).await;
                    }
                    Ok(ResponseResult::success_empty())
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "remove want target"),
                )),
            }
        }
    }
}

pub(super) async fn handle_actantial_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: ActantialRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        ActantialRequest::GetActantialContext { character_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let character_id_typed = match parse_character_id_for_request(&character_id, request_id)
            {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .actantial
                .context
                .get_context(character_id_typed)
                .await
            {
                Ok(Some(context)) => {
                    Ok(ResponseResult::success(actantial_context_to_data(&context)))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get actantial context"),
                )),
            }
        }

        ActantialRequest::AddActantialView {
            character_id,
            want_id,
            target_id,
            target_type,
            role,
            reason,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let character_id_typed = match parse_character_id_for_request(&character_id, request_id)
            {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let want_id_typed = match parse_want_id_for_request(&want_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let target = match map_actantial_target(&target_id, target_type, request_id) {
                Ok(target) => target,
                Err(e) => return Err(e),
            };
            let role = match map_actantial_role(role, request_id) {
                Ok(role) => role,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .actantial
                .context
                .add_view(character_id_typed, want_id_typed, target, role, reason)
                .await
            {
                Ok(record) => {
                    if let Some(world_id) = conn_info.world_id {
                        let msg = ServerMessage::ActantialViewAdded {
                            npc_id: character_id_typed.to_string(),
                            view: actantial_view_record_to_data(&record),
                        };
                        state.connections.broadcast_to_dms(world_id, msg).await;
                    }
                    Ok(ResponseResult::success_empty())
                }
                Err(crate::use_cases::actantial::ActantialError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Target not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "add actantial view"),
                )),
            }
        }

        ActantialRequest::RemoveActantialView {
            character_id,
            want_id,
            target_id,
            target_type,
            role,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let character_id_typed = match parse_character_id_for_request(&character_id, request_id)
            {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let want_id_typed = match parse_want_id_for_request(&want_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let target = match map_actantial_target(&target_id, target_type, request_id) {
                Ok(target) => target,
                Err(e) => return Err(e),
            };
            let role = match map_actantial_role(role, request_id) {
                Ok(role) => role,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .actantial
                .context
                .remove_view(character_id_typed, want_id_typed, target.clone(), role)
                .await
            {
                Ok(()) => {
                    if let Some(world_id) = conn_info.world_id {
                        let msg = ServerMessage::ActantialViewRemoved {
                            npc_id: character_id_typed.to_string(),
                            want_id: want_id_typed.to_string(),
                            target_id: target.id_string(),
                            role: map_actantial_role_data(role),
                        };
                        state.connections.broadcast_to_dms(world_id, msg).await;
                    }
                    Ok(ResponseResult::success_empty())
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "remove actantial view"),
                )),
            }
        }
    }
}

fn want_details_to_response(details: &crate::infrastructure::ports::WantDetails) -> WantResponse {
    WantResponse {
        id: details.want.id.to_string(),
        description: details.want.description.clone(),
        intensity: details.want.intensity,
        priority: details.priority,
        visibility: map_visibility_data(details.want.visibility),
        target: details.target.as_ref().map(want_target_to_data),
        deflection_behavior: details.want.deflection_behavior.clone(),
        tells: details.want.tells.first().cloned(),
    }
}

fn want_context_to_response(want: &wrldbldr_domain::WantContext) -> WantResponse {
    WantResponse {
        id: want.want_id().to_string(),
        description: want.description().to_string(),
        intensity: want.intensity(),
        priority: want.priority(),
        visibility: map_visibility_data(want.visibility()),
        target: want.target().map(want_target_to_data),
        deflection_behavior: want.deflection_behavior().map(|s| s.to_string()),
        tells: want.tells().first().cloned(),
    }
}

fn want_details_to_data(details: &crate::infrastructure::ports::WantDetails) -> WantData {
    WantData {
        id: details.want.id.to_string(),
        description: details.want.description.clone(),
        intensity: details.want.intensity,
        priority: details.priority,
        visibility: map_visibility_data(details.want.visibility),
        target: details.target.as_ref().map(want_target_to_data),
        deflection_behavior: details.want.deflection_behavior.clone(),
        tells: details.want.tells.clone(),
        helpers: Vec::new(),
        opponents: Vec::new(),
        sender: None,
        receiver: None,
    }
}

fn want_context_to_data(want: &wrldbldr_domain::WantContext) -> WantData {
    WantData {
        id: want.want_id().to_string(),
        description: want.description().to_string(),
        intensity: want.intensity(),
        priority: want.priority(),
        visibility: map_visibility_data(want.visibility()),
        target: want.target().map(want_target_to_data),
        deflection_behavior: want.deflection_behavior().map(|s| s.to_string()),
        tells: want.tells().to_vec(),
        helpers: want.helpers().iter().map(actantial_actor_to_data).collect(),
        opponents: want
            .opponents()
            .iter()
            .map(actantial_actor_to_data)
            .collect(),
        sender: want.sender().map(actantial_actor_to_data),
        receiver: want.receiver().map(actantial_actor_to_data),
    }
}

fn actantial_actor_to_data(actor: &ActantialActor) -> ActantialActorData {
    ActantialActorData {
        id: actor.target().id_string(),
        name: actor.name().to_string(),
        actor_type: map_actor_type_data(actor.target().actor_type()),
        reason: actor.reason().to_string(),
    }
}

fn want_target_to_data(target: &WantTarget) -> WantTargetData {
    match target {
        WantTarget::Character { id, name } => WantTargetData {
            id: id.to_string(),
            name: name.clone(),
            target_type: WantTargetTypeData::Character,
            description: None,
        },
        WantTarget::Item { id, name } => WantTargetData {
            id: id.to_string(),
            name: name.clone(),
            target_type: WantTargetTypeData::Item,
            description: None,
        },
        WantTarget::Goal {
            id,
            name,
            description,
        } => WantTargetData {
            id: id.to_string(),
            name: name.clone(),
            target_type: WantTargetTypeData::Goal,
            description: description.clone(),
        },
    }
}

fn actantial_context_to_data(context: &ActantialContext) -> NpcActantialContextData {
    let wants = context.wants().iter().map(want_context_to_data).collect();

    let allies = context
        .social_views()
        .allies()
        .iter()
        .map(|(target, name, reasons)| SocialRelationData {
            id: target.id_string(),
            name: name.clone(),
            actor_type: map_actor_type_data(target.actor_type()),
            reasons: reasons.clone(),
        })
        .collect();

    let enemies = context
        .social_views()
        .enemies()
        .iter()
        .map(|(target, name, reasons)| SocialRelationData {
            id: target.id_string(),
            name: name.clone(),
            actor_type: map_actor_type_data(target.actor_type()),
            reasons: reasons.clone(),
        })
        .collect();

    NpcActantialContextData {
        npc_id: context.character_id().to_string(),
        npc_name: context.character_name().to_string(),
        wants,
        social_views: SocialViewsData { allies, enemies },
    }
}

fn goal_details_to_data(details: &crate::infrastructure::ports::GoalDetails) -> GoalData {
    GoalData {
        id: details.goal.id.to_string(),
        name: details.goal.name.clone(),
        description: details.goal.description.clone(),
        usage_count: details.usage_count,
    }
}

fn actantial_view_record_to_data(
    record: &crate::infrastructure::ports::ActantialViewRecord,
) -> wrldbldr_protocol::messages::ActantialViewData {
    wrldbldr_protocol::messages::ActantialViewData {
        want_id: record.want_id.to_string(),
        target_id: record.target.id_string(),
        target_name: record.target_name.clone(),
        target_type: map_actor_type_data(record.target.actor_type()),
        role: map_actantial_role_data(record.role),
        reason: record.reason.clone(),
    }
}

fn map_visibility_data(visibility: WantVisibility) -> WantVisibilityData {
    match visibility {
        WantVisibility::Known => WantVisibilityData::Known,
        WantVisibility::Suspected => WantVisibilityData::Suspected,
        WantVisibility::Hidden => WantVisibilityData::Hidden,
    }
}

fn map_visibility_from_data(visibility: WantVisibilityData) -> WantVisibility {
    match visibility {
        WantVisibilityData::Known => WantVisibility::Known,
        WantVisibilityData::Suspected => WantVisibility::Suspected,
        WantVisibilityData::Hidden | WantVisibilityData::Unknown => WantVisibility::Hidden,
    }
}

fn map_actor_type_data(actor_type: wrldbldr_domain::ActorType) -> ActorTypeData {
    match actor_type {
        wrldbldr_domain::ActorType::Npc => ActorTypeData::Npc,
        wrldbldr_domain::ActorType::Pc => ActorTypeData::Pc,
    }
}

fn map_actantial_role_data(role: ActantialRole) -> ActantialRoleData {
    match role {
        ActantialRole::Helper => ActantialRoleData::Helper,
        ActantialRole::Opponent => ActantialRoleData::Opponent,
        ActantialRole::Sender => ActantialRoleData::Sender,
        ActantialRole::Receiver => ActantialRoleData::Receiver,
        ActantialRole::Unknown => ActantialRoleData::Unknown,
    }
}

fn map_actantial_role(
    role: ActantialRoleData,
    request_id: &str,
) -> Result<ActantialRole, ServerMessage> {
    match role {
        ActantialRoleData::Helper => Ok(ActantialRole::Helper),
        ActantialRoleData::Opponent => Ok(ActantialRole::Opponent),
        ActantialRoleData::Sender => Ok(ActantialRole::Sender),
        ActantialRoleData::Receiver => Ok(ActantialRole::Receiver),
        ActantialRoleData::Unknown => Err(ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(ErrorCode::BadRequest, "Invalid actantial role"),
        }),
    }
}

fn map_actantial_target(
    target_id: &str,
    target_type: ActorTypeData,
    request_id: &str,
) -> Result<ActantialTarget, ServerMessage> {
    let target_uuid = parse_uuid_for_request(target_id, request_id, "Invalid target ID")?;
    match target_type {
        ActorTypeData::Npc => Ok(ActantialTarget::npc(CharacterId::from_uuid(target_uuid))),
        ActorTypeData::Pc => Ok(ActantialTarget::pc(
            wrldbldr_domain::PlayerCharacterId::from_uuid(target_uuid),
        )),
        ActorTypeData::Unknown => Err(ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(ErrorCode::BadRequest, "Invalid target type"),
        }),
    }
}

fn map_want_target_ref(
    target_id: &str,
    target_type: WantTargetTypeData,
    request_id: &str,
) -> Result<crate::infrastructure::ports::WantTargetRef, ServerMessage> {
    let target_uuid = parse_uuid_for_request(target_id, request_id, "Invalid target ID")?;
    let target = match target_type {
        WantTargetTypeData::Character => crate::infrastructure::ports::WantTargetRef::Character(
            CharacterId::from_uuid(target_uuid),
        ),
        WantTargetTypeData::Item => crate::infrastructure::ports::WantTargetRef::Item(
            wrldbldr_domain::ItemId::from_uuid(target_uuid),
        ),
        WantTargetTypeData::Goal => {
            crate::infrastructure::ports::WantTargetRef::Goal(GoalId::from_uuid(target_uuid))
        }
        WantTargetTypeData::Unknown => {
            return Err(ServerMessage::Response {
                request_id: request_id.to_string(),
                result: ResponseResult::error(ErrorCode::BadRequest, "Invalid target type"),
            });
        }
    };
    Ok(target)
}

async fn resolve_want_data(
    state: &WsState,
    character_id: CharacterId,
    want_id: WantId,
) -> Option<WantData> {
    let context = state
        .app
        .use_cases
        .actantial
        .context
        .get_context(character_id)
        .await
        .ok()
        .flatten()?;

    context
        .wants()
        .iter()
        .find(|want| want.want_id() == want_id)
        .map(want_context_to_data)
}
