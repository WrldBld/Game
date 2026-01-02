//! Application State Port - Interface for accessing application services
//!
//! This port provides a clean abstraction over the application's service composition,
//! enabling adapter-layer handlers to access services without depending on the
//! concrete `AppState` type from the composition layer.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         ADAPTER LAYER                                    │
//! │                                                                          │
//! │  HTTP/WebSocket handlers need access to services:                        │
//! │  - Use cases (movement, inventory, challenge, etc.)                      │
//! │  - Infrastructure services (connection manager, settings, etc.)          │
//! │  - Request handler for CRUD operations                                   │
//! │                                                                          │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//!                   ┌────────────▼─────────────┐
//!                   │     AppStatePort         │ (trait defined here)
//!                   └────────────┬─────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                       COMPOSITION LAYER                                  │
//! │                                                                          │
//! │  AppState implements AppStatePort                                        │
//! │  - Holds Arc<dyn Port> for all services                                 │
//! │  - Provides concrete implementations via getters                         │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Design Principles
//!
//! 1. **Dependency Inversion**: Adapters depend on this trait, not concrete AppState
//! 2. **Minimal Surface**: Only expose what handlers actually need
//! 3. **Port-based Returns**: All getters return `Arc<dyn Port>` types
//! 4. **No Concrete Types**: This trait knows nothing about composition internals

use std::sync::Arc;

use crate::inbound::{
    AssetGenerationQueueUseCasePort, AssetUseCasePort, ChallengeUseCasePort, ConnectionUseCasePort,
    DmApprovalQueueUseCasePort, GenerationQueueProjectionUseCasePort, GenerationUseCasePort,
    InventoryUseCasePort, LlmQueueUseCasePort, MovementUseCasePort, NarrativeEventUseCasePort,
    ObservationUseCasePort, PlayerActionQueueUseCasePort, PlayerActionUseCasePort,
    PromptTemplateUseCasePort, RequestHandler, SceneUseCasePort, SettingsUseCasePort,
    StagingUseCasePort, WorkflowUseCasePort, WorldUseCasePort,
};
use crate::outbound::{
    BroadcastPort, ClockPort, ComfyUIPort, ConnectionBroadcastPort, ConnectionContextPort,
    ConnectionLifecyclePort, ConnectionQueryPort, GenerationReadStatePort, RegionItemPort,
    WorldApprovalPort, WorldConversationPort, WorldDirectorialPort, WorldLifecyclePort,
    WorldScenePort, WorldTimePort,
};

/// Port for accessing application services from adapter handlers.
///
/// This trait abstracts the composition layer's `AppState`, allowing handlers
/// in the adapter layer to access services without a direct dependency on
/// the composition crate.
///
/// # Implementation
///
/// The composition layer's `AppState` struct implements this trait, providing
/// access to all its internal `Arc<dyn Port>` fields.
///
/// # Usage
///
/// ```ignore
/// // In a WebSocket handler (adapter layer):
/// async fn handle_movement(
///     state: &dyn AppStatePort,
///     client_id: Uuid,
///     region_id: Uuid,
/// ) -> Option<ServerMessage> {
///     let ctx = build_context(state.world_connection_manager(), client_id).await?;
///     state.movement_use_case().move_to_region(ctx, region_id).await
/// }
/// ```
pub trait AppStatePort: Send + Sync {
    // =========================================================================
    // Use Cases - High-level operations coordinating domain services
    // =========================================================================

    /// Get the movement use case for PC movement between regions/locations
    fn movement_use_case(&self) -> Arc<dyn MovementUseCasePort>;

    /// Get the staging use case for DM staging operations
    fn staging_use_case(&self) -> Arc<dyn StagingUseCasePort>;

    /// Get the inventory use case for item operations
    fn inventory_use_case(&self) -> Arc<dyn InventoryUseCasePort>;

    /// Get the player action use case for travel and queued actions
    fn player_action_use_case(&self) -> Arc<dyn PlayerActionUseCasePort>;

    /// Get the observation use case for NPC observation and events
    fn observation_use_case(&self) -> Arc<dyn ObservationUseCasePort>;

    /// Get the challenge use case for dice rolls and challenges
    fn challenge_use_case(&self) -> Arc<dyn ChallengeUseCasePort>;

    /// Get the scene use case for scene management
    fn scene_use_case(&self) -> Arc<dyn SceneUseCasePort>;

    /// Get the connection use case for join/leave world operations
    fn connection_use_case(&self) -> Arc<dyn ConnectionUseCasePort>;

    /// Get the narrative event use case for DM approval workflow
    fn narrative_event_use_case(&self) -> Arc<dyn NarrativeEventUseCasePort>;

    // =========================================================================
    // Infrastructure Services
    // =========================================================================

    /// Get the broadcast port for sending events to clients
    fn broadcast(&self) -> Arc<dyn BroadcastPort>;

    /// Get the connection query port for querying connection state
    fn connection_query(&self) -> Arc<dyn ConnectionQueryPort>;

    /// Get the connection context port for resolving client context
    fn connection_context(&self) -> Arc<dyn ConnectionContextPort>;

    /// Get the connection broadcast port for WebSocket message broadcasting
    fn connection_broadcast(&self) -> Arc<dyn ConnectionBroadcastPort>;

    /// Get the connection lifecycle port for connection management
    fn connection_lifecycle(&self) -> Arc<dyn ConnectionLifecyclePort>;

    /// Get the ComfyUI port for image generation and health checks
    fn comfyui(&self) -> Arc<dyn ComfyUIPort>;

    /// Get the region item port for entity conversion (fetching region items)
    fn region_item(&self) -> Arc<dyn RegionItemPort>;

    /// Get the settings use case for runtime configuration
    fn settings_use_case(&self) -> Arc<dyn SettingsUseCasePort>;

    /// Get the prompt template use case for LLM prompts
    fn prompt_template_use_case(&self) -> Arc<dyn PromptTemplateUseCasePort>;

    // =========================================================================
    // Asset Use Cases
    // =========================================================================

    /// Get the asset use case for gallery asset operations
    fn asset_use_case(&self) -> Arc<dyn AssetUseCasePort>;

    /// Get the generation use case for asset generation operations
    fn generation_use_case(&self) -> Arc<dyn GenerationUseCasePort>;

    /// Get the asset generation queue use case for ComfyUI queue operations
    fn asset_generation_queue_use_case(&self) -> Arc<dyn AssetGenerationQueueUseCasePort>;

    /// Get the workflow use case for workflow configuration operations
    fn workflow_use_case(&self) -> Arc<dyn WorkflowUseCasePort>;

    /// Get the generation queue projection use case for queue state views
    fn generation_queue_projection_use_case(&self)
        -> Arc<dyn GenerationQueueProjectionUseCasePort>;

    // =========================================================================
    // Queue Use Cases
    // =========================================================================

    /// Get the player action queue use case for player action processing
    fn player_action_queue_use_case(&self) -> Arc<dyn PlayerActionQueueUseCasePort>;

    /// Get the LLM queue use case for LLM request processing
    fn llm_queue_use_case(&self) -> Arc<dyn LlmQueueUseCasePort>;

    /// Get the DM approval queue use case for approval workflow
    fn dm_approval_queue_use_case(&self) -> Arc<dyn DmApprovalQueueUseCasePort>;

    // =========================================================================
    // Event Infrastructure
    // =========================================================================

    /// Get the generation read state port for tracking read/unread status
    fn generation_read_state(&self) -> Arc<dyn GenerationReadStatePort>;

    // =========================================================================
    // Request Handling
    // =========================================================================

    /// Get the request handler for CRUD operations
    fn request_handler(&self) -> Arc<dyn RequestHandler>;

    // =========================================================================
    // World Use Cases
    // =========================================================================

    /// Get the world use case for world operations (export, query, etc.)
    fn world_use_case(&self) -> Arc<dyn WorldUseCasePort>;

    // =========================================================================
    // World State Ports (ISP-compliant sub-traits)
    // =========================================================================

    /// Get the world time port for game time management
    fn world_time(&self) -> Arc<dyn WorldTimePort>;

    /// Get the world conversation port for conversation history
    fn world_conversation(&self) -> Arc<dyn WorldConversationPort>;

    /// Get the world approval port for pending DM approvals
    fn world_approval(&self) -> Arc<dyn WorldApprovalPort>;

    /// Get the world scene port for current scene tracking
    fn world_scene(&self) -> Arc<dyn WorldScenePort>;

    /// Get the world directorial port for DM directorial context
    fn world_directorial(&self) -> Arc<dyn WorldDirectorialPort>;

    /// Get the world lifecycle port for world initialization/cleanup
    fn world_lifecycle(&self) -> Arc<dyn WorldLifecyclePort>;

    // =========================================================================
    // Infrastructure Ports
    // =========================================================================

    /// Get the clock port for time operations
    fn clock(&self) -> Arc<dyn ClockPort>;
}
