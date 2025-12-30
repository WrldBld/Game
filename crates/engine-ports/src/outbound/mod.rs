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
mod challenge_outcome_approval_service_port;
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
mod environment_port;
mod event_bus_port;
mod event_chain_service_port;
mod event_effect_executor_port;
mod event_notifier_port;
mod file_storage_port;
mod game_events;
mod generation_queue_projection_service_port;
mod generation_read_state_port;
mod generation_service_port;
mod interaction_service_port;
mod item_service_port;
mod llm_port;
mod llm_queue_service_port;
mod location_service_port;
mod narrative_event_approval_service_port;
mod narrative_event_repository;
mod narrative_event_service_port;
mod challenge_repository;
mod character_repository;
mod event_chain_repository;
mod location_repository;
mod player_character_repository;
mod region_repository;
mod scene_repository;
mod story_event_repository;
mod player_action_queue_service_port;
mod player_character_service_port;
mod prompt_context_service_port;
mod prompt_template_port;
mod prompt_template_service_port;
mod queue_notification_port;
mod queue_port;
mod random_port;
mod region_service_port;
mod relationship_service_port;
mod repository_port;
mod scene_resolution_service_port;
mod scene_service_port;
mod settings_port;
mod settings_service_port;
mod sheet_template_service_port;
mod skill_service_port;
mod staging_repository_port;
mod staging_service_port;
mod story_event_service_port;
mod suggestion_enqueue_port;
mod trigger_evaluation_service_port;
mod use_case_types;
mod workflow_service_port;
mod world_connection_manager_port;
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

// Random port - RNG abstraction for deterministic testing
pub use random_port::RandomPort;
#[cfg(any(test, feature = "testing"))]
pub use random_port::MockRandomPort;

// DomainEvent repository - domain-layer interface for event storage
pub use domain_event_repository_port::{DomainEventRepositoryError, DomainEventRepositoryPort};

// Environment port - interface for environment variable access
pub use environment_port::EnvironmentPort;

// DM action processor - interface for processing DM actions
pub use dm_action_processor_port::{DmActionProcessorPort, DmActionResult};

pub use comfyui_port::{
    ComfyUIPort, GeneratedImage, HistoryResponse, NodeOutput, PromptHistory, PromptStatus,
    QueuePromptResponse,
};

pub use event_bus_port::{EventBusError, EventBusPort};

// Event chain service port - interface for event chain (story arc) operations
pub use event_chain_service_port::EventChainServicePort;
#[cfg(any(test, feature = "testing"))]
pub use event_chain_service_port::MockEventChainServicePort;

// Event notifier port - interface for in-process event notification
pub use event_notifier_port::EventNotifierPort;
#[cfg(any(test, feature = "testing"))]
pub use event_notifier_port::MockEventNotifierPort;

// File storage port - interface for file system operations
pub use file_storage_port::FileStoragePort;

#[cfg(any(test, feature = "testing"))]
pub use generation_queue_projection_service_port::MockGenerationQueueProjectionServicePort;
pub use generation_queue_projection_service_port::{
    GenerationBatchSnapshot, GenerationQueueProjectionServicePort, GenerationQueueSnapshot,
    SuggestionTaskSnapshot,
};

pub use generation_read_state_port::{GenerationReadKind, GenerationReadStatePort};

pub use llm_port::{
    ChatMessage, FinishReason, ImageData, LlmPort, LlmRequest, LlmResponse, MessageRole,
    TokenUsage, ToolCall, ToolDefinition,
};

pub use queue_notification_port::{QueueNotificationPort, WaitResult};

// Repository ports - Note: CharacterRepositoryPort and ChallengeRepositoryPort have been
// split into ISP sub-traits (see character_repository/ and challenge_repository/)
pub use repository_port::{
    AssetRepositoryPort, CharacterNode, ContainerInfo, EventChainRepositoryPort, FlagRepositoryPort,
    GoalRepositoryPort, InteractionRepositoryPort, ItemRepositoryPort, LocationRepositoryPort,
    ObservationRepositoryPort, PlayerCharacterRepositoryPort, RegionRepositoryPort,
    RelationshipEdge, RelationshipRepositoryPort, SceneRepositoryPort, SheetTemplateRepositoryPort,
    SkillRepositoryPort, SocialNetwork, WantRepositoryPort, WorkflowRepositoryPort,
    WorldRepositoryPort,
};

// StoryEvent repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - StoryEventCrudPort: Core CRUD + state management (7 methods)
// - StoryEventEdgePort: Edge relationship management (15 methods)
// - StoryEventQueryPort: Query operations (10 methods)
// - StoryEventDialoguePort: Dialogue-specific operations (2 methods)
pub use story_event_repository::{
    StoryEventCrudPort, StoryEventDialoguePort, StoryEventEdgePort, StoryEventQueryPort,
};
#[cfg(any(test, feature = "testing"))]
pub use story_event_repository::MockStoryEventRepository;

// NarrativeEvent repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - NarrativeEventCrudPort: Core CRUD + state management (12 methods)
// - NarrativeEventTiePort: Scene/Location/Act relationships (9 methods)
// - NarrativeEventNpcPort: Featured NPC management (5 methods)
// - NarrativeEventQueryPort: Query by relationships (4 methods)
pub use narrative_event_repository::{
    NarrativeEventCrudPort, NarrativeEventNpcPort, NarrativeEventQueryPort, NarrativeEventTiePort,
};
#[cfg(any(test, feature = "testing"))]
pub use narrative_event_repository::MockNarrativeEventRepository;

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
pub use challenge_repository::{
    ChallengeAvailabilityPort, ChallengeCrudPort, ChallengePrerequisitePort, ChallengeScenePort,
    ChallengeSkillPort,
};
#[cfg(any(test, feature = "testing"))]
pub use challenge_repository::MockChallengeRepository;

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
pub use region_repository::{
    RegionConnectionPort, RegionCrudPort, RegionExitPort, RegionItemPort, RegionNpcPort,
};
#[cfg(any(test, feature = "testing"))]
pub use region_repository::{
    MockRegionConnectionPort, MockRegionCrudPort, MockRegionExitPort, MockRegionItemPort,
    MockRegionNpcPort, MockRegionRepository,
};

// PlayerCharacter repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - PlayerCharacterCrudPort: Core CRUD operations (5 methods)
// - PlayerCharacterQueryPort: Query/lookup operations (4 methods)
// - PlayerCharacterPositionPort: Position/movement operations (3 methods)
// - PlayerCharacterInventoryPort: Inventory management (5 methods)
pub use player_character_repository::{
    PlayerCharacterCrudPort, PlayerCharacterInventoryPort, PlayerCharacterPositionPort,
    PlayerCharacterQueryPort,
};
#[cfg(any(test, feature = "testing"))]
pub use player_character_repository::MockPlayerCharacterRepository;

// Scene repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - SceneCrudPort: Core CRUD operations (5 methods)
// - SceneQueryPort: Query by act/location (2 methods)
// - SceneLocationPort: AT_LOCATION edge management (2 methods)
// - SceneFeaturedCharacterPort: FEATURES_CHARACTER edges (5 methods)
// - SceneCompletionPort: COMPLETED_SCENE tracking (3 methods)
pub use scene_repository::{
    SceneCompletionPort, SceneCrudPort, SceneFeaturedCharacterPort, SceneLocationPort,
    SceneQueryPort,
};
#[cfg(any(test, feature = "testing"))]
pub use scene_repository::MockSceneRepository;

// EventChain repository ports - split for Interface Segregation Principle (Clean ISP)
// Services should depend only on the specific traits they need:
// - EventChainCrudPort: Core CRUD operations (4 methods)
// - EventChainQueryPort: Query/lookup operations (4 methods)
// - EventChainMembershipPort: Event membership management (3 methods)
// - EventChainStatePort: Status and state management (5 methods)
pub use event_chain_repository::{
    EventChainCrudPort, EventChainMembershipPort, EventChainQueryPort, EventChainStatePort,
};
#[cfg(any(test, feature = "testing"))]
pub use event_chain_repository::MockEventChainRepository;

pub use prompt_template_port::{
    PromptTemplateError, PromptTemplateRepositoryPort, PromptTemplateSource, ResolvedPromptTemplate,
};

pub use settings_port::{SettingsError, SettingsRepositoryPort};

pub use staging_repository_port::{StagedNpcRow, StagingRepositoryPort};

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

// Narrative event service port - interface for narrative event operations
pub use narrative_event_service_port::NarrativeEventServicePort;

// Scene service port - interface for scene operations
pub use scene_service_port::{SceneServicePort, SceneWithRelations};

// Disposition service port - interface for NPC disposition operations
pub use disposition_service_port::DispositionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use disposition_service_port::MockDispositionServicePort;

// Skill service port - interface for skill operations
#[cfg(any(test, feature = "testing"))]
pub use skill_service_port::MockSkillServicePort;
pub use skill_service_port::{CreateSkillRequest, SkillServicePort, UpdateSkillRequest};

// Interaction service port - interface for interaction operations
pub use interaction_service_port::InteractionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use interaction_service_port::MockInteractionServicePort;

// World service port - interface for world operations
#[cfg(any(test, feature = "testing"))]
pub use world_service_port::MockWorldServicePort;
pub use world_service_port::WorldServicePort;

// Character service port - interface for character operations
pub use character_service_port::CharacterServicePort;
#[cfg(any(test, feature = "testing"))]
pub use character_service_port::MockCharacterServicePort;

// Location service port - interface for location operations
pub use location_service_port::LocationServicePort;
#[cfg(any(test, feature = "testing"))]
pub use location_service_port::MockLocationServicePort;

// Region service port - interface for region operations
#[cfg(any(test, feature = "testing"))]
pub use region_service_port::MockRegionServicePort;
pub use region_service_port::RegionServicePort;

// Relationship service port - interface for relationship operations
#[cfg(any(test, feature = "testing"))]
pub use relationship_service_port::MockRelationshipServicePort;
pub use relationship_service_port::RelationshipServicePort;

// Scene resolution service port - interface for scene resolution operations
#[cfg(any(test, feature = "testing"))]
pub use scene_resolution_service_port::MockSceneResolutionServicePort;
pub use scene_resolution_service_port::{SceneResolutionResult, SceneResolutionServicePort};

// Sheet template service port - interface for character sheet template operations
#[cfg(any(test, feature = "testing"))]
pub use sheet_template_service_port::MockSheetTemplateServicePort;
pub use sheet_template_service_port::SheetTemplateServicePort;

pub use broadcast_port::BroadcastPort;
pub use game_events::{
    GameEvent, ItemInfo, LocationGroup, NavigationExit, NavigationInfo, NavigationTarget,
    NpcPresenceData, OutcomeBranchInfo, OutcomeTriggerInfo, PcLocationData, PreviousStagingData,
    RegionInfo, RegionItemData, SceneChangedEvent, SplitPartyEvent, StagedNpcData,
    StagingPendingEvent, StagingReadyEvent, StagingRequiredEvent, StateChangeInfo, WaitingPcData,
};
#[cfg(any(test, feature = "testing"))]
pub use world_connection_manager_port::MockWorldConnectionManagerPort;
pub use world_connection_manager_port::{
    ConnectedUserInfo, ConnectionContext, ConnectionManagerError, ConnectionStats, DmInfo,
    WorldConnectionManagerPort,
};

pub use world_state_port::WorldStatePort;

// Item service port - interface for item operations
pub use item_service_port::ItemServicePort;
#[cfg(any(test, feature = "testing"))]
pub use item_service_port::MockItemServicePort;

// Player character service port - interface for player character operations
#[cfg(any(test, feature = "testing"))]
pub use player_character_service_port::MockPlayerCharacterServicePort;
pub use player_character_service_port::PlayerCharacterServicePort;

// Story event service port - interface for story event operations
#[cfg(any(test, feature = "testing"))]
pub use story_event_service_port::MockStoryEventServicePort;
pub use story_event_service_port::StoryEventServicePort;

// Settings service port - interface for settings operations
#[cfg(any(test, feature = "testing"))]
pub use settings_service_port::MockSettingsServicePort;
pub use settings_service_port::{LlmConfig, SettingsServicePort};

// Prompt template service port - interface for prompt template operations
#[cfg(any(test, feature = "testing"))]
pub use prompt_template_service_port::MockPromptTemplateServicePort;
pub use prompt_template_service_port::PromptTemplateServicePort;

// Prompt context service port - interface for building LLM prompt context
#[cfg(any(test, feature = "testing"))]
pub use prompt_context_service_port::MockPromptContextServicePort;
pub use prompt_context_service_port::{PromptContextError, PromptContextServicePort};

// Asset service port - interface for asset gallery operations
#[cfg(any(test, feature = "testing"))]
pub use asset_service_port::MockAssetServicePort;
pub use asset_service_port::{AssetServicePort, CreateAssetRequest};

// Workflow service port - interface for workflow configuration operations
// NOTE: Workflow utility functions (analyze_workflow, validate_workflow, etc.) are in
// engine-app::application::services::WorkflowService, not in this port.
#[cfg(any(test, feature = "testing"))]
pub use workflow_service_port::MockWorkflowServicePort;
pub use workflow_service_port::WorkflowServicePort;

// Generation service port - interface for asset generation operations
#[cfg(any(test, feature = "testing"))]
pub use generation_service_port::MockGenerationServicePort;
pub use generation_service_port::{GenerationRequest, GenerationServicePort};

// Challenge resolution service port - interface for challenge resolution operations
#[cfg(any(test, feature = "testing"))]
pub use challenge_resolution_service_port::MockChallengeResolutionServicePort;
pub use challenge_resolution_service_port::{
    ChallengeResolutionServicePort, DiceRoll, PendingResolution, RollResult,
};

// Staging service port - interface for NPC staging operations
#[cfg(any(test, feature = "testing"))]
pub use staging_service_port::MockStagingServicePort;
pub use staging_service_port::{
    ApprovedNpc, StagedNpcProposal, StagingProposal, StagingServicePort,
};

// LLM queue service port - interface for LLM request queue operations
#[cfg(any(test, feature = "testing"))]
pub use llm_queue_service_port::MockLlmQueueServicePort;
pub use llm_queue_service_port::{
    ChallengeSuggestion, ConfidenceLevel, LlmQueueItem, LlmQueueRequest, LlmQueueServicePort,
    LlmRequestType, LlmResponse as LlmQueueResponse, NarrativeEventSuggestion, ProposedToolCall,
    SuggestionContext as LlmSuggestionContext,
};

// Player action queue service port - interface for player action queue operations
#[cfg(any(test, feature = "testing"))]
pub use player_action_queue_service_port::MockPlayerActionQueueServicePort;
pub use player_action_queue_service_port::{
    PlayerAction, PlayerActionQueueItem, PlayerActionQueueServicePort,
};

// DM approval queue service port - interface for DM approval queue operations
#[cfg(any(test, feature = "testing"))]
pub use dm_approval_queue_service_port::MockDmApprovalQueueServicePort;
pub use dm_approval_queue_service_port::{
    ApprovalDecisionType, ApprovalQueueItem, ApprovalRequest, ApprovalUrgency,
    DmApprovalDecision, DmApprovalQueueServicePort,
};
// Re-export protocol types for API compatibility
pub use wrldbldr_protocol::{
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, NarrativeEventSuggestionInfo,
    ProposedToolInfo,
};

// DM action queue service port - interface for DM action queue operations
#[cfg(any(test, feature = "testing"))]
pub use dm_action_queue_service_port::MockDmActionQueueServicePort;
pub use dm_action_queue_service_port::{
    DmAction, DmActionQueueItem, DmActionQueueServicePort, DmActionType, DmDecision,
};

// Asset generation queue service port - interface for asset generation queue operations
#[cfg(any(test, feature = "testing"))]
pub use asset_generation_queue_service_port::MockAssetGenerationQueueServicePort;
pub use asset_generation_queue_service_port::{
    AssetGenerationQueueItem, AssetGenerationQueueServicePort, AssetGenerationRequest,
    GenerationMetadata as AssetGenerationMetadata, GenerationResult,
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

// Challenge outcome approval service port - interface for DM approval of challenge resolutions
#[cfg(any(test, feature = "testing"))]
pub use challenge_outcome_approval_service_port::MockChallengeOutcomeApprovalServicePort;
pub use challenge_outcome_approval_service_port::{
    ChallengeApprovalResult, ChallengeOutcomeApprovalServicePort,
    OutcomeBranchInfo as ApprovalOutcomeBranchInfo, ResolvedOutcome,
    StateChangeInfo as ApprovalStateChangeInfo,
};

// Narrative event approval service port - interface for DM approval of narrative events
#[cfg(any(test, feature = "testing"))]
pub use narrative_event_approval_service_port::MockNarrativeEventApprovalServicePort;
pub use narrative_event_approval_service_port::{
    NarrativeEventApprovalServicePort, NarrativeEventTriggerResult,
};

// Trigger evaluation service port - interface for evaluating narrative event triggers
#[cfg(any(test, feature = "testing"))]
pub use trigger_evaluation_service_port::MockTriggerEvaluationServicePort;
pub use trigger_evaluation_service_port::{
    CompletedChallenge, CompletedNarrativeEvent, GameStateSnapshot, ImmediateContext,
    TriggerEvaluationResult, TriggerEvaluationServicePort, TriggerSource, TriggeredEventCandidate,
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
pub use world_state_port::MockWorldStatePort;
