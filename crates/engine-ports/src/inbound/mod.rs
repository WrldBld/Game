//! Inbound ports - Interfaces that the application exposes to the outside world

pub mod app_state_port;
pub mod challenge_use_case_port;
pub mod connection_use_case_port;
pub mod inventory_use_case_port;
pub mod movement_use_case_port;
pub mod narrative_event_use_case_port;
pub mod observation_use_case_port;
pub mod player_action_use_case_port;
pub mod request_handler;
pub mod scene_use_case_port;
pub mod staging_use_case_port;
pub mod use_case_context;
pub mod use_case_errors;
pub mod use_case_ports;
pub mod use_cases;

pub use challenge_use_case_port::ChallengeUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use challenge_use_case_port::MockChallengeUseCasePort;
pub use connection_use_case_port::ConnectionUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use connection_use_case_port::MockConnectionUseCasePort;
pub use inventory_use_case_port::InventoryUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use inventory_use_case_port::MockInventoryUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use movement_use_case_port::MockMovementUseCasePort;
pub use movement_use_case_port::MovementUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use narrative_event_use_case_port::MockNarrativeEventUseCasePort;
pub use narrative_event_use_case_port::NarrativeEventUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use observation_use_case_port::MockObservationUseCasePort;
pub use observation_use_case_port::ObservationUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use player_action_use_case_port::MockPlayerActionUseCasePort;
pub use player_action_use_case_port::PlayerActionUseCasePort;
pub use request_handler::{RequestContext, RequestHandler};
#[cfg(any(test, feature = "testing"))]
pub use scene_use_case_port::MockSceneUseCasePort;
pub use scene_use_case_port::SceneUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use staging_use_case_port::MockStagingUseCasePort;
pub use staging_use_case_port::StagingUseCasePort;
pub use use_case_context::UseCaseContext;

// App state port - interface for accessing application services
pub use app_state_port::AppStatePort;

// Re-export all use case error types
pub use use_case_errors::{
    ActionError, ChallengeError, InventoryError, MovementError, NarrativeEventError,
    ObservationError, SceneError, StagingError,
};

// Re-export all use case port traits
pub use use_case_ports::{
    // Challenge port traits
    ChallengeDmApprovalQueuePort,
    ChallengeOutcomeApprovalPort,
    ChallengeResolutionPort,
    NarrativeRollContext,
    // Connection port traits
    ConnectionManagerPort,
    DirectorialContextPort,
    // Scene port traits
    DirectorialContextRepositoryPort,
    // Player action port traits
    DmNotificationPort,
    InteractionServicePort,
    PlayerActionQueuePort,
    PlayerCharacterServicePort,
    SceneDmActionQueuePort,
    SceneServicePort,
    // Staging port traits
    StagingServiceExtPort,
    StagingServicePort,
    StagingStateExtPort,
    StagingStatePort,
    WorldServicePort,
    WorldStatePort,
};

// Re-export types from use_case_ports (which re-exports from outbound)
pub use use_case_ports::{
    // Challenge types
    AdHocOutcomes,
    AdHocResult,
    // Observation types
    ApproachEventData,
    ApprovalItem,
    // Staging types
    ApprovedNpcData,
    // Scene types (DmAction, SceneWithRelations come through aliased from use_case_ports)
    CharacterEntity,
    // Connection types
    ConnectedUser,
    ConnectionInfo,
    DiceInputType,
    DirectorialContextData,
    DmAction,
    InteractionEntity,
    InteractionTarget,
    LocationEntity,
    LocationEventData,
    NpcMotivation,
    OutcomeDecision,
    PcData,
    PendingStagingData,
    PendingStagingInfo,
    ProposedNpc,
    RegeneratedNpc,
    RollResultData,
    SceneApprovalDecision,
    SceneEntity,
    SceneWithRelations,
    StagedNpcData,
    StagingProposalData,
    TimeContext,
    TriggerInfo,
    TriggerResult,
    UserJoinedEvent,
    UserLeftEvent,
    WaitingPcData,
    WaitingPcInfo,
    WorldRole,
};
