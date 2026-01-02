//! Internal service traits - NOT ports, just app-layer contracts
//!
//! These traits define contracts between services within the application layer.
//! They are NOT ports in the hexagonal architecture sense (not adapter-implemented).
//!
//! They exist for:
//! - Dependency injection within the app layer
//! - Testing (mockall mocks)
//! - Decoupling service implementations
//!
//! # Migration Note
//!
//! These traits were moved from `engine-ports/src/outbound/` as part of the
//! hexagonal architecture cleanup. They were incorrectly classified as "outbound ports"
//! but are actually internal application contracts.

// Service trait modules - alphabetically ordered
mod actantial_context_service_port;
mod challenge_resolution_service_port;
mod challenge_service_port;
mod character_service_port;
mod dialogue_context_service_port;
mod disposition_service_port;
mod dm_action_queue_service_port;
mod event_chain_service_port;
mod item_service_port;
mod location_service_port;
mod narrative_event_approval_service_port;
mod narrative_event_service_port;
mod outcome_trigger_service_port;
mod prompt_context_service_port;
mod region_service_port;
mod relationship_service_port;
mod scene_resolution_service_port;
mod sheet_template_service_port;
mod skill_service_port;
mod story_event_admin_service_port;
mod story_event_query_service_port;
mod story_event_recording_service_port;
mod story_event_service_port;
mod trigger_evaluation_service_port;

// 11 INBOUND service ports (internalized - handlers will call use case wrappers)
mod asset_generation_queue_service_port;
mod asset_service_port;
mod dm_approval_queue_service_port;
mod generation_queue_projection_service_port;
mod generation_service_port;
mod llm_queue_service_port;
mod player_action_queue_service_port;
mod prompt_template_service_port;
mod settings_service_port;
mod workflow_service_port;
mod world_service_port;

// Re-export all traits and their mocks
pub use actantial_context_service_port::ActantialContextServicePort;
#[cfg(any(test, feature = "testing"))]
pub use actantial_context_service_port::MockActantialContextServicePort;

pub use challenge_resolution_service_port::{
    ChallengeResolutionServicePort, DiceRoll, PendingResolution, RollResult,
};
#[cfg(any(test, feature = "testing"))]
pub use challenge_resolution_service_port::MockChallengeResolutionServicePort;

pub use challenge_service_port::ChallengeServicePort;
#[cfg(any(test, feature = "testing"))]
pub use challenge_service_port::MockChallengeServicePort;

pub use character_service_port::CharacterServicePort;
#[cfg(any(test, feature = "testing"))]
pub use character_service_port::MockCharacterServicePort;

pub use dialogue_context_service_port::DialogueContextServicePort;
#[cfg(any(test, feature = "testing"))]
pub use dialogue_context_service_port::MockDialogueContextServicePort;

pub use disposition_service_port::DispositionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use disposition_service_port::MockDispositionServicePort;

pub use dm_action_queue_service_port::{
    DmAction, DmActionQueueItem, DmActionQueueServicePort, DmActionType, DmDecision,
};
#[cfg(any(test, feature = "testing"))]
pub use dm_action_queue_service_port::MockDmActionQueueServicePort;

pub use event_chain_service_port::EventChainServicePort;
#[cfg(any(test, feature = "testing"))]
pub use event_chain_service_port::MockEventChainServicePort;

pub use item_service_port::ItemServicePort;
#[cfg(any(test, feature = "testing"))]
pub use item_service_port::MockItemServicePort;

pub use location_service_port::LocationServicePort;
#[cfg(any(test, feature = "testing"))]
pub use location_service_port::MockLocationServicePort;

pub use narrative_event_approval_service_port::{
    NarrativeEventApprovalServicePort, NarrativeEventTriggerResult,
};
#[cfg(any(test, feature = "testing"))]
pub use narrative_event_approval_service_port::MockNarrativeEventApprovalServicePort;

pub use narrative_event_service_port::NarrativeEventServicePort;

pub use outcome_trigger_service_port::{OutcomeTriggerExecutionResult, OutcomeTriggerServicePort};
#[cfg(any(test, feature = "testing"))]
pub use outcome_trigger_service_port::MockOutcomeTriggerServicePort;

pub use prompt_context_service_port::{PromptContextError, PromptContextServicePort};
#[cfg(any(test, feature = "testing"))]
pub use prompt_context_service_port::MockPromptContextServicePort;

pub use region_service_port::RegionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use region_service_port::MockRegionServicePort;

pub use relationship_service_port::RelationshipServicePort;
#[cfg(any(test, feature = "testing"))]
pub use relationship_service_port::MockRelationshipServicePort;

pub use scene_resolution_service_port::{SceneResolutionResult, SceneResolutionServicePort};
#[cfg(any(test, feature = "testing"))]
pub use scene_resolution_service_port::MockSceneResolutionServicePort;

pub use sheet_template_service_port::SheetTemplateServicePort;
#[cfg(any(test, feature = "testing"))]
pub use sheet_template_service_port::MockSheetTemplateServicePort;

pub use skill_service_port::{CreateSkillRequest, SkillServicePort, UpdateSkillRequest};
#[cfg(any(test, feature = "testing"))]
pub use skill_service_port::MockSkillServicePort;

pub use story_event_admin_service_port::StoryEventAdminServicePort;
#[cfg(any(test, feature = "testing"))]
pub use story_event_admin_service_port::MockStoryEventAdminServicePort;

pub use story_event_query_service_port::StoryEventQueryServicePort;
#[cfg(any(test, feature = "testing"))]
pub use story_event_query_service_port::MockStoryEventQueryServicePort;

pub use story_event_recording_service_port::StoryEventRecordingServicePort;
#[cfg(any(test, feature = "testing"))]
pub use story_event_recording_service_port::MockStoryEventRecordingServicePort;

pub use story_event_service_port::StoryEventServicePort;
#[cfg(any(test, feature = "testing"))]
pub use story_event_service_port::MockStoryEventServicePort;

pub use trigger_evaluation_service_port::{
    CompletedChallenge, CompletedNarrativeEvent, GameStateSnapshot, ImmediateContext,
    TriggerEvaluationResult, TriggerEvaluationServicePort, TriggerSource, TriggeredEventCandidate,
};
#[cfg(any(test, feature = "testing"))]
pub use trigger_evaluation_service_port::MockTriggerEvaluationServicePort;

// Re-export types from engine-ports that internal traits depend on
// (These are true outbound port types, not internal traits)
pub use wrldbldr_engine_ports::outbound::{SocialNetwork, StateChange};

// =============================================================================
// 11 INBOUND service ports (internalized)
// =============================================================================
// These traits were previously in engine-ports/outbound but are called by HTTP
// handlers AND depended on by other services. We internalize them here; HTTP
// handlers will call inbound use case ports that delegate to these.

pub use asset_generation_queue_service_port::{
    AssetGenerationQueueItem, AssetGenerationQueueServicePort, AssetGenerationRequest,
    GenerationMetadata as AssetGenerationMetadata, GenerationResult,
};
#[cfg(any(test, feature = "testing"))]
pub use asset_generation_queue_service_port::MockAssetGenerationQueueServicePort;

pub use asset_service_port::{AssetServicePort, CreateAssetRequest};
#[cfg(any(test, feature = "testing"))]
pub use asset_service_port::MockAssetServicePort;

pub use dm_approval_queue_service_port::{
    ApprovalDecisionType, ApprovalQueueItem, ApprovalRequest, ApprovalUrgency, DmApprovalDecision,
    DmApprovalQueueServicePort,
};
#[cfg(any(test, feature = "testing"))]
pub use dm_approval_queue_service_port::MockDmApprovalQueueServicePort;

pub use generation_queue_projection_service_port::{
    GenerationBatchSnapshot, GenerationQueueProjectionServicePort, GenerationQueueSnapshot,
    SuggestionTaskSnapshot,
};
#[cfg(any(test, feature = "testing"))]
pub use generation_queue_projection_service_port::MockGenerationQueueProjectionServicePort;

pub use generation_service_port::{GenerationRequest, GenerationServicePort};
#[cfg(any(test, feature = "testing"))]
pub use generation_service_port::MockGenerationServicePort;

pub use llm_queue_service_port::{
    ChallengeSuggestion, ConfidenceLevel, LlmQueueItem, LlmQueueRequest, LlmQueueResponse,
    LlmQueueServicePort, LlmRequestType, NarrativeEventSuggestion, ProposedToolCall,
    SuggestionContext as LlmSuggestionContext,
};
#[cfg(any(test, feature = "testing"))]
pub use llm_queue_service_port::MockLlmQueueServicePort;

pub use player_action_queue_service_port::{
    PlayerAction, PlayerActionQueueItem, PlayerActionQueueServicePort,
};
#[cfg(any(test, feature = "testing"))]
pub use player_action_queue_service_port::MockPlayerActionQueueServicePort;

pub use prompt_template_service_port::PromptTemplateServicePort;
#[cfg(any(test, feature = "testing"))]
pub use prompt_template_service_port::MockPromptTemplateServicePort;

pub use settings_service_port::{LlmConfig, SettingsServicePort};
#[cfg(any(test, feature = "testing"))]
pub use settings_service_port::MockSettingsServicePort;

pub use workflow_service_port::WorkflowServicePort;
#[cfg(any(test, feature = "testing"))]
pub use workflow_service_port::MockWorkflowServicePort;

pub use world_service_port::WorldServicePort;
#[cfg(any(test, feature = "testing"))]
pub use world_service_port::MockWorldServicePort;

// =============================================================================
// 5 ADAPTER-CALLED service ports (internalized from engine-ports/outbound)
// =============================================================================
// These were wrapper ports called by adapters but implemented by app services.
// Now internalized for app→app dependencies.

mod challenge_outcome_approval_service_port;
mod interaction_service_port;
mod player_character_service_port;
mod scene_service_port;
mod staging_service_port;

pub use challenge_outcome_approval_service_port::{
    ChallengeOutcomeApprovalServicePort, OutcomeDecision,
};
#[cfg(any(test, feature = "testing"))]
pub use challenge_outcome_approval_service_port::MockChallengeOutcomeApprovalServicePort;

pub use interaction_service_port::InteractionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use interaction_service_port::MockInteractionServicePort;

pub use player_character_service_port::PlayerCharacterServicePort;
#[cfg(any(test, feature = "testing"))]
pub use player_character_service_port::MockPlayerCharacterServicePort;

pub use scene_service_port::{SceneServicePort, SceneWithRelations};
#[cfg(any(test, feature = "testing"))]
pub use scene_service_port::MockSceneServicePort;

pub use staging_service_port::{ApprovedNpc, StagedNpcProposal, StagingProposal, StagingServicePort};
#[cfg(any(test, feature = "testing"))]
pub use staging_service_port::MockStagingServicePort;
