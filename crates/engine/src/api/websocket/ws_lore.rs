use super::*;

use crate::api::connections::ConnectionInfo;
use wrldbldr_protocol::RequestPayload;

pub(super) async fn handle_lore_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    payload: &RequestPayload,
) -> Result<Option<ResponseResult>, ServerMessage> {
    match payload {
        RequestPayload::ListLore { world_id: req_world_id } => {
            let world_uuid = match Uuid::parse_str(req_world_id) {
                Ok(u) => wrldbldr_domain::WorldId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world_id"),
                    })
                }
            };

            let lore_list = state
                .app
                .entities
                .lore
                .list_for_world(world_uuid)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            let data: Vec<serde_json::Value> = lore_list
                .into_iter()
                .map(|l| {
                    serde_json::json!({
                        "id": l.id.to_string(),
                        "worldId": l.world_id.to_string(),
                        "title": l.title,
                        "summary": l.summary,
                        "category": format!("{}", l.category),
                        "isCommonKnowledge": l.is_common_knowledge,
                        "tags": l.tags,
                        "chunkCount": l.chunks.len(),
                        "createdAt": l.created_at.to_rfc3339(),
                        "updatedAt": l.updated_at.to_rfc3339(),
                    })
                })
                .collect();

            Ok(Some(ResponseResult::success(data)))
        }

        RequestPayload::GetLore { lore_id } => {
            let lore_uuid = match Uuid::parse_str(lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id"),
                    })
                }
            };

            match state.app.entities.lore.get(lore_uuid).await {
                Ok(Some(lore)) => {
                    let chunks: Vec<serde_json::Value> = lore
                        .chunks
                        .iter()
                        .map(|c| {
                            serde_json::json!({
                                "id": c.id.to_string(),
                                "order": c.order,
                                "title": c.title,
                                "content": c.content,
                                "discoveryHint": c.discovery_hint,
                            })
                        })
                        .collect();

                    Ok(Some(ResponseResult::success(serde_json::json!({
                        "id": lore.id.to_string(),
                        "worldId": lore.world_id.to_string(),
                        "title": lore.title,
                        "summary": lore.summary,
                        "category": format!("{}", lore.category),
                        "isCommonKnowledge": lore.is_common_knowledge,
                        "tags": lore.tags,
                        "chunks": chunks,
                        "createdAt": lore.created_at.to_rfc3339(),
                        "updatedAt": lore.updated_at.to_rfc3339(),
                    }))))
                }
                Ok(None) => Ok(Some(ResponseResult::error(ErrorCode::NotFound, "Lore not found"))),
                Err(e) => Ok(Some(ResponseResult::error(ErrorCode::InternalError, &e.to_string()))),
            }
        }

        RequestPayload::CreateLore { world_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let world_uuid = match Uuid::parse_str(world_id) {
                Ok(u) => wrldbldr_domain::WorldId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world_id"),
                    })
                }
            };

            let category = data
                .category
                .as_deref()
                .unwrap_or("common")
                .parse::<wrldbldr_domain::LoreCategory>()
                .unwrap_or(wrldbldr_domain::LoreCategory::Common);

            let now = chrono::Utc::now();
            let mut lore = wrldbldr_domain::Lore::new(world_uuid, &data.title, category, now);

            if let Some(summary) = &data.summary {
                lore = lore.with_summary(summary);
            }
            if let Some(tags) = &data.tags {
                lore = lore.with_tags(tags.clone());
            }
            if data.is_common_knowledge.unwrap_or(false) {
                lore = lore.as_common_knowledge();
            }

            if let Some(chunks) = &data.chunks {
                let mut domain_chunks = Vec::new();
                for (i, chunk_data) in chunks.iter().enumerate() {
                    let mut chunk = wrldbldr_domain::LoreChunk::new(&chunk_data.content)
                        .with_order(chunk_data.order.unwrap_or(i as u32));
                    if let Some(title) = &chunk_data.title {
                        chunk = chunk.with_title(title);
                    }
                    if let Some(hint) = &chunk_data.discovery_hint {
                        chunk = chunk.with_discovery_hint(hint);
                    }
                    domain_chunks.push(chunk);
                }
                lore = lore.with_chunks(domain_chunks);
            }

            state
                .app
                .entities
                .lore
                .save(&lore)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            Ok(Some(ResponseResult::success(serde_json::json!({
                "id": lore.id.to_string(),
                "title": lore.title,
            }))))
        }

        RequestPayload::UpdateLore { lore_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let lore_uuid = match Uuid::parse_str(lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id"),
                    })
                }
            };

            let mut lore = match state.app.entities.lore.get(lore_uuid).await {
                Ok(Some(l)) => l,
                Ok(None) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "Lore not found"),
                    })
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                    })
                }
            };

            if let Some(title) = &data.title {
                lore.title = title.clone();
            }
            if let Some(summary) = &data.summary {
                lore.summary = summary.clone();
            }
            if let Some(category_str) = &data.category {
                if let Ok(cat) = category_str.parse::<wrldbldr_domain::LoreCategory>() {
                    lore.category = cat;
                }
            }
            if let Some(tags) = &data.tags {
                lore.tags = tags.clone();
            }
            if let Some(is_common) = data.is_common_knowledge {
                lore.is_common_knowledge = is_common;
            }
            lore.updated_at = chrono::Utc::now();

            state
                .app
                .entities
                .lore
                .save(&lore)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            Ok(Some(ResponseResult::success(serde_json::json!({
                "id": lore.id.to_string(),
                "title": lore.title,
            }))))
        }

        RequestPayload::DeleteLore { lore_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let lore_uuid = match Uuid::parse_str(lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id"),
                    })
                }
            };

            state
                .app
                .entities
                .lore
                .delete(lore_uuid)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            Ok(Some(ResponseResult::success(serde_json::json!({ "deleted": true }))))
        }

        RequestPayload::AddLoreChunk { lore_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let lore_uuid = match Uuid::parse_str(lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id"),
                    })
                }
            };

            let mut lore = match state.app.entities.lore.get(lore_uuid).await {
                Ok(Some(l)) => l,
                Ok(None) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::NotFound, "Lore not found"),
                    })
                }
                Err(e) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                    })
                }
            };

            let mut chunk = wrldbldr_domain::LoreChunk::new(&data.content)
                .with_order(data.order.unwrap_or(lore.chunks.len() as u32));
            if let Some(title) = &data.title {
                chunk = chunk.with_title(title);
            }
            if let Some(hint) = &data.discovery_hint {
                chunk = chunk.with_discovery_hint(hint);
            }

            let chunk_id = chunk.id.to_string();
            lore.chunks.push(chunk);
            lore.updated_at = chrono::Utc::now();

            state
                .app
                .entities
                .lore
                .save(&lore)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            Ok(Some(ResponseResult::success(serde_json::json!({
                "chunkId": chunk_id,
            }))))
        }

        RequestPayload::UpdateLoreChunk { chunk_id, data } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let chunk_uuid = match Uuid::parse_str(chunk_id) {
                Ok(u) => wrldbldr_domain::LoreChunkId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid chunk_id"),
                    })
                }
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

            let mut lore = state
                .app
                .entities
                .lore
                .list_for_world(world_id)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?
                .into_iter()
                .find(|l| l.chunks.iter().any(|c| c.id == chunk_uuid))
                .ok_or_else(|| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::NotFound, "Lore chunk not found"),
                })?;

            let chunk = lore
                .chunks
                .iter_mut()
                .find(|c| c.id == chunk_uuid)
                .ok_or_else(|| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::NotFound, "Lore chunk not found"),
                })?;

            if let Some(title) = &data.title {
                chunk.title = Some(title.clone());
            }
            if let Some(content) = &data.content {
                chunk.content = content.clone();
            }
            if let Some(order) = data.order {
                chunk.order = order;
            }
            if let Some(hint) = &data.discovery_hint {
                chunk.discovery_hint = Some(hint.clone());
            }

            lore.updated_at = chrono::Utc::now();

            state
                .app
                .entities
                .lore
                .save(&lore)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            Ok(Some(ResponseResult::success(serde_json::json!({
                "loreId": lore.id.to_string(),
                "chunkId": chunk_id,
            }))))
        }

        RequestPayload::DeleteLoreChunk { chunk_id } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let chunk_uuid = match Uuid::parse_str(chunk_id) {
                Ok(u) => wrldbldr_domain::LoreChunkId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid chunk_id"),
                    })
                }
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

            let mut lore = state
                .app
                .entities
                .lore
                .list_for_world(world_id)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?
                .into_iter()
                .find(|l| l.chunks.iter().any(|c| c.id == chunk_uuid))
                .ok_or_else(|| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::NotFound, "Lore chunk not found"),
                })?;

            let before = lore.chunks.len();
            lore.chunks.retain(|c| c.id != chunk_uuid);
            if lore.chunks.len() == before {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::NotFound, "Lore chunk not found"),
                });
            }

            lore.updated_at = chrono::Utc::now();

            state
                .app
                .entities
                .lore
                .save(&lore)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            Ok(Some(ResponseResult::success(serde_json::json!({
                "deleted": true,
                "loreId": lore.id.to_string(),
                "chunkId": chunk_id,
            }))))
        }

        RequestPayload::GrantLoreKnowledge {
            character_id,
            lore_id,
            chunk_ids,
            discovery_source,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let char_uuid = match Uuid::parse_str(character_id) {
                Ok(u) => wrldbldr_domain::CharacterId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid character_id"),
                    })
                }
            };
            let lore_uuid = match Uuid::parse_str(lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id"),
                    })
                }
            };

            let domain_source = match discovery_source {
                wrldbldr_protocol::types::LoreDiscoverySourceData::ReadBook { book_name } => {
                    wrldbldr_domain::LoreDiscoverySource::ReadBook {
                        book_name: book_name.clone(),
                    }
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::Conversation {
                    npc_id,
                    npc_name,
                } => {
                    let npc_uuid = Uuid::parse_str(npc_id)
                        .map(wrldbldr_domain::CharacterId::from_uuid)
                        .unwrap_or_else(|_| wrldbldr_domain::CharacterId::new());
                    wrldbldr_domain::LoreDiscoverySource::Conversation {
                        npc_id: npc_uuid,
                        npc_name: npc_name.clone(),
                    }
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::Investigation => {
                    wrldbldr_domain::LoreDiscoverySource::Investigation
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::DmGranted { reason } => {
                    wrldbldr_domain::LoreDiscoverySource::DmGranted {
                        reason: reason.clone(),
                    }
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::CommonKnowledge => {
                    wrldbldr_domain::LoreDiscoverySource::CommonKnowledge
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::LlmDiscovered { context } => {
                    wrldbldr_domain::LoreDiscoverySource::LlmDiscovered {
                        context: context.clone(),
                    }
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::Unknown => {
                    wrldbldr_domain::LoreDiscoverySource::DmGranted {
                        reason: Some("Unknown source type".to_string()),
                    }
                }
            };

            let now = chrono::Utc::now();
            let knowledge = if let Some(ids) = chunk_ids {
                let chunk_uuids: Vec<wrldbldr_domain::LoreChunkId> = ids
                    .iter()
                    .filter_map(|id| {
                        Uuid::parse_str(id)
                            .ok()
                            .map(wrldbldr_domain::LoreChunkId::from_uuid)
                    })
                    .collect();
                wrldbldr_domain::LoreKnowledge::partial(
                    lore_uuid,
                    char_uuid,
                    chunk_uuids,
                    domain_source,
                    now,
                )
            } else {
                wrldbldr_domain::LoreKnowledge::full(lore_uuid, char_uuid, domain_source, now)
            };

            state
                .app
                .entities
                .lore
                .grant_knowledge(&knowledge)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            Ok(Some(ResponseResult::success(serde_json::json!({ "granted": true }))))
        }

        RequestPayload::RevokeLoreKnowledge {
            character_id,
            lore_id,
            chunk_ids: _,
        } => {
            if let Err(e) = require_dm_for_request(conn_info, request_id) {
                return Err(e);
            }

            let char_uuid = match Uuid::parse_str(character_id) {
                Ok(u) => wrldbldr_domain::CharacterId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid character_id"),
                    })
                }
            };
            let lore_uuid = match Uuid::parse_str(lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id"),
                    })
                }
            };

            state
                .app
                .entities
                .lore
                .revoke_knowledge(char_uuid, lore_uuid)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            Ok(Some(ResponseResult::success(serde_json::json!({ "revoked": true }))))
        }

        RequestPayload::GetCharacterLore { character_id } => {
            let char_uuid = match Uuid::parse_str(character_id) {
                Ok(u) => wrldbldr_domain::CharacterId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid character_id"),
                    })
                }
            };

            let knowledge_list = state
                .app
                .entities
                .lore
                .get_character_knowledge(char_uuid)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            let data: Vec<serde_json::Value> = knowledge_list
                .into_iter()
                .map(|k| {
                    serde_json::json!({
                        "loreId": k.lore_id.to_string(),
                        "characterId": k.character_id.to_string(),
                        "knownChunkIds": k
                            .known_chunk_ids
                            .iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>(),
                        "discoveredAt": k.discovered_at.to_rfc3339(),
                        "notes": k.notes,
                    })
                })
                .collect();

            Ok(Some(ResponseResult::success(data)))
        }

        RequestPayload::GetLoreKnowers { lore_id } => {
            let lore_uuid = match Uuid::parse_str(lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id"),
                    })
                }
            };

            let knowledge_list = state
                .app
                .entities
                .lore
                .get_knowledge_for_lore(lore_uuid)
                .await
                .map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
                })?;

            let data: Vec<serde_json::Value> = knowledge_list
                .into_iter()
                .map(|k| {
                    serde_json::json!({
                        "characterId": k.character_id.to_string(),
                        "knownChunkIds": k
                            .known_chunk_ids
                            .iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>(),
                        "discoveredAt": k.discovered_at.to_rfc3339(),
                    })
                })
                .collect();

            Ok(Some(ResponseResult::success(data)))
        }

        _ => Ok(None),
    }
}
