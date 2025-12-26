//! Helper functions for WebSocket queue integration
//!
//! These functions assist with building prompts and processing queue items
//! in the WebSocket handler and background workers.
//!
//! NOTE: This module is currently non-functional. The session-based architecture
//! has been removed in favor of world-based state management via WorldStateManager.
//! The `build_prompt_from_action` function needs to be refactored to:
//! 1. Accept a WorldId instead of SessionManager
//! 2. Fetch world snapshot via WorldService
//! 3. Get conversation history from WorldStateManager
//! 
//! See WEBSOCKET_MIGRATION_COMPLETION.md for migration plan.

use std::sync::Arc;

use wrldbldr_domain::value_objects::GamePromptRequest;
use wrldbldr_domain::WorldId;
use wrldbldr_engine_app::application::dto::PlayerActionItem;
use wrldbldr_engine_app::application::services::{
    ActantialContextService, ChallengeService, MoodService,
    NarrativeEventService, SettingsService, SkillService,
};
use wrldbldr_engine_ports::outbound::{
    CharacterRepositoryPort, PlayerCharacterRepositoryPort, QueueError, RegionRepositoryPort,
};

/// Build a GamePromptRequest from a PlayerActionItem
///
/// # TODO: Refactor for world-based architecture
/// 
/// This function previously used SessionManager to access world snapshot and
/// conversation history. It needs to be refactored to:
/// - Accept WorldId and WorldStateManager instead of SessionManager
/// - Fetch world data via WorldService.export_world_snapshot()
/// - Get conversation history via WorldStateManager.get_conversation_history()
///
/// For now, returns an error indicating the function needs refactoring.
#[allow(unused_variables)]
pub async fn build_prompt_from_action(
    world_id: WorldId,
    challenge_service: &Arc<dyn ChallengeService>,
    skill_service: &Arc<dyn SkillService>,
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    character_repo: &Arc<dyn CharacterRepositoryPort>,
    pc_repo: &Arc<dyn PlayerCharacterRepositoryPort>,
    region_repo: &Arc<dyn RegionRepositoryPort>,
    settings_service: &Arc<SettingsService>,
    mood_service: &Arc<dyn MoodService>,
    actantial_service: &Arc<dyn ActantialContextService>,
    action: &PlayerActionItem,
) -> Result<GamePromptRequest, QueueError> {
    // TODO: Refactor to use world-based architecture
    // 
    // Previous implementation used session.world_snapshot for:
    // - current_scene_id
    // - scenes, locations, characters
    // - conversation history
    //
    // New implementation should:
    // 1. Get world snapshot via WorldService.export_world_snapshot(world_id)
    // 2. Get conversation history via WorldStateManager.get_conversation_history(world_id)
    // 3. Get current scene from WorldStateManager.get_current_scene(world_id)
    
    Err(QueueError::Backend(
        "build_prompt_from_action needs refactoring for world-based architecture. \
         See websocket_helpers.rs module docs for migration plan.".to_string()
    ))
}
