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
//! - [x] PlayerActionUseCase - Player action handling
//! - [x] ObservationUseCase - NPC observation events
//! - [x] ChallengeUseCase - Challenge resolution
//! - [x] SceneUseCase - Scene management
//! - [x] ConnectionUseCase - World connection management

use std::sync::Arc;

use wrldbldr_domain::value_objects::{ApprovalRequestData, LlmRequestData, PlayerActionData};
use wrldbldr_engine_app::application::services::staging_service::StagingService;
use wrldbldr_engine_app::application::services::NarrativeEventServiceImpl;
use wrldbldr_engine_app::application::services::{
    ChallengeOutcomeApprovalService, ChallengeResolutionService, ChallengeService,
    DMApprovalQueueService, InteractionService, ItemService, NarrativeEventApprovalService,
    PlayerActionQueueService, PlayerCharacterService, SceneService, SkillService, WorldService,
};
use wrldbldr_engine_app::application::use_cases::{
    ChallengeUseCase, ConnectionUseCase, InventoryUseCase, MovementUseCase, NarrativeEventUseCase,
    ObservationUseCase, PlayerActionUseCase, SceneBuilder, SceneUseCase, StagingApprovalUseCase,
};
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, CharacterRepositoryPort, ClockPort,
    DirectorialContextRepositoryPort as PortDirectorialContextRepositoryPort, LlmPort,
    LocationRepositoryPort, NarrativeEventRepositoryPort, ObservationRepositoryPort,
    PlayerCharacterRepositoryPort, ProcessingQueuePort, QueuePort, RegionRepositoryPort,
    StagingRepositoryPort,
};

use crate::infrastructure::ports::{
    ChallengeDmApprovalQueueAdapter, ChallengeOutcomeApprovalAdapter, ChallengeResolutionAdapter,
    ConnectionDirectorialContextAdapter, ConnectionManagerAdapter, ConnectionWorldStateAdapter,
    DirectorialContextAdapter, DmActionQueuePlaceholder, DmNotificationAdapter,
    InteractionServiceAdapter, PlayerActionQueueAdapter, PlayerCharacterServiceAdapter,
    SceneServiceAdapter, SceneWorldStateAdapter, StagingServiceAdapter, StagingStateAdapter,
    WorldMessageAdapter, WorldServiceAdapter,
};
use crate::infrastructure::queues::QueueBackendEnum;
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

    /// Player action use case for travel and queued actions
    pub player_action: Arc<PlayerActionUseCase>,

    /// Observation use case for NPC observation and event triggering
    pub observation: Arc<ObservationUseCase>,

    /// Challenge use case for dice rolls and challenge resolution
    pub challenge: Arc<ChallengeUseCase>,

    /// Scene use case for scene changes and directorial context
    pub scene: Arc<SceneUseCase>,

    /// Connection use case for join/leave world operations
    pub connection: Arc<ConnectionUseCase>,

    /// Narrative event use case for DM approval of narrative events
    pub narrative_event: Arc<NarrativeEventUseCase<NarrativeEventServiceImpl>>,
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
    /// * `character_repo` - Character repository (for StagingApprovalUseCase and ObservationUseCase)
    /// * `observation_repo` - Observation repository (for ObservationUseCase)
    /// * `staging_service` - The staging service (generic over its dependencies)
    /// * `player_action_queue_service` - The player action queue service
    /// * `scene_service` - Scene service for scene operations
    /// * `interaction_service` - Interaction service for scene interactions
    /// * `directorial_context_repo` - Repository for directorial context persistence
    /// * `world_service` - World service for world snapshots
    /// * `pc_service` - Player character service for PC data
    /// * `challenge_resolution_service` - Service for challenge resolution (generic over dependencies)
    /// * `challenge_outcome_approval_service` - Service for challenge outcome approval (generic over LLM)
    /// * `dm_approval_queue_service` - Service for DM approval queue (generic over queue and item service)
    #[allow(clippy::too_many_arguments)]
    pub fn new<L, R, N, S, PAQ, LQ, COAL, IS, CS, KS, PCS>(
        connection_manager: SharedWorldConnectionManager,
        world_state: Arc<WorldStateManager>,
        pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
        region_repo: Arc<dyn RegionRepositoryPort>,
        location_repo: Arc<dyn LocationRepositoryPort>,
        character_repo: Arc<dyn CharacterRepositoryPort>,
        observation_repo: Arc<dyn ObservationRepositoryPort>,
        staging_service: Arc<StagingService<L, R, N, S>>,
        player_action_queue_service: Arc<PlayerActionQueueService<PAQ, LQ>>,
        // Scene and Connection dependencies
        scene_service: Arc<dyn SceneService>,
        interaction_service: Arc<dyn InteractionService>,
        directorial_context_repo: Arc<dyn PortDirectorialContextRepositoryPort>,
        world_service: Arc<dyn WorldService>,
        pc_service: Arc<dyn PlayerCharacterService>,
        // Challenge dependencies
        challenge_resolution_service: Arc<
            ChallengeResolutionService<
                CS,
                KS,
                QueueBackendEnum<ApprovalRequestData>,
                PCS,
                COAL,
                IS,
            >,
        >,
        challenge_outcome_approval_service: Arc<ChallengeOutcomeApprovalService<COAL>>,
        dm_approval_queue_service: Arc<
            DMApprovalQueueService<QueueBackendEnum<ApprovalRequestData>, IS>,
        >,
        // Narrative event dependencies
        narrative_event_approval_service: Arc<
            NarrativeEventApprovalService<NarrativeEventServiceImpl>,
        >,
        // Clock for time operations
        clock: Arc<dyn ClockPort>,
    ) -> Self
    where
        L: LlmPort + Send + Sync + 'static,
        R: RegionRepositoryPort + Send + Sync + 'static,
        N: NarrativeEventRepositoryPort + Send + Sync + 'static,
        S: StagingRepositoryPort + Send + Sync + 'static,
        PAQ: QueuePort<PlayerActionData> + Send + Sync + 'static,
        LQ: ProcessingQueuePort<LlmRequestData> + Send + Sync + 'static,
        COAL: LlmPort + Send + Sync + 'static,
        IS: ItemService + Send + Sync + 'static,
        CS: ChallengeService + Send + Sync + 'static,
        KS: SkillService + Send + Sync + 'static,
        PCS: PlayerCharacterService + Send + Sync + 'static,
    {
        // Create broadcast adapter
        let broadcast: Arc<dyn BroadcastPort> =
            Arc::new(WebSocketBroadcastAdapter::new(connection_manager.clone()));

        // Create DM notification adapter (clone connection_manager since we'll use it again)
        let dm_notification = Arc::new(DmNotificationAdapter::new(connection_manager.clone()));

        // Create staging adapters
        // Note: StagingStateAdapter implements both StagingStatePort and StagingStateExtPort
        // Note: StagingServiceAdapter implements both StagingServicePort and StagingServiceExtPort
        let staging_state_adapter = Arc::new(StagingStateAdapter::new(world_state.clone()));
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
            pc_repo.clone(),
            region_repo.clone(),
            broadcast.clone(),
        ));

        // Create staging approval use case
        let staging = Arc::new(StagingApprovalUseCase::new(
            staging_service_adapter,
            staging_state_adapter,
            character_repo.clone(),
            region_repo,
            location_repo,
            broadcast.clone(),
            scene_builder,
        ));

        // Create player action use case
        let player_action_queue_adapter =
            Arc::new(PlayerActionQueueAdapter::new(player_action_queue_service));
        let player_action = Arc::new(PlayerActionUseCase::new(
            movement.clone(),
            player_action_queue_adapter,
            dm_notification,
            broadcast.clone(),
        ));

        // Create observation adapters
        // Note: observation_repo now directly implements the same ObservationRepositoryPort
        // used by ObservationUseCase (consolidated from engine-ports)
        let world_message_adapter = Arc::new(WorldMessageAdapter::new(connection_manager.clone()));

        // Create observation use case
        let observation = Arc::new(ObservationUseCase::new(
            pc_repo.clone(),
            character_repo,
            observation_repo,
            world_message_adapter,
            broadcast.clone(),
            clock,
        ));

        // =========================================================================
        // Challenge Use Case
        // =========================================================================
        // Adapter wraps the ChallengeResolutionService to implement ChallengeResolutionPort
        let challenge_resolution_adapter = Arc::new(ChallengeResolutionAdapter::new(
            challenge_resolution_service,
        ));
        let challenge_outcome_adapter = Arc::new(ChallengeOutcomeApprovalAdapter::new(
            challenge_outcome_approval_service,
        ));
        let challenge_dm_queue_adapter = Arc::new(ChallengeDmApprovalQueueAdapter::new(
            dm_approval_queue_service,
        ));

        let challenge = Arc::new(ChallengeUseCase::new(
            challenge_resolution_adapter,
            challenge_outcome_adapter,
            challenge_dm_queue_adapter,
            broadcast.clone(),
        ));

        // =========================================================================
        // Scene Use Case
        // =========================================================================
        let scene_service_adapter = Arc::new(SceneServiceAdapter::new(scene_service));
        let interaction_service_adapter =
            Arc::new(InteractionServiceAdapter::new(interaction_service));
        let scene_world_state_adapter = Arc::new(SceneWorldStateAdapter::new(world_state.clone()));
        let scene_directorial_adapter = Arc::new(DirectorialContextAdapter::new(
            directorial_context_repo.clone(),
        ));
        let dm_action_queue_placeholder = Arc::new(DmActionQueuePlaceholder::new());

        let scene = Arc::new(SceneUseCase::new(
            scene_service_adapter,
            interaction_service_adapter,
            scene_world_state_adapter,
            scene_directorial_adapter,
            dm_action_queue_placeholder,
            broadcast.clone(),
        ));

        // =========================================================================
        // Connection Use Case
        // =========================================================================
        let connection_manager_adapter =
            Arc::new(ConnectionManagerAdapter::new(connection_manager));
        let world_service_adapter = Arc::new(WorldServiceAdapter::new(world_service));
        let pc_service_adapter = Arc::new(PlayerCharacterServiceAdapter::new(pc_service));
        let connection_directorial_adapter = Arc::new(ConnectionDirectorialContextAdapter::new(
            directorial_context_repo,
        ));
        let connection_world_state_adapter =
            Arc::new(ConnectionWorldStateAdapter::new(world_state));

        let connection = Arc::new(ConnectionUseCase::new(
            connection_manager_adapter,
            world_service_adapter,
            pc_service_adapter,
            connection_directorial_adapter,
            connection_world_state_adapter,
            broadcast.clone(),
        ));

        // =========================================================================
        // Narrative Event Use Case
        // =========================================================================
        let narrative_event = Arc::new(NarrativeEventUseCase::new(
            narrative_event_approval_service,
            broadcast.clone(),
        ));

        Self {
            broadcast,
            movement,
            staging,
            inventory,
            player_action,
            observation,
            challenge,
            scene,
            connection,
            narrative_event,
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
