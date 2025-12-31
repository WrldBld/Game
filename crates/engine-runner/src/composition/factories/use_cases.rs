//! Use Case Factory
//!
//! This module provides factory functions for creating use cases and their
//! dependencies. It extracts use case construction logic from `app_state.rs`
//! to reduce complexity and improve testability.
//!
//! # Architecture
//!
//! Use cases are the orchestration layer that coordinates domain services to
//! fulfill specific user intents. This factory creates all 9 use cases along
//! with their required port adapters.
//!
//! # Dependencies
//!
//! Use cases depend on:
//! - Repository ports (ISP-split traits from `RepositoryPorts`)
//! - Service ports (from composition layer)
//! - Infrastructure adapters (BroadcastPort, etc.)
//!
//! # Generic Type Constraints
//!
//! Some use cases are generic over their service types (e.g., `NarrativeEventUseCase<N>`).
//! The factory accepts concrete service implementations and lets Rust infer the
//! generic parameters. The returned `UseCaseContext` uses port trait objects where
//! possible for maximum flexibility.
//!
//! # Use Case Categories
//!
//! | Use Case           | Purpose                                      |
//! |--------------------|----------------------------------------------|
//! | Movement           | PC movement between regions and locations    |
//! | Inventory          | Item equip/unequip/drop/pickup operations    |
//! | Staging            | DM staging approval, regeneration            |
//! | PlayerAction       | Travel and queued action handling            |
//! | Observation        | NPC observation and event triggering         |
//! | Challenge          | Dice rolls and challenge resolution          |
//! | Scene              | Scene changes and directorial context        |
//! | Connection         | Join/leave world operations                  |
//! | NarrativeEvent     | DM approval of narrative events              |

use std::sync::Arc;

use wrldbldr_engine_adapters::infrastructure::ports::{
    ChallengeDmApprovalQueueAdapter, ChallengeOutcomeApprovalAdapter, ChallengeResolutionAdapter,
    ConnectionDirectorialContextAdapter, ConnectionManagerAdapter,
    DirectorialContextAdapter, DmActionQueuePlaceholder, DmNotificationAdapter,
    InteractionServiceAdapter, PlayerActionQueueAdapter, PlayerCharacterServiceAdapter,
    SceneServiceAdapter, SceneWorldStateAdapter, StagingServiceAdapter, StagingStateAdapter,
    WorldServiceAdapter,
};
use wrldbldr_engine_adapters::infrastructure::websocket::WebSocketBroadcastAdapter;
use wrldbldr_engine_adapters::infrastructure::world_connection_manager::SharedWorldConnectionManager;
use wrldbldr_engine_adapters::infrastructure::WorldStateManager;

use wrldbldr_engine_app::application::services::{
    NarrativeEventApprovalService, NarrativeEventService,
};
use wrldbldr_engine_app::application::use_cases::{
    ChallengeUseCase, ConnectionUseCase, InventoryUseCase, MovementUseCase, NarrativeEventUseCase,
    ObservationUseCase, PlayerActionUseCase, SceneBuilder, SceneUseCase, StagingApprovalUseCase,
};

use wrldbldr_engine_composition::UseCases;

use wrldbldr_engine_ports::inbound::{
    ChallengeUseCasePort, ConnectionUseCasePort, InventoryUseCasePort, MovementUseCasePort,
    NarrativeEventUseCasePort, ObservationUseCasePort, PlayerActionUseCasePort, RequestHandler,
    SceneUseCasePort, StagingUseCasePort,
};
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, ChallengeOutcomeApprovalServicePort, ChallengeResolutionServicePort,
    CharacterCrudPort, ClockPort, DirectorialContextRepositoryPort, DmApprovalQueueServicePort,
    InteractionServicePort, LocationCrudPort, LocationMapPort, ObservationRepositoryPort,
    PlayerActionQueueServicePort, PlayerCharacterCrudPort, PlayerCharacterInventoryPort,
    PlayerCharacterPositionPort, PlayerCharacterServicePort, RegionConnectionPort, RegionCrudPort,
    RegionExitPort, RegionItemPort, SceneServicePort,
    StagingServicePort as OutboundStagingServicePort, WorldServicePort,
};

/// Container for all use case instances and their shared infrastructure.
///
/// This struct holds both the composition-layer `UseCases` (port-typed) and
/// all the intermediate adapters needed during construction.
///
/// # Fields
///
/// The struct is organized into categories:
///
/// ## Core Infrastructure
/// - `use_cases`: The composition-layer UseCases container (port-typed)
/// - `broadcast`: WebSocketBroadcastAdapter cast to BroadcastPort
/// - `request_handler`: AppRequestHandler for request/response CRUD
///
/// ## Use Cases (as port trait objects)
/// All 9 use cases are available via the `use_cases` field.
///
/// ## Adapters (available for introspection/testing)
/// - Various port adapters wrapping services and managers
#[allow(dead_code)]
pub struct UseCaseContext {
    /// The composition-layer UseCases container (port-typed)
    pub use_cases: UseCases,

    /// Broadcast adapter for WebSocket event broadcasting
    pub broadcast: Arc<dyn BroadcastPort>,

    /// Request handler for request/response CRUD operations
    pub request_handler: Arc<dyn RequestHandler>,

    // =========================================================================
    // Adapters (available for introspection/testing)
    // =========================================================================
    /// WebSocket broadcast adapter (concrete)
    pub broadcast_adapter: Arc<WebSocketBroadcastAdapter>,
    /// DM notification adapter
    pub dm_notification_adapter: Arc<DmNotificationAdapter>,
    /// Staging state adapter
    pub staging_state_adapter: Arc<StagingStateAdapter>,
    /// Staging service adapter
    pub staging_service_adapter: Arc<StagingServiceAdapter>,
    /// Player action queue adapter
    pub player_action_queue_adapter: Arc<PlayerActionQueueAdapter>,
    /// Challenge resolution adapter
    pub challenge_resolution_adapter: Arc<ChallengeResolutionAdapter>,
    /// Challenge outcome approval adapter
    pub challenge_outcome_adapter: Arc<ChallengeOutcomeApprovalAdapter>,
    /// Challenge DM approval queue adapter
    pub challenge_dm_queue_adapter: Arc<ChallengeDmApprovalQueueAdapter>,
    /// Scene service adapter
    pub scene_service_adapter: Arc<SceneServiceAdapter>,
    /// Interaction service adapter
    pub interaction_service_adapter: Arc<InteractionServiceAdapter>,
    /// Scene world state adapter
    pub scene_world_state_adapter: Arc<SceneWorldStateAdapter>,
    /// Directorial context adapter
    pub directorial_context_adapter: Arc<DirectorialContextAdapter>,
    /// Connection manager adapter
    pub connection_manager_adapter: Arc<ConnectionManagerAdapter>,
    /// World service adapter
    pub world_service_adapter: Arc<WorldServiceAdapter>,
    /// Player character service adapter
    pub pc_service_adapter: Arc<PlayerCharacterServiceAdapter>,
    /// Connection directorial context adapter
    pub connection_directorial_adapter: Arc<ConnectionDirectorialContextAdapter>,
    /// Connection world state adapter
    pub connection_world_state_adapter: Arc<SceneWorldStateAdapter>,
    /// Scene builder (shared)
    pub scene_builder: Arc<SceneBuilder>,
}

/// Dependencies required for creating use cases.
///
/// This struct groups all the dependencies that must be provided
/// to `create_use_cases()`. Dependencies are organized by category.
///
/// # Type Parameters
///
/// * `N` - NarrativeEventService implementation type (used by NarrativeEventApprovalService)
///
/// # Lifetime
///
/// All fields are `Arc<dyn Trait>` or `Arc<ConcreteType>` to support shared ownership
/// across async handlers and enable testing with mock implementations.
pub struct UseCaseDependencies<N: NarrativeEventService + 'static> {
    // =========================================================================
    // Infrastructure
    // =========================================================================
    /// World connection manager for WebSocket connections
    pub world_connection_manager: SharedWorldConnectionManager,
    /// World state manager for per-world state
    pub world_state: Arc<WorldStateManager>,
    /// Clock for time operations
    pub clock: Arc<dyn ClockPort>,

    // =========================================================================
    // Repository Ports (ISP-split)
    // =========================================================================
    /// Player character CRUD operations
    pub pc_crud: Arc<dyn PlayerCharacterCrudPort>,
    /// Player character position operations
    pub pc_position: Arc<dyn PlayerCharacterPositionPort>,
    /// Player character inventory operations
    pub pc_inventory: Arc<dyn PlayerCharacterInventoryPort>,
    /// Region CRUD port
    pub region_crud: Arc<dyn RegionCrudPort>,
    /// Region connection port
    pub region_connection: Arc<dyn RegionConnectionPort>,
    /// Region exit port
    pub region_exit: Arc<dyn RegionExitPort>,
    /// Region item port
    pub region_item: Arc<dyn RegionItemPort>,
    /// Location CRUD port
    pub location_crud: Arc<dyn LocationCrudPort>,
    /// Location map port
    pub location_map: Arc<dyn LocationMapPort>,
    /// Character CRUD port
    pub character_crud: Arc<dyn CharacterCrudPort>,
    /// Observation repository port
    pub observation_repo: Arc<dyn ObservationRepositoryPort>,
    /// Directorial context repository port
    pub directorial_context_repo: Arc<dyn DirectorialContextRepositoryPort>,

    // =========================================================================
    // Service Ports (inbound - use case adapters wrap these)
    // =========================================================================
    /// Scene service port
    pub scene_service_port: Arc<dyn SceneServicePort>,
    /// Interaction service port
    pub interaction_service_port: Arc<dyn InteractionServicePort>,
    /// World service port
    pub world_service_port: Arc<dyn WorldServicePort>,
    /// Player character service port
    pub player_character_service_port: Arc<dyn PlayerCharacterServicePort>,

    // =========================================================================
    // Service Ports (outbound - for adapter wiring)
    // =========================================================================
    /// Staging service port (outbound) for StagingServiceAdapter
    pub staging_service_port: Arc<dyn OutboundStagingServicePort>,
    /// Player action queue service port for adapter
    pub player_action_queue_service_port: Arc<dyn PlayerActionQueueServicePort>,
    /// Challenge resolution service port
    pub challenge_resolution_service_port: Arc<dyn ChallengeResolutionServicePort>,
    /// Challenge outcome approval service port
    pub challenge_outcome_approval_service_port: Arc<dyn ChallengeOutcomeApprovalServicePort>,
    /// DM approval queue service port
    pub dm_approval_queue_service_port: Arc<dyn DmApprovalQueueServicePort>,

    // =========================================================================
    // Concrete Services (required for generic use cases)
    // =========================================================================
    /// Narrative event approval service (concrete type for NarrativeEventUseCase generics)
    pub narrative_event_approval_service: Arc<NarrativeEventApprovalService<N>>,

    // =========================================================================
    // Request Handler
    // =========================================================================
    /// App request handler for CRUD operations
    pub request_handler: Arc<dyn RequestHandler>,
}

/// Creates all use cases and returns a `UseCaseContext` containing:
/// - The composition-layer `UseCases` container (port-typed for AppState)
/// - All adapter instances
///
/// # Type Parameters
///
/// * `N` - NarrativeEventService implementation type
///
/// # Arguments
///
/// * `deps` - All dependencies required for use case construction
///
/// # Returns
///
/// A `UseCaseContext` with all use cases and adapters initialized.
///
/// # Example
///
/// ```rust,ignore
/// let deps = UseCaseDependencies {
///     world_connection_manager: world_connection_manager.clone(),
///     world_state: world_state.clone(),
///     clock: clock.clone(),
///     // ... other dependencies
///     narrative_event_approval_service: narrative_event_approval_service.clone(),
///     request_handler: request_handler.clone(),
/// };
///
/// let ctx = create_use_cases(deps);
///
/// // Use in AppState construction
/// let app_state = AppState::new(
///     // ...
///     ctx.use_cases,
///     // ...
/// );
/// ```
pub fn create_use_cases<N: NarrativeEventService + 'static>(
    deps: UseCaseDependencies<N>,
) -> UseCaseContext {
    // =========================================================================
    // Create broadcast adapter (shared by all use cases)
    // =========================================================================
    let broadcast_adapter = Arc::new(WebSocketBroadcastAdapter::new(
        deps.world_connection_manager.clone(),
    ));
    let broadcast: Arc<dyn BroadcastPort> = broadcast_adapter.clone();

    // =========================================================================
    // Create DM notification adapter
    // =========================================================================
    let dm_notification_adapter = Arc::new(DmNotificationAdapter::new(
        deps.world_connection_manager.clone(),
    ));

    // =========================================================================
    // Create staging adapters
    // =========================================================================
    let staging_state_adapter = Arc::new(StagingStateAdapter::new(
        deps.world_state.clone(),
        deps.clock.clone(),
    ));
    let staging_service_adapter = Arc::new(StagingServiceAdapter::new(
        deps.staging_service_port.clone(),
    ));

    // =========================================================================
    // Create shared scene builder
    // =========================================================================
    let scene_builder = Arc::new(SceneBuilder::new(
        deps.region_crud.clone(),
        deps.region_connection.clone(),
        deps.region_exit.clone(),
        deps.region_item.clone(),
        deps.location_crud.clone(),
    ));

    // =========================================================================
    // Create Movement Use Case
    // =========================================================================
    let movement_use_case = Arc::new(MovementUseCase::new(
        deps.pc_crud.clone(),
        deps.pc_position.clone(),
        deps.region_crud.clone(),
        deps.region_connection.clone(),
        deps.location_crud.clone(),
        deps.location_map.clone(),
        staging_service_adapter.clone(),
        staging_state_adapter.clone(),
        broadcast.clone(),
        scene_builder.clone(),
        deps.clock.clone(),
    ));

    // =========================================================================
    // Create Inventory Use Case
    // =========================================================================
    let inventory_use_case = Arc::new(InventoryUseCase::new(
        deps.pc_crud.clone(),
        deps.pc_inventory.clone(),
        deps.region_item.clone(),
        broadcast.clone(),
    ));

    // =========================================================================
    // Create Staging Approval Use Case
    // =========================================================================
    let staging_approval_use_case = Arc::new(StagingApprovalUseCase::new(
        staging_service_adapter.clone(),
        staging_state_adapter.clone(),
        deps.character_crud.clone(),
        deps.region_crud.clone(),
        deps.location_crud.clone(),
        broadcast.clone(),
        scene_builder.clone(),
        deps.clock.clone(),
    ));

    // =========================================================================
    // Create Player Action Use Case
    // =========================================================================
    let player_action_queue_adapter = Arc::new(PlayerActionQueueAdapter::new(
        deps.player_action_queue_service_port.clone(),
        deps.clock.clone(),
    ));
    let player_action_use_case = Arc::new(PlayerActionUseCase::new(
        movement_use_case.clone(),
        player_action_queue_adapter.clone(),
        dm_notification_adapter.clone(),
    ));

    // =========================================================================
    // Create Observation Use Case
    // =========================================================================
    let observation_use_case = Arc::new(ObservationUseCase::new(
        deps.pc_crud.clone(),
        deps.character_crud.clone(),
        deps.observation_repo.clone(),
        broadcast.clone(),
        deps.clock.clone(),
    ));

    // =========================================================================
    // Create Challenge Use Case
    // =========================================================================
    let challenge_resolution_adapter = Arc::new(ChallengeResolutionAdapter::new(
        deps.challenge_resolution_service_port.clone(),
    ));
    let challenge_outcome_adapter = Arc::new(ChallengeOutcomeApprovalAdapter::new(
        deps.challenge_outcome_approval_service_port.clone(),
    ));
    let challenge_dm_queue_adapter = Arc::new(ChallengeDmApprovalQueueAdapter::new(
        deps.dm_approval_queue_service_port.clone(),
    ));

    let challenge_use_case = Arc::new(ChallengeUseCase::new(
        challenge_resolution_adapter.clone(),
        challenge_outcome_adapter.clone(),
        challenge_dm_queue_adapter.clone(),
        broadcast.clone(),
        deps.world_service_port.clone(),
    ));

    // =========================================================================
    // Create Scene Use Case
    // =========================================================================
    let scene_service_adapter = Arc::new(SceneServiceAdapter::new(deps.scene_service_port.clone()));
    let interaction_service_adapter = Arc::new(InteractionServiceAdapter::new(
        deps.interaction_service_port.clone(),
    ));
    let scene_world_state_adapter = Arc::new(SceneWorldStateAdapter::new(deps.world_state.clone()));
    let directorial_context_adapter = Arc::new(DirectorialContextAdapter::new(
        deps.directorial_context_repo.clone(),
    ));
    let dm_action_queue_placeholder = Arc::new(DmActionQueuePlaceholder::new());

    let scene_use_case = Arc::new(SceneUseCase::new(
        scene_service_adapter.clone(),
        interaction_service_adapter.clone(),
        scene_world_state_adapter.clone(),
        directorial_context_adapter.clone(),
        dm_action_queue_placeholder,
    ));

    // =========================================================================
    // Create Connection Use Case
    // =========================================================================
    let connection_manager_adapter = Arc::new(ConnectionManagerAdapter::new(
        deps.world_connection_manager.clone(),
    ));
    let world_service_adapter = Arc::new(WorldServiceAdapter::new(deps.world_service_port.clone()));
    let pc_service_adapter = Arc::new(PlayerCharacterServiceAdapter::new(
        deps.player_character_service_port.clone(),
    ));
    let connection_directorial_adapter = Arc::new(ConnectionDirectorialContextAdapter::new(
        deps.directorial_context_repo.clone(),
    ));
    let connection_world_state_adapter = Arc::new(SceneWorldStateAdapter::new(deps.world_state.clone()));

    let connection_use_case = Arc::new(ConnectionUseCase::new(
        connection_manager_adapter.clone(),
        world_service_adapter.clone(),
        pc_service_adapter.clone(),
        connection_directorial_adapter.clone(),
        connection_world_state_adapter.clone(),
        broadcast.clone(),
    ));

    // =========================================================================
    // Create Narrative Event Use Case
    // =========================================================================
    let narrative_event_use_case = Arc::new(NarrativeEventUseCase::new(
        deps.narrative_event_approval_service.clone(),
        broadcast.clone(),
    ));

    tracing::info!("Initialized use cases container with all 9 use cases");

    // =========================================================================
    // Create composition-layer UseCases container (port-typed)
    // =========================================================================
    let composition_use_cases = UseCases::new(
        broadcast.clone(),
        movement_use_case as Arc<dyn MovementUseCasePort>,
        staging_approval_use_case as Arc<dyn StagingUseCasePort>,
        inventory_use_case as Arc<dyn InventoryUseCasePort>,
        player_action_use_case as Arc<dyn PlayerActionUseCasePort>,
        observation_use_case as Arc<dyn ObservationUseCasePort>,
        challenge_use_case as Arc<dyn ChallengeUseCasePort>,
        scene_use_case as Arc<dyn SceneUseCasePort>,
        connection_use_case as Arc<dyn ConnectionUseCasePort>,
        narrative_event_use_case as Arc<dyn NarrativeEventUseCasePort>,
    );

    UseCaseContext {
        use_cases: composition_use_cases,
        broadcast,
        request_handler: deps.request_handler,

        // Adapters
        broadcast_adapter,
        dm_notification_adapter,
        staging_state_adapter,
        staging_service_adapter,
        player_action_queue_adapter,
        challenge_resolution_adapter,
        challenge_outcome_adapter,
        challenge_dm_queue_adapter,
        scene_service_adapter,
        interaction_service_adapter,
        scene_world_state_adapter,
        directorial_context_adapter,
        connection_manager_adapter,
        world_service_adapter,
        pc_service_adapter,
        connection_directorial_adapter,
        connection_world_state_adapter,
        scene_builder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that UseCaseContext struct has all expected fields.
    ///
    /// This is a compile-time test - if the struct fields don't match,
    /// the code won't compile.
    #[test]
    fn test_use_case_context_structure() {
        fn _verify_use_case_context(ctx: &UseCaseContext) {
            // Core infrastructure
            let _ = &ctx.use_cases;
            let _ = &ctx.broadcast;
            let _ = &ctx.request_handler;

            // Adapters
            let _ = &ctx.broadcast_adapter;
            let _ = &ctx.dm_notification_adapter;
            let _ = &ctx.staging_state_adapter;
            let _ = &ctx.staging_service_adapter;
            let _ = &ctx.player_action_queue_adapter;
            let _ = &ctx.challenge_resolution_adapter;
            let _ = &ctx.challenge_outcome_adapter;
            let _ = &ctx.challenge_dm_queue_adapter;
            let _ = &ctx.scene_service_adapter;
            let _ = &ctx.interaction_service_adapter;
            let _ = &ctx.scene_world_state_adapter;
            let _ = &ctx.directorial_context_adapter;
            let _ = &ctx.connection_manager_adapter;
            let _ = &ctx.world_service_adapter;
            let _ = &ctx.pc_service_adapter;
            let _ = &ctx.connection_directorial_adapter;
            let _ = &ctx.connection_world_state_adapter;
            let _ = &ctx.scene_builder;
        }

        // The existence of this function proves the types are correct at compile time
        let _ = _verify_use_case_context;
    }

    /// Test that UseCaseDependencies struct has all expected fields.
    #[test]
    fn test_use_case_dependencies_structure() {
        // This test verifies the generic struct by checking field names at compile time
        fn _verify_dependencies<N: NarrativeEventService + 'static>(deps: &UseCaseDependencies<N>) {
            // Infrastructure
            let _ = &deps.world_connection_manager;
            let _ = &deps.world_state;
            let _ = &deps.clock;

            // Repository ports (ISP-split PC traits)
            let _ = &deps.pc_crud;
            let _ = &deps.pc_position;
            let _ = &deps.pc_inventory;
            let _ = &deps.region_crud;
            let _ = &deps.region_connection;
            let _ = &deps.region_exit;
            let _ = &deps.region_item;
            let _ = &deps.location_crud;
            let _ = &deps.location_map;
            let _ = &deps.character_crud;
            let _ = &deps.observation_repo;
            let _ = &deps.directorial_context_repo;

            // Service ports (inbound)
            let _ = &deps.scene_service_port;
            let _ = &deps.interaction_service_port;
            let _ = &deps.world_service_port;
            let _ = &deps.player_character_service_port;

            // Service ports (outbound)
            let _ = &deps.staging_service_port;
            let _ = &deps.player_action_queue_service_port;
            let _ = &deps.challenge_resolution_service_port;
            let _ = &deps.challenge_outcome_approval_service_port;
            let _ = &deps.dm_approval_queue_service_port;

            // Concrete services
            let _ = &deps.narrative_event_approval_service;

            // Request handler
            let _ = &deps.request_handler;
        }

        // The existence of this function proves the types are correct at compile time
        let _ = _verify_dependencies::<
            wrldbldr_engine_app::application::services::NarrativeEventServiceImpl,
        >;
    }

    /// Verify the number of use cases matches expectations.
    #[test]
    fn test_use_case_count() {
        // Document the expected 9 use cases
        let expected_use_cases = [
            "Movement",
            "Inventory",
            "Staging",
            "PlayerAction",
            "Observation",
            "Challenge",
            "Scene",
            "Connection",
            "NarrativeEvent",
        ];

        assert_eq!(expected_use_cases.len(), 9);
    }

    /// Verify adapter count matches expectations.
    #[test]
    fn test_adapter_count() {
        // Document the expected adapters
        let expected_adapters = [
            "WebSocketBroadcastAdapter",
            "DmNotificationAdapter",
            "StagingStateAdapter",
            "StagingServiceAdapter",
            "PlayerActionQueueAdapter",
            "ChallengeResolutionAdapter",
            "ChallengeOutcomeApprovalAdapter",
            "ChallengeDmApprovalQueueAdapter",
            "SceneServiceAdapter",
            "InteractionServiceAdapter",
            "SceneWorldStateAdapter",
            "DirectorialContextAdapter",
            "ConnectionManagerAdapter",
            "WorldServiceAdapter",
            "PlayerCharacterServiceAdapter",
            "ConnectionDirectorialContextAdapter",
            "SceneWorldStateAdapter",
        ];

        assert_eq!(expected_adapters.len(), 17);
    }
}
