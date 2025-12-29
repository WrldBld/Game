//! Inbound ports - Interfaces that the application exposes to the outside world

pub mod request_handler;
pub mod use_case_context;
pub mod use_case_ports;
pub mod use_cases;

pub use request_handler::{RequestContext, RequestHandler};
pub use use_case_context::UseCaseContext;

// Re-export all use case port traits
pub use use_case_ports::{
    // Challenge port traits
    ChallengeDmApprovalQueuePort, ChallengeOutcomeApprovalPort, ChallengeResolutionPort,
    // Connection port traits
    ConnectionManagerPort, DirectorialContextPort, PlayerCharacterServicePort, WorldServicePort,
    // Scene port traits
    DirectorialContextRepositoryPort, InteractionServicePort, SceneDmActionQueuePort,
    SceneServicePort, WorldStatePort,
    // Player action port traits
    DmNotificationPort, PlayerActionQueuePort,
    // Staging port traits
    StagingServiceExtPort, StagingServicePort, StagingStateExtPort, StagingStatePort,
    // Observation port traits
    WorldMessagePort,
};

// Re-export types from use_case_ports (which re-exports from outbound)
pub use use_case_ports::{
    // Challenge types
    AdHocOutcomes, AdHocResult, ApprovalItem, DiceInputType, OutcomeDecision, RollResultData,
    TriggerInfo, TriggerResult,
    // Connection types
    ConnectedUser, ConnectionInfo, PcData, UserJoinedEvent, UserLeftEvent, WorldRole,
    // Scene types (DmAction, SceneWithRelations come through aliased from use_case_ports)
    ApprovalDecision, CharacterEntity, DirectorialContextData, DmAction, InteractionEntity,
    InteractionTarget, LocationEntity, NpcMotivation, SceneEntity, SceneWithRelations, TimeContext,
    // Staging types
    ApprovedNpcData, PendingStagingData, PendingStagingInfo, ProposedNpc, RegeneratedNpc,
    StagedNpcData, StagingProposalData, WaitingPcData, WaitingPcInfo,
    // Observation types
    ApproachEventData, LocationEventData,
};
