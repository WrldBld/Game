//! Player Character, Observation, and Character-Region relationship request handlers
//!
//! Handles: Player Character CRUD, Observations, Character-Region relationships

use std::sync::Arc;

use wrldbldr_domain::entities::{CharacterSheetData, NpcObservation, ObservationType};
use wrldbldr_domain::value_objects::RegionShift;
use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_engine_ports::outbound::{
    CharacterLocationPort, ClockPort, ObservationRepositoryPort, RegionRepositoryPort,
};
use wrldbldr_protocol::{
    CreateObservationData, CreatePlayerCharacterData, ErrorCode, ResponseResult,
    UpdatePlayerCharacterData,
};

use super::common::{
    parse_character_id, parse_location_id, parse_player_character_id, parse_region_id,
    parse_world_id,
};
use crate::application::dto::PlayerCharacterResponseDto;
use crate::application::services::{
    CreatePlayerCharacterRequest, PlayerCharacterService, UpdatePlayerCharacterRequest,
};

// =============================================================================
// Player Character Handlers
// =============================================================================

/// Handle ListPlayerCharacters request
pub async fn list_player_characters(
    player_character_service: &Arc<dyn PlayerCharacterService>,
    world_id: &str,
) -> ResponseResult {
    let world_id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match player_character_service.get_pcs_by_world(&world_id).await {
        Ok(pcs) => {
            let dtos: Vec<PlayerCharacterResponseDto> =
                pcs.into_iter().map(|pc| pc.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetPlayerCharacter request
pub async fn get_player_character(
    player_character_service: &Arc<dyn PlayerCharacterService>,
    pc_id: &str,
) -> ResponseResult {
    let id = match parse_player_character_id(pc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match player_character_service.get_pc(id).await {
        Ok(Some(pc)) => {
            let dto: PlayerCharacterResponseDto = pc.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Player character not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeletePlayerCharacter request (DM only)
pub async fn delete_player_character(
    player_character_service: &Arc<dyn PlayerCharacterService>,
    ctx: &RequestContext,
    pc_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_player_character_id(pc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match player_character_service.delete_pc(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreatePlayerCharacter request
pub async fn create_player_character(
    player_character_service: &Arc<dyn PlayerCharacterService>,
    region_repo: &Arc<dyn RegionRepositoryPort>,
    ctx: &RequestContext,
    world_id: &str,
    data: CreatePlayerCharacterData,
) -> ResponseResult {
    let wid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // If starting_region_id is provided, use it to get the location
    // Otherwise, we need to find a spawn point or return an error
    let (starting_location_id, starting_region_id) = if let Some(ref region_id_str) =
        data.starting_region_id
    {
        let region_id = match parse_region_id(region_id_str) {
            Ok(id) => id,
            Err(e) => return e,
        };
        // Fetch the region to get its location_id
        match region_repo.get(region_id).await {
            Ok(Some(region)) => (region.location_id, Some(region_id)),
            Ok(None) => {
                return ResponseResult::error(
                    ErrorCode::NotFound,
                    format!("Starting region not found: {}", region_id_str),
                )
            }
            Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
        }
    } else {
        // No starting region provided - try to find a spawn point in the world
        match region_repo.list_spawn_points(wid).await {
            Ok(spawn_points) if !spawn_points.is_empty() => {
                let spawn = &spawn_points[0];
                (spawn.location_id, Some(spawn.id))
            }
            Ok(_) => {
                return ResponseResult::error(
                    ErrorCode::BadRequest,
                    "No starting_region_id provided and no spawn points found in world",
                );
            }
            Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
        }
    };

    // Get user_id from context or data
    let user_id = data.user_id.clone().unwrap_or_else(|| ctx.user_id.clone());

    // Parse sheet_data if provided
    let sheet_data = data
        .sheet_data
        .as_ref()
        .and_then(|v| serde_json::from_value::<CharacterSheetData>(v.clone()).ok());

    let request = CreatePlayerCharacterRequest {
        user_id,
        world_id: wid,
        name: data.name.clone(),
        description: None,
        starting_location_id,
        sheet_data,
        sprite_asset: None,
        portrait_asset: None,
    };

    match player_character_service.create_pc(request).await {
        Ok(mut pc) => {
            // Set the starting region if provided
            if let Some(region_id) = starting_region_id {
                if let Err(e) = player_character_service
                    .update_pc_location(pc.id, starting_location_id)
                    .await
                {
                    tracing::warn!(pc_id = %pc.id, region_id = %region_id, error = %e,
                        "Failed to set starting region for PC");
                }
                // Also update the region_id on the PC for the response
                pc.current_region_id = Some(region_id);
            }
            let dto: PlayerCharacterResponseDto = pc.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdatePlayerCharacter request
pub async fn update_player_character(
    player_character_service: &Arc<dyn PlayerCharacterService>,
    pc_id: &str,
    data: UpdatePlayerCharacterData,
) -> ResponseResult {
    let id = match parse_player_character_id(pc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Parse sheet_data from protocol JSON value
    let sheet_data = data.sheet_data.as_ref().and_then(|v| {
        match serde_json::from_value::<CharacterSheetData>(v.clone()) {
            Ok(data) => Some(data),
            Err(e) => {
                tracing::debug!(error = %e, "Failed to parse sheet_data, ignoring");
                None
            }
        }
    });
    let request = UpdatePlayerCharacterRequest {
        name: data.name,
        description: None,
        sheet_data,
        sprite_asset: None,
        portrait_asset: None,
    };
    match player_character_service.update_pc(id, request).await {
        Ok(pc) => {
            let dto: PlayerCharacterResponseDto = pc.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdatePlayerCharacterLocation request
pub async fn update_player_character_location(
    player_character_service: &Arc<dyn PlayerCharacterService>,
    pc_id: &str,
    region_id: &str,
) -> ResponseResult {
    let pid = match parse_player_character_id(pc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Note: The service uses LocationId, but protocol passes region_id
    // For now, we'll try to parse as LocationId and update
    let lid = match parse_location_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match player_character_service.update_pc_location(pid, lid).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetMyPlayerCharacter request
pub async fn get_my_player_character(
    player_character_service: &Arc<dyn PlayerCharacterService>,
    world_id: &str,
    user_id: &str,
) -> ResponseResult {
    let wid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match player_character_service
        .get_pc_by_user_and_world(user_id, &wid)
        .await
    {
        Ok(Some(pc)) => {
            let dto: PlayerCharacterResponseDto = pc.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(
            ErrorCode::NotFound,
            "No player character found for user in this world",
        ),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// Observation Handlers
// =============================================================================

/// Handle ListObservations request
pub async fn list_observations(
    observation_repo: &Arc<dyn ObservationRepositoryPort>,
    pc_id: &str,
) -> ResponseResult {
    let id = match parse_player_character_id(pc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match observation_repo.get_for_pc(id).await {
        Ok(observations) => {
            let dtos: Vec<serde_json::Value> = observations
                .iter()
                .map(|obs| {
                    serde_json::json!({
                        "pc_id": obs.pc_id.to_string(),
                        "npc_id": obs.npc_id.to_string(),
                        "location_id": obs.location_id.to_string(),
                        "region_id": obs.region_id.to_string(),
                        "game_time": obs.game_time.to_rfc3339(),
                        "observation_type": obs.observation_type.to_string(),
                        "is_revealed_to_player": obs.is_revealed_to_player,
                        "notes": obs.notes,
                        "created_at": obs.created_at.to_rfc3339(),
                    })
                })
                .collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateObservation request (DM only)
pub async fn create_observation(
    observation_repo: &Arc<dyn ObservationRepositoryPort>,
    clock: &Arc<dyn ClockPort>,
    ctx: &RequestContext,
    pc_id: &str,
    data: CreateObservationData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let pid = match parse_player_character_id(pc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let npc_id = match parse_character_id(&data.npc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Parse location_id and region_id (required for creating observation)
    let location_id = match data.location_id {
        Some(ref lid) => match parse_location_id(lid) {
            Ok(id) => id,
            Err(e) => return e,
        },
        None => {
            return ResponseResult::error(
                ErrorCode::BadRequest,
                "location_id is required for creating an observation",
            );
        }
    };
    let region_id = match data.region_id {
        Some(ref rid) => match parse_region_id(rid) {
            Ok(id) => id,
            Err(e) => return e,
        },
        None => {
            return ResponseResult::error(
                ErrorCode::BadRequest,
                "region_id is required for creating an observation",
            );
        }
    };
    // Parse observation type
    let observation_type = data
        .observation_type
        .parse::<ObservationType>()
        .unwrap_or(ObservationType::Direct);
    // Use current time as game_time (in a real implementation, this might come from world state)
    let game_time = clock.now();
    // Create the observation based on type
    let now = clock.now();
    let observation = match observation_type {
        ObservationType::Direct => {
            NpcObservation::direct(pid, npc_id, location_id, region_id, game_time, now)
        }
        ObservationType::HeardAbout => NpcObservation::heard_about(
            pid,
            npc_id,
            location_id,
            region_id,
            game_time,
            data.notes.clone(),
            now,
        ),
        ObservationType::Deduced => NpcObservation::deduced(
            pid,
            npc_id,
            location_id,
            region_id,
            game_time,
            data.notes.clone(),
            now,
        ),
    };
    match observation_repo.upsert(&observation).await {
        Ok(()) => ResponseResult::success(serde_json::json!({
            "pc_id": observation.pc_id.to_string(),
            "npc_id": observation.npc_id.to_string(),
            "location_id": observation.location_id.to_string(),
            "region_id": observation.region_id.to_string(),
            "observation_type": observation.observation_type.to_string(),
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteObservation request (DM only)
pub async fn delete_observation(
    observation_repo: &Arc<dyn ObservationRepositoryPort>,
    ctx: &RequestContext,
    pc_id: &str,
    npc_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let pid = match parse_player_character_id(pc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let nid = match parse_character_id(npc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match observation_repo.delete(pid, nid).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// Character-Region Relationship Handlers
// =============================================================================

/// Handle ListCharacterRegionRelationships request
pub async fn list_character_region_relationships(
    character_location: &Arc<dyn CharacterLocationPort>,
    character_id: &str,
) -> ResponseResult {
    let id = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match character_location.get_region_relationships(id).await {
        Ok(relationships) => {
            let dtos: Vec<serde_json::Value> = relationships
                .iter()
                .map(|rel| {
                    serde_json::json!({
                        "region_id": rel.region_id.to_string(),
                        "region_name": rel.region_name,
                        "relationship_type": serde_json::to_value(&rel.relationship_type).unwrap_or(serde_json::Value::Null),
                    })
                })
                .collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetCharacterHomeRegion request (DM only)
pub async fn set_character_home_region(
    character_location: &Arc<dyn CharacterLocationPort>,
    ctx: &RequestContext,
    character_id: &str,
    region_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let cid = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let rid = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match character_location.set_home_region(cid, rid).await {
        Ok(_) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetCharacterWorkRegion request (DM only)
pub async fn set_character_work_region(
    character_location: &Arc<dyn CharacterLocationPort>,
    ctx: &RequestContext,
    character_id: &str,
    region_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let cid = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let rid = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Default to "always" shift since the protocol doesn't include shift data
    match character_location
        .set_work_region(cid, rid, RegionShift::Always)
        .await
    {
        Ok(_) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle RemoveCharacterRegionRelationship request (DM only)
pub async fn remove_character_region_relationship(
    character_location: &Arc<dyn CharacterLocationPort>,
    ctx: &RequestContext,
    character_id: &str,
    region_id: &str,
    relationship_type: String,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let cid = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let rid = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match character_location
        .remove_region_relationship(cid, rid, relationship_type)
        .await
    {
        Ok(_) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
