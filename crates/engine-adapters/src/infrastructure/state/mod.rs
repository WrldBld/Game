//! Shared application state
//!
//! This module provides a modular application state structure that composes
//! several sub-structures for better organization and maintainability.
//!
//! Note: The AppState struct is defined here with its field types, but the
//! construction logic (`new()`) has been moved to engine-runner/composition/app_state.rs
//! to maintain proper hexagonal architecture (composition root in runner crate).

mod asset_services;
mod core_services;
mod event_infra;
mod game_services;
mod player_services;
mod queue_services;
mod use_cases;

pub use asset_services::AssetServices;
pub use core_services::CoreServices;
pub use event_infra::EventInfrastructure;
pub use game_services::GameServices;
pub use player_services::PlayerServices;
pub use queue_services::QueueServices;
pub use use_cases::UseCases;

use std::sync::Arc;

use crate::infrastructure::comfyui::ComfyUIClient;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::ollama::OllamaClient;
use crate::infrastructure::persistence::{
    Neo4jNarrativeEventRepository, Neo4jRegionRepository, Neo4jRepository, Neo4jStagingRepository,
};

use crate::infrastructure::world_connection_manager::SharedWorldConnectionManager;
use crate::infrastructure::WorldStateManager;

use wrldbldr_engine_app::application::services::{
    staging_service::StagingService, PromptTemplateService, SettingsService,
};
use wrldbldr_engine_ports::inbound::RequestHandler;

/// Shared application state
///
/// This struct composes several sub-structures that group related services
/// for better organization and maintainability.
///
/// **Note**: Construction of AppState is handled by the composition root
/// in engine-runner. This module only defines the struct shape.
pub struct AppState {
    pub config: AppConfig,
    /// Neo4j repository - direct access for specialized operations
    ///
    /// While most data access should go through service layers, some operations
    /// (like region management) may need direct repository access.
    pub repository: Neo4jRepository,
    pub llm_client: OllamaClient,
    pub comfyui_client: ComfyUIClient,

    // Grouped services
    pub core: CoreServices,
    pub game: GameServices<OllamaClient>,
    pub queues: QueueServices,
    pub assets: AssetServices,
    pub player: PlayerServices,
    pub events: EventInfrastructure,
    pub settings_service: Arc<SettingsService>,
    /// Prompt template service for configurable LLM prompts
    pub prompt_template_service: Arc<PromptTemplateService>,
    /// Staging service for NPC presence management
    pub staging_service: Arc<
        StagingService<
            OllamaClient,
            Neo4jRegionRepository,
            Neo4jNarrativeEventRepository,
            Neo4jStagingRepository,
        >,
    >,

    /// World connection manager for WebSocket-first architecture
    ///
    /// Manages world-scoped connections, replacing session-based model.
    /// Handles JoinWorld/LeaveWorld, role enforcement, and connection tracking.
    pub world_connection_manager: SharedWorldConnectionManager,

    /// World state manager for per-world state (game time, conversation, approvals)
    ///
    /// Provides world-scoped storage for game time, conversation history,
    /// pending approvals, and current scene state.
    pub world_state: Arc<WorldStateManager>,

    /// Request handler for WebSocket-first architecture
    ///
    /// Handles all Request payloads, routing them to appropriate services.
    pub request_handler: Arc<dyn RequestHandler>,

    /// Directorial context repository for persisting DM notes
    ///
    /// Stores directorial context (scene notes, tone, NPC motivations)
    /// so it survives server restarts.
    pub directorial_context_repo:
        Arc<dyn wrldbldr_engine_ports::outbound::DirectorialContextRepositoryPort>,

    /// Use cases for WebSocket handlers
    ///
    /// Container for all use cases that coordinate domain services to fulfill
    /// specific user intents. Use cases are called by WebSocket handlers and
    /// return domain result types.
    ///
    /// Note: This is a partial implementation (Phase 4.1). Full wiring of all
    /// use cases requires creating adapters for port traits defined in each use case.
    pub use_cases: UseCases,
}
