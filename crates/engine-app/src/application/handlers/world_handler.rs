//! World domain request handlers
//!
//! Handles: World CRUD, Acts, SheetTemplates, GameTime, Export

use std::sync::Arc;

use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_protocol::{
    CreateActData, CreateWorldData, ErrorCode, ResponseResult, UpdateWorldData,
};

use super::common::{parse_world_id, to_protocol_game_time};
use crate::application::dto::{ActResponseDto, SheetTemplateResponseDto, WorldResponseDto};
use crate::application::services::{
    CreateActRequest, CreateWorldRequest, SheetTemplateService, UpdateWorldRequest, WorldService,
};

/// Handle ListWorlds request
pub async fn list_worlds(world_service: &Arc<dyn WorldService>) -> ResponseResult {
    match world_service.list_worlds().await {
        Ok(worlds) => {
            let dtos: Vec<WorldResponseDto> = worlds.into_iter().map(|w| w.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetWorld request
pub async fn get_world(world_service: &Arc<dyn WorldService>, world_id: &str) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match world_service.get_world(id).await {
        Ok(Some(world)) => {
            let dto: WorldResponseDto = world.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "World not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle ExportWorld request
pub async fn export_world(world_service: &Arc<dyn WorldService>, world_id: &str) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match world_service.export_world_snapshot(id).await {
        Ok(snapshot) => ResponseResult::success(snapshot),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetSheetTemplate request
pub async fn get_sheet_template(
    sheet_template_service: &Arc<SheetTemplateService>,
    world_id: &str,
) -> ResponseResult {
    let wid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match sheet_template_service.get_default_for_world(&wid).await {
        Ok(Some(template)) => {
            let dto: SheetTemplateResponseDto = template.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(
            ErrorCode::NotFound,
            "No sheet template found for world",
        ),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateWorld request (DM only)
pub async fn create_world(
    world_service: &Arc<dyn WorldService>,
    ctx: &RequestContext,
    data: CreateWorldData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let request = CreateWorldRequest {
        name: data.name,
        description: data.description.unwrap_or_default(),
        rule_system: None,
    };
    match world_service.create_world(request).await {
        Ok(world) => {
            let dto: WorldResponseDto = world.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateWorld request (DM only)
pub async fn update_world(
    world_service: &Arc<dyn WorldService>,
    ctx: &RequestContext,
    world_id: &str,
    data: UpdateWorldData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let request = UpdateWorldRequest {
        name: data.name,
        description: data.description,
        rule_system: None,
    };
    match world_service.update_world(id, request).await {
        Ok(world) => {
            let dto: WorldResponseDto = world.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteWorld request (DM only)
pub async fn delete_world(
    world_service: &Arc<dyn WorldService>,
    ctx: &RequestContext,
    world_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match world_service.delete_world(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle ListActs request
pub async fn list_acts(world_service: &Arc<dyn WorldService>, world_id: &str) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match world_service.get_acts(id).await {
        Ok(acts) => {
            let dtos: Vec<ActResponseDto> = acts.into_iter().map(|a| a.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateAct request (DM only)
pub async fn create_act(
    world_service: &Arc<dyn WorldService>,
    ctx: &RequestContext,
    world_id: &str,
    data: CreateActData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let request = CreateActRequest {
        name: data.name,
        stage: wrldbldr_domain::entities::MonomythStage::OrdinaryWorld, // Default stage
        description: data.description,
        order: data.order.unwrap_or(0),
    };
    match world_service.create_act(id, request).await {
        Ok(act) => {
            let dto: ActResponseDto = act.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetGameTime request
pub async fn get_game_time(
    world_service: &Arc<dyn WorldService>,
    world_id: &str,
) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match world_service.get_game_time(id).await {
        Ok(game_time) => ResponseResult::success(to_protocol_game_time(&game_time)),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle AdvanceGameTime request (DM only)
pub async fn advance_game_time(
    world_service: &Arc<dyn WorldService>,
    ctx: &RequestContext,
    world_id: &str,
    hours: u32,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match world_service.advance_game_time(id, hours).await {
        Ok(game_time) => ResponseResult::success(to_protocol_game_time(&game_time)),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
