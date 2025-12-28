//! Use Case Container
//!
//! Container for all use cases used by WebSocket handlers.
//! Use cases are constructed with their port dependencies during AppState initialization.
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
//!
//! # Implementation Status
//!
//! - [x] MovementUseCase - PC movement between regions/locations
//! - [x] StagingApprovalUseCase - DM staging approval, regeneration, pre-staging
//! - [x] InventoryUseCase - Item management
//! - [ ] ChallengeUseCase - Challenge resolution (needs adapters for local ports)
//! - [ ] ObservationUseCase - NPC observation events (needs adapters for local ports)
//! - [ ] SceneUseCase - Scene management (needs adapters for local ports)
//! - [ ] ConnectionUseCase - World connection management (needs adapters for local ports)
//! - [ ] PlayerActionUseCase - Player action handling (needs adapters for local ports)

use std::sync::Arc;

use wrldbldr_engine_app::application::services::staging_service::StagingService;
use wrldbldr_engine_app::application::use_cases::{
    InventoryUseCase, MovementUseCase, SceneBuilder, StagingApprovalUseCase,
};
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, CharacterRepositoryPort, LlmPort, LocationRepositoryPort,
    NarrativeEventRepositoryPort, PlayerCharacterRepositoryPort, RegionRepositoryPort,
    StagingRepositoryPort,
};

use crate::infrastructure::ports::{StagingServiceAdapter, StagingStateAdapter};
use crate::infrastructure::websocket::WebSocketBroadcastAdapter;
use crate::infrastructure::world_connection_manager::SharedWorldConnectionManager;
use crate::infrastructure::WorldStateManager;

/// Container for all use cases
///
/// Use cases coordinate domain services to fulfill specific user intents.
/// They are called by WebSocket handlers and return domain result types.
pub struct UseCases {
    /// Broadcast adapter for all use cases to share
    pub broadcast: Arc<dyn BroadcastPort>,

    /// Movement use case for PC movement between regions/locations
    pub movement: Arc<MovementUseCase>,

    /// Staging approval use case for DM staging operations
    pub staging: Arc<StagingApprovalUseCase>,

    /// Inventory use case for item equip/unequip/drop/pickup
    pub inventory: Arc<InventoryUseCase>,

    // Future: Add other use case instances as their port adapters are created
    // These use cases define their own port traits locally and need adapters:
    // pub challenge: Arc<ChallengeUseCase>,     // needs: ChallengeResolutionPort, ChallengeOutcomeApprovalPort, DmApprovalQueuePort
    // pub observation: Arc<ObservationUseCase>, // needs: ObservationRepositoryPort, WorldMessagePort
    // pub scene: Arc<SceneUseCase>,             // needs: SceneServicePort, InteractionServicePort, WorldStatePort, DirectorialContextRepositoryPort, DmActionQueuePort
    // pub connection: Arc<ConnectionUseCase>,   // needs: ConnectionManagerPort, WorldServicePort, PlayerCharacterServicePort, DirectorialContextPort, WorldStatePort
    // pub player_action: Arc<PlayerActionUseCase>, // needs: PlayerActionQueuePort, DmNotificationPort (depends on MovementUseCase)
}

impl UseCases {
    /// Create a new UseCases container with all use cases wired
    ///
    /// # Arguments
    ///
    /// * `connection_manager` - WorldConnectionManager for broadcast routing
    /// * `world_state` - WorldStateManager for staging state
    /// * `pc_repo` - Player character repository
    /// * `region_repo` - Region repository
    /// * `location_repo` - Location repository
    /// * `character_repo` - Character repository (for StagingApprovalUseCase)
    /// * `staging_service` - The staging service (generic over its dependencies)
    pub fn new<L, R, N, S>(
        connection_manager: SharedWorldConnectionManager,
        world_state: Arc<WorldStateManager>,
        pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
        region_repo: Arc<dyn RegionRepositoryPort>,
        location_repo: Arc<dyn LocationRepositoryPort>,
        character_repo: Arc<dyn CharacterRepositoryPort>,
        staging_service: Arc<StagingService<L, R, N, S>>,
    ) -> Self
    where
        L: LlmPort + Send + Sync + 'static,
        R: RegionRepositoryPort + Send + Sync + 'static,
        N: NarrativeEventRepositoryPort + Send + Sync + 'static,
        S: StagingRepositoryPort + Send + Sync + 'static,
    {
        // Create broadcast adapter
        let broadcast: Arc<dyn BroadcastPort> =
            Arc::new(WebSocketBroadcastAdapter::new(connection_manager));

        // Create staging adapters
        // Note: StagingStateAdapter implements both StagingStatePort and StagingStateExtPort
        // Note: StagingServiceAdapter implements both StagingServicePort and StagingServiceExtPort
        let staging_state_adapter = Arc::new(StagingStateAdapter::new(world_state));
        let staging_service_adapter = Arc::new(StagingServiceAdapter::new(staging_service));

        // Create shared scene builder
        let scene_builder = Arc::new(SceneBuilder::new(
            region_repo.clone(),
            location_repo.clone(),
        ));

        // Create movement use case
        let movement = Arc::new(MovementUseCase::new(
            pc_repo.clone(),
            region_repo.clone(),
            location_repo.clone(),
            staging_service_adapter.clone(),
            staging_state_adapter.clone(),
            broadcast.clone(),
            scene_builder.clone(),
        ));

        // Create inventory use case
        let inventory = Arc::new(InventoryUseCase::new(
            pc_repo,
            region_repo.clone(),
            broadcast.clone(),
        ));

        // Create staging approval use case
        let staging = Arc::new(StagingApprovalUseCase::new(
            staging_service_adapter,
            staging_state_adapter,
            character_repo,
            region_repo,
            location_repo,
            broadcast.clone(),
            scene_builder,
        ));

        Self {
            broadcast,
            movement,
            staging,
            inventory,
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
    // Tests will be added as more use cases are wired in
}
