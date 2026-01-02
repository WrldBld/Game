//! AI suggestion and generation queue request handlers
//!
//! Handles: AI suggestions (deflection, tells, wants, actantial), generation queue,
//! content suggestions, and item placement operations.

use std::sync::Arc;

use wrldbldr_engine_ports::inbound::RequestContext;
use crate::application::services::internal::GenerationQueueProjectionUseCasePort;
use wrldbldr_engine_ports::outbound::{
    GenerationReadKind, GenerationReadStatePort, SuggestionEnqueueContext, SuggestionEnqueuePort,
    SuggestionEnqueueRequest,
};
use wrldbldr_protocol::{
    ActantialRoleData, CreateItemData, ErrorCode, ResponseResult, SuggestionContextData,
};

use super::common::{
    parse_character_id, parse_item_id, parse_region_id, parse_want_id, parse_world_id,
};
use crate::application::services::{CharacterService, CreateItemRequest, ItemService};

// =============================================================================
// AI Suggestion Operations
// =============================================================================

/// Handle SuggestDeflectionBehavior request (DM only)
///
/// Enqueues a request for AI-generated deflection behavior suggestions for an NPC want.
pub async fn suggest_deflection_behavior(
    character_service: &Arc<dyn CharacterService>,
    suggestion_enqueue: &Arc<dyn SuggestionEnqueuePort>,
    ctx: &RequestContext,
    npc_id: &str,
    want_id: &str,
    want_description: String,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }

    let char_id = match parse_character_id(npc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let wid = match parse_want_id(want_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Get character for context
    let character = match character_service.get_character(char_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return ResponseResult::error(
                ErrorCode::NotFound,
                format!("Character {} not found", npc_id),
            )
        }
        Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    };

    // Build suggestion context
    let context = SuggestionEnqueueContext {
        entity_type: Some("npc".to_string()),
        entity_name: Some(character.name.clone()),
        world_setting: None, // Could fetch from world settings
        hints: Some(want_description),
        additional_context: Some(character.description.clone()),
        world_id: Some(character.world_id.to_uuid().to_string()),
    };

    let request = SuggestionEnqueueRequest {
        field_type: "deflection_behavior".to_string(),
        entity_id: Some(wid.to_uuid().to_string()),
        world_id: Some(character.world_id.to_uuid()),
        context,
    };

    match suggestion_enqueue.enqueue_suggestion(request).await {
        Ok(response) => ResponseResult::success(serde_json::json!({
            "request_id": response.request_id,
            "status": "queued"
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SuggestBehavioralTells request (DM only)
///
/// Enqueues a request for AI-generated behavioral tells suggestions for an NPC want.
pub async fn suggest_behavioral_tells(
    character_service: &Arc<dyn CharacterService>,
    suggestion_enqueue: &Arc<dyn SuggestionEnqueuePort>,
    ctx: &RequestContext,
    npc_id: &str,
    want_id: &str,
    want_description: String,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }

    let char_id = match parse_character_id(npc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let wid = match parse_want_id(want_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Get character for context
    let character = match character_service.get_character(char_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return ResponseResult::error(
                ErrorCode::NotFound,
                format!("Character {} not found", npc_id),
            )
        }
        Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    };

    // Build suggestion context
    let context = SuggestionEnqueueContext {
        entity_type: Some("npc".to_string()),
        entity_name: Some(character.name.clone()),
        world_setting: None,
        hints: Some(want_description),
        additional_context: Some(character.description.clone()),
        world_id: Some(character.world_id.to_uuid().to_string()),
    };

    let request = SuggestionEnqueueRequest {
        field_type: "behavioral_tells".to_string(),
        entity_id: Some(wid.to_uuid().to_string()),
        world_id: Some(character.world_id.to_uuid()),
        context,
    };

    match suggestion_enqueue.enqueue_suggestion(request).await {
        Ok(response) => ResponseResult::success(serde_json::json!({
            "request_id": response.request_id,
            "status": "queued"
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SuggestWantDescription request (DM only)
///
/// Enqueues a request for AI-generated want description suggestions for an NPC.
pub async fn suggest_want_description(
    character_service: &Arc<dyn CharacterService>,
    suggestion_enqueue: &Arc<dyn SuggestionEnqueuePort>,
    ctx: &RequestContext,
    npc_id: &str,
    context: Option<String>,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }

    let char_id = match parse_character_id(npc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Get character for context
    let character = match character_service.get_character(char_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return ResponseResult::error(
                ErrorCode::NotFound,
                format!("Character {} not found", npc_id),
            )
        }
        Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    };

    // Build suggestion context
    let suggestion_context = SuggestionEnqueueContext {
        entity_type: Some("npc".to_string()),
        entity_name: Some(character.name.clone()),
        world_setting: None,
        hints: context, // Use provided context as hints
        additional_context: Some(character.description.clone()),
        world_id: Some(character.world_id.to_uuid().to_string()),
    };

    let request = SuggestionEnqueueRequest {
        field_type: "want_description".to_string(),
        entity_id: Some(char_id.to_uuid().to_string()),
        world_id: Some(character.world_id.to_uuid()),
        context: suggestion_context,
    };

    match suggestion_enqueue.enqueue_suggestion(request).await {
        Ok(response) => ResponseResult::success(serde_json::json!({
            "request_id": response.request_id,
            "status": "queued"
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SuggestActantialReason request (DM only)
///
/// Enqueues a request for AI-generated actantial relationship reason suggestions.
pub async fn suggest_actantial_reason(
    character_service: &Arc<dyn CharacterService>,
    suggestion_enqueue: &Arc<dyn SuggestionEnqueuePort>,
    ctx: &RequestContext,
    npc_id: &str,
    want_id: &str,
    target_id: &str,
    role: ActantialRoleData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }

    let char_id = match parse_character_id(npc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let wid = match parse_want_id(want_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Get character for context
    let character = match character_service.get_character(char_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return ResponseResult::error(
                ErrorCode::NotFound,
                format!("Character {} not found", npc_id),
            )
        }
        Err(e) => return ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    };

    // Try to get target character name
    let target_name = if let Ok(target_char_id) = parse_character_id(target_id) {
        match character_service.get_character(target_char_id).await {
            Ok(Some(c)) => c.name,
            _ => target_id.to_string(),
        }
    } else {
        target_id.to_string()
    };

    // Build suggestion context
    // hints: Target of the actantial relationship
    // additional_context: The actantial role (e.g., "a helper", "an opponent")
    let role_str = match role {
        ActantialRoleData::Helper | ActantialRoleData::Unknown => "a helper", // Default unknown to helper
        ActantialRoleData::Opponent => "an opponent",
        ActantialRoleData::Sender => "a sender",
        ActantialRoleData::Receiver => "a receiver",
    };

    let context = SuggestionEnqueueContext {
        entity_type: Some("npc".to_string()),
        entity_name: Some(character.name.clone()),
        world_setting: None,
        hints: Some(target_name),
        additional_context: Some(role_str.to_string()),
        world_id: Some(character.world_id.to_uuid().to_string()),
    };

    let request = SuggestionEnqueueRequest {
        field_type: "actantial_reason".to_string(),
        entity_id: Some(wid.to_uuid().to_string()),
        world_id: Some(character.world_id.to_uuid()),
        context,
    };

    match suggestion_enqueue.enqueue_suggestion(request).await {
        Ok(response) => ResponseResult::success(serde_json::json!({
            "request_id": response.request_id,
            "status": "queued"
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// Generation Queue Operations
// =============================================================================

/// Handle GetGenerationQueue request
///
/// Retrieves the current generation queue state for a world.
pub async fn get_generation_queue(
    generation_queue_projection: &Arc<dyn GenerationQueueProjectionUseCasePort>,
    ctx: &RequestContext,
    world_id: &str,
    user_id: Option<String>,
) -> ResponseResult {
    let wid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Use provided user_id or fall back to context user_id
    let effective_user_id = user_id.or_else(|| Some(ctx.user_id.clone()));

    match generation_queue_projection
        .project_queue(effective_user_id, wid)
        .await
    {
        Ok(snapshot) => ResponseResult::success(snapshot),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SyncGenerationReadState request
///
/// Marks batches and suggestions as read for the current user.
pub async fn sync_generation_read_state(
    generation_read_state: &Arc<dyn GenerationReadStatePort>,
    ctx: &RequestContext,
    world_id: &str,
    read_batches: &[String],
    read_suggestions: &[String],
) -> ResponseResult {
    let user_id = &ctx.user_id;

    // Mark batches as read
    for batch_id in read_batches {
        if let Err(e) = generation_read_state
            .mark_read(user_id, world_id, batch_id, GenerationReadKind::Batch)
            .await
        {
            return ResponseResult::error(
                ErrorCode::InternalError,
                format!("Failed to mark batch read: {}", e),
            );
        }
    }

    // Mark suggestions as read
    for request_id in read_suggestions {
        if let Err(e) = generation_read_state
            .mark_read(
                user_id,
                world_id,
                request_id,
                GenerationReadKind::Suggestion,
            )
            .await
        {
            return ResponseResult::error(
                ErrorCode::InternalError,
                format!("Failed to mark suggestion read: {}", e),
            );
        }
    }

    ResponseResult::success_empty()
}

// =============================================================================
// Content Suggestion Operations (General LLM Suggestions)
// =============================================================================

/// Handle EnqueueContentSuggestion request (DM only)
///
/// Enqueues a general content suggestion request to the LLM queue.
pub async fn enqueue_content_suggestion(
    suggestion_enqueue: &Arc<dyn SuggestionEnqueuePort>,
    ctx: &RequestContext,
    world_id: &str,
    suggestion_type: String,
    context: SuggestionContextData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }

    let world_uuid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Convert protocol context to port context
    let suggestion_context = SuggestionEnqueueContext {
        entity_type: context.entity_type,
        entity_name: context.entity_name,
        world_setting: context.world_setting,
        hints: context.hints,
        additional_context: context.additional_context,
        world_id: Some(world_id.to_string()),
    };

    let request = SuggestionEnqueueRequest {
        field_type: suggestion_type,
        entity_id: None, // General content suggestions don't have a specific entity
        world_id: Some(world_uuid.to_uuid()),
        context: suggestion_context,
    };

    match suggestion_enqueue.enqueue_suggestion(request).await {
        Ok(response) => ResponseResult::success(serde_json::json!({
            "request_id": response.request_id,
            "status": "queued"
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CancelContentSuggestion request (DM only)
///
/// Cancels a pending content suggestion request.
pub async fn cancel_content_suggestion(
    suggestion_enqueue: &Arc<dyn SuggestionEnqueuePort>,
    ctx: &RequestContext,
    request_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }

    match suggestion_enqueue.cancel_suggestion(request_id).await {
        Ok(cancelled) => ResponseResult::success(serde_json::json!({
            "cancelled": cancelled
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// Item Placement Operations (DM only)
// =============================================================================

/// Handle PlaceItemInRegion request (DM only)
///
/// Places an existing item in a region.
pub async fn place_item_in_region(
    item_service: &Arc<dyn ItemService>,
    ctx: &RequestContext,
    region_id: &str,
    item_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }

    let rid = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let iid = match parse_item_id(item_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    match item_service.place_item_in_region(rid, iid).await {
        Ok(()) => ResponseResult::success(serde_json::json!({ "success": true })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateAndPlaceItem request (DM only)
///
/// Creates a new item and places it in a region.
pub async fn create_and_place_item(
    item_service: &Arc<dyn ItemService>,
    ctx: &RequestContext,
    world_id: &str,
    region_id: &str,
    data: CreateItemData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }

    let wid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let rid = match parse_region_id(region_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let request = CreateItemRequest {
        name: data.name,
        description: data.description,
        item_type: data.item_type,
        properties: data.properties.map(|v| v.to_string()),
        ..Default::default()
    };

    match item_service.create_and_place_item(wid, rid, request).await {
        Ok(item) => {
            // Return a simple item response
            ResponseResult::success(serde_json::json!({
                "id": item.id.to_string(),
                "name": item.name,
                "description": item.description,
                "item_type": item.item_type,
            }))
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
