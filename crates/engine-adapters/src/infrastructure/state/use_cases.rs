//! Use Case Container
//!
//! Container for all use cases used by WebSocket handlers.
//! Use cases are constructed with their port dependencies during AppState initialization.
//!
//! # Phase 4 Status
//!
//! This is a partial implementation for Phase 4.1. Full wiring of all use cases
//! requires creating adapters for each port trait defined in the use cases.
//!
//! Current status:
//! - [x] Structure defined
//! - [ ] MovementUseCase ports implemented
//! - [ ] StagingApprovalUseCase ports implemented
//! - [ ] Other use cases wired
//!
//! # Architecture
//!
//! ```text
//! WebSocket Handler
//!       │
//!       ▼
//! AppState.use_cases.movement.move_to_region(...)
//!       │
//!       ├──> StagingStatePort (→ StagingStateAdapter)
//!       ├──> StagingServicePort (→ StagingServiceAdapter)
//!       └──> BroadcastPort (→ WebSocketBroadcastAdapter)
//! ```

use std::sync::Arc;

use wrldbldr_engine_app::application::use_cases::{
    // Currently we can't instantiate these without adapters for their port traits
    // MovementUseCase, StagingApprovalUseCase, etc.
};
use wrldbldr_engine_ports::outbound::BroadcastPort;

use crate::infrastructure::websocket::WebSocketBroadcastAdapter;
use crate::infrastructure::world_connection_manager::SharedWorldConnectionManager;

/// Container for all use cases
///
/// Use cases coordinate domain services to fulfill specific user intents.
/// They are called by WebSocket handlers and return domain result types.
///
/// # Note
///
/// This is a stub implementation for Phase 4.1. Full use case wiring requires
/// creating adapter implementations for the port traits defined in each use case:
///
/// - `StagingStatePort` - wraps WorldStateManager
/// - `StagingServicePort` - wraps StagingService
/// - `ChallengeResolutionPort` - wraps ChallengeResolutionService
/// - etc.
///
/// Until those adapters are created, handlers continue using the existing
/// service-based approach while the infrastructure is incrementally migrated.
pub struct UseCases {
    /// Broadcast adapter for all use cases to share
    pub broadcast: Arc<dyn BroadcastPort>,
    
    // Future: Add use case instances here as adapters are created
    // pub movement: Arc<MovementUseCase>,
    // pub staging: Arc<StagingApprovalUseCase>,
    // pub inventory: Arc<InventoryUseCase>,
    // pub challenge: Arc<ChallengeUseCase>,
    // pub observation: Arc<ObservationUseCase>,
    // pub scene: Arc<SceneUseCase>,
    // pub connection: Arc<ConnectionUseCase>,
    // pub player_action: Arc<PlayerActionUseCase>,
}

impl UseCases {
    /// Create a new UseCases container with the broadcast adapter
    ///
    /// # Arguments
    ///
    /// * `connection_manager` - WorldConnectionManager for broadcast routing
    pub fn new(connection_manager: SharedWorldConnectionManager) -> Self {
        let broadcast: Arc<dyn BroadcastPort> = Arc::new(WebSocketBroadcastAdapter::new(connection_manager));

        Self {
            broadcast,
            // Future: Construct use cases with their port adapters
        }
    }
    
    /// Get a reference to the broadcast port
    ///
    /// This allows use cases and services to broadcast events without
    /// needing a direct reference to the WebSocketBroadcastAdapter.
    pub fn broadcast(&self) -> &Arc<dyn BroadcastPort> {
        &self.broadcast
    }
}

#[cfg(test)]
mod tests {
    // Tests will be added as use cases are wired in
}
