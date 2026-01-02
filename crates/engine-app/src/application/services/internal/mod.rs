//! Internal service traits and inbound port re-exports
//!
//! This module provides a unified internal API for service dependencies within the
//! application layer. It contains two categories of traits:
//!
//! ## 1. Internal Service Contracts (NOT hexagonal ports)
//!
//! Traits like `CharacterServicePort`, `SceneServicePort`, etc. are **internal app-layer
//! contracts**. They are:
//! - Implemented by services in `engine-app`
//! - Called by other services/use cases in `engine-app`
//! - NOT called by adapters
//!
//! These exist for dependency injection and testing within the app layer.
//!
//! ## 2. Inbound Port Re-exports (dual-use traits)
//!
//! Traits like `WorldUseCasePort`, `DmApprovalQueueUseCasePort`, etc. are **re-exported
//! from `engine-ports/src/inbound/`**. These traits serve dual purposes:
//! - Called by HTTP handlers (inbound port usage - handlers import from `engine_ports::inbound`)
//! - Depended on by other services/use cases (internal dependency - import via `internal::`)
//!
//! The re-export pattern provides:
//! - A consistent internal API (all service dependencies via `internal::`)
//! - No trait duplication between inbound ports and internal contracts
//! - Clear separation: handlers use `engine_ports::inbound`, app code uses `internal::`
//!
//! ## Naming Convention
//!
//! Traits here use `*ServicePort` or `*UseCasePort` suffixes despite NOT being true
//! hexagonal ports. This is intentional for consistency - the `Port` suffix indicates
//! "injectable dependency" rather than "hexagonal boundary". Future cleanup may rename
//! internal-only traits to drop the `Port` suffix for clarity.
//!
//! See `docs/architecture/hexagonal-architecture.md` for the full architectural context.

// Service trait modules - alphabetically ordered
mod actantial_context_service;
mod challenge_resolution_service;
mod challenge_service;
mod character_service;
mod dialogue_context_service;
mod disposition_service;
mod dm_action_queue_service;
mod event_chain_service;
mod item_service;
mod location_service;
mod narrative_event_approval_service;
mod narrative_event_service;
mod outcome_trigger_service;
mod prompt_context_service;
mod region_service;
mod relationship_service;
mod scene_resolution_service;
mod sheet_template_service;
mod skill_service;
mod story_event_admin_service;
mod story_event_query_service;
mod story_event_recording_service;
mod story_event_service;
mod trigger_evaluation_service;

// 11 INBOUND service traits (internalized - handlers will call use case wrappers)
mod asset_generation_queue_service;
mod asset_service;
mod dm_approval_queue_service;
mod generation_queue_projection_service;
mod generation_service;
mod llm_queue_service;
mod player_action_queue_service;
mod prompt_template_service;
mod settings_service;
mod workflow_service;
mod world_service;

// Re-export all traits and their mocks
pub use actantial_context_service::ActantialContextServicePort;
#[cfg(any(test, feature = "testing"))]
pub use actantial_context_service::MockActantialContextServicePort;

pub use challenge_resolution_service::{
    AdHocResult, ChallengeResolutionServicePort, DiceRoll, NarrativeRollContext,
    OutcomeTriggerInfo, PendingResolution, RollResult, RollResultData, TriggerResult,
};
#[cfg(any(test, feature = "testing"))]
pub use challenge_resolution_service::MockChallengeResolutionServicePort;

pub use challenge_service::ChallengeServicePort;
#[cfg(any(test, feature = "testing"))]
pub use challenge_service::MockChallengeServicePort;

pub use character_service::CharacterServicePort;
#[cfg(any(test, feature = "testing"))]
pub use character_service::MockCharacterServicePort;

pub use dialogue_context_service::DialogueContextServicePort;
#[cfg(any(test, feature = "testing"))]
pub use dialogue_context_service::MockDialogueContextServicePort;

pub use disposition_service::DispositionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use disposition_service::MockDispositionServicePort;

pub use dm_action_queue_service::{
    DmAction, DmActionQueueItem, DmActionQueueServicePort, DmActionType, DmDecision,
};
#[cfg(any(test, feature = "testing"))]
pub use dm_action_queue_service::MockDmActionQueueServicePort;

pub use event_chain_service::EventChainServicePort;
#[cfg(any(test, feature = "testing"))]
pub use event_chain_service::MockEventChainServicePort;

pub use item_service::ItemServicePort;
#[cfg(any(test, feature = "testing"))]
pub use item_service::MockItemServicePort;

pub use location_service::LocationServicePort;
#[cfg(any(test, feature = "testing"))]
pub use location_service::MockLocationServicePort;

pub use narrative_event_approval_service::{
    NarrativeEventApprovalServicePort, NarrativeEventTriggerResult,
};
#[cfg(any(test, feature = "testing"))]
pub use narrative_event_approval_service::MockNarrativeEventApprovalServicePort;

pub use narrative_event_service::NarrativeEventServicePort;

pub use outcome_trigger_service::{OutcomeTriggerExecutionResult, OutcomeTriggerServicePort};
#[cfg(any(test, feature = "testing"))]
pub use outcome_trigger_service::MockOutcomeTriggerServicePort;

pub use prompt_context_service::{PromptContextError, PromptContextServicePort};
#[cfg(any(test, feature = "testing"))]
pub use prompt_context_service::MockPromptContextServicePort;

pub use region_service::RegionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use region_service::MockRegionServicePort;

pub use relationship_service::RelationshipServicePort;
#[cfg(any(test, feature = "testing"))]
pub use relationship_service::MockRelationshipServicePort;

pub use scene_resolution_service::{SceneResolutionResult, SceneResolutionServicePort};
#[cfg(any(test, feature = "testing"))]
pub use scene_resolution_service::MockSceneResolutionServicePort;

pub use sheet_template_service::SheetTemplateServicePort;
#[cfg(any(test, feature = "testing"))]
pub use sheet_template_service::MockSheetTemplateServicePort;

pub use skill_service::{CreateSkillRequest, SkillServicePort, UpdateSkillRequest};
#[cfg(any(test, feature = "testing"))]
pub use skill_service::MockSkillServicePort;

pub use story_event_admin_service::StoryEventAdminServicePort;
#[cfg(any(test, feature = "testing"))]
pub use story_event_admin_service::MockStoryEventAdminServicePort;

pub use story_event_query_service::StoryEventQueryServicePort;
#[cfg(any(test, feature = "testing"))]
pub use story_event_query_service::MockStoryEventQueryServicePort;

pub use story_event_recording_service::StoryEventRecordingServicePort;
#[cfg(any(test, feature = "testing"))]
pub use story_event_recording_service::MockStoryEventRecordingServicePort;

pub use story_event_service::StoryEventServicePort;
#[cfg(any(test, feature = "testing"))]
pub use story_event_service::MockStoryEventServicePort;

pub use trigger_evaluation_service::{
    CompletedChallenge, CompletedNarrativeEvent, GameStateSnapshot, ImmediateContext,
    TriggerEvaluationResult, TriggerEvaluationServicePort, TriggerSource, TriggeredEventCandidate,
};
#[cfg(any(test, feature = "testing"))]
pub use trigger_evaluation_service::MockTriggerEvaluationServicePort;

// Re-export types from engine-ports that internal traits depend on
// (These are true outbound port types, not internal traits)
pub use wrldbldr_engine_ports::outbound::{SocialNetwork, StateChange};

// =============================================================================
// 11 INBOUND service ports (internalized)
// =============================================================================
// These traits were previously in engine-ports/outbound but are called by HTTP
// handlers AND depended on by other services. We internalize them here; HTTP
// handlers will call inbound use case ports that delegate to these.

pub use asset_generation_queue_service::{
    AssetGenerationQueueItem, AssetGenerationQueueUseCasePort, AssetGenerationRequest,
    GenerationMetadata as AssetGenerationMetadata, GenerationResult,
};
#[cfg(any(test, feature = "testing"))]
pub use asset_generation_queue_service::MockAssetGenerationQueueUseCasePort;

pub use asset_service::{AssetUseCasePort, CreateAssetRequest};
#[cfg(any(test, feature = "testing"))]
pub use asset_service::MockAssetUseCasePort;

pub use dm_approval_queue_service::{
    ApprovalDecisionType, ApprovalQueueItem, ApprovalRequest, ApprovalUrgency, DmApprovalDecision,
    DmApprovalQueueUseCasePort,
};
#[cfg(any(test, feature = "testing"))]
pub use dm_approval_queue_service::MockDmApprovalQueueUseCasePort;

pub use generation_queue_projection_service::{
    GenerationBatchSnapshot, GenerationQueueProjectionUseCasePort, GenerationQueueSnapshot,
    SuggestionTaskSnapshot,
};
#[cfg(any(test, feature = "testing"))]
pub use generation_queue_projection_service::MockGenerationQueueProjectionUseCasePort;

pub use generation_service::{GenerationRequest, GenerationUseCasePort};
#[cfg(any(test, feature = "testing"))]
pub use generation_service::MockGenerationUseCasePort;

pub use llm_queue_service::{
    ChallengeSuggestion, ConfidenceLevel, LlmQueueItem, LlmQueueRequest, LlmQueueResponse,
    LlmQueueUseCasePort, LlmRequestType, LlmSuggestionContext, NarrativeEventSuggestion,
    ProposedToolCall,
};
#[cfg(any(test, feature = "testing"))]
pub use llm_queue_service::MockLlmQueueUseCasePort;

pub use player_action_queue_service::{
    PlayerAction, PlayerActionQueueItem, PlayerActionQueueUseCasePort,
};
#[cfg(any(test, feature = "testing"))]
pub use player_action_queue_service::MockPlayerActionQueueUseCasePort;

pub use prompt_template_service::PromptTemplateUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use prompt_template_service::MockPromptTemplateUseCasePort;

pub use settings_service::{LlmConfig, SettingsUseCasePort};
#[cfg(any(test, feature = "testing"))]
pub use settings_service::MockSettingsUseCasePort;

pub use workflow_service::WorkflowUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use workflow_service::MockWorkflowUseCasePort;

pub use world_service::WorldUseCasePort;
#[cfg(any(test, feature = "testing"))]
pub use world_service::MockWorldUseCasePort;

// =============================================================================
// 5 ADAPTER-CALLED service ports (internalized from engine-ports/outbound)
// =============================================================================
// These were wrapper ports called by adapters but implemented by app services.
// Now internalized for app→app dependencies.

mod challenge_outcome_approval_service;
mod interaction_service;
mod player_character_service;
mod scene_service;

pub use challenge_outcome_approval_service::{
    ChallengeOutcomeApprovalServicePort, OutcomeDecision,
};
#[cfg(any(test, feature = "testing"))]
pub use challenge_outcome_approval_service::MockChallengeOutcomeApprovalServicePort;

pub use interaction_service::InteractionServicePort;
#[cfg(any(test, feature = "testing"))]
pub use interaction_service::MockInteractionServicePort;

pub use player_character_service::PlayerCharacterServicePort;
#[cfg(any(test, feature = "testing"))]
pub use player_character_service::MockPlayerCharacterServicePort;

pub use scene_service::{SceneServicePort, SceneWithRelations};
#[cfg(any(test, feature = "testing"))]
pub use scene_service::MockSceneServicePort;

// NOTE: StagingServicePort has been removed - StagingService now directly
// implements StagingQueryPort from engine-ports.
