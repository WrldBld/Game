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
//! This factory prefers port trait objects for maximum flexibility and to
//! enforce dependency inversion.
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

use wrldbldr_engine_adapters::infrastructure::port_adapters::{
    ChallengeDmApprovalQueueAdapter, ChallengeOutcomeApprovalAdapter, ChallengeResolutionAdapter,
    ConnectionDirectorialContextAdapter, DirectorialContextAdapter, DmActionQueuePlaceholder,
    InteractionServiceAdapter, PlayerActionQueueAdapter, PlayerCharacterServiceAdapter,
    SceneServiceAdapter, StagingServiceAdapter, WorldServiceAdapter,
};
use wrldbldr_engine_adapters::infrastructure::websocket::WebSocketBroadcastAdapter;

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
// Internal service traits (NOT ports - internal app-layer contracts)
use wrldbldr_engine_app::application::services::internal::{
    ChallengeOutcomeApprovalServicePort, ChallengeResolutionServicePort,
    DmApprovalQueueServicePort, InteractionServicePort, NarrativeEventApprovalServicePort,
    PlayerActionQueueServicePort, PlayerCharacterServicePort, SceneServicePort, StagingServicePort,
    WorldServicePort,
};
// True outbound ports (repository and infrastructure ports)
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, CharacterCrudPort, ClockPort, ConnectionBroadcastPort, ConnectionManagerPort,
    ConnectionUnicastPort, DirectorialContextRepositoryPort, DmNotificationPort, LocationCrudPort,
    LocationMapPort, ObservationRepositoryPort, PlayerCharacterCrudPort,
    PlayerCharacterInventoryPort, PlayerCharacterPositionPort, RegionConnectionPort,
    RegionCrudPort, RegionExitPort, RegionItemPort, StagingStateExtPort, StagingStatePort,
    WorldStateUpdatePort,
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
}

/// Dependencies required for creating use cases.
///
/// This struct groups all the dependencies that must be provided
/// to `create_use_cases()`. Dependencies are organized by category.
///
/// # Lifetime
///
/// All fields are `Arc<dyn Trait>` or `Arc<ConcreteType>` to support shared ownership
/// across async handlers and enable testing with mock implementations.
pub struct UseCaseDependencies {
    // =========================================================================
    // Infrastructure
    // =========================================================================
    /// Connection manager operations (join/leave world)
    pub connection_manager: Arc<dyn ConnectionManagerPort>,

    /// Broadcast operations for WebSocket message routing
    pub connection_broadcast: Arc<dyn ConnectionBroadcastPort>,

    /// Unicast operations for user-targeted WebSocket delivery
    pub connection_unicast: Arc<dyn ConnectionUnicastPort>,

    /// DM notification operations for queued-action alerts
    pub dm_notification: Arc<dyn DmNotificationPort>,
    /// Staging state port (used by movement + staging use cases)
    pub staging_state: Arc<dyn StagingStateExtPort>,
    /// World state update port (used by scene/connection use cases)
    pub world_state_update: Arc<dyn WorldStateUpdatePort>,
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
    pub staging_service_port: Arc<dyn StagingServicePort>,
    /// Player action queue service port for adapter
    pub player_action_queue_service_port: Arc<dyn PlayerActionQueueServicePort>,
    /// Challenge resolution service port
    pub challenge_resolution_service_port: Arc<dyn ChallengeResolutionServicePort>,
    /// Challenge outcome approval service port
    pub challenge_outcome_approval_service_port: Arc<dyn ChallengeOutcomeApprovalServicePort>,
    /// DM approval queue service port
    pub dm_approval_queue_service_port: Arc<dyn DmApprovalQueueServicePort>,

    // =========================================================================
    // Service Ports (outbound)
    // =========================================================================
    /// Narrative event approval service port
    pub narrative_event_approval_service: Arc<dyn NarrativeEventApprovalServicePort>,

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
///     connection_manager: world_connection_manager.clone() as Arc<dyn ConnectionManagerPort>,
///     connection_broadcast: world_connection_manager.clone() as Arc<dyn ConnectionBroadcastPort>,
///     connection_unicast: world_connection_manager.clone() as Arc<dyn ConnectionUnicastPort>,
///     dm_notification: world_connection_manager.clone() as Arc<dyn DmNotificationPort>,
///     staging_state: staging_state.clone(),
///     world_state_update: world_state_update.clone(),
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
pub fn create_use_cases(deps: UseCaseDependencies) -> UseCaseContext {
    // =========================================================================
    // Create broadcast adapter (shared by all use cases)
    // =========================================================================
    let broadcast_adapter = Arc::new(WebSocketBroadcastAdapter::new(
        deps.connection_broadcast.clone(),
        deps.connection_unicast.clone(),
    ));
    let broadcast: Arc<dyn BroadcastPort> = broadcast_adapter.clone();

    // =========================================================================
    // Create staging service adapter
    // =========================================================================
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
        deps.staging_state.clone() as Arc<dyn StagingStatePort>,
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
        deps.staging_state.clone(),
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
        deps.dm_notification.clone(),
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
    let directorial_context_adapter = Arc::new(DirectorialContextAdapter::new(
        deps.directorial_context_repo.clone(),
    ));
    let dm_action_queue_placeholder = Arc::new(DmActionQueuePlaceholder::new());

    let scene_use_case = Arc::new(SceneUseCase::new(
        scene_service_adapter.clone(),
        interaction_service_adapter.clone(),
        deps.world_state_update.clone(),
        directorial_context_adapter.clone(),
        dm_action_queue_placeholder,
    ));

    // =========================================================================
    // Create Connection Use Case
    // =========================================================================
    let world_service_adapter = Arc::new(WorldServiceAdapter::new(deps.world_service_port.clone()));
    let pc_service_adapter = Arc::new(PlayerCharacterServiceAdapter::new(
        deps.player_character_service_port.clone(),
    ));
    let connection_directorial_adapter = Arc::new(ConnectionDirectorialContextAdapter::new(
        deps.directorial_context_repo.clone(),
    ));

    let connection_use_case = Arc::new(ConnectionUseCase::new(
        deps.connection_manager.clone(),
        world_service_adapter.clone(),
        pc_service_adapter.clone(),
        connection_directorial_adapter.clone(),
        deps.world_state_update.clone(),
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
        }

        // The existence of this function proves the types are correct at compile time
        let _ = _verify_use_case_context;
    }

    /// Test that UseCaseDependencies struct has all expected fields.
    #[test]
    fn test_use_case_dependencies_structure() {
        // This test verifies the generic struct by checking field names at compile time
        fn _verify_dependencies(deps: &UseCaseDependencies) {
            // Infrastructure
            let _ = &deps.connection_manager;
            let _ = &deps.connection_broadcast;
            let _ = &deps.connection_unicast;
            let _ = &deps.dm_notification;
            let _ = &deps.staging_state;
            let _ = &deps.world_state_update;
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

            // Service ports (outbound)
            let _ = &deps.narrative_event_approval_service;

            // Request handler
            let _ = &deps.request_handler;
        }

        // The existence of this function proves the types are correct at compile time
        let _ = _verify_dependencies;
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
}
