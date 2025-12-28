//! Outbound ports - interfaces the engine application requires.
//!
//! This crate is being extracted from `wrldbldr-engine-app`. For now, we only
//! expose ports that do not depend on engine-app-internal domain/DTO types.
//!
//! Ports that still depend on engine-app internals remain in `wrldbldr-engine-app`
//! until the shared types move into `wrldbldr-domain`/`wrldbldr-protocol`.

mod broadcast_port;
mod comfyui_port;
mod directorial_context_port;
mod domain_event_repository_port;
mod event_bus_port;
mod game_events;
mod generation_read_state_port;
mod llm_port;
mod queue_notification_port;
mod queue_port;
mod repository_port;
mod prompt_template_port;
mod settings_port;
mod staging_repository_port;
mod suggestion_enqueue_port;
mod world_exporter_port;

// DomainEvent repository - domain-layer interface for event storage
pub use domain_event_repository_port::{DomainEventRepositoryError, DomainEventRepositoryPort};

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

pub use broadcast_port::BroadcastPort;
pub use game_events::*;

// Re-export mocks for test builds
#[cfg(any(test, feature = "testing"))]
pub use broadcast_port::MockBroadcastPort;
#[cfg(any(test, feature = "testing"))]
pub use repository_port::MockChallengeRepositoryPort;
