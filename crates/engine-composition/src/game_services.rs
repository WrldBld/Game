//! Game Services Container - Port-based abstraction for game mechanics services
//!
//! This module provides `GameServices`, a grouped structure for game mechanics
//! and narrative services using **port traits** from `wrldbldr-engine-ports`.
//!
//! # Architecture
//!
//! Unlike `engine-adapters/src/infrastructure/state/game_services.rs` which uses
//! concrete service implementations and generics, this struct uses only port traits.
//! This enables:
//!
//! - **Testability**: Easy mocking via port traits
//! - **Hexagonal purity**: Composition layer depends only on ports, not implementations
//! - **Flexibility**: Any implementation satisfying the port can be injected
//!
//! # Usage
//!
//! ```ignore
//! use wrldbldr_engine_composition::GameServices;
//!
//! // Construct with implementations cast to Arc<dyn Port>
//! let game_services = GameServices::new(
//!     story_event_service,
//!     challenge_service,
//!     // ... other services
//! );
//!
//! // Access via port traits
//! let events = game_services.story_event_service.list_by_world(world_id, 10).await?;
//! ```
//!
//! # Services Included
//!
//! - **Story Events**: Recording and querying gameplay events
//! - **Challenges**: Challenge CRUD and resolution workflow
//! - **Narrative Events**: Story arc events with DM approval
//! - **Event Chains**: Story arc management
//! - **Trigger Evaluation**: Checking event trigger conditions
//! - **Effect Execution**: Applying narrative event outcomes
//! - **Disposition**: NPC mood and relationship tracking
//! - **Actantial Context**: Character motivation modeling

use std::sync::Arc;

use wrldbldr_engine_ports::outbound::{
    ActantialContextServicePort, ChallengeOutcomeApprovalServicePort,
    ChallengeResolutionServicePort, ChallengeServicePort, DispositionServicePort,
    EventChainServicePort, EventEffectExecutorPort, NarrativeEventApprovalServicePort,
    NarrativeEventServicePort, StoryEventServicePort, TriggerEvaluationServicePort,
};

/// Container for game mechanics and narrative services.
///
/// This struct groups all services related to gameplay and storytelling,
/// using port traits for clean hexagonal architecture boundaries.
///
/// All fields are `Arc<dyn ...Port>` for:
/// - Shared ownership across handlers and workers
/// - Dynamic dispatch enabling mock injection for tests
/// - No generic type parameters for simpler composition
///
/// # Service Categories
///
/// ## Core Game Events
/// - `story_event_service`: Recording gameplay timeline events
///
/// ## Challenge System
/// - `challenge_service`: Challenge CRUD operations
/// - `challenge_resolution_service`: Dice rolling and outcome determination
/// - `challenge_outcome_approval_service`: DM approval workflow for challenges
///
/// ## Narrative Event System
/// - `narrative_event_service`: Narrative event CRUD operations
/// - `narrative_event_approval_service`: DM approval workflow for narrative events
/// - `event_chain_service`: Story arc (event chain) management
/// - `trigger_evaluation_service`: Evaluating event trigger conditions
/// - `event_effect_executor`: Executing event outcome effects
///
/// ## Character Systems
/// - `disposition_service`: NPC mood and relationship tracking (Tier 1/2)
/// - `actantial_context_service`: Character motivation modeling (Tier 3)
#[derive(Clone)]
pub struct GameServices {
    /// Story event service for recording and querying gameplay events.
    ///
    /// Used to maintain the narrative timeline of what has happened in the world.
    pub story_event_service: Arc<dyn StoryEventServicePort>,

    /// Challenge CRUD service.
    ///
    /// Provides read access to challenge data for prompt building and context gathering.
    pub challenge_service: Arc<dyn ChallengeServicePort>,

    /// Challenge resolution and dice rolling service.
    ///
    /// Handles the flow from triggering a challenge through dice rolling to outcome
    /// determination, queueing results for DM approval.
    pub challenge_resolution_service: Arc<dyn ChallengeResolutionServicePort>,

    /// Challenge outcome approval workflow service.
    ///
    /// Manages DM approval of challenge resolutions, including accepting, editing,
    /// requesting LLM suggestions, and selecting outcome branches.
    pub challenge_outcome_approval_service: Arc<dyn ChallengeOutcomeApprovalServicePort>,

    /// Narrative event CRUD service.
    ///
    /// Provides access to narrative events for prompt context and DM triggering.
    pub narrative_event_service: Arc<dyn NarrativeEventServicePort>,

    /// Narrative event approval workflow service.
    ///
    /// Handles DM decisions on narrative event suggestions, triggering approved
    /// events and recording them in the story timeline.
    pub narrative_event_approval_service: Arc<dyn NarrativeEventApprovalServicePort>,

    /// Event chain (story arc) management service.
    ///
    /// Manages story arcs that contain multiple narrative events, tracking
    /// progress and chain status.
    pub event_chain_service: Arc<dyn EventChainServicePort>,

    /// Trigger evaluation service for narrative events.
    ///
    /// Evaluates game state against narrative event trigger conditions to
    /// determine which events should be suggested to the DM.
    pub trigger_evaluation_service: Arc<dyn TriggerEvaluationServicePort>,

    /// Event effect executor service.
    ///
    /// Executes effects from narrative event outcomes (set flags, enable challenges,
    /// modify relationships, etc.).
    pub event_effect_executor: Arc<dyn EventEffectExecutorPort>,

    /// NPC disposition and relationship tracking service.
    ///
    /// Manages the Three-Tier Emotional Model:
    /// - Tier 1: Disposition (emotional response toward PC)
    /// - Tier 2: Relationship (social distance/familiarity)
    pub disposition_service: Arc<dyn DispositionServicePort>,

    /// Actantial context service for character motivations.
    ///
    /// Provides character motivation context based on the actantial model:
    /// - Wants and their targets
    /// - Helpers, opponents, senders, receivers
    /// - Social stance toward other characters
    pub actantial_context_service: Arc<dyn ActantialContextServicePort>,
}

impl GameServices {
    /// Creates a new `GameServices` instance with all game mechanic services.
    ///
    /// # Arguments
    ///
    /// All arguments are `Arc<dyn ...Port>` to allow any implementation:
    ///
    /// * `story_event_service` - For gameplay timeline recording
    /// * `challenge_service` - For challenge CRUD operations
    /// * `challenge_resolution_service` - For dice rolling and outcomes
    /// * `challenge_outcome_approval_service` - For DM approval of challenges
    /// * `narrative_event_service` - For narrative event access
    /// * `narrative_event_approval_service` - For DM approval of narrative events
    /// * `event_chain_service` - For story arc management
    /// * `trigger_evaluation_service` - For evaluating event triggers
    /// * `event_effect_executor` - For executing event effects
    /// * `disposition_service` - For NPC mood/relationship tracking
    /// * `actantial_context_service` - For character motivation context
    ///
    /// # Example
    ///
    /// ```ignore
    /// let game_services = GameServices::new(
    ///     Arc::new(story_event_service_impl) as Arc<dyn StoryEventServicePort>,
    ///     Arc::new(challenge_service_impl) as Arc<dyn ChallengeServicePort>,
    ///     // ... other services
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        story_event_service: Arc<dyn StoryEventServicePort>,
        challenge_service: Arc<dyn ChallengeServicePort>,
        challenge_resolution_service: Arc<dyn ChallengeResolutionServicePort>,
        challenge_outcome_approval_service: Arc<dyn ChallengeOutcomeApprovalServicePort>,
        narrative_event_service: Arc<dyn NarrativeEventServicePort>,
        narrative_event_approval_service: Arc<dyn NarrativeEventApprovalServicePort>,
        event_chain_service: Arc<dyn EventChainServicePort>,
        trigger_evaluation_service: Arc<dyn TriggerEvaluationServicePort>,
        event_effect_executor: Arc<dyn EventEffectExecutorPort>,
        disposition_service: Arc<dyn DispositionServicePort>,
        actantial_context_service: Arc<dyn ActantialContextServicePort>,
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
