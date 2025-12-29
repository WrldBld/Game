//! Game mechanics and narrative services
//!
//! This module provides a grouped structure for game mechanics services,
//! using trait objects where possible for flexibility and testability.

use std::sync::Arc;

use wrldbldr_domain::value_objects::ApprovalRequestData;
use wrldbldr_engine_app::application::services::{
    challenge_resolution_service::ChallengeResolutionService, ActantialContextService,
    ChallengeOutcomeApprovalService, ChallengeService, ChallengeServiceImpl, DispositionService,
    EventChainService, EventEffectExecutor, ItemServiceImpl, NarrativeEventApprovalService,
    NarrativeEventService, NarrativeEventServiceImpl, PlayerCharacterServiceImpl, SkillServiceImpl,
    StoryEventService, TriggerEvaluationService,
};
use wrldbldr_engine_ports::outbound::LlmPort;

/// Services for game mechanics, challenges, and narrative events
///
/// This struct groups services related to the gameplay and storytelling
/// aspects: story events, challenges, narrative events, and their approval workflows.
///
/// Services with simple traits use `Arc<dyn Trait>` for flexibility:
/// - `challenge_service`, `narrative_event_service`, `event_chain_service`
/// - `disposition_service`, `actantial_context_service`
///
/// Complex generic services remain concrete for type safety:
/// - `challenge_resolution_service`, `challenge_outcome_approval_service`
/// - `narrative_event_approval_service`
/// - `trigger_evaluation_service`, `event_effect_executor`
///
/// Generic over `L: LlmPort` for LLM-powered suggestion generation.
pub struct GameServices<L: LlmPort> {
    /// Story event service for recording gameplay events
    pub story_event_service: Arc<dyn StoryEventService>,

    /// Challenge CRUD service
    pub challenge_service: Arc<dyn ChallengeService>,

    /// Challenge resolution and dice rolling
    pub challenge_resolution_service: Arc<
        ChallengeResolutionService<
            ChallengeServiceImpl,
            SkillServiceImpl,
            crate::infrastructure::queues::QueueBackendEnum<ApprovalRequestData>,
            PlayerCharacterServiceImpl,
            L,
            ItemServiceImpl,
        >,
    >,

    /// Challenge outcome approval workflow
    pub challenge_outcome_approval_service: Arc<ChallengeOutcomeApprovalService<L>>,

    /// Narrative event CRUD service  
    pub narrative_event_service: Arc<dyn NarrativeEventService>,

    /// Narrative event approval workflow
    pub narrative_event_approval_service:
        Arc<NarrativeEventApprovalService<NarrativeEventServiceImpl>>,

    /// Event chain (story arc) management
    pub event_chain_service: Arc<dyn EventChainService>,

    /// Service for evaluating narrative event triggers (Phase 2)
    pub trigger_evaluation_service: Arc<TriggerEvaluationService>,

    /// Service for executing narrative event outcome effects (Phase 2)
    pub event_effect_executor: Arc<EventEffectExecutor>,

    /// Service for NPC disposition and relationship tracking (P1.4)
    pub disposition_service: Arc<dyn DispositionService>,

    /// Service for actantial model context (P1.5)
    pub actantial_context_service: Arc<dyn ActantialContextService>,
}

impl<L: LlmPort + 'static> GameServices<L> {
    /// Creates a new GameServices instance with all game mechanic services
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        story_event_service: Arc<dyn StoryEventService>,
        challenge_service: Arc<dyn ChallengeService>,
        challenge_resolution_service: Arc<
            ChallengeResolutionService<
                ChallengeServiceImpl,
                SkillServiceImpl,
                crate::infrastructure::queues::QueueBackendEnum<ApprovalRequestData>,
                PlayerCharacterServiceImpl,
                L,
                ItemServiceImpl,
            >,
        >,
        challenge_outcome_approval_service: Arc<ChallengeOutcomeApprovalService<L>>,
        narrative_event_service: Arc<dyn NarrativeEventService>,
        narrative_event_approval_service: Arc<
            NarrativeEventApprovalService<NarrativeEventServiceImpl>,
        >,
        event_chain_service: Arc<dyn EventChainService>,
        trigger_evaluation_service: Arc<TriggerEvaluationService>,
        event_effect_executor: Arc<EventEffectExecutor>,
        disposition_service: Arc<dyn DispositionService>,
        actantial_context_service: Arc<dyn ActantialContextService>,
    ) -> Self {
        Self {
            story_event_service,
            challenge_service,
            challenge_resolution_service,
            challenge_outcome_approval_service,
            narrative_event_service,
            narrative_event_approval_service,
            event_chain_service,
            trigger_evaluation_service,
            event_effect_executor,
            disposition_service,
            actantial_context_service,
        }
    }
}
