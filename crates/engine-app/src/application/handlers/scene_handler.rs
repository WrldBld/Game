//! Scene and Interaction domain request handlers
//!
//! Handles: Scene CRUD, Interaction CRUD, Interaction availability

use std::sync::Arc;

use wrldbldr_domain::entities::{InteractionTarget, InteractionTemplate, InteractionType};
use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_protocol::{
    CreateInteractionData, CreateSceneData, ErrorCode, ResponseResult, UpdateInteractionData,
    UpdateSceneData,
};

use super::common::{parse_act_id, parse_interaction_id, parse_location_id, parse_scene_id};
use crate::application::dto::{InteractionResponseDto, SceneResponseDto};
use crate::application::services::{
    CreateSceneRequest, InteractionService, SceneService, UpdateSceneRequest,
};

// =============================================================================
// Scene Handlers
// =============================================================================

/// Handle ListScenes request
pub async fn list_scenes(scene_service: &Arc<dyn SceneService>, act_id: &str) -> ResponseResult {
    let id = match parse_act_id(act_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match scene_service.list_scenes_by_act(id).await {
        Ok(scenes) => {
            let dtos: Vec<SceneResponseDto> = scenes.into_iter().map(|s| s.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetScene request
pub async fn get_scene(scene_service: &Arc<dyn SceneService>, scene_id: &str) -> ResponseResult {
    let id = match parse_scene_id(scene_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match scene_service.get_scene(id).await {
        Ok(Some(scene)) => {
            let dto: SceneResponseDto = scene.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Scene not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteScene request (DM only)
pub async fn delete_scene(
    scene_service: &Arc<dyn SceneService>,
    ctx: &RequestContext,
    scene_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_scene_id(scene_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match scene_service.delete_scene(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateScene request (DM only)
pub async fn create_scene(
    scene_service: &Arc<dyn SceneService>,
    ctx: &RequestContext,
    act_id: &str,
    data: CreateSceneData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let aid = match parse_act_id(act_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Parse location_id if provided
    let location_id = match data.location_id {
        Some(ref lid) => match parse_location_id(lid) {
            Ok(id) => id,
            Err(e) => return e,
        },
        None => {
            return ResponseResult::error(
                ErrorCode::BadRequest,
                "location_id is required for creating a scene",
            );
        }
    };
    let request = CreateSceneRequest {
        act_id: aid,
        name: data.name,
        location_id,
        time_context: None,
        backdrop_override: None,
        featured_characters: vec![],
        directorial_notes: data.description,
        entry_conditions: vec![],
        order: 0,
    };
    match scene_service.create_scene(request).await {
        Ok(scene) => {
            let dto: SceneResponseDto = scene.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateScene request (DM only)
pub async fn update_scene(
    scene_service: &Arc<dyn SceneService>,
    ctx: &RequestContext,
    scene_id: &str,
    data: UpdateSceneData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_scene_id(scene_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let request = UpdateSceneRequest {
        name: data.name,
        time_context: None,
        backdrop_override: None,
        entry_conditions: None,
        order: None,
    };
    match scene_service.update_scene(id, request).await {
        Ok(scene) => {
            let dto: SceneResponseDto = scene.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// Interaction Handlers
// =============================================================================

/// Handle ListInteractions request
pub async fn list_interactions(
    interaction_service: &Arc<dyn InteractionService>,
    scene_id: &str,
) -> ResponseResult {
    let id = match parse_scene_id(scene_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match interaction_service.list_interactions(id).await {
        Ok(interactions) => {
            let dtos: Vec<InteractionResponseDto> =
                interactions.into_iter().map(|i| i.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetInteraction request
pub async fn get_interaction(
    interaction_service: &Arc<dyn InteractionService>,
    interaction_id: &str,
) -> ResponseResult {
    let id = match parse_interaction_id(interaction_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match interaction_service.get_interaction(id).await {
        Ok(Some(interaction)) => {
            let dto: InteractionResponseDto = interaction.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Interaction not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteInteraction request (DM only)
pub async fn delete_interaction(
    interaction_service: &Arc<dyn InteractionService>,
    ctx: &RequestContext,
    interaction_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_interaction_id(interaction_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match interaction_service.delete_interaction(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetInteractionAvailability request (DM only)
pub async fn set_interaction_availability(
    interaction_service: &Arc<dyn InteractionService>,
    ctx: &RequestContext,
    interaction_id: &str,
    available: bool,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_interaction_id(interaction_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match interaction_service
        .set_interaction_availability(id, available)
        .await
    {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateInteraction request (DM only)
pub async fn create_interaction(
    interaction_service: &Arc<dyn InteractionService>,
    ctx: &RequestContext,
    scene_id: &str,
    data: CreateInteractionData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let sid = match parse_scene_id(scene_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Create a new InteractionTemplate entity
    let interaction = InteractionTemplate::new(
        sid,
        data.name,
        InteractionType::Dialogue, // Default type
        InteractionTarget::None,
    )
    .with_prompt_hints(data.description.unwrap_or_default());

    // Set availability if specified
    let interaction = if data.available == Some(false) {
        interaction.disabled()
    } else {
        interaction
    };

    match interaction_service.create_interaction(&interaction).await {
        Ok(()) => {
            let dto: InteractionResponseDto = interaction.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateInteraction request (DM only)
pub async fn update_interaction(
    interaction_service: &Arc<dyn InteractionService>,
    ctx: &RequestContext,
    interaction_id: &str,
    data: UpdateInteractionData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_interaction_id(interaction_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Fetch existing interaction first
    let existing = match interaction_service.get_interaction(id).await {
        Ok(Some(i)) => i,
        Ok(None) => return ResponseResult::error(ErrorCode::NotFound, "Interaction not found"),
        Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    };
    // Apply updates
    let mut updated = existing;
    if let Some(name) = data.name {
        updated.name = name;
    }
    if let Some(description) = data.description {
        updated.prompt_hints = description;
    }
    if let Some(available) = data.available {
        updated.is_available = available;
    }
    match interaction_service.update_interaction(&updated).await {
        Ok(()) => {
            let dto: InteractionResponseDto = updated.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
