//! Adapter State - Infrastructure-layer state extension
//!
//! This module provides `AdapterState`, which extends the composition-layer `AppState`
//! with infrastructure-specific concrete types that adapter handlers need direct access to.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────────┐
//! │                           engine-runner                                     │
//! │  Creates both AppState (composition) and AdapterState (infrastructure)      │
//! └────────────────────────────────────────────────────────────────────┬───────┘
//!                                                  │
//!          ┌───────────────────────────────────────┼───────────────────────────┐
//!          │                                       │                           │
//!          ▼                                       ▼                           ▼
//! ┌─────────────────────┐            ┌──────────────────────┐       ┌─────────────────┐
//! │   AdapterState      │            │      AppState        │       │   engine-app    │
//! │  (infrastructure)   │───────────►│   (composition)      │◄──────│   (use cases)   │
//! │                     │  contains  │   Arc<dyn Port>      │ uses  │                 │
//! │ - config            │            │   for all services   │       │                 │
//! │ - connection_manager│            └──────────────────────┘       └─────────────────┘
//! │ - comfyui_client    │
//! │ - region_repo       │
//! └─────────────────────┘
//! ```
//!
//! # Design Principles
//!
//! 1. **Composition over inheritance**: `AdapterState` contains `AppState`, not extends it
//! 2. **Layer separation**: Infrastructure concerns stay in adapters layer
//! 3. **Port access**: Use cases access services via `state.app.*` (all `Arc<dyn Port>`)
//! 4. **Infrastructure access**: Handlers access concrete types directly on `AdapterState`
//!
//! # Usage
//!
//! ```ignore
//! // In WebSocket handlers (adapter layer):
//! async fn handle_message(state: &AdapterState, client_id: Uuid) {
//!     // Infrastructure access - direct concrete type
//!     let conn = state.connection_manager.get_connection_by_client_id(&client_id.to_string()).await;
//!     
//!     // Infrastructure config access
//!     let cors_origins = &state.config.cors_allowed_origins;
//!     let queue_config = &state.config.queue;
//!     
//!     // App-layer access - via ports
//!     let world = state.app.core.world_service.get_world(world_id).await?;
//! }
//!
//! // In use cases (app layer):
//! async fn execute(&self, app_state: &AppState) {
//!     // Only port access available
//!     let world = app_state.core.world_service.get_world(world_id).await?;
//! }
//! ```

use std::sync::Arc;

use wrldbldr_engine_composition::AppState;
use wrldbldr_engine_ports::outbound::RegionRepositoryPort;

use crate::infrastructure::comfyui::ComfyUIClient;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::world_connection_manager::SharedWorldConnectionManager;

/// Adapter-layer state that extends AppState with infrastructure-specific types.
///
/// This struct is used by infrastructure handlers (WebSocket, HTTP) that need
/// access to both:
/// - **App-layer services** via `self.app.*` (all `Arc<dyn Port>`)
/// - **Infrastructure types** via direct fields (concrete types)
///
/// # Infrastructure Fields
///
/// ## `config`
/// Full adapter-layer configuration including:
/// - Queue configuration (`config.queue.*`) - worker batch sizes, timeouts, backend
/// - CORS configuration (`config.cors_allowed_origins`) - allowed origins list
/// - Database/service URLs - Neo4j, Ollama, ComfyUI
///
/// Note: `app.config` has a minimal subset for composition layer; use `config`
/// for infrastructure-specific settings like queue and CORS.
///
/// ## `connection_manager`
/// WebSocket connection tracking and management. Provides:
/// - `get_connection_by_client_id()` - Look up connection info
/// - `unregister_connection()` - Remove disconnected clients
/// - `broadcast_to_world()` - Send messages to all connections in a world
/// - `broadcast_to_dms()` - Send messages to DM connections
///
/// ## `comfyui_client`
/// Direct HTTP client for ComfyUI integration. Used for:
/// - Health checks (`health_check()`)
/// - Workflow testing in HTTP routes
///
/// ## `region_repo`
/// Region repository for entity converters. Needed by `converters.rs` to:
/// - Fetch region items when converting scene data
///
/// # Clone Semantics
///
/// `AdapterState` is cheaply cloneable via `Arc` sharing. All fields are
/// either `Arc`-wrapped or contain only `Arc` fields internally.
#[derive(Clone)]
pub struct AdapterState {
    /// Composition-layer application state with all services as port traits.
    ///
    /// Use this to access services in a hexagonal-compliant way:
    /// - `app.core.world_service`
    /// - `app.game.challenge_service`
    /// - `app.use_cases.movement`
    /// - etc.
    pub app: AppState,

    /// Full adapter-layer configuration.
    ///
    /// Contains infrastructure-specific settings that the composition layer
    /// doesn't need, such as:
    /// - `queue` - Queue worker configuration (batch sizes, timeouts, backend)
    /// - `cors_allowed_origins` - CORS allowed origins for HTTP server
    /// - `session` - Session configuration (conversation history limits)
    ///
    /// Use this for infrastructure concerns in server.rs and HTTP handlers.
    /// For basic server settings, you can also use `app.config`.
    pub config: AppConfig,

    /// WebSocket connection manager for world-scoped connections.
    ///
    /// Manages the mapping between WebSocket connections and worlds,
    /// handles JoinWorld/LeaveWorld lifecycle, and provides message
    /// broadcasting to connected clients.
    ///
    /// # Infrastructure Methods
    /// - `get_connection_by_client_id(&str) -> Option<ConnectionInfo>`
    /// - `unregister_connection(Uuid) -> Option<ConnectionInfo>`
    /// - `broadcast_to_world(Uuid, ServerMessage)`
    /// - `broadcast_to_dms(Uuid, ServerMessage)`
    /// - `broadcast_to_players(Uuid, ServerMessage)`
    pub connection_manager: SharedWorldConnectionManager,

    /// ComfyUI client for direct health checks and workflow testing.
    ///
    /// While asset generation goes through the queue service, some
    /// HTTP handlers need direct access for:
    /// - Health check endpoint
    /// - Workflow testing before saving
    pub comfyui_client: ComfyUIClient,

    /// Region repository for entity converters.
    ///
    /// The `converters.rs` module needs to fetch region items when
    /// converting scene data to protocol format. This provides direct
    /// repository access for that purpose.
    pub region_repo: Arc<dyn RegionRepositoryPort>,
}

impl AdapterState {
    /// Creates a new `AdapterState` by composing an `AppState` with infrastructure types.
    ///
    /// # Arguments
    ///
    /// * `app` - The composition-layer AppState with all services as port traits
    /// * `config` - Full adapter-layer configuration (queue, CORS, etc.)
    /// * `connection_manager` - WebSocket connection manager
    /// * `comfyui_client` - ComfyUI HTTP client
    /// * `region_repo` - Region repository for entity conversion
    ///
    /// # Example
    ///
    /// ```ignore
    /// // In engine-runner composition:
    /// let config = AppConfig::from_env()?;
    /// let app_state = AppState::new(/* ... */);
    /// let adapter_state = AdapterState::new(
    ///     app_state,
    ///     config,
    ///     connection_manager,
    ///     comfyui_client,
    ///     Arc::new(region_repo) as Arc<dyn RegionRepositoryPort>,
    /// );
    /// ```
    pub fn new(
        app: AppState,
        config: AppConfig,
        connection_manager: SharedWorldConnectionManager,
        comfyui_client: ComfyUIClient,
        region_repo: Arc<dyn RegionRepositoryPort>,
    ) -> Self {
        Self {
            app,
            config,
            connection_manager,
            comfyui_client,
            region_repo,
        }
    }
}

impl std::fmt::Debug for AdapterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterState")
            .field("app", &self.app)
            .field("config", &self.config)
            .field("connection_manager", &"SharedWorldConnectionManager")
            .field("comfyui_client", &"ComfyUIClient")
            .field("region_repo", &"Arc<dyn RegionRepositoryPort>")
            .finish()
    }
}
