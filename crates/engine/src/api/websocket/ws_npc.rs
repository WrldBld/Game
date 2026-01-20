use super::*;

use crate::api::connections::ConnectionInfo;

use wrldbldr_shared::NpcRequest;

pub(super) async fn handle_npc_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: NpcRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        NpcRequest::SetNpcDisposition {
            npc_id,
            pc_id,
            disposition,
            reason,
        } => {
            require_dm_for_request(conn_info, request_id)?;

            let npc_id_typed = match parse_character_id_for_request(&npc_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let pc_uuid = match parse_uuid_for_request(&pc_id, request_id, "Invalid PC ID") {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let pc_id_typed = PlayerCharacterId::from_uuid(pc_uuid);

            // disposition is already typed as DispositionLevel from the wire format
            if disposition == wrldbldr_domain::DispositionLevel::Unknown {
                return Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    "Invalid disposition value",
                ));
            }

            let update = match state
                .app
                .use_cases
                .npc
                .disposition
                .set_disposition(npc_id_typed, pc_id_typed, disposition, reason.clone())
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        e.to_string(),
                    ))
                }
            };

            if let Some(world_id) = conn_info.world_id {
                let msg = ServerMessage::NpcDispositionChanged {
                    npc_id: update.npc_id.to_string(),
                    npc_name: update.npc_name,
                    pc_id: update.pc_id.to_string(),
                    disposition: update.disposition.to_string(),
                    relationship: update.relationship.to_string(),
                    reason: update.reason,
                };
                state.connections.broadcast_to_dms(world_id, msg).await;
            }

            Ok(ResponseResult::success_empty())
        }

        NpcRequest::SetNpcRelationship {
            npc_id,
            pc_id,
            relationship,
        } => {
            require_dm_for_request(conn_info, request_id)?;

            let npc_id_typed = match parse_character_id_for_request(&npc_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let pc_uuid = match parse_uuid_for_request(&pc_id, request_id, "Invalid PC ID") {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let pc_id_typed = PlayerCharacterId::from_uuid(pc_uuid);

            let relationship_level: wrldbldr_domain::RelationshipLevel =
                relationship.parse().map_err(|e| ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(
                        ErrorCode::BadRequest,
                        format!("Invalid relationship value: {}", e),
                    ),
                })?;

            if relationship_level == wrldbldr_domain::RelationshipLevel::Unknown {
                return Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    "Invalid relationship value",
                ));
            }

            let update = match state
                .app
                .use_cases
                .npc
                .disposition
                .set_relationship(npc_id_typed, pc_id_typed, relationship_level)
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        e.to_string(),
                    ))
                }
            };

            if let Some(world_id) = conn_info.world_id {
                let msg = ServerMessage::NpcDispositionChanged {
                    npc_id: update.npc_id.to_string(),
                    npc_name: update.npc_name,
                    pc_id: update.pc_id.to_string(),
                    disposition: update.disposition.to_string(),
                    relationship: update.relationship.to_string(),
                    reason: update.reason,
                };
                state.connections.broadcast_to_dms(world_id, msg).await;
            }

            Ok(ResponseResult::success_empty())
        }

        NpcRequest::GetNpcDispositions { pc_id } => {
            let pc_uuid = match parse_uuid_for_request(&pc_id, request_id, "Invalid PC ID") {
                Ok(id) => id,
                Err(e) => return Err(e),
            };
            let pc_id_typed = PlayerCharacterId::from_uuid(pc_uuid);

            let domain_data = match state
                .app
                .use_cases
                .npc
                .disposition
                .list_for_pc(pc_id_typed)
                .await
            {
                Ok(list) => list,
                Err(e) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::InternalError,
                        e.to_string(),
                    ))
                }
            };

            // Convert domain types to protocol types
            let dispositions = domain_data
                .into_iter()
                .map(|d| wrldbldr_shared::NpcDispositionData {
                    npc_id: d.npc_id,
                    npc_name: d.npc_name,
                    disposition: d.disposition,
                    relationship: d.relationship,
                    sentiment: d.sentiment,
                    last_reason: d.last_reason,
                })
                .collect();

            let msg = ServerMessage::NpcDispositionsResponse {
                pc_id: pc_id_typed.to_string(),
                dispositions,
            };

            match state
                .connections
                .send_critical(conn_info.connection_id, msg)
                .await
            {
                Ok(()) => Ok(ResponseResult::success_empty()),
                Err(_) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    "Failed to send dispositions response",
                )),
            }
        }

        NpcRequest::ListCharacterRegionRelationships { character_id } => {
            let char_id_typed = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .npc
                .region_relationships
                .list_for_character(char_id_typed)
                .await
            {
                Ok(relationships) => {
                    let data: Vec<serde_json::Value> = relationships
                        .into_iter()
                        .map(|r| {
                            serde_json::json!({
                                "region_id": r.region_id.to_string(),
                                "relationship_type": format!("{}", r.relationship_type),
                                "shift": r.shift,
                                "frequency": r.frequency,
                                "time_of_day": r.time_of_day,
                                "reason": r.reason,
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(crate::use_cases::npc::NpcError::NotFound) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::SetCharacterHomeRegion {
            character_id,
            region_id,
        } => {
            require_dm_for_request(conn_info, request_id)?;

            let char_uuid = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let region_uuid = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .npc
                .region_relationships
                .set_home_region(char_uuid, region_uuid)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(crate::use_cases::npc::NpcError::NotFound) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::SetCharacterWorkRegion {
            character_id,
            region_id,
        } => {
            require_dm_for_request(conn_info, request_id)?;

            let char_uuid = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let region_uuid = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .npc
                .region_relationships
                .set_work_region(char_uuid, region_uuid)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(crate::use_cases::npc::NpcError::NotFound) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::RemoveCharacterRegionRelationship {
            character_id,
            region_id,
            relationship_type,
        } => {
            require_dm_for_request(conn_info, request_id)?;

            let char_uuid = match parse_character_id_for_request(&character_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            let region_uuid = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            // Parse string to enum at API boundary
            let relationship_type_enum: crate::infrastructure::ports::NpcRegionRelationType =
                match relationship_type.parse() {
                    Ok(rt) => rt,
                    Err(_) => {
                        return Ok(ResponseResult::error(
                            ErrorCode::BadRequest,
                            format!(
                                "Invalid relationship type: '{}'. Valid types: HOME_REGION, WORKS_AT_REGION, FREQUENTS_REGION, AVOIDS_REGION",
                                relationship_type
                            ),
                        ))
                    }
                };

            match state
                .app
                .use_cases
                .npc
                .region_relationships
                .remove_relationship(char_uuid, region_uuid, relationship_type_enum)
                .await
            {
                Ok(()) => Ok(ResponseResult::success(
                    serde_json::json!({"success": true}),
                )),
                Err(crate::use_cases::npc::NpcError::NotFound) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::ListRegionNpcs { region_id } => {
            let region_id_typed = match parse_region_id_for_request(&region_id, request_id) {
                Ok(id) => id,
                Err(e) => return Err(e),
            };

            match state
                .app
                .use_cases
                .npc
                .region_relationships
                .list_region_npcs(region_id_typed)
                .await
            {
                Ok(npcs) => {
                    let data: Vec<serde_json::Value> = npcs
                        .into_iter()
                        .map(|n| {
                            serde_json::json!({
                                "character_id": n.character_id.to_string(),
                                "name": n.name,
                                "sprite_asset": n.sprite_asset,
                                "portrait_asset": n.portrait_asset,
                                "relationship_type": format!("{}", n.relationship_type),
                                "shift": n.shift,
                                "frequency": n.frequency,
                                "time_of_day": n.time_of_day,
                                "reason": n.reason,
                            })
                        })
                        .collect();
                    Ok(ResponseResult::success(data))
                }
                Err(crate::use_cases::npc::NpcError::NotFound) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Region not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::SetNpcMood {
            npc_id,
            region_id,
            mood,
            reason,
        } => {
            let npc_uuid = match Uuid::parse_str(&npc_id) {
                Ok(u) => CharacterId::from(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid NPC ID"),
                    })
                }
            };

            let region_uuid = match Uuid::parse_str(&region_id) {
                Ok(u) => RegionId::from(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid region ID"),
                    })
                }
            };

            // mood is already typed as MoodState from the wire format

            if !conn_info.is_dm() {
                return Err(ServerMessage::Response {
                    request_id: request_id.to_string(),
                    result: ResponseResult::error(ErrorCode::Forbidden, "Only DM can set NPC mood"),
                });
            }

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
                .npc
                .mood
                .set_mood(region_uuid, npc_uuid, mood)
                .await
            {
                Ok(change) => {
                    let mood_changed_msg = ServerMessage::NpcMoodChanged {
                        npc_id: change.npc_id.to_string(),
                        npc_name: change.npc_name.clone(),
                        old_mood: change.old_mood.to_string(),
                        new_mood: change.new_mood.to_string(),
                        reason: reason.map(|s| s.to_string()),
                        region_id: Some(region_id.to_string()),
                    };

                    state
                        .connections
                        .broadcast_to_world(world_id, mood_changed_msg)
                        .await;

                    Ok(ResponseResult::success(serde_json::json!({
                        "npc_id": npc_id,
                        "region_id": region_id,
                        "mood": mood.to_string(),
                    })))
                }
                Err(crate::use_cases::npc::NpcError::NotFound) => {
                    Ok(ResponseResult::error(ErrorCode::NotFound, "NPC not found"))
                }
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        NpcRequest::GetNpcMood { npc_id, region_id } => {
            let npc_uuid = match Uuid::parse_str(&npc_id) {
                Ok(u) => CharacterId::from(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid NPC ID"),
                    })
                }
            };

            let region_uuid = match Uuid::parse_str(&region_id) {
                Ok(u) => RegionId::from(u),
                Err(_) => {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid region ID"),
                    })
                }
            };

            match state
                .app
                .use_cases
                .npc
                .mood
                .get_mood(region_uuid, npc_uuid)
                .await
            {
                Ok(mood) => Ok(ResponseResult::success(serde_json::json!({
                    "npc_id": npc_id,
                    "region_id": region_id,
                    "mood": mood.to_string(),
                    "default_expression": mood.default_expression(),
                }))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }
    }
}
