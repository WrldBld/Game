use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use wrldbldr_protocol::character_sheet::CharacterSheetValues;
use wrldbldr_protocol::{ObservationRequest, PlayerCharacterRequest, RelationshipRequest};

pub(super) async fn handle_player_character_request(
    state: &WsState,
    request_id: &str,
    _conn_info: &ConnectionInfo,
    request: PlayerCharacterRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        PlayerCharacterRequest::ListPlayerCharacters { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .player_character
                .list_in_world(world_id_typed)
                .await
            {
                Ok(pcs) => {
                    let data: Vec<serde_json::Value> = pcs.into_iter().map(pc_to_json).collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "list player characters"),
                )),
            }
        }

        PlayerCharacterRequest::GetPlayerCharacter { pc_id } => {
            let pc_id_typed = match parse_pc_id(&pc_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .player_character
                .get(pc_id_typed)
                .await
            {
                Ok(Some(pc)) => Ok(ResponseResult::success(pc_to_json(pc))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Player character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get player character"),
                )),
            }
        }

        PlayerCharacterRequest::GetMyPlayerCharacter { world_id, user_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .player_character
                .get_by_user(world_id_typed, user_id)
                .await
            {
                Ok(Some(pc)) => Ok(ResponseResult::success(pc_to_json(pc))),
                Ok(None) => Ok(ResponseResult::success(None::<CharacterSheetValues>)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get my player character"),
                )),
            }
        }

        PlayerCharacterRequest::CreatePlayerCharacter { world_id, data } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let starting_region_id = match data.starting_region_id {
                Some(id) => Some(parse_region_id_for_request(&id, request_id)?),
                None => None,
            };

            match state
                .app
                .use_cases
                .management
                .player_character
                .create(
                    world_id_typed,
                    data.name,
                    data.user_id,
                    starting_region_id,
                    data.sheet_data,
                )
                .await
            {
                Ok(pc) => Ok(ResponseResult::success(pc_to_json(pc))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create player character"),
                )),
            }
        }

        PlayerCharacterRequest::UpdatePlayerCharacter { pc_id, data } => {
            let pc_id_typed = match parse_pc_id(&pc_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .player_character
                .update(pc_id_typed, data.name, data.sheet_data)
                .await
            {
                Ok(pc) => Ok(ResponseResult::success(pc_to_json(pc))),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Player character not found"),
                ),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "update player character"),
                )),
            }
        }

        PlayerCharacterRequest::DeletePlayerCharacter { pc_id } => {
            let pc_id_typed = match parse_pc_id(&pc_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .player_character
                .delete(pc_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Player character not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete player character"),
                )),
            }
        }

        PlayerCharacterRequest::UpdatePlayerCharacterLocation { pc_id, region_id } => {
            let pc_id_typed = match parse_pc_id(&pc_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let region_id_typed = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .player_character
                .update_location(pc_id_typed, region_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(serde_json::json!({
                    "success": true,
                    "scene_id": serde_json::Value::Null,
                }))),
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Player character not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "update player character location"),
                )),
            }
        }
    }
}

pub(super) async fn handle_relationship_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: RelationshipRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        RelationshipRequest::GetSocialNetwork { world_id } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .relationship
                .list_for_world(world_id_typed)
                .await
            {
                Ok(relationships) => {
                    let data: Vec<serde_json::Value> = relationships
                        .into_iter()
                        .map(|r| {
                            serde_json::json!({
                                "id": r.id().to_string(),
                                "from_character_id": r.from_character().to_string(),
                                "to_character_id": r.to_character().to_string(),
                                "relationship_type": relationship_type_to_string(r.relationship_type()),
                                "sentiment": r.sentiment(),
                                "known_to_player": r.known_to_player(),
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get social network"),
                )),
            }
        }

        RelationshipRequest::CreateRelationship { data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let from_id = match parse_character_id_for_request(&data.from_character_id, request_id)
            {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let to_id = match parse_character_id_for_request(&data.to_character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .relationship
                .create(from_id, to_id, data.relationship_type, data.description)
                .await
            {
                Ok(relationship) => Ok(ResponseResult::success(serde_json::json!({
                    "id": relationship.id().to_string(),
                }))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create relationship"),
                )),
            }
        }

        RelationshipRequest::DeleteRelationship { relationship_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let rel_id = match parse_uuid_for_request(
                &relationship_id,
                request_id,
                "Invalid relationship ID",
            ) {
                Ok(id) => wrldbldr_domain::RelationshipId::from(id),
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .relationship
                .delete(rel_id)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete relationship"),
                )),
            }
        }
    }
}

pub(super) async fn handle_observation_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: ObservationRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        ObservationRequest::ListObservations { pc_id } => {
            let pc_id_typed = match parse_pc_id(&pc_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .observation
                .list_summaries(pc_id_typed)
                .await
            {
                Ok(observations) => Ok(ResponseResult::success(serde_json::json!(observations))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "list observations"),
                )),
            }
        }

        ObservationRequest::CreateObservation { pc_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let pc_id_typed = match parse_pc_id(&pc_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let npc_id = match parse_character_id_for_request(&data.npc_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let location_id = match data.location_id {
                Some(id) => Some(parse_location_id_for_request(&id, request_id)?),
                None => None,
            };
            let region_id = match data.region_id {
                Some(id) => Some(parse_region_id_for_request(&id, request_id)?),
                None => None,
            };

            match state
                .app
                .use_cases
                .management
                .observation
                .create(
                    pc_id_typed,
                    npc_id,
                    data.observation_type,
                    location_id,
                    region_id,
                    data.notes,
                )
                .await
            {
                Ok(observation) => Ok(ResponseResult::success(serde_json::json!({
                    "npc_id": observation.npc_id().to_string(),
                    "location_id": observation.location_id().to_string(),
                    "region_id": observation.region_id().to_string(),
                    "observation_type": format!("{:?}", observation.observation_type()),
                }))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(crate::use_cases::management::ManagementError::NotFound) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Observation target not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create observation"),
                )),
            }
        }

        ObservationRequest::DeleteObservation { pc_id, npc_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let pc_id_typed = match parse_pc_id(&pc_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let npc_id_typed = match parse_character_id_for_request(&npc_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .observation
                .delete(pc_id_typed, npc_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete observation"),
                )),
            }
        }
    }
}

fn pc_to_json(pc: wrldbldr_domain::PlayerCharacter) -> serde_json::Value {
    let sheet_data = pc.sheet_data().cloned();
    serde_json::json!({
        "id": pc.id().to_string(),
        "user_id": pc.user_id(),
        "world_id": pc.world_id().to_string(),
        "name": pc.name().to_string(),
        "description": pc.description(),
        "sheet_data": sheet_data,
        "current_location_id": pc.current_location_id().to_string(),
        "starting_location_id": pc.starting_location_id().to_string(),
        "sprite_asset": pc.sprite_asset(),
        "portrait_asset": pc.portrait_asset(),
        "created_at": pc.created_at().to_rfc3339(),
        "last_active_at": pc.last_active_at().to_rfc3339(),
    })
}

fn relationship_type_to_string(relationship_type: &wrldbldr_domain::RelationshipType) -> String {
    match relationship_type {
        wrldbldr_domain::RelationshipType::Family(family) => format!("family:{:?}", family),
        wrldbldr_domain::RelationshipType::Romantic => "romantic".to_string(),
        wrldbldr_domain::RelationshipType::Professional => "professional".to_string(),
        wrldbldr_domain::RelationshipType::Rivalry => "rivalry".to_string(),
        wrldbldr_domain::RelationshipType::Friendship => "friendship".to_string(),
        wrldbldr_domain::RelationshipType::Mentorship => "mentorship".to_string(),
        wrldbldr_domain::RelationshipType::Enmity => "enmity".to_string(),
        wrldbldr_domain::RelationshipType::Custom(value) => value.clone(),
    }
}
