use super::*;

use crate::api::connections::ConnectionInfo;

use wrldbldr_protocol::LoreRequest;

pub(super) async fn handle_lore_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: LoreRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        LoreRequest::ListLore { world_id } => {
            let world_uuid = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state.app.use_cases.lore.ops.list(world_uuid).await {
                Ok(data) => Ok(ResponseResult::success(data)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        LoreRequest::GetLore { lore_id } => {
            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state.app.use_cases.lore.ops.get(lore_uuid).await {
                Ok(Some(lore)) => Ok(ResponseResult::success(lore)),
                Ok(None) => Ok(ResponseResult::error(ErrorCode::NotFound, "Lore not found")),
                Err(crate::use_cases::lore::LoreError::NotFound) => {
                    Ok(ResponseResult::error(ErrorCode::NotFound, "Lore not found"))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    &e.to_string(),
                )),
            }
        }

        LoreRequest::CreateLore { world_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_uuid = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state.app.use_cases.lore.ops.create(world_uuid, data).await {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(crate::use_cases::lore::LoreError::InvalidCategory(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    &e.to_string(),
                )),
            }
        }

        LoreRequest::UpdateLore { lore_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state.app.use_cases.lore.ops.update(lore_uuid, data).await {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(crate::use_cases::lore::LoreError::NotFound) => Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::NotFound, "Lore not found"),
                }),
                Err(crate::use_cases::lore::LoreError::InvalidCategory(msg)) => {
                    Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, &msg),
                    })
                }
                Err(e) => Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                }),
            }
        }

        LoreRequest::DeleteLore { lore_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state.app.use_cases.lore.ops.delete(lore_uuid).await {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    &e.to_string(),
                )),
            }
        }

        LoreRequest::AddLoreChunk { lore_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .add_chunk(lore_uuid, data)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(crate::use_cases::lore::LoreError::NotFound) => Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::NotFound, "Lore not found"),
                }),
                Err(e) => Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                }),
            }
        }

        LoreRequest::UpdateLoreChunk { chunk_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let chunk_uuid = match parse_uuid_for_request(&chunk_id, request_id, "Invalid chunk_id")
            {
                Ok(u) => wrldbldr_domain::LoreChunkId::from_uuid(u),
                Err(e) => return Err(e),
            };

            let world_id = match conn_info.world_id {
                Some(wid) => wid,
                None => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            "Not connected to a world",
                        ),
                    })
                }
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .update_chunk(world_id, chunk_uuid, data)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(crate::use_cases::lore::LoreError::ChunkNotFound) => {
                    Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "Lore chunk not found"),
                    })
                }
                Err(e) => Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                }),
            }
        }

        LoreRequest::DeleteLoreChunk { chunk_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let chunk_uuid = match parse_uuid_for_request(&chunk_id, request_id, "Invalid chunk_id")
            {
                Ok(u) => wrldbldr_domain::LoreChunkId::from_uuid(u),
                Err(e) => return Err(e),
            };

            let world_id = match conn_info.world_id {
                Some(wid) => wid,
                None => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            "Not connected to a world",
                        ),
                    })
                }
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .delete_chunk(world_id, chunk_uuid)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(crate::use_cases::lore::LoreError::ChunkNotFound) => {
                    Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "Lore chunk not found"),
                    })
                }
                Err(e) => Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                }),
            }
        }

        LoreRequest::GrantLoreKnowledge {
            character_id,
            lore_id,
            chunk_ids,
            discovery_source,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let char_uuid = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            let chunk_uuids = match chunk_ids {
                Some(ids) => {
                    let mut valid_uuids = Vec::with_capacity(ids.len());
                    let mut invalid_ids = Vec::new();

                    for id in ids {
                        match Uuid::parse_str(&id) {
                            Ok(uuid) => {
                                valid_uuids.push(wrldbldr_domain::LoreChunkId::from_uuid(uuid))
                            }
                            Err(_) => invalid_ids.push(id),
                        }
                    }

                    if !invalid_ids.is_empty() {
                        return Ok(ResponseResult::error(
                            ErrorCode::BadRequest,
                            format!("Invalid chunk_ids: {}", invalid_ids.join(", ")),
                        ));
                    }

                    Some(valid_uuids)
                }
                None => None,
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .grant_knowledge(char_uuid, lore_uuid, chunk_uuids, discovery_source)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(crate::use_cases::lore::LoreError::NotFound) => {
                    Ok(ResponseResult::error(ErrorCode::NotFound, "Lore not found"))
                }
                Err(crate::use_cases::lore::LoreError::InvalidChunkIds(msg)) => {
                    Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        format!("Invalid chunk IDs: {}", msg),
                    ))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    &e.to_string(),
                )),
            }
        }

        LoreRequest::RevokeLoreKnowledge {
            character_id,
            lore_id,
            chunk_ids,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            // Partial revocation is not supported - reject if chunk_ids is provided
            if chunk_ids.is_some() {
                return Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    "Partial revocation not supported. Omit chunk_ids to revoke all knowledge of this lore.",
                ));
            }

            let char_uuid = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .revoke_knowledge(char_uuid, lore_uuid)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    &e.to_string(),
                )),
            }
        }

        LoreRequest::GetCharacterLore { character_id } => {
            let char_uuid = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .get_character_lore(char_uuid)
                .await
            {
                Ok(data) => Ok(ResponseResult::success(data)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    &e.to_string(),
                )),
            }
        }

        LoreRequest::GetLoreKnowers { lore_id } => {
            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .get_lore_knowers(lore_uuid)
                .await
            {
                Ok(data) => Ok(ResponseResult::success(data)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    &e.to_string(),
                )),
            }
        }
    }
}
