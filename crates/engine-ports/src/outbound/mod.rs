//! Outbound ports - interfaces the engine application requires.
//!
//! This crate is being extracted from `wrldbldr-engine-app`. For now, we only
//! expose ports that do not depend on engine-app-internal domain/DTO types.
//!
//! Ports that still depend on engine-app internals remain in `wrldbldr-engine-app`
//! until the shared types move into `wrldbldr-domain`/`wrldbldr-protocol`.

mod app_event_repository_port;
mod async_session_port;
mod comfyui_port;
mod event_bus_port;
mod generation_read_state_port;
mod llm_port;
mod queue_notification_port;
mod queue_port;
mod repository_port;
mod session_management_port;
mod prompt_template_port;
mod settings_port;
mod staging_repository_port;
mod world_exporter_port;

pub use app_event_repository_port::{AppEventRepositoryError, AppEventRepositoryPort};

pub use async_session_port::{
    AsyncSessionError, AsyncSessionPort, SessionJoinInfo, SessionParticipantInfo,
    SessionParticipantRole, SessionWorldData,
};

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

pub use queue_port::{
    ApprovalQueuePort, ProcessingQueuePort, QueueError, QueueItem, QueueItemId, QueueItemStatus,
    QueuePort,
};

pub use session_management_port::{
    BroadcastMessage, CharacterContextInfo, ParticipantSummary, PendingApprovalInfo,
    SessionJoinResult, SessionLifecyclePort, SessionManagementError, SessionManagementPort,
    SessionWorldContext,
};

pub use world_exporter_port::{
    CharacterData, ExportOptions, LocationData, PlayerWorldSnapshot, SceneData, WorldData,
    WorldExporterPort,
};
