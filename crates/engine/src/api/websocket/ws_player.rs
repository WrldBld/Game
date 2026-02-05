use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::api::websocket::apply_pagination_limits;
use wrldbldr_domain::UserId;
use wrldbldr_shared::character_sheet::CharacterSheetValues;
use wrldbldr_shared::{ObservationRequest, PlayerCharacterRequest, RelationshipRequest};

pub(super) async fn handle_player_character_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: PlayerCharacterRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        // Note: list_in_world doesn't support pagination yet
        PlayerCharacterRequest::ListPlayerCharacters { world_id, limit: _, offset: _ } => {
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
                Ok(Some(pc)) => {
                    // Verify ownership (allow DMs to access any PC)
                    if !conn_info.is_dm() && pc.user_id() != conn_info.user_id.as_str() {
                        return Err(ServerMessage::Response {
                            request_id: request_id.to_string(),
                            result: ResponseResult::error(
                                ErrorCode::Unauthorized,
                                "Cannot access other player's character",
                            ),
                        });
                    }
                    Ok(ResponseResult::success(pc_to_json(pc)))
                }
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

            // Parse user_id from request (validated type)
            let user_id = match UserId::new(user_id) {
                Ok(id) => id,
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            "Invalid user ID",
                        ),
                    })
                }
            };

            // Verify user_id matches connection's user_id (no spoofing)
            if user_id != conn_info.user_id {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::Unauthorized,
                        "Cannot access other user's character",
                    ),
                });
            }

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
                    Some(conn_info.user_id.as_str().to_string()),  // Use authenticated user
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

            // Verify ownership (allow DMs to update any PC)
            if !conn_info.is_dm() && conn_info.pc_id != Some(pc_id_typed) {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::Unauthorized,
                        "Can only update your own player character",
                    ),
                });
            }

            match state
                .app
                .use_cases
                .management
                .player_character
                .update(pc_id_typed, data.name, data.sheet_data)
                .await
            {
                Ok(pc) => Ok(ResponseResult::success(pc_to_json(pc))),
                Err(crate::use_cases::management::ManagementError::NotFound { .. }) => Ok(
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

            // Verify ownership (allow DMs to delete any PC)
            if !conn_info.is_dm() && conn_info.pc_id != Some(pc_id_typed) {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::Unauthorized,
                        "Can only delete your own player character",
                    ),
                });
            }

            match state
                .app
                .use_cases
                .management
                .player_character
                .delete(pc_id_typed)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound { .. }) => Ok(
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

            // Verify ownership (allow DMs to update location of any PC)
            if !conn_info.is_dm() && conn_info.pc_id != Some(pc_id_typed) {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::Unauthorized,
                        "Can only update your own player character location",
                    ),
                });
            }

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
                Err(crate::use_cases::management::ManagementError::NotFound { .. }) => Ok(
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
        RelationshipRequest::GetSocialNetwork { world_id, limit, offset } => {
            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let settings = match state
                .app
                .use_cases
                .settings
                .get_for_world(world_id_typed)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        world_id = %world_id,
                        "Failed to load settings for get social network, using defaults"
                    );
                    crate::infrastructure::app_settings::AppSettings::default()
                }
            };
            let (limit, offset) = apply_pagination_limits(&settings, limit, offset);

            match state
                .app
                .use_cases
                .management
                .relationship
                .list_for_world(world_id_typed, Some(limit), offset)
                .await
            {
                Ok(relationships) => {
                    let data: Vec<serde_json::Value> = relationships
                        .into_iter()
                        .map(|r| {
                            serde_json::json!({
                                "id": r.id.to_string(),
                                "from_character_id": r.from_character.to_string(),
                                "to_character_id": r.to_character.to_string(),
                                "relationship_type": relationship_type_to_string(&r.relationship_type),
                                "sentiment": r.sentiment,
                                "known_to_player": r.known_to_player,
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
            require_dm_for_request(conn_info, request_id)?;

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
                    "id": relationship.id.to_string(),
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
            require_dm_for_request(conn_info, request_id)?;

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

            // Enforce auth: only DM or matching PC can access observations
            if !conn_info.is_dm() && conn_info.pc_id != Some(pc_id_typed) {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::Unauthorized,
                        "Can only list your own observations",
                    ),
                });
            }

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
            require_dm_for_request(conn_info, request_id)?;

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
                Ok(observation) => {
                    // Broadcast ObservationUpdated to the PC
                    match state
                        .app
                        .use_cases
                        .management
                        .observation
                        .list_summaries(pc_id_typed)
                        .await
                    {
                        Ok(summaries) => {
                            if let Some(summary) = summaries.iter().find(|s| s.npc_id == observation.npc_id().to_string()) {
                                let msg = ServerMessage::ObservationUpdated {
                                    pc_id: pc_id.clone(),
                                    observation: summary.clone(),
                                };
                                state.connections.send_to_pc(pc_id_typed, msg).await;
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to query observation for broadcast");
                        }
                    }

                    Ok(ResponseResult::success(serde_json::json!({
                        "npc_id": observation.npc_id().to_string(),
                        "location_id": observation.location_id().to_string(),
                        "region_id": observation.region_id().to_string(),
                        "observation_type": format!("{:?}", observation.observation_type()),
                    })))
                }
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(crate::use_cases::management::ManagementError::NotFound { .. }) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Observation target not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create observation"),
                )),
            }
        }

        ObservationRequest::DeleteObservation { pc_id, npc_id } => {
            require_dm_for_request(conn_info, request_id)?;

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
                Ok(()) => {
                    // Broadcast ObservationDeleted to the PC
                    let msg = ServerMessage::ObservationDeleted {
                        pc_id: pc_id.clone(),
                        npc_id: npc_id.clone(),
                    };
                    state.connections.send_to_pc(pc_id_typed, msg).await;

                    Ok(ResponseResult::success_empty())
                }
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;
    use wrldbldr_domain::{PlayerCharacterId, UserId, WorldId};
    use crate::api::websocket::test_support::*;
    use crate::api::websocket::{WsState, PendingStagingStoreImpl, TimeSuggestionStore, GenerationStateStoreImpl};
    use crate::api::connections::ConnectionManager;
    use wrldbldr_shared::{ClientMessage, RequestPayload, ServerMessage};
    use crate::api::websocket::ProtoWorldRole;

    #[tokio::test]
    async fn list_observations_unauthorized_for_different_pc() {
        let now = chrono::Utc::now();
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();
        let other_pc_id = PlayerCharacterId::new();
        let user_id = UserId::new("user1").expect("test: user_id should be valid");

        // Setup world repo
        let mut world_repo = MockWorldRepo::new();
        let world = crate::test_fixtures::world_seeder::create_test_world(world_id, now);
        world_repo
            .expect_get()
            .returning(move |_| Ok(Some(world.clone())));

        let mut repos = TestAppRepos::new(world_repo);
        // JoinWorld with pc_id triggers player_character_repo.get() to load the PC
        let pc_for_get = crate::test_fixtures::world_seeder::create_test_pc(pc_id, world_id, now);
        repos.player_character_repo
            .expect_get()
            .returning(move |_| Ok(Some(pc_for_get.clone())));
        let app = build_test_app(repos, now);
        let connections = Arc::new(ConnectionManager::new());

        let ws_state = Arc::new(WsState {
            app,
            connections,
            pending_time_suggestions: Arc::new(TimeSuggestionStore::new()),
            pending_staging_requests: Arc::new(PendingStagingStoreImpl::new()),
            generation_read_state: GenerationStateStoreImpl::new(),
        });

        let (addr, server) = spawn_ws_server(ws_state.clone()).await;
        let mut ws = ws_connect(addr).await;

        // Join world as player with pc_id
        ws_send_client(
            &mut ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Player,
                user_id: user_id.to_string(),
                pc_id: Some(*pc_id.as_uuid()),
                spectate_pc_id: None,
            },
        ).await;

        let _ = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        }).await;

        // Try to list observations for different PC using RequestPayload
        let request_id = uuid::Uuid::new_v4().to_string();
        ws_send_client(
            &mut ws,
            &ClientMessage::Request {
                request_id: request_id.clone(),
                payload: RequestPayload::Observation(
                    wrldbldr_shared::ObservationRequest::ListObservations {
                        pc_id: other_pc_id.to_string(),
                    }
                ),
            },
        ).await;

        // Expect error response
        let response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
            if let ServerMessage::Response { request_id: rid, .. } = m {
                rid == &request_id
            } else {
                false
            }
        }).await;

        if let ServerMessage::Response { result, .. } = response {
            assert!(!result.is_success(), "Should get Unauthorized error");
        } else {
            panic!("Expected Response message");
        }

        server.abort();
    }

    #[tokio::test]
    async fn list_observations_allows_dm_access() {
        let now = chrono::Utc::now();
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();
        let dm_user_id = UserId::new("dm").expect("test: user_id should be valid");

        // Setup world repo
        let mut world_repo = MockWorldRepo::new();
        let world = crate::test_fixtures::world_seeder::create_test_world(world_id, now);
        world_repo
            .expect_get()
            .returning(move |_| Ok(Some(world.clone())));

        let mut repos = TestAppRepos::new(world_repo);
        // DM listing observations calls observation_repo.get_observations()
        repos.observation_repo
            .expect_get_observations()
            .returning(|_| Ok(vec![]));
        let app = build_test_app(repos, now);
        let connections = Arc::new(ConnectionManager::new());

        let ws_state = Arc::new(WsState {
            app,
            connections,
            pending_time_suggestions: Arc::new(TimeSuggestionStore::new()),
            pending_staging_requests: Arc::new(PendingStagingStoreImpl::new()),
            generation_read_state: GenerationStateStoreImpl::new(),
        });

        let (addr, server) = spawn_ws_server(ws_state.clone()).await;
        let mut ws = ws_connect(addr).await;

        // Join world as DM
        ws_send_client(
            &mut ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Dm,
                user_id: dm_user_id.to_string(),
                pc_id: None,
                spectate_pc_id: None,
            },
        ).await;

        let _ = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        }).await;

        // DM can list observations for any PC using RequestPayload
        let request_id = uuid::Uuid::new_v4().to_string();
        ws_send_client(
            &mut ws,
            &ClientMessage::Request {
                request_id: request_id.clone(),
                payload: RequestPayload::Observation(
                    wrldbldr_shared::ObservationRequest::ListObservations {
                        pc_id: pc_id.to_string(),
                    }
                ),
            },
        ).await;

        // Expect success response (or NotFound if PC doesn't exist, but not Unauthorized)
        let response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
            if let ServerMessage::Response { request_id: rid, .. } = m {
                rid == &request_id
            } else {
                false
            }
        }).await;

        if let ServerMessage::Response { result, .. } = response {
            // DM should not get Unauthorized - they can access any PC
            // They might get NotFound if PC doesn't exist, but that's ok
            let is_unauthorized = match &result {
                wrldbldr_shared::ResponseResult::Error { code, .. } => *code == wrldbldr_shared::ErrorCode::Unauthorized,
                _ => false,
            };
            assert!(!is_unauthorized, "DM should not get Unauthorized error");
        } else {
            panic!("Expected Response message");
        }

        server.abort();
    }
}
