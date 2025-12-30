//! Application State - Fully abstracted composition root state
//!
//! This module provides the `AppState` struct, which serves as the central composition
//! point for all application services. It uses ONLY `Arc<dyn Trait>` for service references
//! to maintain clean hexagonal architecture boundaries.
//!
//! # Architecture
//!
//! The `AppState` struct lives in the composition layer and:
//! - **Depends only on port traits** from `wrldbldr-engine-ports`
//! - **Groups services** into logical containers (CoreServices, GameServices, etc.)
//! - **Enables dependency injection** at the runner layer
//! - **Supports testing** through mock implementations of all ports
//!
//! ```text
//! Runner (constructs AppState with concrete adapters)
//!    │
//!    ▼
//! AppState (holds Arc<dyn Port> references)
//!    │
//!    ├── CoreServices
//!    ├── GameServices
//!    ├── QueueServices
//!    ├── AssetServices
//!    ├── PlayerServices
//!    ├── EventInfra
//!    └── UseCases
//! ```
//!
//! # Design Principles
//!
//! 1. **No Concrete Types**: All service fields use `Arc<dyn Trait>` except for:
//!    - `config`: A simple data struct with no behavior
//!    - Service container structs (which themselves only hold trait objects)
//!
//! 2. **Single Source of Truth**: AppState is the authoritative composition root
//!    that defines what services are available throughout the application.
//!
//! 3. **Clone-friendly**: All fields are `Clone` (via `Arc`), allowing AppState
//!    to be shared across async tasks and handlers.
//!
//! # Usage
//!
//! ```ignore
//! use wrldbldr_engine_composition::{AppState, AppConfig};
//!
//! // In engine-runner's composition module:
//! let app_state = AppState::new(
//!     config,
//!     llm_client,
//!     comfyui_client,
//!     region_repo,
//!     core_services,
//!     game_services,
//!     queue_services,
//!     asset_services,
//!     player_services,
//!     events,
//!     settings_service,
//!     prompt_template_service,
//!     staging_service,
//!     world_connection_manager,
//!     world_state,
//!     request_handler,
//!     directorial_context_repo,
//!     use_cases,
//!     prompt_context_service,
//! );
//!
//! // Pass to Axum as shared state:
//! let router = Router::new()
//!     .route("/api/...", get(handler))
//!     .with_state(app_state);
//! ```

use std::sync::Arc;

use wrldbldr_engine_ports::inbound::{
    AppStatePort, ChallengeUseCasePort, ConnectionUseCasePort, InventoryUseCasePort,
    MovementUseCasePort, NarrativeEventUseCasePort, ObservationUseCasePort,
    PlayerActionUseCasePort, RequestHandler, SceneUseCasePort, StagingUseCasePort,
};
use wrldbldr_engine_ports::outbound::{
    AssetGenerationQueueServicePort, AssetServicePort, BroadcastPort, ComfyUIPort,
    ConnectionBroadcastPort, ConnectionContextPort, ConnectionLifecyclePort, ConnectionQueryPort,
    DirectorialContextRepositoryPort, DmApprovalQueueServicePort,
    GenerationQueueProjectionServicePort, GenerationReadStatePort, GenerationServicePort,
    LlmPort, LlmQueueServicePort, PlayerActionQueueServicePort, PromptContextServicePort,
    PromptTemplateServicePort, RegionItemPort, SettingsServicePort, StagingServicePort,
    WorkflowServicePort, WorldServicePort, WorldStatePort,
};

/// Type alias for LlmPort with anyhow::Error as the associated error type.
///
/// This allows `LlmPort` to be used as a trait object in `Arc<dyn LlmPortDyn>`.
/// The runner layer wraps concrete LLM implementations to erase their specific
/// error types into `anyhow::Error`.
pub trait LlmPortDyn: Send + Sync {
    /// Generate a response from the LLM
    fn generate(
        &self,
        request: wrldbldr_engine_ports::outbound::LlmRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<wrldbldr_engine_ports::outbound::LlmResponse, anyhow::Error>,
                > + Send
                + '_,
        >,
    >;

    /// Generate a response with tool/function calling support
    fn generate_with_tools(
        &self,
        request: wrldbldr_engine_ports::outbound::LlmRequest,
        tools: Vec<wrldbldr_engine_ports::outbound::ToolDefinition>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<wrldbldr_engine_ports::outbound::LlmResponse, anyhow::Error>,
                > + Send
                + '_,
        >,
    >;
}

/// Blanket implementation to wrap any LlmPort into LlmPortDyn
impl<T: LlmPort> LlmPortDyn for T {
    fn generate(
        &self,
        request: wrldbldr_engine_ports::outbound::LlmRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<wrldbldr_engine_ports::outbound::LlmResponse, anyhow::Error>,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            <Self as LlmPort>::generate(self, request)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        })
    }

    fn generate_with_tools(
        &self,
        request: wrldbldr_engine_ports::outbound::LlmRequest,
        tools: Vec<wrldbldr_engine_ports::outbound::ToolDefinition>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<wrldbldr_engine_ports::outbound::LlmResponse, anyhow::Error>,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            <Self as LlmPort>::generate_with_tools(self, request, tools)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))
        })
    }
}

use crate::{
    AssetServices, CoreServices, EventInfra, GameServices, PlayerServices, QueueServices, UseCases,
};

/// Application configuration.
///
/// This struct holds essential configuration values needed by the application.
/// Unlike service ports, this is a simple data struct with no behavior, so it's
/// acceptable to use concrete types here.
///
/// # Fields
///
/// - `server_host`: The hostname/IP address the server binds to (e.g., "0.0.0.0")
/// - `server_port`: The port number the server listens on (e.g., 8080)
/// - `database_url`: Neo4j connection URL (e.g., "bolt://localhost:7687")
/// - `ollama_url`: Ollama LLM API URL (e.g., "http://localhost:11434")
/// - `comfyui_url`: ComfyUI API URL (e.g., "http://localhost:8188")
///
/// # Example
///
/// ```ignore
/// let config = AppConfig {
///     server_host: "0.0.0.0".to_string(),
///     server_port: 8080,
///     database_url: "bolt://localhost:7687".to_string(),
///     ollama_url: "http://localhost:11434".to_string(),
///     comfyui_url: "http://localhost:8188".to_string(),
/// };
/// ```
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// The hostname or IP address the server binds to.
    ///
    /// Common values: "0.0.0.0" (all interfaces), "127.0.0.1" (localhost only)
    pub server_host: String,

    /// The port number the server listens on.
    ///
    /// Default is typically 8080 for development, may vary in production.
    pub server_port: u16,

    /// Neo4j database connection URL.
    ///
    /// Format: "bolt://host:port" or "neo4j://host:port"
    pub database_url: String,

    /// Ollama LLM API base URL.
    ///
    /// The application appends specific endpoints (e.g., "/api/chat")
    pub ollama_url: String,

    /// ComfyUI API base URL.
    ///
    /// Used for image generation workflows.
    pub comfyui_url: String,
}

impl AppConfig {
    /// Creates a new `AppConfig` with the specified values.
    ///
    /// # Arguments
    ///
    /// * `server_host` - Hostname/IP for server binding
    /// * `server_port` - Port number for server
    /// * `database_url` - Neo4j connection URL
    /// * `ollama_url` - Ollama API URL
    /// * `comfyui_url` - ComfyUI API URL
    pub fn new(
        server_host: String,
        server_port: u16,
        database_url: String,
        ollama_url: String,
        comfyui_url: String,
    ) -> Self {
        Self {
            server_host,
            server_port,
            database_url,
            ollama_url,
            comfyui_url,
        }
    }

    /// Returns the full server address as "host:port".
    ///
    /// Useful for binding the server or displaying the address.
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}

impl Default for AppConfig {
    /// Creates a default configuration suitable for local development.
    fn default() -> Self {
        Self {
            server_host: "0.0.0.0".to_string(),
            server_port: 8080,
            database_url: "bolt://localhost:7687".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            comfyui_url: "http://localhost:8188".to_string(),
        }
    }
}

/// Shared application state - the composition root for all services.
///
/// This struct composes all services needed by the application, using port traits
/// for clean hexagonal architecture. It is designed to be shared across async tasks
/// via `Clone` (all fields are `Arc`-wrapped).
///
/// # Service Organization
///
/// Services are organized into logical groups for better maintainability:
///
/// - **Core Services**: Fundamental domain entities (worlds, characters, locations, etc.)
/// - **Game Services**: Game mechanics (challenges, narrative events, dispositions)
/// - **Queue Services**: Async processing (player actions, LLM requests, asset generation)
/// - **Asset Services**: Asset management and generation (ComfyUI integration)
/// - **Player Services**: Player-facing operations (character sheets, scene resolution)
/// - **Event Infrastructure**: Domain events (event bus, notifications, persistence)
/// - **Use Cases**: High-level operations coordinating multiple services
///
/// # Top-Level Services
///
/// Some services don't fit neatly into groups and are exposed directly:
///
/// - `llm`: LLM API client for chat completions
/// - `comfyui`: ComfyUI client for image generation
/// - `region_repo`: Region repository for entity converters
/// - `settings_service`: Application settings management
/// - `prompt_template_service`: LLM prompt template management
/// - `staging_service`: NPC staging and presence management
/// - `world_connection_manager`: WebSocket connection tracking per world
/// - `world_state`: Per-world game state (time, conversations, approvals)
/// - `request_handler`: WebSocket request routing and handling
/// - `directorial_context_repo`: DM directorial notes persistence
/// - `prompt_context_service`: LLM prompt context building
///
/// # Clone Semantics
///
/// `AppState` implements `Clone` through `Arc` sharing. Cloning is cheap and
/// creates a new handle to the same underlying services. This is the intended
/// way to share state across async tasks and handlers.
#[derive(Clone)]
pub struct AppState {
    /// Application configuration (server addresses, URLs, etc.)
    pub config: AppConfig,

    /// LLM service for chat completions and text generation.
    ///
    /// Used for NPC dialogue, suggestions, and narrative content.
    /// Uses `LlmPortDyn` trait object to erase the associated error type.
    pub llm: Arc<dyn LlmPortDyn>,

    /// ComfyUI service for image generation workflows.
    ///
    /// Used for character portraits, location images, and other visual assets.
    pub comfyui: Arc<dyn ComfyUIPort>,

    /// Region item port for entity converters (fetching region items).
    ///
    /// Uses ISP: Only RegionItemPort needed for region item lookups.
    /// Used by entity conversion utilities (e.g., `converters.rs`).
    pub region_item: Arc<dyn RegionItemPort>,

    /// Core domain services (worlds, characters, locations, scenes, etc.)
    pub core: CoreServices,

    /// Game mechanics services (challenges, narrative events, dispositions, etc.)
    pub game: GameServices,

    /// Queue processing services (player actions, LLM requests, asset generation)
    pub queues: QueueServices,

    /// Asset management and generation services.
    pub assets: AssetServices,

    /// Player-facing services (character sheets, scene resolution).
    pub player: PlayerServices,

    /// Event infrastructure (event bus, notifications, persistence).
    pub events: EventInfra,

    /// Settings service for application configuration.
    ///
    /// Manages runtime settings like LLM configuration, feature flags, etc.
    pub settings_service: Arc<dyn SettingsServicePort>,

    /// Prompt template service for LLM prompts.
    ///
    /// Manages configurable prompt templates with variable substitution.
    pub prompt_template_service: Arc<dyn PromptTemplateServicePort>,

    /// Staging service for NPC presence management.
    ///
    /// Handles NPC staging proposals, approvals, and region presence.
    pub staging_service: Arc<dyn StagingServicePort>,

    /// Connection query port for querying connection state.
    ///
    /// Provides access to DM presence, connected users, roles, and statistics.
    pub connection_query: Arc<dyn ConnectionQueryPort>,

    /// Connection context port for resolving client/connection context.
    ///
    /// Used by WebSocket handlers to build RequestContext from client IDs.
    pub connection_context: Arc<dyn ConnectionContextPort>,

    /// Connection broadcast port for WebSocket message broadcasting.
    ///
    /// Sends serialized messages to world members, DMs, players, etc.
    pub connection_broadcast: Arc<dyn ConnectionBroadcastPort>,

    /// Connection lifecycle port for connection management.
    ///
    /// Handles connection cleanup on disconnect.
    pub connection_lifecycle: Arc<dyn ConnectionLifecyclePort>,

    /// World state manager for per-world game state.
    ///
    /// Provides world-scoped storage for:
    /// - Game time tracking
    /// - Conversation history
    /// - Pending approval queues
    /// - Current scene state
    pub world_state: Arc<dyn WorldStatePort>,

    /// Request handler for WebSocket-first architecture.
    ///
    /// Routes incoming Request payloads to appropriate services and returns
    /// Response payloads. This is the main entry point for WebSocket messages.
    pub request_handler: Arc<dyn RequestHandler>,

    /// Directorial context repository for DM notes persistence.
    ///
    /// Stores directorial context (scene notes, tone, NPC motivations) so it
    /// survives server restarts. Used by DMs to maintain narrative continuity.
    pub directorial_context_repo: Arc<dyn DirectorialContextRepositoryPort>,

    /// Use cases container for high-level operations.
    ///
    /// Contains all use cases that coordinate domain services to fulfill
    /// specific user intents. Called by WebSocket handlers.
    pub use_cases: UseCases,

    /// Prompt context service for building LLM prompts.
    ///
    /// Orchestrates gathering context from world snapshot, conversation history,
    /// challenges, narrative events, etc. to build complete prompt requests.
    pub prompt_context_service: Arc<dyn PromptContextServicePort>,
}

impl AppState {
    /// Creates a new `AppState` with all required services.
    ///
    /// This constructor takes all dependencies as parameters, enabling the runner
    /// layer to wire up concrete implementations while this composition layer
    /// remains decoupled from specific adapters.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    /// * `llm` - LLM service implementation
    /// * `comfyui` - ComfyUI service implementation
    /// * `region_item` - Region item port implementation (ISP)
    /// * `core` - Core domain services container
    /// * `game` - Game mechanics services container
    /// * `queues` - Queue processing services container
    /// * `assets` - Asset services container
    /// * `player` - Player services container
    /// * `events` - Event infrastructure container
    /// * `settings_service` - Settings service implementation
    /// * `prompt_template_service` - Prompt template service implementation
    /// * `staging_service` - Staging service implementation
    /// * `connection_query` - Connection query port implementation
    /// * `connection_context` - Connection context port implementation
    /// * `connection_broadcast` - Connection broadcast port implementation
    /// * `connection_lifecycle` - Connection lifecycle port implementation
    /// * `world_state` - World state manager implementation
    /// * `request_handler` - Request handler implementation
    /// * `directorial_context_repo` - Directorial context repository implementation
    /// * `use_cases` - Use cases container
    /// * `prompt_context_service` - Prompt context service implementation
    ///
    /// # Example
    ///
    /// ```ignore
    /// // In engine-runner composition:
    /// let app_state = AppState::new(
    ///     config,
    ///     Arc::new(ollama_client) as Arc<dyn LlmPortDyn>,
    ///     Arc::new(comfyui_client) as Arc<dyn ComfyUIPort>,
    ///     Arc::new(neo4j_region_repo) as Arc<dyn RegionItemPort>,
    ///     core_services,
    ///     game_services,
    ///     queue_services,
    ///     asset_services,
    ///     player_services,
    ///     event_infra,
    ///     Arc::new(settings_service) as Arc<dyn SettingsServicePort>,
    ///     Arc::new(prompt_template_service) as Arc<dyn PromptTemplateServicePort>,
    ///     Arc::new(staging_service) as Arc<dyn StagingServicePort>,
    ///     connection_query,
    ///     connection_context,
    ///     connection_broadcast,
    ///     connection_lifecycle,
    ///     Arc::new(world_state_manager) as Arc<dyn WorldStatePort>,
    ///     Arc::new(request_handler) as Arc<dyn RequestHandler>,
    ///     Arc::new(directorial_context_repo) as Arc<dyn DirectorialContextRepositoryPort>,
    ///     use_cases,
    ///     Arc::new(prompt_context_service) as Arc<dyn PromptContextServicePort>,
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: AppConfig,
        llm: Arc<dyn LlmPortDyn>,
        comfyui: Arc<dyn ComfyUIPort>,
        region_item: Arc<dyn RegionItemPort>,
        core: CoreServices,
        game: GameServices,
        queues: QueueServices,
        assets: AssetServices,
        player: PlayerServices,
        events: EventInfra,
        settings_service: Arc<dyn SettingsServicePort>,
        prompt_template_service: Arc<dyn PromptTemplateServicePort>,
        staging_service: Arc<dyn StagingServicePort>,
        connection_query: Arc<dyn ConnectionQueryPort>,
        connection_context: Arc<dyn ConnectionContextPort>,
        connection_broadcast: Arc<dyn ConnectionBroadcastPort>,
        connection_lifecycle: Arc<dyn ConnectionLifecyclePort>,
        world_state: Arc<dyn WorldStatePort>,
        request_handler: Arc<dyn RequestHandler>,
        directorial_context_repo: Arc<dyn DirectorialContextRepositoryPort>,
        use_cases: UseCases,
        prompt_context_service: Arc<dyn PromptContextServicePort>,
    ) -> Self {
        Self {
            config,
            llm,
            comfyui,
            region_item,
            core,
            game,
            queues,
            assets,
            player,
            events,
            settings_service,
            prompt_template_service,
            staging_service,
            connection_query,
            connection_context,
            connection_broadcast,
            connection_lifecycle,
            world_state,
            request_handler,
            directorial_context_repo,
            use_cases,
            prompt_context_service,
        }
    }
}

// =============================================================================
// AppStatePort Implementation
// =============================================================================

impl AppStatePort for AppState {
    // Use Cases
    fn movement_use_case(&self) -> Arc<dyn MovementUseCasePort> {
        self.use_cases.movement.clone()
    }

    fn staging_use_case(&self) -> Arc<dyn StagingUseCasePort> {
        self.use_cases.staging.clone()
    }

    fn inventory_use_case(&self) -> Arc<dyn InventoryUseCasePort> {
        self.use_cases.inventory.clone()
    }

    fn player_action_use_case(&self) -> Arc<dyn PlayerActionUseCasePort> {
        self.use_cases.player_action.clone()
    }

    fn observation_use_case(&self) -> Arc<dyn ObservationUseCasePort> {
        self.use_cases.observation.clone()
    }

    fn challenge_use_case(&self) -> Arc<dyn ChallengeUseCasePort> {
        self.use_cases.challenge.clone()
    }

    fn scene_use_case(&self) -> Arc<dyn SceneUseCasePort> {
        self.use_cases.scene.clone()
    }

    fn connection_use_case(&self) -> Arc<dyn ConnectionUseCasePort> {
        self.use_cases.connection.clone()
    }

    fn narrative_event_use_case(&self) -> Arc<dyn NarrativeEventUseCasePort> {
        self.use_cases.narrative_event.clone()
    }

    // Infrastructure Services
    fn broadcast(&self) -> Arc<dyn BroadcastPort> {
        self.use_cases.broadcast.clone()
    }

    fn connection_query(&self) -> Arc<dyn ConnectionQueryPort> {
        self.connection_query.clone()
    }

    fn connection_context(&self) -> Arc<dyn ConnectionContextPort> {
        self.connection_context.clone()
    }

    fn connection_broadcast(&self) -> Arc<dyn ConnectionBroadcastPort> {
        self.connection_broadcast.clone()
    }

    fn connection_lifecycle(&self) -> Arc<dyn ConnectionLifecyclePort> {
        self.connection_lifecycle.clone()
    }

    fn comfyui(&self) -> Arc<dyn ComfyUIPort> {
        self.comfyui.clone()
    }

    fn region_item(&self) -> Arc<dyn RegionItemPort> {
        self.region_item.clone()
    }

    fn settings_service(&self) -> Arc<dyn SettingsServicePort> {
        self.settings_service.clone()
    }

    fn prompt_template_service(&self) -> Arc<dyn PromptTemplateServicePort> {
        self.prompt_template_service.clone()
    }

    // Asset Services
    fn asset_service(&self) -> Arc<dyn AssetServicePort> {
        self.assets.asset_service.clone()
    }

    fn generation_service(&self) -> Arc<dyn GenerationServicePort> {
        self.assets.generation_service.clone()
    }

    fn asset_generation_queue_service(&self) -> Arc<dyn AssetGenerationQueueServicePort> {
        self.queues.asset_generation_queue_service.clone()
    }

    fn workflow_service(&self) -> Arc<dyn WorkflowServicePort> {
        self.assets.workflow_config_service.clone()
    }

    fn generation_queue_projection_service(&self) -> Arc<dyn GenerationQueueProjectionServicePort> {
        self.assets.generation_queue_projection_service.clone()
    }

    // Queue Services
    fn player_action_queue_service(&self) -> Arc<dyn PlayerActionQueueServicePort> {
        self.queues.player_action_queue_service.clone()
    }

    fn llm_queue_service(&self) -> Arc<dyn LlmQueueServicePort> {
        self.queues.llm_queue_service.clone()
    }

    fn dm_approval_queue_service(&self) -> Arc<dyn DmApprovalQueueServicePort> {
        self.queues.dm_approval_queue_service.clone()
    }

    // Event Infrastructure
    fn generation_read_state(&self) -> Arc<dyn GenerationReadStatePort> {
        self.events.generation_read_state_repository.clone()
    }

    // World Services
    fn world_service(&self) -> Arc<dyn WorldServicePort> {
        self.core.world_service.clone()
    }

    // Request Handling
    fn request_handler(&self) -> Arc<dyn RequestHandler> {
        self.request_handler.clone()
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .field("llm", &"Arc<dyn LlmPortDyn>")
            .field("comfyui", &"Arc<dyn ComfyUIPort>")
            .field("region_item", &"Arc<dyn RegionItemPort>")
            .field("core", &self.core)
            .field("game", &"GameServices")
            .field("queues", &self.queues)
            .field("assets", &self.assets)
            .field("player", &self.player)
            .field("events", &self.events)
            .field("settings_service", &"Arc<dyn SettingsServicePort>")
            .field(
                "prompt_template_service",
                &"Arc<dyn PromptTemplateServicePort>",
            )
            .field("staging_service", &"Arc<dyn StagingServicePort>")
            .field("connection_query", &"Arc<dyn ConnectionQueryPort>")
            .field("connection_context", &"Arc<dyn ConnectionContextPort>")
            .field("connection_broadcast", &"Arc<dyn ConnectionBroadcastPort>")
            .field("connection_lifecycle", &"Arc<dyn ConnectionLifecyclePort>")
            .field("world_state", &"Arc<dyn WorldStatePort>")
            .field("request_handler", &"Arc<dyn RequestHandler>")
            .field(
                "directorial_context_repo",
                &"Arc<dyn DirectorialContextRepositoryPort>",
            )
            .field("use_cases", &self.use_cases)
            .field(
                "prompt_context_service",
                &"Arc<dyn PromptContextServicePort>",
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_new() {
        let config = AppConfig::new(
            "127.0.0.1".to_string(),
            3000,
            "bolt://localhost:7687".to_string(),
            "http://localhost:11434".to_string(),
            "http://localhost:8188".to_string(),
        );

        assert_eq!(config.server_host, "127.0.0.1");
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.database_url, "bolt://localhost:7687");
        assert_eq!(config.ollama_url, "http://localhost:11434");
        assert_eq!(config.comfyui_url, "http://localhost:8188");
    }

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();

        assert_eq!(config.server_host, "0.0.0.0");
        assert_eq!(config.server_port, 8080);
        assert_eq!(config.database_url, "bolt://localhost:7687");
        assert_eq!(config.ollama_url, "http://localhost:11434");
        assert_eq!(config.comfyui_url, "http://localhost:8188");
    }

    #[test]
    fn test_app_config_server_address() {
        let config = AppConfig::new(
            "192.168.1.100".to_string(),
            9000,
            "bolt://db:7687".to_string(),
            "http://ollama:11434".to_string(),
            "http://comfyui:8188".to_string(),
        );

        assert_eq!(config.server_address(), "192.168.1.100:9000");
    }

    #[test]
    fn test_app_config_clone() {
        let config = AppConfig::default();
        let cloned = config.clone();

        assert_eq!(config.server_host, cloned.server_host);
        assert_eq!(config.server_port, cloned.server_port);
    }

    #[test]
    fn test_app_config_debug() {
        let config = AppConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("AppConfig"));
        assert!(debug_str.contains("server_host"));
        assert!(debug_str.contains("server_port"));
    }
}
