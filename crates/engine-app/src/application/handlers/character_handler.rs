//! Character domain request handlers
//!
//! Handles: Character CRUD, Inventory, Archetype changes

use std::sync::Arc;

use wrldbldr_domain::value_objects::CampbellArchetype;
use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_protocol::{
    ChangeArchetypeData, CreateCharacterData, ErrorCode, ResponseResult, UpdateCharacterData,
};

use super::common::{parse_character_id, parse_player_character_id, parse_world_id};
use crate::application::dto::CharacterResponseDto;
use crate::application::services::{
    ChangeArchetypeRequest, CharacterService, CreateCharacterRequest, ItemService,
    UpdateCharacterRequest,
};

/// Handle ListCharacters request
pub async fn list_characters(
    character_service: &Arc<dyn CharacterService>,
    world_id: &str,
) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match character_service.list_characters(id).await {
        Ok(characters) => {
            let dtos: Vec<CharacterResponseDto> =
                characters.into_iter().map(|c| c.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetCharacter request
pub async fn get_character(
    character_service: &Arc<dyn CharacterService>,
    character_id: &str,
) -> ResponseResult {
    let id = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match character_service.get_character(id).await {
        Ok(Some(character)) => {
            let dto: CharacterResponseDto = character.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Character not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteCharacter request (DM only)
pub async fn delete_character(
    character_service: &Arc<dyn CharacterService>,
    ctx: &RequestContext,
    character_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match character_service.delete_character(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateCharacter request (DM only)
pub async fn create_character(
    character_service: &Arc<dyn CharacterService>,
    ctx: &RequestContext,
    world_id: &str,
    data: CreateCharacterData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Parse archetype (default to Ally if not specified)
    let archetype = data
        .archetype
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(CampbellArchetype::Ally);

    let request = CreateCharacterRequest {
        world_id: id,
        name: data.name,
        description: data.description,
        archetype,
        sprite_asset: data.sprite_asset,
        portrait_asset: data.portrait_asset,
        stats: None,
        initial_wants: vec![],
    };
    match character_service.create_character(request).await {
        Ok(character) => {
            let dto: CharacterResponseDto = character.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateCharacter request (DM only)
pub async fn update_character(
    character_service: &Arc<dyn CharacterService>,
    ctx: &RequestContext,
    character_id: &str,
    data: UpdateCharacterData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let request = UpdateCharacterRequest {
        name: data.name,
        description: data.description,
        sprite_asset: data.sprite_asset,
        portrait_asset: data.portrait_asset,
        stats: None,
        is_alive: data.is_alive,
        is_active: data.is_active,
    };
    match character_service.update_character(id, request).await {
        Ok(character) => {
            let dto: CharacterResponseDto = character.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle ChangeArchetype request (DM only)
pub async fn change_archetype(
    character_service: &Arc<dyn CharacterService>,
    ctx: &RequestContext,
    character_id: &str,
    data: ChangeArchetypeData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let archetype = data
        .new_archetype
        .parse()
        .unwrap_or(CampbellArchetype::Ally);
    let request = ChangeArchetypeRequest {
        new_archetype: archetype,
        reason: data.reason,
    };
    match character_service.change_archetype(id, request).await {
        Ok(character) => {
            let dto: CharacterResponseDto = character.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetCharacterInventory request
pub async fn get_character_inventory(
    item_service: &Arc<dyn ItemService>,
    character_id: &str,
) -> ResponseResult {
    let pc_id = match parse_player_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match item_service.get_pc_inventory(pc_id).await {
        Ok(items) => {
            let dtos: Vec<serde_json::Value> = items
                .iter()
                .map(|inv_item| {
                    serde_json::json!({
                        "item_id": inv_item.item.id.to_string(),
                        "item_name": inv_item.item.name,
                        "item_description": inv_item.item.description,
                        "quantity": inv_item.quantity,
                        "is_equipped": inv_item.equipped,
                        "acquired_at": inv_item.acquired_at.to_rfc3339(),
                        "acquisition_method": inv_item.acquisition_method.as_ref().map(|m| format!("{:?}", m)),
                    })
                })
                .collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
