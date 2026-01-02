//! Outbound ports - interfaces the engine application requires.
//!
//! This crate is being extracted from `wrldbldr-engine-app`. For now, we only
//! expose ports that do not depend on engine-app-internal domain/DTO types.
//!
//! Ports that still depend on engine-app internals remain in `wrldbldr-engine-app`
//! until the shared types move into `wrldbldr-domain`/`wrldbldr-protocol`.

mod approval_request_lookup_port;
mod broadcast_port;

mod challenge_outcome_pending_port;
mod challenge_repository;
mod character_repository;
mod clock_port;
mod comfyui_port;
mod connection_manager_port;
mod directorial_context_dto_repository_port;
mod directorial_context_port;
mod dm_action_enqueue_port;
mod dm_action_processor_port;
mod dm_notification_port;
mod domain_event_repository_port;
mod environment_port;
mod event_bus_port;
mod event_chain_repository;
mod event_effect_executor_port;
mod event_notifier_port;
mod file_storage_port;
mod game_events;
mod generation_active_batches_port;
mod generation_read_state_port;

mod llm_port;
mod llm_suggestion_queue_port;
mod location_repository;
mod narrative_event_repository;
mod player_character_repository;

mod prompt_template_cache_port;
mod prompt_template_port;
mod queue_notification_port;
mod queue_port;
mod random_port;
mod region_repository;
mod repository_port;
mod scene_dm_action_queue_port;
mod scene_repository;
mod settings_cache_port;
mod settings_port;
mod staging_repository_port;

mod staging_state_ports;
mod staging_use_case_service_ports;
mod state_change;
mod story_event_repository;
mod suggestion_enqueue_port;
mod use_case_errors;
mod use_case_types;
mod world_connection_manager;
mod world_exporter_port;
mod world_state;
mod world_state_update_port;

pub use approval_request_lookup_port::ApprovalRequestLookupPort;
#[cfg(any(test, feature = "testing"))]
pub use approval_request_lookup_port::MockApprovalRequestLookupPort;

// State change DTOs (tool/trigger execution results)
pub use state_change::StateChange;

// Clock port - time abstraction for deterministic testing
pub use clock_port::ClockPort;

// Random port - RNG abstraction for deterministic testing
#[cfg(any(test, feature = "testing"))]
pub use random_port::MockRandomPort;
pub use random_port::RandomPort;

pub use use_case_errors::{
    ActionError, ChallengeError, InventoryError, NarrativeEventError, ObservationError, SceneError,
    StagingError,
};

// DomainEvent repository - domain-layer interface for event storage
pub use domain_event_repository_port::{DomainEventRepositoryError, DomainEventRepositoryPort};

// Environment port - interface for environment variable access
pub use environment_port::EnvironmentPort;

// DM action enqueue - outbound port for enqueueing DM actions
pub use dm_action_enqueue_port::{
    DmActionEnqueuePort, DmActionEnqueueRequest, DmActionEnqueueType, DmEnqueueDecision,
};

// DM action processor - interface for processing DM actions
pub use dm_action_processor_port::{DmActionProcessorPort, DmActionResult};

pub use dm_notification_port::DmNotificationPort;

pub use comfyui_port::{
    ComfyUIPort, GeneratedImage, HistoryResponse, NodeOutput, PromptHistory, PromptStatus,
    QueuePromptResponse,
};

pub use connection_manager_port::ConnectionManagerPort;

pub use challenge_outcome_pending_port::ChallengeOutcomePendingPort;

pub use generation_active_batches_port::{
    ActiveGenerationBatch, ActiveGenerationBatchesPort, ActiveGenerationBatchesSnapshot,
};

pub use event_bus_port::{EventBusError, EventBusPort};

// Event notifier port - interface for in-process event notification
pub use event_notifier_port::EventNotifierPort;
#[cfg(any(test, feature = "testing"))]
pub use event_notifier_port::MockEventNotifierPort;

// File storage port - interface for file system operations
pub use file_storage_port::FileStoragePort;



pub use generation_read_state_port::{GenerationReadKind, GenerationReadStatePort};

pub use llm_port::{
    ChatMessage, FinishReason, ImageData, LlmPort, LlmRequest, LlmResponse, MessageRole,
    TokenUsage, ToolCall, ToolDefinition,
};

// LLM Suggestion Queue port - outbound interface for LLM suggestion queue operations
pub use llm_suggestion_queue_port::{LlmSuggestionQueuePort, LlmSuggestionQueueRequest};
#[cfg(any(test, feature = "testing"))]
pub use llm_suggestion_queue_port::MockLlmSuggestionQueuePort;

pub use prompt_template_cache_port::{PromptTemplateCachePort, PromptTemplateCacheSnapshot};

pub use queue_notification_port::{QueueNotificationPort, WaitResult};

pub use settings_cache_port::{SettingsCachePort, SettingsCacheSnapshot};

// Repository ports - Note: Many repository ports have been split into ISP sub-traits.
// See the *_repository/ modules for the focused trait definitions.
// God traits have been removed for: Location, Region, EventChain, Scene, PlayerCharacter.
// Use ISP traits from the respective *_repository/ modules instead.
pub use repository_port::{
    AssetRepositoryPort, CharacterNode, ContainerInfo, FlagRepositoryPort, GoalRepositoryPort,
    InteractionRepositoryPort, ItemRepositoryPort, ObservationRepositoryPort, RelationshipEdge,
    RelationshipRepositoryPort, SheetTemplateRepositoryPort, SkillRepositoryPort, SocialNetwork,
    WantRepositoryPort, WorkflowRepositoryPort, WorldRepositoryPort,
};

// StoryEvent repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - StoryEventCrudPort: Core CRUD + state management (7 methods)
// - StoryEventEdgePort: Edge relationship management (15 methods)
// - StoryEventQueryPort: Query operations (10 methods)
// - StoryEventDialoguePort: Dialogue-specific operations (2 methods)
#[cfg(any(test, feature = "testing"))]
pub use story_event_repository::MockStoryEventRepository;
pub use story_event_repository::{
    StoryEventCrudPort, StoryEventDialoguePort, StoryEventEdgePort, StoryEventQueryPort,
};

// NarrativeEvent repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - NarrativeEventCrudPort: Core CRUD + state management (12 methods)
// - NarrativeEventTiePort: Scene/Location/Act relationships (9 methods)
// - NarrativeEventNpcPort: Featured NPC management (5 methods)
// - NarrativeEventQueryPort: Query by relationships (4 methods)
#[cfg(any(test, feature = "testing"))]
pub use narrative_event_repository::MockNarrativeEventRepository;
pub use narrative_event_repository::{
    NarrativeEventCrudPort, NarrativeEventNpcPort, NarrativeEventQueryPort, NarrativeEventTiePort,
};

// Character repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - CharacterCrudPort: Core CRUD operations (6 methods)
// - CharacterWantPort: Want management (7 methods)
// - CharacterActantialPort: Actantial view management (5 methods)
// - CharacterInventoryPort: Inventory management (5 methods)
// - CharacterLocationPort: Location relationships (13 methods)
// - CharacterDispositionPort: NPC disposition tracking (6 methods)
pub use character_repository::{
    CharacterActantialPort, CharacterCrudPort, CharacterDispositionPort, CharacterInventoryPort,
    CharacterLocationPort, CharacterWantPort,
};
#[cfg(any(test, feature = "testing"))]
pub use character_repository::{
    MockCharacterActantialPort, MockCharacterCrudPort, MockCharacterDispositionPort,
    MockCharacterInventoryPort, MockCharacterLocationPort, MockCharacterRepository,
    MockCharacterWantPort,
};

// Challenge repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - ChallengeCrudPort: Core CRUD + state management (12 methods)
// - ChallengeSkillPort: Skill relationship management (3 methods)
// - ChallengeScenePort: Scene relationship management (3 methods)
// - ChallengePrerequisitePort: Prerequisite chain management (4 methods)
// - ChallengeAvailabilityPort: Location/region availability + unlocks (9 methods)
#[cfg(any(test, feature = "testing"))]
pub use challenge_repository::MockChallengeRepository;
pub use challenge_repository::{
    ChallengeAvailabilityPort, ChallengeCrudPort, ChallengePrerequisitePort, ChallengeScenePort,
    ChallengeSkillPort,
};

// Location repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - LocationCrudPort: Core CRUD operations (5 methods)
// - LocationHierarchyPort: Parent-child relationships (4 methods)
// - LocationConnectionPort: Navigation connections (5 methods)
// - LocationMapPort: Grid maps and regions (5 methods)
pub use location_repository::{
    LocationConnectionPort, LocationCrudPort, LocationHierarchyPort, LocationMapPort,
};
#[cfg(any(test, feature = "testing"))]
pub use location_repository::{
    MockLocationConnectionPort, MockLocationCrudPort, MockLocationHierarchyPort,
    MockLocationMapPort, MockLocationRepository,
};

// Region repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - RegionCrudPort: Core CRUD operations (5 methods)
// - RegionConnectionPort: Region-to-region connections (4 methods)
// - RegionExitPort: Region-to-location exits (3 methods)
// - RegionNpcPort: NPC relationship queries (1 method)
// - RegionItemPort: Item placement in regions (3 stub methods)
#[cfg(any(test, feature = "testing"))]
pub use region_repository::{
    MockRegionConnectionPort, MockRegionCrudPort, MockRegionExitPort, MockRegionItemPort,
    MockRegionNpcPort, MockRegionRepository,
};
pub use region_repository::{
    RegionConnectionPort, RegionCrudPort, RegionExitPort, RegionItemPort, RegionNpcPort,
};

// PlayerCharacter repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - PlayerCharacterCrudPort: Core CRUD operations (5 methods)
// - PlayerCharacterQueryPort: Query/lookup operations (4 methods)
// - PlayerCharacterPositionPort: Position/movement operations (3 methods)
// - PlayerCharacterInventoryPort: Inventory management (5 methods)
#[cfg(any(test, feature = "testing"))]
pub use player_character_repository::MockPlayerCharacterRepository;
pub use player_character_repository::{
    PlayerCharacterCrudPort, PlayerCharacterInventoryPort, PlayerCharacterPositionPort,
    PlayerCharacterQueryPort,
};

// Scene repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - SceneCrudPort: Core CRUD operations (5 methods)
// - SceneQueryPort: Query by act/location (2 methods)
// - SceneLocationPort: AT_LOCATION edge management (2 methods)
// - SceneFeaturedCharacterPort: FEATURES_CHARACTER edges (5 methods)
// - SceneCompletionPort: COMPLETED_SCENE tracking (3 methods)
#[cfg(any(test, feature = "testing"))]
pub use scene_repository::MockSceneRepository;
pub use scene_repository::{
    SceneCompletionPort, SceneCrudPort, SceneFeaturedCharacterPort, SceneLocationPort,
    SceneQueryPort,
};

// EventChain repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - EventChainCrudPort: Core CRUD operations (4 methods)
// - EventChainQueryPort: Query/lookup operations (4 methods)
// - EventChainMembershipPort: Event membership management (3 methods)
// - EventChainStatePort: Status and state management (5 methods)
#[cfg(any(test, feature = "testing"))]
pub use event_chain_repository::MockEventChainRepository;
pub use event_chain_repository::{
    EventChainCrudPort, EventChainMembershipPort, EventChainQueryPort, EventChainStatePort,
};

pub use prompt_template_port::{
    PromptTemplateError, PromptTemplateRepositoryPort, PromptTemplateSource, ResolvedPromptTemplate,
};

pub use settings_port::{SettingsError, SettingsRepositoryPort};

pub use staging_repository_port::{StagedNpcRow, StagingRepositoryPort};

// Staging use-case dependency ports
pub use staging_state_ports::{StagingStateExtPort, StagingStatePort};
pub use staging_use_case_service_ports::{StagingUseCaseServiceExtPort, StagingUseCaseServicePort};

pub use suggestion_enqueue_port::{
    SuggestionEnqueueContext, SuggestionEnqueuePort, SuggestionEnqueueRequest,
    SuggestionEnqueueResponse,
};

// Queue port - generic queue interface for domain payloads (no serde bounds)
pub use queue_port::{
    ApprovalQueuePort, ProcessingQueuePort, QueueError, QueueItem, QueueItemId, QueueItemStatus,
    QueuePort,
};

pub use world_exporter_port::{
    CharacterData, ExportOptions, LocationData, PlayerWorldSnapshot, SceneData, WorldData,
    WorldExporterPort,
};

pub use directorial_context_port::DirectorialContextRepositoryPort;

pub use directorial_context_dto_repository_port::DirectorialContextDtoRepositoryPort;

// Minimal world state updates used by use cases
pub use world_state_update_port::WorldStateUpdatePort;



// Scene use-case DTO query ports
pub use scene_dm_action_queue_port::SceneDmActionQueuePort;

pub use broadcast_port::BroadcastPort;
pub use game_events::{
    GameEvent, ItemInfo, LocationGroup, NavigationExit, NavigationInfo, NavigationTarget,
    NpcPresenceData, OutcomeBranchInfo, OutcomeTriggerInfo, PcLocationData, PreviousStagingData,
    RegionInfo, RegionItemData, SceneChangedEvent, SplitPartyEvent, StagedNpcData,
    StagingPendingEvent, StagingReadyEvent, StagingRequiredEvent, StateChangeInfo, WaitingPcData,
};
// WorldConnectionManager ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - ConnectionQueryPort: Query connection state (8 methods)
// - ConnectionContextPort: Resolve client/connection context (7 methods)
// - ConnectionBroadcastPort: Broadcast messages (4 methods)
// - ConnectionLifecyclePort: Connection lifecycle (1 method)
#[cfg(any(test, feature = "testing"))]
pub use world_connection_manager::MockWorldConnectionManager;
pub use world_connection_manager::{
    ConnectedUserInfo, ConnectionBroadcastPort, ConnectionContext, ConnectionContextPort,
    ConnectionLifecyclePort, ConnectionManagerError, ConnectionQueryPort, ConnectionStats,
    ConnectionUnicastPort, DmInfo,
};

// WorldState ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - WorldTimePort: Game time management (3 methods)
// - WorldConversationPort: Conversation history (3 methods)
// - WorldApprovalPort: Pending DM approvals (3 methods)
// - WorldScenePort: Current scene tracking (2 methods)
// - WorldDirectorialPort: DM directorial context (3 methods)
// - WorldLifecyclePort: World initialization/cleanup (3 methods)
pub use world_state::{
    WorldApprovalPort, WorldConversationPort, WorldDirectorialPort, WorldLifecyclePort,
    WorldScenePort, WorldStatePort, WorldTimePort,
};

// Use case types - input/output types for use case operations
pub use use_case_types::{
    // Player action types
    ActionResult,
    // Challenge types
    AdHocOutcomes,
    AdHocResult,
    // Observation types
    ApproachEventData,
    ApprovalItem,
    // Staging types
    ApproveInput,
    ApproveResult,
    ApprovedNpcData,
    ApprovedNpcInput,
    ChallengeSuggestionDecisionInput,
    CharacterEntity,
    // Connection types
    ConnectedUser,
    // Error types
    ConnectionError,
    ConnectionInfo,
    CreateAdHocInput,
    DiceInputType,
    DirectorialContextData,
    DirectorialUpdateResult,
    DiscardChallengeInput,
    DiscardResult,
    DmAction as SceneDmAction,
    // Inventory types
    DropInput,
    DropResult,
    EquipInput,
    EquipResult,
    ErrorCode,
    // Movement types
    ExitToLocationInput,
    InteractionEntity,
    InteractionTarget,
    JoinWorldInput,
    JoinWorldResult,
    LeaveWorldResult,
    LocationEntity,
    LocationEventData,
    MoveToRegionInput,
    MovementError,
    MovementResult,
    // Narrative event types
    NarrativeEventDecisionResult,
    NarrativeEventSuggestionDecisionInput,
    NarrativeRollContext,
    NpcMotivation,
    OutcomeDecision,
    OutcomeDecisionInput,
    OutcomeDecisionResult,
    OutcomeDetail,
    PcData,
    PendingStagingData,
    PendingStagingInfo,
    PickupInput,
    PickupResult,
    PlayerActionInput,
    PreStageInput,
    PreStageResult,
    ProposedNpc,
    RegenerateInput,
    RegenerateOutcomeInput,
    RegenerateResult,
    RegeneratedNpc,
    RequestBranchesInput,
    RequestSceneChangeInput,
    RequestSuggestionInput,
    RollResultData,
    // Scene types
    SceneApprovalDecision,
    SceneApprovalDecisionInput,
    SceneApprovalDecisionResult,
    SceneChangeResult,
    SceneCharacterData,
    SceneData as UseCaseSceneData,
    SceneEntity,
    SceneInteractionData,
    SceneWithRelations as UseCaseSceneWithRelations,
    SelectBranchInput,
    SelectCharacterInput,
    SelectCharacterResult,
    SetSpectateTargetInput,
    ShareNpcLocationInput,
    ShareNpcLocationResult,
    SpectateTargetResult,
    StagingApprovalSource,
    StagingProposalData,
    StagingRegenerateResult,
    SubmitDiceInputInput,
    SubmitRollInput,
    TimeContext,
    TriggerApproachInput,
    TriggerApproachResult,
    TriggerChallengeInput,
    TriggerInfo,
    TriggerLocationEventInput,
    TriggerLocationEventResult,
    TriggerResult,
    UnequipInput,
    UnequipResult,
    UpdateDirectorialInput,
    UserJoinedEvent,
    UserLeftEvent,
    WaitingPcInfo,
    WorldRole,
};



// Event effect executor port - interface for executing narrative event outcome effects
#[cfg(any(test, feature = "testing"))]
pub use event_effect_executor_port::MockEventEffectExecutorPort;
pub use event_effect_executor_port::{
    EffectExecutionResult, EventEffectExecutorPort, OutcomeExecutionResult,
};

// Re-export mocks for test builds
#[cfg(any(test, feature = "testing"))]
pub use broadcast_port::MockBroadcastPort;
#[cfg(any(test, feature = "testing"))]
pub use world_state::{
    MockWorldApprovalPort, MockWorldConversationPort, MockWorldDirectorialPort,
    MockWorldLifecyclePort, MockWorldScenePort, MockWorldStatePort, MockWorldTimePort,
};

// Queue payload types - re-exported from domain (canonical location per queue-system.md)
// Phase 1A redesign (2026-01-02): These are domain value objects, not infrastructure.
// See docs/plans/ARCHITECTURE_REMEDIATION_MASTER_PLAN.md for rationale.
//
// NOTE: LlmRequestType and DmActionType are already exported from their respective
// queue_service_port modules above. We only re-export the payload data types here.
// DmActionPayloadType aliases DmActionType for cases needing the full enum.
pub use wrldbldr_domain::value_objects::{
    ApprovalRequestData, AssetGenerationData, ChallengeOutcomeData, DmActionData, LlmRequestData,
    PlayerActionData, ProposedTool, SuggestionContext,
    // Domain types with different names to avoid conflict with LlmQueueServicePort exports
    ChallengeSuggestion as DomainChallengeSuggestion,
    ChallengeSuggestionOutcomes as DomainChallengeSuggestionOutcomes,
    NarrativeEventSuggestion as DomainNarrativeEventSuggestion,
    // DmActionType with full DmApprovalDecision (as DmActionPayloadType to avoid conflict
    // with simplified DmActionType from dm_action_queue_service_port)
    DmActionType as DmActionPayloadType,
};
