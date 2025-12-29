//! Outbound ports - interfaces the engine application requires.
//!
//! This crate is being extracted from `wrldbldr-engine-app`. For now, we only
//! expose ports that do not depend on engine-app-internal domain/DTO types.
//!
//! Ports that still depend on engine-app internals remain in `wrldbldr-engine-app`
//! until the shared types move into `wrldbldr-domain`/`wrldbldr-protocol`.

mod actantial_context_service_port;
mod asset_generation_queue_service_port;
mod asset_service_port;
mod broadcast_port;
mod challenge_resolution_service_port;
mod challenge_service_port;
mod character_service_port;
mod clock_port;
mod comfyui_port;
mod directorial_context_port;
mod disposition_service_port;
mod dm_action_processor_port;
mod dm_action_queue_service_port;
mod dm_approval_queue_service_port;
mod domain_event_repository_port;
mod event_bus_port;
mod game_events;
mod generation_read_state_port;
mod generation_service_port;
mod interaction_service_port;
mod item_service_port;
mod llm_port;
mod llm_queue_service_port;
mod location_service_port;
mod narrative_event_service_port;
mod player_action_queue_service_port;
mod player_character_service_port;
mod prompt_template_port;
mod prompt_template_service_port;
mod queue_notification_port;
mod queue_port;
mod region_service_port;
mod repository_port;
mod scene_service_port;
mod settings_port;
mod settings_service_port;
mod skill_service_port;
mod staging_repository_port;
mod staging_service_port;
mod story_event_service_port;
mod suggestion_enqueue_port;
mod workflow_service_port;
mod world_exporter_port;
mod world_service_port;
mod world_state_port;

// Actantial context service port - interface for character motivation context
pub use actantial_context_service_port::ActantialContextServicePort;
#[cfg(any(test, feature = "testing"))]
pub use actantial_context_service_port::MockActantialContextServicePort;

// Challenge service port - interface for challenge operations
pub use challenge_service_port::ChallengeServicePort;
#[cfg(any(test, feature = "testing"))]
pub use challenge_service_port::MockChallengeServicePort;

// Clock port - time abstraction for deterministic testing
pub use clock_port::ClockPort;
#[cfg(any(test, feature = "testing"))]
pub use clock_port::MockClockPort;

// DomainEvent repository - domain-layer interface for event storage
pub use domain_event_repository_port::{DomainEventRepositoryError, DomainEventRepositoryPort};

// DM action processor - interface for processing DM actions
pub use dm_action_processor_port::{DmActionProcessorPort, DmActionResult};

pub use comfyui_port::{
    ComfyUIPort, GeneratedImage, HistoryResponse, NodeOutput, PromptHistory, PromptStatus,
    QueuePromptResponse,
};

pub use event_bus_port::{EventBusError, EventBusPort};

pub use generation_read_state_port::{GenerationReadKind, GenerationReadStatePort};

pub use llm_port::{
    ChatMessage, FinishReason, ImageData, LlmPort, LlmRequest, LlmResponse, MessageRole,
    TokenUsage, ToolCall, ToolDefinition,
};

pub use queue_notification_port::{QueueNotificationPort, WaitResult};

pub use repository_port::{
    AssetRepositoryPort, ChallengeRepositoryPort, CharacterNode, CharacterRepositoryPort,
    EventChainRepositoryPort, FlagRepositoryPort, GoalRepositoryPort, InteractionRepositoryPort,
    ItemRepositoryPort, LocationRepositoryPort, NarrativeEventRepositoryPort, ObservationRepositoryPort,
    PlayerCharacterRepositoryPort, RegionRepositoryPort, RelationshipEdge, RelationshipRepositoryPort,
    SceneRepositoryPort, SheetTemplateRepositoryPort, SkillRepositoryPort, SocialNetwork,
    StoryEventRepositoryPort, WantRepositoryPort, WorkflowRepositoryPort, WorldRepositoryPort,
};

pub use prompt_template_port::{
    PromptTemplateError, PromptTemplateRepositoryPort, PromptTemplateSource, ResolvedPromptTemplate,
};

pub use settings_port::{SettingsError, SettingsRepositoryPort};

pub use staging_repository_port::{StagedNpcRow, StagingRepositoryPort};

pub use suggestion_enqueue_port::{
    SuggestionEnqueueContext, SuggestionEnqueuePort, SuggestionEnqueueRequest,
    SuggestionEnqueueResponse,
};

pub use queue_port::{
    ApprovalQueuePort, ProcessingQueuePort, QueueError, QueueItem, QueueItemId, QueueItemStatus,
    QueuePort,
};

pub use world_exporter_port::{
    CharacterData, ExportOptions, LocationData, PlayerWorldSnapshot, SceneData, WorldData,
    WorldExporterPort,
};

pub use directorial_context_port::DirectorialContextRepositoryPort;

// Narrative event service port - interface for narrative event operations
pub use narrative_event_service_port::NarrativeEventServicePort;

// Scene service port - interface for scene operations
pub use scene_service_port::{SceneServicePort, SceneWithRelations};

// Disposition service port - interface for NPC disposition operations
pub use disposition_service_port::DispositionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use disposition_service_port::MockDispositionServicePort;

// Skill service port - interface for skill operations
pub use skill_service_port::{CreateSkillRequest, SkillServicePort, UpdateSkillRequest};
#[cfg(any(test, feature = "testing"))]
pub use skill_service_port::MockSkillServicePort;

// Interaction service port - interface for interaction operations
pub use interaction_service_port::InteractionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use interaction_service_port::MockInteractionServicePort;

// World service port - interface for world operations
pub use world_service_port::WorldServicePort;
#[cfg(any(test, feature = "testing"))]
pub use world_service_port::MockWorldServicePort;

// Character service port - interface for character operations
pub use character_service_port::CharacterServicePort;
#[cfg(any(test, feature = "testing"))]
pub use character_service_port::MockCharacterServicePort;

// Location service port - interface for location operations
pub use location_service_port::LocationServicePort;
#[cfg(any(test, feature = "testing"))]
pub use location_service_port::MockLocationServicePort;

// Region service port - interface for region operations
pub use region_service_port::RegionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use region_service_port::MockRegionServicePort;

pub use broadcast_port::BroadcastPort;
pub use game_events::{
    GameEvent, ItemInfo, LocationGroup, NavigationExit, NavigationInfo, NavigationTarget,
    NpcPresenceData, OutcomeBranchInfo, OutcomeTriggerInfo, PcLocationData, PreviousStagingData,
    RegionInfo, RegionItemData, SceneChangedEvent, SplitPartyEvent, StagedNpcData,
    StagingPendingEvent, StagingReadyEvent, StagingRequiredEvent, StateChangeInfo, WaitingPcData,
};
pub use world_state_port::WorldStatePort;

// Item service port - interface for item operations
pub use item_service_port::ItemServicePort;
#[cfg(any(test, feature = "testing"))]
pub use item_service_port::MockItemServicePort;

// Player character service port - interface for player character operations
pub use player_character_service_port::PlayerCharacterServicePort;
#[cfg(any(test, feature = "testing"))]
pub use player_character_service_port::MockPlayerCharacterServicePort;

// Story event service port - interface for story event operations
pub use story_event_service_port::StoryEventServicePort;
#[cfg(any(test, feature = "testing"))]
pub use story_event_service_port::MockStoryEventServicePort;

// Settings service port - interface for settings operations
pub use settings_service_port::{LlmConfig, SettingsServicePort};
#[cfg(any(test, feature = "testing"))]
pub use settings_service_port::MockSettingsServicePort;

// Prompt template service port - interface for prompt template operations
pub use prompt_template_service_port::{PromptTemplate, PromptTemplateServicePort, PromptTemplateSource as ServicePromptTemplateSource};
#[cfg(any(test, feature = "testing"))]
pub use prompt_template_service_port::MockPromptTemplateServicePort;

// Asset service port - interface for asset gallery operations
pub use asset_service_port::{AssetServicePort, CreateAssetRequest};
#[cfg(any(test, feature = "testing"))]
pub use asset_service_port::MockAssetServicePort;

// Workflow service port - interface for workflow configuration operations
pub use workflow_service_port::WorkflowServicePort;
#[cfg(any(test, feature = "testing"))]
pub use workflow_service_port::MockWorkflowServicePort;

// Generation service port - interface for asset generation operations
pub use generation_service_port::{GenerationRequest, GenerationServicePort};
#[cfg(any(test, feature = "testing"))]
pub use generation_service_port::MockGenerationServicePort;

// Challenge resolution service port - interface for challenge resolution operations
pub use challenge_resolution_service_port::{
    ChallengeResolutionServicePort, DiceRoll, PendingResolution, RollResult,
};
#[cfg(any(test, feature = "testing"))]
pub use challenge_resolution_service_port::MockChallengeResolutionServicePort;

// Staging service port - interface for NPC staging operations
pub use staging_service_port::{
    ApprovedNpc, StagedNpcProposal, StagingProposal, StagingServicePort,
};
#[cfg(any(test, feature = "testing"))]
pub use staging_service_port::MockStagingServicePort;

// LLM queue service port - interface for LLM request queue operations
pub use llm_queue_service_port::{
    ChallengeSuggestion, ConfidenceLevel, LlmQueueItem, LlmQueueRequest, LlmQueueServicePort,
    LlmRequestType, LlmResponse as LlmQueueResponse, NarrativeEventSuggestion,
    ProposedToolCall, SuggestionContext as LlmSuggestionContext,
};
#[cfg(any(test, feature = "testing"))]
pub use llm_queue_service_port::MockLlmQueueServicePort;

// Player action queue service port - interface for player action queue operations
pub use player_action_queue_service_port::{
    PlayerAction, PlayerActionQueueItem, PlayerActionQueueServicePort,
};
#[cfg(any(test, feature = "testing"))]
pub use player_action_queue_service_port::MockPlayerActionQueueServicePort;

// DM approval queue service port - interface for DM approval queue operations
pub use dm_approval_queue_service_port::{
    ApprovalDecision, ApprovalDecisionType, ApprovalQueueItem, ApprovalRequest, ApprovalUrgency,
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, DmApprovalQueueServicePort,
    NarrativeEventSuggestionInfo, ProposedToolInfo,
};
#[cfg(any(test, feature = "testing"))]
pub use dm_approval_queue_service_port::MockDmApprovalQueueServicePort;

// DM action queue service port - interface for DM action queue operations
pub use dm_action_queue_service_port::{
    DmAction, DmActionQueueItem, DmActionQueueServicePort, DmActionType, DmDecision,
};
#[cfg(any(test, feature = "testing"))]
pub use dm_action_queue_service_port::MockDmActionQueueServicePort;

// Asset generation queue service port - interface for asset generation queue operations
pub use asset_generation_queue_service_port::{
    AssetGenerationQueueItem, AssetGenerationQueueServicePort, AssetGenerationRequest,
    GenerationMetadata as AssetGenerationMetadata, GenerationResult,
};
#[cfg(any(test, feature = "testing"))]
pub use asset_generation_queue_service_port::MockAssetGenerationQueueServicePort;

// Re-export mocks for test builds
#[cfg(any(test, feature = "testing"))]
pub use broadcast_port::MockBroadcastPort;
#[cfg(any(test, feature = "testing"))]
pub use repository_port::MockChallengeRepositoryPort;
#[cfg(any(test, feature = "testing"))]
pub use world_state_port::MockWorldStatePort;
