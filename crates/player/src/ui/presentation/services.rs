//! Service providers for the presentation layer
//!
//! This module provides Dioxus context providers for application services.
//! Components can use `use_context` to access services without depending
//! on infrastructure implementations.
//!
//! ## Architecture Note
//!
//! The presentation layer depends on application-level services and port traits.
//! It should not depend directly on infrastructure adapter types.
//!
//! ## Service Types
//!
//! Services are split into two categories:
//! - **WebSocket services**: Use CommandBus for real-time operations
//! - **REST services**: Use `ApiPort` for HTTP-based operations (file uploads, large payloads)
use dioxus::prelude::*;
use std::sync::Arc;

use crate::application::services::{
    ActantialService, AssetService, ChallengeService, CharacterService, CharacterSheetService,
    ConversationService, EventChainService, GenerationService, LocationService,
    NarrativeEventService, ObservationService, PlayerCharacterService, SettingsService,
    SkillService, StoryEventService, SuggestionService, WorkflowService, WorldService,
};
use crate::infrastructure::messaging::{CommandBus, ConnectionKeepAlive};
use crate::infrastructure::websocket::Connection;
use crate::ports::outbound::{ApiPort, RawApiPort};

use crate::application::api::Api;

/// Concrete service bundle type used by the UI.
pub type UiServices = Services<Api>;

/// All services wrapped for context provision
///
/// This struct holds both WebSocket-based services and REST-based services.
/// WebSocket services use the `CommandBus` for sending commands/requests.
/// REST services still use the generic `A: ApiPort` pattern for file uploads and large payloads.
#[derive(Clone)]
pub struct Services<A: ApiPort> {
    /// Keeps the WebSocket connection alive for the app's lifetime.
    /// This must be stored to prevent the connection from being dropped.
    #[allow(dead_code)]
    connection_keep_alive: ConnectionKeepAlive,
    /// Shared command bus for sending WebSocket commands.
    pub command_bus: CommandBus,
    /// Shared event bus for receiving WebSocket events.
    pub event_bus: crate::infrastructure::messaging::EventBus,
    /// Observe connection state (for UI binding).
    pub state_observer: crate::infrastructure::messaging::ConnectionStateObserver,
    // WebSocket-based services (non-generic)
    pub world: Arc<WorldService>,
    pub character: Arc<CharacterService>,
    pub location: Arc<LocationService>,
    pub player_character: Arc<PlayerCharacterService>,
    pub challenge: Arc<ChallengeService>,
    pub narrative_event: Arc<NarrativeEventService>,
    pub story_event: Arc<StoryEventService>,
    pub event_chain: Arc<EventChainService>,
    pub observation: Arc<ObservationService>,
    pub actantial: Arc<ActantialService>,
    pub skill: Arc<SkillService>,
    pub generation: Arc<GenerationService>,
    pub suggestion: Arc<SuggestionService>,
    pub conversation: Arc<ConversationService>,
    pub character_sheet: Arc<CharacterSheetService>,
    // REST-based services (generic over ApiPort) - file uploads, large payloads, admin config
    pub workflow: Arc<WorkflowService<A>>,
    pub asset: Arc<AssetService<A>>,
    pub settings: Arc<SettingsService<A>>,
}

impl<A: ApiPort + Clone> Services<A> {
    /// Create all services with the given connection and API ports
    ///
    /// # Arguments
    /// * `api` - The REST API port for HTTP-based services
    /// * `raw_api` - The raw API port for services that need lower-level access
    /// * `connection` - The WebSocket connection (handle will be kept alive)
    pub fn new(api: A, raw_api: Arc<dyn RawApiPort>, connection: Connection) -> Self {
        let command_bus = connection.command_bus;
        let event_bus = connection.event_bus;
        let state_observer = connection.state_observer;
        let keep_alive = ConnectionKeepAlive::new(connection.handle);

        Self {
            connection_keep_alive: keep_alive,
            command_bus: command_bus.clone(),
            event_bus,
            state_observer,
            // WebSocket-based services use CommandBus
            world: Arc::new(WorldService::new(command_bus.clone(), raw_api)),
            character: Arc::new(CharacterService::new(command_bus.clone())),
            location: Arc::new(LocationService::new(command_bus.clone())),
            player_character: Arc::new(PlayerCharacterService::new(command_bus.clone())),
            challenge: Arc::new(ChallengeService::new(command_bus.clone())),
            narrative_event: Arc::new(NarrativeEventService::new(command_bus.clone())),
            story_event: Arc::new(StoryEventService::new(command_bus.clone())),
            event_chain: Arc::new(EventChainService::new(command_bus.clone())),
            observation: Arc::new(ObservationService::new(command_bus.clone())),
            actantial: Arc::new(ActantialService::new(command_bus.clone())),
            skill: Arc::new(SkillService::new(command_bus.clone())),
            generation: Arc::new(GenerationService::new(command_bus.clone())),
            suggestion: Arc::new(SuggestionService::new(command_bus.clone())),
            conversation: Arc::new(ConversationService::new(command_bus.clone())),
            character_sheet: Arc::new(CharacterSheetService::new(command_bus.clone())),
            // REST-based services - file uploads, large payloads, admin config
            workflow: Arc::new(WorkflowService::new(api.clone())),
            asset: Arc::new(AssetService::new(api.clone())),
            settings: Arc::new(SettingsService::new(api)),
        }
    }
}

/// Hook to access the shared CommandBus from context
pub fn use_command_bus() -> CommandBus {
    let services = use_context::<UiServices>();
    services.command_bus.clone()
}

/// Hook to access the shared EventBus from context
pub fn use_event_bus() -> crate::infrastructure::messaging::EventBus {
    let services = use_context::<UiServices>();
    services.event_bus.clone()
}

/// Hook to access the connection state observer from context
pub fn use_state_observer() -> crate::infrastructure::messaging::ConnectionStateObserver {
    let services = use_context::<UiServices>();
    services.state_observer.clone()
}

/// Hook to access the WorldService from context
pub fn use_world_service() -> Arc<WorldService> {
    let services = use_context::<UiServices>();
    services.world.clone()
}

/// Hook to access the CharacterService from context
pub fn use_character_service() -> Arc<CharacterService> {
    let services = use_context::<UiServices>();
    services.character.clone()
}

/// Hook to access the LocationService from context
pub fn use_location_service() -> Arc<LocationService> {
    let services = use_context::<UiServices>();
    services.location.clone()
}

/// Hook to access the PlayerCharacterService from context
pub fn use_player_character_service() -> Arc<PlayerCharacterService> {
    let services = use_context::<UiServices>();
    services.player_character.clone()
}

/// Hook to access the SkillService from context
pub fn use_skill_service() -> Arc<SkillService> {
    let services = use_context::<UiServices>();
    services.skill.clone()
}

/// Hook to access the ChallengeService from context
pub fn use_challenge_service() -> Arc<ChallengeService> {
    let services = use_context::<UiServices>();
    services.challenge.clone()
}

/// Hook to access the StoryEventService from context
pub fn use_story_event_service() -> Arc<StoryEventService> {
    let services = use_context::<UiServices>();
    services.story_event.clone()
}

/// Hook to access the NarrativeEventService from context
pub fn use_narrative_event_service() -> Arc<NarrativeEventService> {
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
pub fn use_suggestion_service() -> Arc<SuggestionService> {
    let services = use_context::<UiServices>();
    services.suggestion.clone()
}

/// Hook to access the EventChainService from context
pub fn use_event_chain_service() -> Arc<EventChainService> {
    let services = use_context::<UiServices>();
    services.event_chain.clone()
}

/// Hook to access the GenerationService from context
pub fn use_generation_service() -> Arc<GenerationService> {
    let services = use_context::<UiServices>();
    services.generation.clone()
}

/// Hook to access the SettingsService from context
pub fn use_settings_service() -> Arc<SettingsService<Api>> {
    let services = use_context::<UiServices>();
    services.settings.clone()
}

/// Hook to access the ObservationService from context
pub fn use_observation_service() -> Arc<ObservationService> {
    let services = use_context::<UiServices>();
    services.observation.clone()
}

/// Hook to access the ActantialService from context
pub fn use_actantial_service() -> Arc<ActantialService> {
    let services = use_context::<UiServices>();
    services.actantial.clone()
}

/// Hook to access the CharacterSheetService from context
pub fn use_character_sheet_service() -> Arc<CharacterSheetService> {
    let services = use_context::<UiServices>();
    services.character_sheet.clone()
}

/// Hook to access ConversationService from context
pub fn use_conversation_service() -> Arc<ConversationService> {
    let services = use_context::<UiServices>();
    services.conversation.clone()
}

use crate::ports::outbound::PlatformPort;
use crate::presentation::state::{
    BatchStatus, GenerationBatch, GenerationState, SuggestionStatus, SuggestionTask,
};
use anyhow::Result;

/// Hydrate GenerationState from the Engine's unified generation queue endpoint.
///
/// # Arguments
/// * `generation_service` - The GenerationService to fetch queue state from
/// * `generation_state` - The mutable state to populate
/// * `user_id` - Optional user ID to filter queue items
/// * `world_id` - World ID to scope the queue to
pub async fn hydrate_generation_queue(
    generation_service: &GenerationService,
    generation_state: &mut GenerationState,
    user_id: Option<&str>,
    world_id: &str,
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

        generation_state.add_batch(GenerationBatch {
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

    Ok(())
}

const STORAGE_KEY_GEN_READ_BATCHES: &str = "wrldbldr_gen_read_batches";
const STORAGE_KEY_GEN_READ_SUGGESTIONS: &str = "wrldbldr_gen_read_suggestions";

/// Persist the read/unread state of generation queue items to local storage
pub fn persist_generation_read_state(platform: &dyn PlatformPort, state: &GenerationState) {
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
#[allow(dead_code)]
fn apply_generation_read_state(platform: &dyn PlatformPort, state: &mut GenerationState) {
    if let Some(batch_str) = platform.storage_load(STORAGE_KEY_GEN_READ_BATCHES) {
        for id in batch_str
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
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
pub async fn sync_generation_read_state(
    generation_service: &GenerationService,
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

/// View-model helpers for generation queue filtering and actions.
/// Get visible batches based on show_read filter
pub fn visible_batches(state: &GenerationState, show_read: bool) -> Vec<GenerationBatch> {
    state
        .get_batches()
        .into_iter()
        .filter(|b| show_read || !b.is_read)
        .collect()
}

/// Get visible suggestions based on show_read filter
pub fn visible_suggestions(state: &GenerationState, show_read: bool) -> Vec<SuggestionTask> {
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
pub async fn mark_batch_read_and_sync(
    generation_service: &GenerationService,
    state: &mut GenerationState,
    batch_id: &str,
    world_id: Option<&str>,
    platform: &dyn PlatformPort,
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
pub async fn mark_suggestion_read_and_sync(
    generation_service: &GenerationService,
    state: &mut GenerationState,
    request_id: &str,
    world_id: Option<&str>,
    platform: &dyn PlatformPort,
) -> Result<()> {
    state.mark_suggestion_read(request_id);
    persist_generation_read_state(platform, state);
    sync_generation_read_state(generation_service, state, world_id).await
}
