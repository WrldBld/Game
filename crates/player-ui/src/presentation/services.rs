//! Service providers for the presentation layer
//!
//! This module provides Dioxus context providers for application services.
//! Components can use `use_context` to access services without depending
//! on infrastructure implementations.
//!
//! ## Architecture Note
//!
//! The presentation layer only depends on the application layer (`wrldbldr-player-app`)
//! and ports (`wrldbldr-player-ports`). It does not depend on adapter types.
//!
//! Service hooks are typed to `wrldbldr_player_app::application::api::Api`, which is a
//! typed wrapper around an object-safe `RawApiPort` provided by the composition root.
use dioxus::prelude::*;
use std::sync::Arc;

use wrldbldr_player_app::application::services::{
    ActantialService, AssetService, CharacterService, ChallengeService, EventChainService, GenerationService, LocationService, NarrativeEventService,
    ObservationService, PlayerCharacterService, SettingsService, SkillService, StoryEventService, SuggestionService, WorkflowService, WorldService,
};
use wrldbldr_player_ports::outbound::ApiPort;

use wrldbldr_player_app::application::api::Api;

/// Concrete service bundle type used by the UI.
///
/// The actual `RawApiPort` implementation is chosen in the player binary crate.
pub type UiServices = Services<Api>;

/// All services wrapped for context provision
#[derive(Clone)]
pub struct Services<A: ApiPort> {
    pub world: Arc<WorldService<A>>,
    pub character: Arc<CharacterService<A>>,
    pub location: Arc<LocationService<A>>,
    pub player_character: Arc<PlayerCharacterService<A>>,
    pub skill: Arc<SkillService<A>>,
    pub challenge: Arc<ChallengeService<A>>,
    pub story_event: Arc<StoryEventService<A>>,
    pub narrative_event: Arc<NarrativeEventService<A>>,
    pub workflow: Arc<WorkflowService<A>>,
    pub asset: Arc<AssetService<A>>,
    pub suggestion: Arc<SuggestionService<A>>,
    pub event_chain: Arc<EventChainService<A>>,
    pub generation: Arc<GenerationService<A>>,
    pub settings: Arc<SettingsService<A>>,
    pub observation: Arc<ObservationService<A>>,
    pub actantial: Arc<ActantialService<A>>,
}

impl<A: ApiPort + Clone> Services<A> {
    /// Create all services with the given API port implementation
    pub fn new(api: A) -> Self {
        Self {
            world: Arc::new(WorldService::new(api.clone())),
            character: Arc::new(CharacterService::new(api.clone())),
            location: Arc::new(LocationService::new(api.clone())),
            player_character: Arc::new(PlayerCharacterService::new(api.clone())),
            skill: Arc::new(SkillService::new(api.clone())),
            challenge: Arc::new(ChallengeService::new(api.clone())),
            story_event: Arc::new(StoryEventService::new(api.clone())),
            narrative_event: Arc::new(NarrativeEventService::new(api.clone())),
            workflow: Arc::new(WorkflowService::new(api.clone())),
            asset: Arc::new(AssetService::new(api.clone())),
            suggestion: Arc::new(SuggestionService::new(api.clone())),
            event_chain: Arc::new(EventChainService::new(api.clone())),
            generation: Arc::new(GenerationService::new(api.clone())),
            settings: Arc::new(SettingsService::new(api.clone())),
            observation: Arc::new(ObservationService::new(api.clone())),
            actantial: Arc::new(ActantialService::new(api)),
        }
    }
}

/// Hook to access the WorldService from context
pub fn use_world_service() -> Arc<WorldService<Api>> {
    let services = use_context::<UiServices>();
    services.world.clone()
}

/// Hook to access the CharacterService from context
pub fn use_character_service() -> Arc<CharacterService<Api>> {
    let services = use_context::<UiServices>();
    services.character.clone()
}

/// Hook to access the LocationService from context
pub fn use_location_service() -> Arc<LocationService<Api>> {
    let services = use_context::<UiServices>();
    services.location.clone()
}

/// Hook to access the PlayerCharacterService from context
pub fn use_player_character_service() -> Arc<PlayerCharacterService<Api>> {
    let services = use_context::<UiServices>();
    services.player_character.clone()
}

/// Hook to access the SkillService from context
pub fn use_skill_service() -> Arc<SkillService<Api>> {
    let services = use_context::<UiServices>();
    services.skill.clone()
}

/// Hook to access the ChallengeService from context
pub fn use_challenge_service() -> Arc<ChallengeService<Api>> {
    let services = use_context::<UiServices>();
    services.challenge.clone()
}

/// Hook to access the StoryEventService from context
pub fn use_story_event_service() -> Arc<StoryEventService<Api>> {
    let services = use_context::<UiServices>();
    services.story_event.clone()
}

/// Hook to access the NarrativeEventService from context
pub fn use_narrative_event_service() -> Arc<NarrativeEventService<Api>> {
    let services = use_context::<UiServices>();
    services.narrative_event.clone()
}

/// Hook to access the WorkflowService from context
pub fn use_workflow_service() -> Arc<WorkflowService<Api>> {
    let services = use_context::<UiServices>();
    services.workflow.clone()
}

/// Hook to access the AssetService from context
pub fn use_asset_service() -> Arc<AssetService<Api>> {
    let services = use_context::<UiServices>();
    services.asset.clone()
}

/// Hook to access the SuggestionService from context
pub fn use_suggestion_service() -> Arc<SuggestionService<Api>> {
    let services = use_context::<UiServices>();
    services.suggestion.clone()
}

/// Hook to access the EventChainService from context
pub fn use_event_chain_service() -> Arc<EventChainService<Api>> {
    let services = use_context::<UiServices>();
    services.event_chain.clone()
}

/// Hook to access the GenerationService from context
pub fn use_generation_service() -> Arc<GenerationService<Api>> {
    let services = use_context::<UiServices>();
    services.generation.clone()
}

/// Hook to access the SettingsService from context
pub fn use_settings_service() -> Arc<SettingsService<Api>> {
    let services = use_context::<UiServices>();
    services.settings.clone()
}

/// Hook to access the ObservationService from context
pub fn use_observation_service() -> Arc<ObservationService<Api>> {
    let services = use_context::<UiServices>();
    services.observation.clone()
}

/// Hook to access the ActantialService from context
pub fn use_actantial_service() -> Arc<ActantialService<Api>> {
    let services = use_context::<UiServices>();
    services.actantial.clone()
}

use crate::presentation::state::{BatchStatus, GenerationBatch, GenerationState, SuggestionStatus, SuggestionTask};
use wrldbldr_player_ports::outbound::Platform;
use anyhow::Result;

/// Hydrate GenerationState from the Engine's unified generation queue endpoint.
///
/// # Arguments
/// * `generation_service` - The GenerationService to fetch queue state from
/// * `generation_state` - The mutable state to populate
/// * `user_id` - Optional user ID to filter queue items
/// * `platform` - The platform adapter for storage access
/// * `world_id` - World ID to scope the queue to
pub async fn hydrate_generation_queue<A: ApiPort>(
    generation_service: &GenerationService<A>,
    generation_state: &mut GenerationState,
    user_id: Option<&str>,
    world_id: &str,
    platform: &Platform,
) -> Result<()> {
    let snapshot = generation_service.fetch_queue(user_id, world_id).await?;

    // Clear existing state and repopulate from snapshot
    generation_state.clear();

    for b in snapshot.batches {
        let status = match b.status.as_str() {
            "queued" => BatchStatus::Queued {
                position: b.position.unwrap_or(0),
            },
            "generating" => BatchStatus::Generating {
                progress: b.progress.unwrap_or(0),
            },
            "ready" => BatchStatus::Ready {
                asset_count: b.asset_count.unwrap_or(0),
            },
            "failed" => BatchStatus::Failed {
                error: b.error.unwrap_or_else(|| "Unknown error".to_string()),
            },
            _ => BatchStatus::Queued { position: 0 },
        };

        generation_state.add_batch(crate::presentation::state::GenerationBatch {
            batch_id: b.batch_id,
            entity_type: b.entity_type,
            entity_id: b.entity_id,
            asset_type: b.asset_type,
            status,
            is_read: b.is_read,
        });
    }

    for s in snapshot.suggestions {
        let status = match s.status.as_str() {
            "queued" => SuggestionStatus::Queued,
            "processing" => SuggestionStatus::Processing,
            "ready" => SuggestionStatus::Ready {
                suggestions: s.suggestions.unwrap_or_default(),
            },
            "failed" => SuggestionStatus::Failed {
                error: s.error.unwrap_or_else(|| "Unknown error".to_string()),
            },
            _ => SuggestionStatus::Queued,
        };

        generation_state.add_suggestion_task(
            s.request_id.clone(),
            s.field_type,
            s.entity_id,
            None, // Context not available from snapshot
            None, // World ID not available from snapshot (but not needed - only original requester can retry)
        );
        // Override status if needed using the same request_id
        let req_id = s.request_id;
        match status {
            SuggestionStatus::Queued => {}
            SuggestionStatus::Processing => {
                generation_state.suggestion_progress(&req_id, "processing");
            }
            SuggestionStatus::Ready { suggestions } => {
                generation_state.suggestion_complete(&req_id, suggestions);
            }
            SuggestionStatus::Failed { error } => {
                generation_state.suggestion_failed(&req_id, error);
            }
        }
    }

    // Re-apply persisted read/unread state based on local storage (secondary layer)
    apply_generation_read_state(platform, generation_state);

    Ok(())
}

const STORAGE_KEY_GEN_READ_BATCHES: &str = "wrldbldr_gen_read_batches";
const STORAGE_KEY_GEN_READ_SUGGESTIONS: &str = "wrldbldr_gen_read_suggestions";

/// Persist the read/unread state of generation queue items to local storage
pub fn persist_generation_read_state(platform: &Platform, state: &GenerationState) {
    // Persist read batch IDs
    let read_batch_ids: Vec<String> = state
        .get_batches()
        .into_iter()
        .filter(|b| b.is_read)
        .map(|b| b.batch_id)
        .collect();
    let batch_value = read_batch_ids.join(",");
    platform.storage_save(STORAGE_KEY_GEN_READ_BATCHES, &batch_value);

    // Persist read suggestion IDs
    let read_suggestion_ids: Vec<String> = state
        .get_suggestions()
        .into_iter()
        .filter(|s| s.is_read)
        .map(|s| s.request_id)
        .collect();
    let suggestion_value = read_suggestion_ids.join(",");
    platform.storage_save(STORAGE_KEY_GEN_READ_SUGGESTIONS, &suggestion_value);
}

/// Apply persisted read/unread state from local storage to the current GenerationState
fn apply_generation_read_state(platform: &Platform, state: &mut GenerationState) {
    if let Some(batch_str) = platform.storage_load(STORAGE_KEY_GEN_READ_BATCHES) {
        for id in batch_str.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            state.mark_batch_read(id);
        }
    }

    if let Some(sugg_str) = platform.storage_load(STORAGE_KEY_GEN_READ_SUGGESTIONS) {
        for id in sugg_str.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            state.mark_suggestion_read(id);
        }
    }
}

/// Sync generation read state to the backend.
///
/// This helper collects all read batches and suggestions from the given state
/// and sends them to the Engine via the GenerationService.
///
/// # Arguments
/// * `generation_service` - The GenerationService to sync with
/// * `state` - The GenerationState to sync read markers from
/// * `world_id` - Optional world ID to scope read markers
pub async fn sync_generation_read_state<A: ApiPort>(
    generation_service: &GenerationService<A>,
    state: &GenerationState,
    world_id: Option<&str>,
) -> Result<()> {
    let read_batches: Vec<String> = state
        .get_batches()
        .into_iter()
        .filter(|b| b.is_read)
        .map(|b| b.batch_id)
        .collect();

    let read_suggestions: Vec<String> = state
        .get_suggestions()
        .into_iter()
        .filter(|s| s.is_read)
        .map(|s| s.request_id)
        .collect();

    // Only sync if there are read items
    if read_batches.is_empty() && read_suggestions.is_empty() {
        return Ok(());
    }

    generation_service
        .sync_read_state(read_batches, read_suggestions, world_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to sync generation read state: {}", e))?;

    Ok(())
}

/// View-model helpers for generation queue filtering and actions

/// Get visible batches based on show_read filter
pub fn visible_batches(
    state: &GenerationState,
    show_read: bool,
) -> Vec<GenerationBatch> {
    state
        .get_batches()
        .into_iter()
        .filter(|b| show_read || !b.is_read)
        .collect()
}

/// Get visible suggestions based on show_read filter
pub fn visible_suggestions(
    state: &GenerationState,
    show_read: bool,
) -> Vec<SuggestionTask> {
    state
        .get_suggestions()
        .into_iter()
        .filter(|s| show_read || !s.is_read)
        .collect()
}

/// Mark a batch as read and sync to backend
///
/// # Arguments
/// * `generation_service` - The GenerationService to sync with
/// * `state` - The mutable GenerationState
/// * `batch_id` - The batch ID to mark as read
/// * `world_id` - Optional world ID scope
/// * `platform` - The platform adapter for storage access
pub async fn mark_batch_read_and_sync<A: ApiPort>(
    generation_service: &GenerationService<A>,
    state: &mut GenerationState,
    batch_id: &str,
    world_id: Option<&str>,
    platform: &Platform,
) -> Result<()> {
    state.mark_batch_read(batch_id);
    persist_generation_read_state(platform, state);
    sync_generation_read_state(generation_service, state, world_id).await
}

/// Mark a suggestion as read and sync to backend
///
/// # Arguments
/// * `generation_service` - The GenerationService to sync with
/// * `state` - The mutable GenerationState
/// * `request_id` - The request ID to mark as read
/// * `world_id` - Optional world ID scope
/// * `platform` - The platform adapter for storage access
pub async fn mark_suggestion_read_and_sync<A: ApiPort>(
    generation_service: &GenerationService<A>,
    state: &mut GenerationState,
    request_id: &str,
    world_id: Option<&str>,
    platform: &Platform,
) -> Result<()> {
    state.mark_suggestion_read(request_id);
    persist_generation_read_state(platform, state);
    sync_generation_read_state(generation_service, state, world_id).await
}
