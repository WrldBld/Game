use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::api::websocket::apply_pagination_limits;

use wrldbldr_shared::CharacterRequest;

pub(super) async fn handle_character_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: CharacterRequest,
) -> Result<ResponseResult, ServerMessage> {
    // Log correlation context for request tracing
    let correlation_id = conn_info.correlation_id;
    tracing::debug!(
        request_id,
        connection_id = %conn_info.connection_id,
        correlation_id = %correlation_id,
        correlation_id_short = %correlation_id.short(),
        request_type = ?request,
        "Handling character request"
    );

    match request {
        CharacterRequest::ListCharacters { world_id, limit, offset } => {
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
                        "Failed to load settings for list characters, using defaults"
                    );
                    crate::infrastructure::app_settings::AppSettings::default()
                }
            };
            let (limit, offset) = apply_pagination_limits(&settings, limit, offset);

            match state
                .app
                .use_cases
                .management
                .character
                .list_in_world(world_id_typed, Some(limit), offset)
                .await
            {
                Ok(chars) => {
                    let data: Vec<serde_json::Value> = chars
                        .into_iter()
                        .map(|c| {
                            serde_json::json!({
                                "id": c.id().to_string(),
                                "name": c.name().to_string(),
                                "archetype": Some(c.current_archetype().to_string()),
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        operation = "list characters",
                        correlation_id = %correlation_id,
                        correlation_id_short = %correlation_id.short(),
                        "Repository error"
                    );
                    Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "list characters"),
                    ))
                }
            }
        }

        CharacterRequest::GetCharacter { character_id } => {
            let char_id = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state.app.use_cases.management.character.get(char_id).await {
                Ok(Some(character)) => {
                    // Cross-world validation: non-DMs can only access characters from their world
                    if !conn_info.is_dm() {
                        if let Some(world_id) = conn_info.world_id {
                            if character.world_id() != world_id {
                                return Ok(ResponseResult::error(
                                    ErrorCode::Unauthorized,
                                    "Character not in current world",
                                ));
                            }
                        } else {
                            return Ok(ResponseResult::error(
                                ErrorCode::BadRequest,
                                "Not connected to a world",
                            ));
                        }
                    }

                    Ok(ResponseResult::success(serde_json::json!({
                        "id": character.id().to_string(),
                        "name": character.name().to_string(),
                        "description": if character.description().is_empty() { None } else { Some(character.description().to_string()) },
                        "archetype": Some(character.current_archetype().to_string()),
                        "sprite_asset": character.sprite_asset(),
                        "portrait_asset": character.portrait_asset(),
                        "sheet_data": None::<wrldbldr_shared::character_sheet::CharacterSheetValues>,
                    })))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get character"),
                )),
            }
        }

        CharacterRequest::CreateCharacter { world_id, data } => {
            require_dm_for_request(conn_info, request_id)?;

            let world_id_typed = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .character
                .create(
                    world_id_typed,
                    data.name,
                    data.description,
                    data.archetype,
                    data.sprite_asset,
                    data.portrait_asset,
                )
                .await
            {
                Ok(character) => Ok(ResponseResult::success(serde_json::json!({
                    "id": character.id().to_string(),
                    "name": character.name().to_string(),
                    "description": if character.description().is_empty() { None } else { Some(character.description().to_string()) },
                    "archetype": Some(character.current_archetype().to_string()),
                    "sprite_asset": character.sprite_asset(),
                    "portrait_asset": character.portrait_asset(),
                    "sheet_data": None::<wrldbldr_shared::character_sheet::CharacterSheetValues>,
                }))),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        CharacterRequest::UpdateCharacter { character_id, data } => {
            require_dm_for_request(conn_info, request_id)?;

            let char_id = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let world_id = conn_info.world_id.ok_or_else(|| {
                error_response(
                    ErrorCode::BadRequest,
                    "Not connected to a world",
                )
            })?;

            match state
                .app
                .use_cases
                .management
                .character
                .update(
                    world_id,
                    char_id,
                    data.name,
                    data.description,
                    data.sprite_asset,
                    data.portrait_asset,
                    data.is_alive,
                    data.is_active,
                )
                .await
            {
                Ok(character) => Ok(ResponseResult::success(serde_json::json!({
                    "id": character.id().to_string(),
                    "name": character.name().to_string(),
                    "description": if character.description().is_empty() { None } else { Some(character.description().to_string()) },
                    "archetype": Some(character.current_archetype().to_string()),
                    "sprite_asset": character.sprite_asset(),
                    "portrait_asset": character.portrait_asset(),
                    "sheet_data": None::<wrldbldr_shared::character_sheet::CharacterSheetValues>,
                }))),
                Err(crate::use_cases::management::ManagementError::NotFound { .. }) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Character not found"),
                ),
                Err(crate::use_cases::management::ManagementError::Unauthorized { .. }) => Ok(
                    ResponseResult::error(ErrorCode::Unauthorized, "Character not in current world"),
                ),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        CharacterRequest::DeleteCharacter { character_id } => {
            require_dm_for_request(conn_info, request_id)?;

            let char_id = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .character
                .delete(char_id)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound { .. }) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Character not found"),
                ),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        CharacterRequest::ChangeArchetype { character_id, data } => {
            require_dm_for_request(conn_info, request_id)?;

            let char_id = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .management
                .character
                .change_archetype(char_id, data.new_archetype, data.reason)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(crate::use_cases::management::ManagementError::NotFound { .. }) => Ok(
                    ResponseResult::error(ErrorCode::NotFound, "Character not found"),
                ),
                Err(crate::use_cases::management::ManagementError::InvalidInput(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        CharacterRequest::GetCharacterInventory { character_id } => {
            let _ = conn_info;
            let char_uuid =
                match parse_uuid_for_request(&character_id, request_id, "Invalid character ID") {
                    Ok(id) => id,
                    Err(e) => return Err(e),
                };

            let pc_id = PlayerCharacterId::from_uuid(char_uuid);
            let items = match state
                .app
                .repositories
                .player_character
                .get_inventory(pc_id)
                .await
            {
                Ok(items) => items,
                Err(_) => {
                    let npc_id = CharacterId::from_uuid(char_uuid);
                    state
                        .app
                        .repositories
                        .character
                        .get_inventory(npc_id)
                        .await
                        .map_err(|e| ServerMessage::Response {
                            request_id: request_id.to_string(),
                            result: ResponseResult::error(
                                ErrorCode::InternalError,
                                sanitize_repo_error(&e, "retrieve character inventory"),
                            ),
                        })?
                }
            };

            let data: Vec<serde_json::Value> = items
                .into_iter()
                .map(|item| {
                    serde_json::json!({
                        "id": item.id,
                        "name": item.name,
                        "description": item.description,
                        "item_type": item.item_type,
                        "is_unique": item.is_unique,
                        "properties": item.properties,
                    })
                })
                .collect();
            Ok(ResponseResult::success(data))
        }
    }
}
