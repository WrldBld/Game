//! Application services - Use case implementations
//!
//! This module contains the application services that implement the use cases
//! for the WrldBldr Engine. Each service follows hexagonal architecture principles,
//! accepting repository dependencies and returning domain entities or DTOs.

// Internal service traits - NOT ports, just app-layer contracts
// These traits define contracts between services within the application layer.
pub mod internal;

pub mod actantial_context_service;
pub mod asset_generation_queue_service;
pub mod asset_service;
pub mod challenge_approval_event_publisher;
pub mod challenge_approval_events;
pub mod challenge_outcome_approval_service;
pub mod challenge_resolution_service;
pub mod challenge_service;
pub mod character_service;
pub mod disposition_service;
pub mod dm_action_processor_service;
pub mod dm_action_queue_service;
pub mod dm_approval_queue_service;
pub mod event_chain_service;
pub mod generation_event_publisher;
pub mod generation_queue_projection_service;
pub mod generation_service;
pub mod interaction_service;
pub mod item_service;
pub mod llm;
pub mod llm_queue_service;
pub mod location_service;
pub mod region_service;

// Re-export LLM service types for backward compatibility
pub mod event_effect_executor;
pub mod narrative_event_approval_service;
pub mod narrative_event_service;
pub mod outcome_suggestion_service;
pub mod outcome_trigger_service;
pub mod player_action_queue_service;
pub mod player_character_service;
pub mod prompt_context_service;
pub mod prompt_template_service;
pub mod relationship_service;
pub mod scene_resolution_service;
pub mod scene_service;
pub mod settings_service;
pub mod sheet_template_service;
pub mod skill_service;
pub mod staging_context_provider;
pub mod staging_service;
pub mod story_event_service;
pub mod suggestion_service;
pub mod tool_execution_service;
pub mod trigger_evaluation_service;
pub mod workflow_config_service;
pub mod workflow_service;
pub mod world_service;
pub mod world_session_policy;

// Note: ActiveNarrativeEventContext and GamePromptRequest are now in domain::value_objects

// Re-export world service types (used in HTTP routes and websocket)
pub use world_service::{
    CreateActRequest, CreateWorldRequest, UpdateWorldRequest, WorldService, WorldServiceImpl,
};

// Re-export scene service types
pub use scene_service::{CreateSceneRequest, SceneService, SceneServiceImpl, UpdateSceneRequest};

// Re-export scene resolution service types
pub use scene_resolution_service::{SceneResolutionService, SceneResolutionServiceImpl};

// Re-export character service types
pub use character_service::{
    ChangeArchetypeRequest, CharacterService, CharacterServiceImpl, CreateCharacterRequest,
    UpdateCharacterRequest,
};

// Re-export player character service types
pub use player_character_service::{
    CreatePlayerCharacterRequest, PlayerCharacterService, PlayerCharacterServiceImpl,
    UpdatePlayerCharacterRequest,
};

// Re-export location service types
pub use location_service::{
    CreateConnectionRequest, CreateLocationRequest, LocationService, LocationServiceImpl,
    UpdateLocationRequest,
};

// Re-export suggestion service types
pub use suggestion_service::{SuggestionContext, SuggestionService, SuggestionType};

// Re-export workflow services
pub use workflow_config_service::WorkflowConfigService;
pub use workflow_service::WorkflowService;

// ToolExecutionService is only used internally within services module, not re-exported

// Re-export story event service types
pub use story_event_service::{StoryEventService, StoryEventServiceImpl};

// Re-export narrative event approval service
pub use narrative_event_approval_service::{
    NarrativeEventApprovalError, NarrativeEventApprovalService, NarrativeEventTriggerResult,
};

// Re-export skill service types
pub use skill_service::{SkillService, SkillServiceImpl};
// Re-export request types from internal (canonical definitions after migration)
pub use internal::{CreateSkillRequest, UpdateSkillRequest};

// Re-export interaction service types (used in HTTP routes)
pub use interaction_service::{InteractionService, InteractionServiceImpl};

// Re-export item service types
pub use item_service::{
    CreateItemRequest, GiveItemRequest, GiveItemResult, ItemService, ItemServiceImpl,
};

// Re-export challenge service types (used in HTTP routes)
pub use challenge_service::{ChallengeService, ChallengeServiceImpl};

// Re-export challenge resolution service types
pub use challenge_resolution_service::{
    AdHocChallengeResult, ChallengeResolutionError, ChallengeResolutionService,
    ChallengeTriggerResult, OutcomeTriggerInfo, RollSubmissionResult,
};
// DiceInputType comes from engine-ports (canonical internal definition)
pub use wrldbldr_engine_ports::outbound::DiceInputType;

// Re-export relationship service types
pub use relationship_service::{RelationshipService, RelationshipServiceImpl};

// Re-export asset service types (used in HTTP routes)
pub use asset_service::{AssetService, AssetServiceImpl, CreateAssetRequest};

// Re-export sheet template service types
pub use sheet_template_service::SheetTemplateService;

// Re-export settings service types
pub use settings_service::{SettingsLoaderFn, SettingsService};

// Re-export prompt template service types
pub use prompt_template_service::PromptTemplateService;

// Re-export narrative event service types (used in HTTP routes)
pub use narrative_event_service::{NarrativeEventService, NarrativeEventServiceImpl};

// Re-export event chain service types (used in HTTP routes)
pub use event_chain_service::{EventChainService, EventChainServiceImpl};

// Re-export queue service types (used in infrastructure layer)
pub use asset_generation_queue_service::AssetGenerationQueueService;
pub use dm_action_processor_service::{ApprovalProcessorPort, DmActionProcessorService};
pub use dm_action_queue_service::DmActionQueueService;
pub use dm_approval_queue_service::{ApprovalOutcome, DMApprovalQueueService};
pub use generation_event_publisher::GenerationEventPublisher;
pub use llm_queue_service::LLMQueueService;
pub use player_action_queue_service::PlayerActionQueueService;

// Re-export generation queue projection service (snapshot types are used by HTTP layer)
pub use generation_queue_projection_service::{
    GenerationQueueProjectionService, GenerationQueueSnapshot,
};

// Re-export outcome trigger service
pub use outcome_trigger_service::OutcomeTriggerService;

// Re-export challenge outcome approval service (P3.3)
pub use challenge_outcome_approval_service::{
    ChallengeApprovalResult, ChallengeOutcomeApprovalService, ChallengeOutcomeError,
    OutcomeBranchInfo, ResolvedOutcome,
};

// Re-export outcome suggestion service (P3.3)
pub use outcome_suggestion_service::{OutcomeSuggestionService, SuggestionError};

// Re-export challenge approval event types (P3.3 refactor)
pub use challenge_approval_event_publisher::ChallengeApprovalEventPublisher;
pub use challenge_approval_events::{
    ChallengeApprovalEvent, OutcomeBranchData as ChallengeOutcomeBranchData,
    OutcomeTriggerData as ChallengeOutcomeTriggerData,
};

// Re-export trigger evaluation service (Phase 2)
pub use trigger_evaluation_service::{
    CompletedChallenge, CompletedNarrativeEvent, GameStateSnapshot, ImmediateContext,
    TriggerEvaluationError, TriggerEvaluationResult, TriggerEvaluationService, TriggerSource,
    TriggeredEventCandidate,
};

// Re-export event effect executor (Phase 2)
pub use event_effect_executor::{
    EffectExecutionError, EffectExecutionResult, EventEffectExecutor, OutcomeExecutionResult,
};

// Re-export staging services (Staging System - replaces legacy PresenceService)
pub use staging_context_provider::{build_staging_prompt, StagingContextProvider};
pub use staging_service::StagingService;
pub use internal::{StagingProposal, StagedNpcProposal, ApprovedNpc};

// Re-export disposition service (P1.4)
pub use disposition_service::{DispositionService, DispositionServiceImpl};

// Re-export actantial context service (P1.5)
pub use actantial_context_service::{
    ActantialContextService, ActantialContextServiceImpl, ActorTargetType, CreateWantRequest,
    UpdateWantRequest,
};

// Re-export region service
pub use region_service::{RegionService, RegionServiceImpl};

// Re-export prompt context service for building LLM prompts from player actions
pub use prompt_context_service::{PromptContextService, PromptContextServiceImpl};

// Re-export world session policy service (business rules for join validation)
pub use world_session_policy::{JoinPolicyError, JoinValidation, WorldSessionPolicy};

// Note: PlayerActionService and ApprovalService were removed - functionality moved to queue services
