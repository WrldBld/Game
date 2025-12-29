//! Outbound ports - interfaces the engine application requires.
//!
//! This crate is being extracted from `wrldbldr-engine-app`. For now, we only
//! expose ports that do not depend on engine-app-internal domain/DTO types.
//!
//! Ports that still depend on engine-app internals remain in `wrldbldr-engine-app`
//! until the shared types move into `wrldbldr-domain`/`wrldbldr-protocol`.

mod actantial_context_service_port;
mod broadcast_port;
mod challenge_service_port;
mod clock_port;
mod comfyui_port;
mod directorial_context_port;
mod disposition_service_port;
mod dm_action_processor_port;
mod domain_event_repository_port;
mod event_bus_port;
mod game_events;
mod generation_read_state_port;
mod llm_port;
mod narrative_event_service_port;
mod queue_notification_port;
mod queue_port;
mod repository_port;
mod prompt_template_port;
mod scene_service_port;
mod settings_port;
mod skill_service_port;
mod staging_repository_port;
mod suggestion_enqueue_port;
mod world_exporter_port;
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

pub use broadcast_port::BroadcastPort;
pub use game_events::{
    GameEvent, ItemInfo, LocationGroup, NavigationExit, NavigationInfo, NavigationTarget,
    NpcPresenceData, OutcomeBranchInfo, OutcomeTriggerInfo, PcLocationData, PreviousStagingData,
    RegionInfo, RegionItemData, SceneChangedEvent, SplitPartyEvent, StagedNpcData,
    StagingPendingEvent, StagingReadyEvent, StagingRequiredEvent, StateChangeInfo, WaitingPcData,
};
pub use world_state_port::WorldStatePort;

// Re-export mocks for test builds
#[cfg(any(test, feature = "testing"))]
pub use broadcast_port::MockBroadcastPort;
#[cfg(any(test, feature = "testing"))]
pub use repository_port::MockChallengeRepositoryPort;
#[cfg(any(test, feature = "testing"))]
pub use world_state_port::MockWorldStatePort;
