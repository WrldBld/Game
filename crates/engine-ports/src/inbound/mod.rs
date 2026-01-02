//! Inbound ports - Interfaces that the application exposes to the outside world

pub mod app_state_port;
pub mod asset_use_case_port;
pub mod challenge_use_case_port;
pub mod connection_use_case_port;
pub mod generation_use_case_port;
pub mod inventory_use_case_port;
pub mod movement_use_case_port;
pub mod narrative_event_use_case_port;
pub mod observation_use_case_port;
pub mod player_action_use_case_port;
pub mod prompt_template_use_case_port;
pub mod queue_use_case_port;
pub mod request_handler;
pub mod scene_use_case_port;
pub mod settings_use_case_port;
pub mod staging_use_case_port;
pub mod use_case_context;
pub mod use_case_ports;
pub mod use_cases;
pub mod workflow_use_case_port;
pub mod world_use_case_port;

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

// New service-oriented use case ports (Step 3 of ServicePort migration)
pub use asset_use_case_port::{AssetUseCasePort, CreateAssetRequest};
#[cfg(any(test, feature = "testing"))]
pub use asset_use_case_port::MockAssetUseCasePort;
pub use generation_use_case_port::{GenerationRequest, GenerationUseCasePort};
#[cfg(any(test, feature = "testing"))]
pub use generation_use_case_port::MockGenerationUseCasePort;
pub use prompt_template_use_case_port::PromptTemplateUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use prompt_template_use_case_port::MockPromptTemplateUseCasePort;
pub use queue_use_case_port::{
    // Trait ports
    AssetGenerationQueueUseCasePort, DmApprovalQueueUseCasePort,
    GenerationQueueProjectionUseCasePort, LlmQueueUseCasePort, PlayerActionQueueUseCasePort,
    // Asset generation types
    AssetGenerationQueueItem, AssetGenerationRequest, GenerationMetadata, GenerationResult,
    // Generation queue projection types
    GenerationBatchSnapshot, GenerationQueueSnapshot, SuggestionTaskSnapshot,
    // Player action types
    PlayerAction, PlayerActionQueueItem,
    // LLM queue types
    ChallengeSuggestion, ConfidenceLevel, LlmQueueItem, LlmQueueRequest, LlmQueueResponse,
    LlmRequestType, NarrativeEventSuggestion, ProposedToolCall,
    // DM approval types
    ApprovalDecisionType, ApprovalQueueItem, ApprovalRequest, ApprovalUrgency, DmApprovalDecision,
    // Shared types (re-exported from outbound)
    QueueItemStatus,
};
#[cfg(any(test, feature = "testing"))]
pub use queue_use_case_port::{
    MockAssetGenerationQueueUseCasePort, MockDmApprovalQueueUseCasePort,
    MockGenerationQueueProjectionUseCasePort, MockLlmQueueUseCasePort,
    MockPlayerActionQueueUseCasePort,
};
pub use settings_use_case_port::{LlmConfig, SettingsUseCasePort};
#[cfg(any(test, feature = "testing"))]
pub use settings_use_case_port::MockSettingsUseCasePort;
pub use workflow_use_case_port::WorkflowUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use workflow_use_case_port::MockWorkflowUseCasePort;
pub use world_use_case_port::WorldUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use world_use_case_port::MockWorldUseCasePort;

// App state port - interface for accessing application services
pub use app_state_port::AppStatePort;

// Note: use-case errors live in outbound ports and should be imported from there.

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
