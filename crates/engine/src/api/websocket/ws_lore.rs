use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::use_cases::lore::{
    CreateLoreChunkInput, CreateLoreInput, LoreDetail, LoreDiscoverySourceInput,
    UpdateLoreChunkInput, UpdateLoreInput,
};

use wrldbldr_shared::{AboutTargetData, LoreDiscoverySourceData, LoreRequest};

/// Maximum string lengths for lore fields to prevent unbounded inputs.
const MAX_LORE_TITLE: usize = 200;
const MAX_LORE_SUMMARY: usize = 1000;
const MAX_LORE_CHUNK_CONTENT: usize = 10000;
const MAX_LORE_CHUNKS: usize = 100;
const MAX_LORE_TAGS: usize = 50;
const MAX_TAG_LENGTH: usize = 50;

/// Convert LoreDetail to LoreData.
fn convert_lore_detail_to_data(detail: LoreDetail) -> wrldbldr_shared::LoreData {
    let category = match detail.category.as_str() {
        "Historical" => wrldbldr_shared::LoreCategoryData::Historical,
        "Legend" => wrldbldr_shared::LoreCategoryData::Legend,
        "Secret" => wrldbldr_shared::LoreCategoryData::Secret,
        "Common" => wrldbldr_shared::LoreCategoryData::Common,
        "Technical" => wrldbldr_shared::LoreCategoryData::Technical,
        "Political" => wrldbldr_shared::LoreCategoryData::Political,
        "Natural" => wrldbldr_shared::LoreCategoryData::Natural,
        "Religious" => wrldbldr_shared::LoreCategoryData::Religious,
        _ => wrldbldr_shared::LoreCategoryData::Unknown,
    };

    let chunks = detail.chunks.into_iter().map(|c| wrldbldr_shared::LoreChunkData {
        id: c.id,
        order: c.order,
        title: c.title,
        content: c.content,
        discovery_hint: c.discovery_hint,
    }).collect();

    wrldbldr_shared::LoreData {
        id: detail.id,
        world_id: detail.world_id,
        title: detail.title,
        summary: detail.summary,
        category,
        chunks,
        is_common_knowledge: detail.is_common_knowledge,
        tags: detail.tags,
    }
}

/// Convert protocol discovery source to domain input type.
fn proto_discovery_source_to_domain(source: LoreDiscoverySourceData) -> Result<LoreDiscoverySourceInput, String> {
    match source {
        LoreDiscoverySourceData::ReadBook { book_name } => {
            Ok(LoreDiscoverySourceInput::ReadBook { book_name })
        }
        LoreDiscoverySourceData::Conversation { npc_id, npc_name } => {
            let npc_uuid = uuid::Uuid::parse_str(&npc_id)
                .map_err(|e| format!("Invalid NPC ID: {}", e))?;
            Ok(LoreDiscoverySourceInput::Conversation {
                npc_id: wrldbldr_domain::CharacterId::from_uuid(npc_uuid),
                npc_name,
            })
        }
        LoreDiscoverySourceData::Investigation => Ok(LoreDiscoverySourceInput::Investigation),
        LoreDiscoverySourceData::DmGranted { reason } => {
            Ok(LoreDiscoverySourceInput::DmGranted { reason })
        }
        LoreDiscoverySourceData::CommonKnowledge => Ok(LoreDiscoverySourceInput::CommonKnowledge),
        LoreDiscoverySourceData::LlmDiscovered { context } => {
            Ok(LoreDiscoverySourceInput::LlmDiscovered { context })
        }
        LoreDiscoverySourceData::Unknown => Ok(LoreDiscoverySourceInput::Unknown),
    }
}

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
                    sanitize_repo_error(&e, "list lore"),
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
                    sanitize_repo_error(&e, "get lore"),
                )),
            }
        }

        LoreRequest::CreateLore { world_id, data } => {
            require_dm_for_request(conn_info, request_id)?;

            let world_uuid = match parse_world_id_for_request(&world_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            // Validate title length
            if data.title.len() > MAX_LORE_TITLE {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::BadRequest,
                        format!("Title too long (max {} chars)", MAX_LORE_TITLE),
                    ),
                });
            }

            // Validate summary length
            if let Some(summary) = &data.summary {
                if summary.len() > MAX_LORE_SUMMARY {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            format!("Summary too long (max {} chars)", MAX_LORE_SUMMARY),
                        ),
                    });
                }
            }

            // Validate chunk count
            if data.chunks.as_ref().map(|c| c.len()).unwrap_or(0) > MAX_LORE_CHUNKS {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::BadRequest,
                        format!("Too many chunks (max {})", MAX_LORE_CHUNKS),
                    ),
                });
            }

            // Validate each chunk
            for chunk in data.chunks.iter().flatten() {
                if chunk.content.len() > MAX_LORE_CHUNK_CONTENT {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            format!("Chunk content too long (max {} chars)", MAX_LORE_CHUNK_CONTENT),
                        ),
                    });
                }
            }

            // Validate tags count
            if let Some(tags) = &data.tags {
                if tags.len() > MAX_LORE_TAGS {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            format!("Too many tags (max {})", MAX_LORE_TAGS),
                        ),
                    });
                }

                // Validate each tag length
                for tag in tags {
                    if tag.len() > MAX_TAG_LENGTH {
                        return Err(ServerMessage::Response {
                            request_id: request_id.to_string(),
                            result: ResponseResult::error(
                                ErrorCode::BadRequest,
                                format!("Tag too long (max {} chars)", MAX_TAG_LENGTH),
                            ),
                        });
                    }
                }
            }

            // Convert protocol data to domain input
            let input = CreateLoreInput {
                title: data.title,
                summary: data.summary,
                category: data.category,
                tags: data.tags,
                is_common_knowledge: data.is_common_knowledge,
                chunks: data.chunks.map(|chunks| {
                    chunks
                        .into_iter()
                        .map(|c| CreateLoreChunkInput {
                            title: c.title,
                            content: c.content,
                            order: c.order,
                            discovery_hint: c.discovery_hint,
                        })
                        .collect()
                }),
            };
            match state.app.use_cases.lore.ops.create(world_uuid, input).await {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(crate::use_cases::lore::LoreError::InvalidCategory(msg)) => {
                    Ok(ResponseResult::error(ErrorCode::BadRequest, &msg))
                }
                Err(crate::use_cases::lore::LoreError::DuplicateChunkOrder(order)) => {
                    Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        format!("Duplicate chunk order: {}", order),
                    ))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "create lore"),
                )),
            }
        }

        LoreRequest::UpdateLore { lore_id, data } => {
            require_dm_for_request(conn_info, request_id)?;

            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            // Validate title length
            if let Some(title) = &data.title {
                if title.len() > MAX_LORE_TITLE {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            format!("Title too long (max {} chars)", MAX_LORE_TITLE),
                        ),
                    });
                }
            }

            // Validate summary length
            if let Some(summary) = &data.summary {
                if summary.len() > MAX_LORE_SUMMARY {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            format!("Summary too long (max {} chars)", MAX_LORE_SUMMARY),
                        ),
                    });
                }
            }

            // Validate tags count
            if let Some(tags) = &data.tags {
                if tags.len() > MAX_LORE_TAGS {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            format!("Too many tags (max {})", MAX_LORE_TAGS),
                        ),
                    });
                }

                // Validate each tag length
                for tag in tags {
                    if tag.len() > MAX_TAG_LENGTH {
                        return Err(ServerMessage::Response {
                            request_id: request_id.to_string(),
                            result: ResponseResult::error(
                                ErrorCode::BadRequest,
                                format!("Tag too long (max {} chars)", MAX_TAG_LENGTH),
                            ),
                        });
                    }
                }
            }

            // Convert protocol data to domain input
            let input = UpdateLoreInput {
                title: data.title,
                summary: data.summary,
                category: data.category,
                tags: data.tags,
                is_common_knowledge: data.is_common_knowledge,
            };
            match state.app.use_cases.lore.ops.update(lore_uuid, input).await {
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
                    result: ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "update lore"),
                    ),
                }),
            }
        }

        LoreRequest::DeleteLore { lore_id } => {
            require_dm_for_request(conn_info, request_id)?;

            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state.app.use_cases.lore.ops.delete(lore_uuid).await {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "delete lore"),
                )),
            }
        }

        LoreRequest::AddLoreChunk { lore_id, data } => {
            require_dm_for_request(conn_info, request_id)?;

            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            // Validate chunk content length
            if data.content.len() > MAX_LORE_CHUNK_CONTENT {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::BadRequest,
                        format!("Chunk content too long (max {} chars)", MAX_LORE_CHUNK_CONTENT),
                    ),
                });
            }

            // Convert protocol data to domain input
            let input = CreateLoreChunkInput {
                title: data.title,
                content: data.content,
                order: data.order,
                discovery_hint: data.discovery_hint,
            };
            match state
                .app
                .use_cases
                .lore
                .ops
                .add_chunk(lore_uuid, input)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(crate::use_cases::lore::LoreError::NotFound) => Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::NotFound, "Lore not found"),
                }),
                Err(crate::use_cases::lore::LoreError::DuplicateChunkOrder(order)) => {
                    Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            format!("Duplicate chunk order: {}", order),
                        ),
                    })
                }
                Err(e) => Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "add lore chunk"),
                    ),
                }),
            }
        }

        LoreRequest::UpdateLoreChunk { chunk_id, data } => {
            require_dm_for_request(conn_info, request_id)?;

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

            // Convert protocol data to domain input
            let input = UpdateLoreChunkInput {
                title: data.title,
                content: data.content,
                order: data.order,
                discovery_hint: data.discovery_hint,
            };
            match state
                .app
                .use_cases
                .lore
                .ops
                .update_chunk(world_id, chunk_uuid, input)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(crate::use_cases::lore::LoreError::ChunkNotFound) => {
                    Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "Lore chunk not found"),
                    })
                }
                Err(crate::use_cases::lore::LoreError::DuplicateChunkOrder(order)) => {
                    Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::BadRequest,
                            format!("Duplicate chunk order: {}", order),
                        ),
                    })
                }
                Err(e) => Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "update lore chunk"),
                    ),
                }),
            }
        }

        LoreRequest::DeleteLoreChunk { chunk_id } => {
            require_dm_for_request(conn_info, request_id)?;

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
                    result: ResponseResult::error(
                        ErrorCode::InternalError,
                        sanitize_repo_error(&e, "delete lore chunk"),
                    ),
                }),
            }
        }

        LoreRequest::GrantLoreKnowledge {
            character_id,
            lore_id,
            chunk_ids,
            discovery_source,
        } => {
            require_dm_for_request(conn_info, request_id)?;

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

            // Convert protocol discovery source to domain input
            let source_input = match proto_discovery_source_to_domain(discovery_source.clone()) {
                Ok(s) => s,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        &format!("Invalid discovery source: {}", e),
                    ))
                }
            };
            // Clone values for later use in response
            let chunk_uuids_clone = chunk_uuids.clone();

            match state
                .app
                .use_cases
                .lore
                .ops
                .grant_knowledge(char_uuid, lore_uuid, chunk_uuids, source_input)
                .await
            {
                Ok(result) => {
                    // Emit LoreDiscovered event to the owning player
                    if let Ok(lore_data) = state.app.use_cases.lore.ops.get(lore_uuid).await {
                        if let Some(lore_detail) = lore_data {
                            let discovered_chunk_ids: Vec<String> = chunk_uuids_clone
                                .as_ref()
                                .unwrap_or(&vec![])
                                .iter()
                                .map(|id| id.to_string())
                                .collect();

                            let lore_discovered_msg = ServerMessage::LoreDiscovered {
                                character_id,
                                lore: convert_lore_detail_to_data(lore_detail),
                                discovered_chunk_ids,
                                discovery_source,
                            };
                            // Note: We need PlayerCharacterId, but char_uuid is CharacterId
                            // The system tracks player characters separately from NPCs
                            // We'll need to look up the PlayerCharacterId from CharacterId
                            // For now, broadcast to world so the player gets it
                            state.connections.broadcast_to_world(world_id, lore_discovered_msg).await;
                        }
                    }
                    Ok(ResponseResult::success(result))
                }
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
                    sanitize_repo_error(&e, "grant lore knowledge"),
                )),
            }
        }

        LoreRequest::RevokeLoreKnowledge {
            character_id,
            lore_id,
            chunk_ids,
        } => {
            require_dm_for_request(conn_info, request_id)?;

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

            let char_uuid = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            // Parse chunk_ids if provided (for partial revocation)
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
                .revoke_knowledge(char_uuid, lore_uuid, chunk_uuids)
                .await
            {
                Ok(result) => {
                    // Emit LoreRevoked event to the owning player
                    let lore_revoked_msg = ServerMessage::LoreRevoked {
                        character_id,
                        lore_id,
                    };
                    // Broadcast to world so the player gets it
                    state.connections.broadcast_to_world(world_id, lore_revoked_msg).await;
                    Ok(ResponseResult::success(result))
                }
                Err(crate::use_cases::lore::LoreError::NotFound) => {
                    Ok(ResponseResult::error(ErrorCode::NotFound, "Lore not found"))
                }
                Err(crate::use_cases::lore::LoreError::InvalidChunkIds(msg)) => {
                    Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        format!("Invalid chunk IDs: {}", msg),
                    ))
                }
                Err(crate::use_cases::lore::LoreError::EmptyChunkList) => {
                    Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Empty chunk list provided - omit chunkIds for full revocation",
                    ))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "revoke lore knowledge"),
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
                .get_character_lore_journal(char_uuid)
                .await
            {
                Ok(data) => Ok(ResponseResult::success(data)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get character lore"),
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
                    sanitize_repo_error(&e, "get lore knowers"),
                )),
            }
        }

        // ========================================================================
        // P1: Lore linking + common lore backend
        // ========================================================================

        LoreRequest::LinkLoreAbout { lore_id, target } => {
            require_dm_for_request(conn_info, request_id)?;

            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            let domain_target = match proto_about_target_to_domain(&target) {
                Ok(t) => t,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        &format!("Invalid about target: {}", e),
                    ))
                }
            };

            let input = crate::use_cases::lore::LinkLoreAboutInput {
                target: domain_target,
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .link_lore_about(lore_uuid, input)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "link lore about"),
                )),
            }
        }

        LoreRequest::UnlinkLoreAbout { lore_id, target } => {
            require_dm_for_request(conn_info, request_id)?;

            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            let domain_target = match proto_about_target_to_domain(&target) {
                Ok(t) => t,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        &format!("Invalid about target: {}", e),
                    ))
                }
            };

            let input = crate::use_cases::lore::LinkLoreAboutInput {
                target: domain_target,
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .unlink_lore_about(lore_uuid, input)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "unlink lore about"),
                )),
            }
        }

        LoreRequest::GetLoreAboutTargets { lore_id } => {
            let lore_uuid = match parse_uuid_for_request(&lore_id, request_id, "Invalid lore_id") {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .get_lore_about_targets(lore_uuid)
                .await
            {
                Ok(targets) => {
                    let data: Vec<_> = targets.into_iter().map(|t| match t {
                        wrldbldr_domain::value_objects::AboutTarget::Character { character_id } => {
                            wrldbldr_shared::AboutTargetData::Character {
                                character_id: character_id.to_string(),
                            }
                        }
                        wrldbldr_domain::value_objects::AboutTarget::Location { location_id } => {
                            wrldbldr_shared::AboutTargetData::Location {
                                location_id: location_id.to_string(),
                            }
                        }
                        wrldbldr_domain::value_objects::AboutTarget::Region { region_id } => {
                            wrldbldr_shared::AboutTargetData::Region {
                                region_id: region_id.to_string(),
                            }
                        }
                        wrldbldr_domain::value_objects::AboutTarget::Item { item_id } => {
                            wrldbldr_shared::AboutTargetData::Item {
                                item_id: item_id.to_string(),
                            }
                        }
                    }).collect();

                    Ok(ResponseResult::success(data))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get lore about targets"),
                )),
            }
        }

        LoreRequest::MarkCommonLore { location_id, lore_id, chunk_ids } => {
            require_dm_for_request(conn_info, request_id)?;

            let location_uuid = match parse_uuid_for_request(&location_id, request_id, "Invalid location_id") {
                Ok(u) => wrldbldr_domain::LocationId::from_uuid(u),
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

            let input = crate::use_cases::lore::MarkCommonLoreInput {
                chunk_ids: chunk_uuids,
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .mark_common_lore(location_uuid, lore_uuid, input)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "mark common lore"),
                )),
            }
        }

        LoreRequest::UnmarkCommonLore { location_id, lore_id } => {
            require_dm_for_request(conn_info, request_id)?;

            let location_uuid = match parse_uuid_for_request(&location_id, request_id, "Invalid location_id") {
                Ok(u) => wrldbldr_domain::LocationId::from_uuid(u),
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
                .unmark_common_lore(location_uuid, lore_uuid)
                .await
            {
                Ok(result) => Ok(ResponseResult::success(result)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "unmark common lore"),
                )),
            }
        }

        LoreRequest::GetLocationCommonLore { location_id } => {
            let location_uuid = match parse_uuid_for_request(&location_id, request_id, "Invalid location_id") {
                Ok(u) => wrldbldr_domain::LocationId::from_uuid(u),
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .lore
                .ops
                .get_location_common_lore(location_uuid)
                .await
            {
                Ok(data) => Ok(ResponseResult::success(data)),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    sanitize_repo_error(&e, "get location common lore"),
                )),
            }
        }
    }
}

/// Convert protocol AboutTargetData to domain AboutTarget
fn proto_about_target_to_domain(target: &AboutTargetData) -> Result<wrldbldr_domain::value_objects::AboutTarget, String> {
    match target {
        AboutTargetData::Character { character_id } => {
            let uuid = Uuid::parse_str(character_id)
                .map_err(|e| format!("Invalid character_id: {}", e))?;
            Ok(wrldbldr_domain::value_objects::AboutTarget::Character {
                character_id: wrldbldr_domain::CharacterId::from_uuid(uuid),
            })
        }
        AboutTargetData::Location { location_id } => {
            let uuid = Uuid::parse_str(location_id)
                .map_err(|e| format!("Invalid location_id: {}", e))?;
            Ok(wrldbldr_domain::value_objects::AboutTarget::Location {
                location_id: wrldbldr_domain::LocationId::from_uuid(uuid),
            })
        }
        AboutTargetData::Region { region_id } => {
            let uuid = Uuid::parse_str(region_id)
                .map_err(|e| format!("Invalid region_id: {}", e))?;
            Ok(wrldbldr_domain::value_objects::AboutTarget::Region {
                region_id: wrldbldr_domain::RegionId::from_uuid(uuid),
            })
        }
        AboutTargetData::Item { item_id } => {
            let uuid = Uuid::parse_str(item_id)
                .map_err(|e| format!("Invalid item_id: {}", e))?;
            Ok(wrldbldr_domain::value_objects::AboutTarget::Item {
                item_id: wrldbldr_domain::ItemId::from_uuid(uuid),
            })
        }
    }
}
