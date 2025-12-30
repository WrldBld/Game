//! Game Services Factory
//!
//! This module provides factory functions for creating game service port trait objects
//! from their concrete implementations. It reduces boilerplate in the composition root
//! by centralizing game service construction logic.
//!
//! # Architecture
//!
//! Game services are the heart of the WrldBldr engine, handling:
//! - Story event recording and timeline management
//! - Challenge resolution and DM approval workflows
//! - Narrative event triggering and effect execution
//! - NPC disposition and actantial context modeling
//! - Staging (NPC placement) management
//!
//! # Service Categories
//!
//! ## Core Event Services
//! - `StoryEventServiceImpl` - Records gameplay events to the story timeline
//! - `NarrativeEventServiceImpl` - Manages story arc events with DM approval
//! - `EventChainServiceImpl` - Manages story arc (event chain) groupings
//!
//! ## Challenge Services
//! - `ChallengeServiceImpl` - Challenge CRUD operations
//! - `ChallengeResolutionService` - Dice rolling and outcome determination
//! - `ChallengeOutcomeApprovalService` - DM approval workflow for challenges
//!
//! ## Narrative Services
//! - `NarrativeEventApprovalService` - DM approval workflow for narrative events
//! - `TriggerEvaluationService` - Evaluates event trigger conditions
//! - `EventEffectExecutor` - Executes narrative event outcome effects
//!
//! ## Character Services
//! - `DispositionServiceImpl` - NPC mood and relationship tracking
//! - `ActantialContextServiceImpl` - Character motivation modeling
//!
//! ## Supporting Services
//! - `StagingService` - NPC placement in regions
//! - `PromptContextServiceImpl` - LLM prompt context building
//! - `RegionServiceImpl` - Region CRUD and navigation

use std::sync::Arc;

use tokio::sync::mpsc;

use wrldbldr_engine_app::application::services::ChallengeApprovalEvent;
use wrldbldr_domain::value_objects::{ApprovalRequestData, ChallengeOutcomeData};

use wrldbldr_engine_adapters::infrastructure::persistence::Neo4jRepository;

use wrldbldr_engine_app::application::services::{
    ActantialContextServiceImpl, ChallengeOutcomeApprovalService,
    ChallengeServiceImpl, DMApprovalQueueService, DispositionServiceImpl, EventChainServiceImpl,
    EventEffectExecutor, ItemServiceImpl, NarrativeEventApprovalService,
    NarrativeEventServiceImpl, OutcomeTriggerService, PlayerCharacterServiceImpl,
    PromptContextServiceImpl, PromptTemplateService, RegionServiceImpl, SettingsService,
    SkillServiceImpl, StoryEventServiceImpl, TriggerEvaluationService,
    challenge_resolution_service::ChallengeResolutionService,
};

use wrldbldr_engine_ports::outbound::{
    // Core port traits for output
    ActantialContextServicePort, ChallengeOutcomeApprovalServicePort,
    ChallengeResolutionServicePort, ChallengeServicePort, DispositionServicePort,
    EventBusPort, EventChainServicePort, EventEffectExecutorPort,
    NarrativeEventApprovalServicePort, NarrativeEventServicePort, PromptContextServicePort,
    StagingServicePort, StoryEventServicePort, TriggerEvaluationServicePort,
    // Repository ports for dependencies
    CharacterActantialPort, CharacterCrudPort, CharacterDispositionPort, CharacterWantPort,
    ClockPort, EventChainRepositoryPort, GoalRepositoryPort, ItemRepositoryPort, LlmPort,
    LocationCrudPort, LocationMapPort, PlayerCharacterRepositoryPort, QueuePort, RandomPort,
    RelationshipRepositoryPort, WorldRepositoryPort, WorldStatePort,
};

use wrldbldr_engine_app::application::services::{
    ActantialContextService, ChallengeService, DispositionService, NarrativeEventService,
    SkillService, StoryEventService, WorldService,
};

use wrldbldr_engine_adapters::infrastructure::queues::QueueBackendEnum;

// Re-export for use in composition root
pub use super::repositories::{ChallengePorts, NarrativeEventPorts, RegionPorts, StoryEventPorts};

/// Container for all game service port trait objects.
///
/// This struct groups services related to gameplay mechanics and narrative systems,
/// using port traits for clean hexagonal architecture boundaries.
///
/// # Service Categories
///
/// ## Story Event System
/// - `story_event_service`: Recording gameplay timeline events
///
/// ## Challenge System
/// - `challenge_service`: Challenge CRUD operations
/// - `challenge_resolution_service`: Dice rolling and outcome determination
/// - `challenge_outcome_approval_service`: DM approval for challenge outcomes
///
/// ## Narrative Event System
/// - `narrative_event_service`: Narrative event CRUD
/// - `narrative_event_approval_service`: DM approval for narrative events
/// - `event_chain_service`: Story arc (event chain) management
/// - `trigger_evaluation_service`: Evaluating event trigger conditions
/// - `event_effect_executor`: Executing event outcome effects
///
/// ## Character Services
/// - `disposition_service`: NPC mood/relationship tracking
/// - `actantial_context_service`: Character motivation modeling
///
/// ## Supporting Services
/// - `staging_service`: NPC placement in regions
/// - `prompt_context_service`: LLM prompt context building
/// - `region_service`: Region CRUD and navigation (app-layer trait)
#[derive(Clone)]
pub struct GameServicePorts {
    /// Story event service for recording gameplay events
    pub story_event_service: Arc<dyn StoryEventServicePort>,

    /// Challenge CRUD service
    pub challenge_service: Arc<dyn ChallengeServicePort>,

    /// Challenge resolution and dice rolling service
    pub challenge_resolution_service: Arc<dyn ChallengeResolutionServicePort>,

    /// Challenge outcome approval workflow service
    pub challenge_outcome_approval_service: Arc<dyn ChallengeOutcomeApprovalServicePort>,

    /// Narrative event CRUD service
    pub narrative_event_service: Arc<dyn NarrativeEventServicePort>,

    /// Narrative event approval workflow service
    pub narrative_event_approval_service: Arc<dyn NarrativeEventApprovalServicePort>,

    /// Event chain (story arc) management service
    pub event_chain_service: Arc<dyn EventChainServicePort>,

    /// Trigger evaluation service for narrative events
    pub trigger_evaluation_service: Arc<dyn TriggerEvaluationServicePort>,

    /// Event effect executor service
    pub event_effect_executor: Arc<dyn EventEffectExecutorPort>,

    /// NPC disposition and relationship tracking service
    pub disposition_service: Arc<dyn DispositionServicePort>,

    /// Actantial context service for character motivations
    pub actantial_context_service: Arc<dyn ActantialContextServicePort>,

    /// Staging service for NPC placement
    pub staging_service: Arc<dyn StagingServicePort>,

    /// Prompt context service for LLM prompt building
    pub prompt_context_service: Arc<dyn PromptContextServicePort>,

    /// Region service for region management (app-layer trait)
    pub region_service: Arc<dyn wrldbldr_engine_app::application::services::RegionService>,
}

/// Dependencies required for creating game services.
///
/// This struct encapsulates all the external dependencies needed to construct
/// game services, including repository ports, infrastructure services, and
/// other service dependencies.
///
/// # Note on StagingService
///
/// `StagingService` is heavily generic and requires concrete repository types.
/// The `staging_service` field accepts a pre-constructed service that has been
/// cast to `Arc<dyn StagingServicePort>`. This is because the generics cannot
/// be abstracted over with trait objects alone.
///
/// # Categories
///
/// ## Repository Port Groups (ISP-split)
/// Repository trait objects split according to Interface Segregation Principle.
///
/// ## Infrastructure Services
/// - `event_bus`: For publishing domain events
/// - `clock`: For time operations
/// - `rng`: For random number generation
/// - `llm_port`: For LLM interactions
///
/// ## Service Dependencies
/// - `settings_service`: For configurable values
/// - `prompt_template_service`: For prompt template resolution
///
/// ## Pre-constructed Services
/// - `staging_service`: Pre-constructed due to complex generics
pub struct GameServiceDependencies<L: LlmPort + 'static> {
    // Repository port groups (ISP-split)
    pub story_event_ports: StoryEventPorts,
    pub narrative_event_ports: NarrativeEventPorts,
    pub challenge_ports: ChallengePorts,
    pub region_ports: RegionPorts,

    // Non-ISP repository ports
    pub event_chain_repo: Arc<dyn EventChainRepositoryPort>,
    pub player_character_repo: Arc<dyn PlayerCharacterRepositoryPort>,
    pub character_crud: Arc<dyn CharacterCrudPort>,
    pub character_want: Arc<dyn CharacterWantPort>,
    pub character_actantial: Arc<dyn CharacterActantialPort>,
    pub character_disposition: Arc<dyn CharacterDispositionPort>,
    pub relationship_repo: Arc<dyn RelationshipRepositoryPort>,
    pub item_repo: Arc<dyn ItemRepositoryPort>,
    pub goal_repo: Arc<dyn GoalRepositoryPort>,
    pub want_repo: Arc<dyn wrldbldr_engine_ports::outbound::WantRepositoryPort>,
    pub skill_repo: Arc<dyn wrldbldr_engine_ports::outbound::SkillRepositoryPort>,
    pub world_repo: Arc<dyn WorldRepositoryPort>,
    pub location_crud: Arc<dyn LocationCrudPort>,
    pub location_map: Arc<dyn LocationMapPort>,
    pub region_item: Arc<dyn wrldbldr_engine_ports::outbound::RegionItemPort>,

    // Infrastructure services
    pub event_bus: Arc<dyn EventBusPort>,
    pub clock: Arc<dyn ClockPort>,
    pub rng: Arc<dyn RandomPort>,
    pub llm_port: Arc<L>,

    // Service dependencies
    pub settings_service: Arc<SettingsService>,
    pub prompt_template_service: Arc<PromptTemplateService>,

    // Queue services
    pub challenge_outcome_queue: Arc<dyn QueuePort<ChallengeOutcomeData> + Send + Sync>,
    pub dm_approval_queue_service:
        Arc<DMApprovalQueueService<QueueBackendEnum<ApprovalRequestData>, ItemServiceImpl>>,

    // World state for prompt context
    pub world_state: Arc<dyn WorldStatePort>,

    // Pre-constructed services (due to complex generics)
    /// Staging service - must be pre-constructed due to complex generics
    pub staging_service: Arc<dyn StagingServicePort>,

    // App-layer services needed as dependencies
    pub world_service: Arc<dyn WorldService>,
    pub skill_service: Arc<dyn SkillService>,
    pub challenge_service_app: Arc<dyn ChallengeService>,
    pub narrative_event_service_app: Arc<dyn NarrativeEventService>,
    pub story_event_service_app: Arc<dyn StoryEventService>,

    // Event channel (created by event_infra, passed in)
    /// Challenge approval event sender - created by event_infra factory
    pub challenge_approval_tx: mpsc::Sender<ChallengeApprovalEvent>,
}

/// Result of creating game services.
///
/// Note: The challenge_approval_rx channel is NOT included here - it is created
/// by and returned from the event_infra factory, which owns all event channels.
pub struct GameServicesResult {
    /// The game service ports
    pub ports: GameServicePorts,
}

/// Creates all game service port trait objects from their dependencies.
///
/// This function:
/// 1. Instantiates all game services with their dependencies
/// 2. Coerces concrete implementations to port trait objects
/// 3. Returns a `GameServicesResult` with ports
///
/// Note: Event channels (including challenge_approval) are created by the
/// event_infra factory and passed in via `GameServiceDependencies`.
///
/// # Arguments
///
/// * `deps` - The dependencies required for game service construction
///
/// # Returns
///
/// A `GameServicesResult` containing all port trait objects.
///
/// # Example
///
/// ```ignore
/// let deps = GameServiceDependencies {
///     story_event_ports: repos.story_event,
///     challenge_ports: repos.challenge,
///     staging_service: Arc::new(staging_service) as Arc<dyn StagingServicePort>,
///     challenge_approval_tx: event_infra.challenge_approval_tx,
///     // ... other dependencies
/// };
///
/// let result = create_game_services(deps);
///
/// // Use ports
/// let events = result.ports.story_event_service.list_by_world(world_id, 10).await?;
/// ```
pub fn create_game_services<L: LlmPort + 'static>(
    deps: GameServiceDependencies<L>,
) -> GameServicesResult {

    // ==========================================================================
    // Story Event Service
    // ==========================================================================
    let story_event_service_impl = StoryEventServiceImpl::new(
        deps.story_event_ports.crud.clone(),
        deps.story_event_ports.edge.clone(),
        deps.story_event_ports.query.clone(),
        deps.story_event_ports.dialogue.clone(),
        deps.event_bus.clone(),
        deps.clock.clone(),
    );
    let story_event_service: Arc<dyn StoryEventServicePort> = Arc::new(story_event_service_impl);

    // ==========================================================================
    // Challenge Service
    // ==========================================================================
    let challenge_service_impl = ChallengeServiceImpl::new(
        deps.challenge_ports.crud.clone(),
        deps.challenge_ports.skill.clone(),
        deps.challenge_ports.scene.clone(),
        deps.challenge_ports.prerequisite.clone(),
        deps.challenge_ports.availability.clone(),
    );
    let challenge_service: Arc<dyn ChallengeServicePort> = Arc::new(challenge_service_impl.clone());

    // ==========================================================================
    // Narrative Event Service
    // ==========================================================================
    let narrative_event_service_impl = NarrativeEventServiceImpl::new(
        deps.narrative_event_ports.crud.clone(),
        deps.narrative_event_ports.tie.clone(),
        deps.narrative_event_ports.npc.clone(),
        deps.narrative_event_ports.query.clone(),
        deps.event_bus.clone(),
    );
    let narrative_event_service: Arc<dyn NarrativeEventServicePort> =
        Arc::new(narrative_event_service_impl.clone());

    // ==========================================================================
    // Event Chain Service
    // ==========================================================================
    let event_chain_service_impl = EventChainServiceImpl::new(deps.event_chain_repo.clone());
    let event_chain_service: Arc<dyn EventChainServicePort> = Arc::new(event_chain_service_impl);

    // ==========================================================================
    // Disposition Service
    // ==========================================================================
    let disposition_service_impl =
        DispositionServiceImpl::new(deps.character_disposition.clone(), deps.clock.clone());
    let disposition_service: Arc<dyn DispositionServicePort> =
        Arc::new(disposition_service_impl.clone());

    // ==========================================================================
    // Actantial Context Service
    // ==========================================================================
    let actantial_context_service_impl = ActantialContextServiceImpl::new(
        deps.character_crud.clone(),
        deps.character_want.clone(),
        deps.character_actantial.clone(),
        deps.player_character_repo.clone(),
        deps.goal_repo.clone(),
        deps.want_repo.clone(),
        deps.clock.clone(),
    );
    let actantial_context_service: Arc<dyn ActantialContextServicePort> =
        Arc::new(actantial_context_service_impl.clone());

    // ==========================================================================
    // Trigger Evaluation Service
    // ==========================================================================
    let trigger_evaluation_service = Arc::new(TriggerEvaluationService::new(
        deps.narrative_event_ports.crud.clone(),
        deps.player_character_repo.clone(),
        deps.story_event_ports.query.clone(),
        deps.story_event_ports.edge.clone(),
    ));
    let trigger_evaluation_service_port: Arc<dyn TriggerEvaluationServicePort> =
        trigger_evaluation_service.clone();

    // ==========================================================================
    // Event Effect Executor
    // ==========================================================================
    let event_effect_executor = Arc::new(EventEffectExecutor::new(
        deps.challenge_ports.crud.clone(),
        deps.narrative_event_ports.crud.clone(),
        deps.relationship_repo.clone(),
    ));
    let event_effect_executor_port: Arc<dyn EventEffectExecutorPort> = event_effect_executor;

    // ==========================================================================
    // Challenge Outcome Approval Service
    // ==========================================================================
    let outcome_trigger_service =
        Arc::new(OutcomeTriggerService::new(deps.challenge_ports.crud.clone()));

    let challenge_outcome_approval_service = Arc::new(ChallengeOutcomeApprovalService::new(
        deps.challenge_approval_tx.clone(),
        outcome_trigger_service,
        deps.player_character_repo.clone(),
        deps.item_repo.clone(),
        deps.prompt_template_service.clone(),
        deps.challenge_outcome_queue.clone(),
        deps.llm_port.clone(),
        deps.settings_service.clone(),
        deps.clock.clone(),
    ));
    let challenge_outcome_approval_service_port: Arc<dyn ChallengeOutcomeApprovalServicePort> =
        challenge_outcome_approval_service.clone();

    // ==========================================================================
    // Challenge Resolution Service
    // ==========================================================================
    // Create concrete services for generics
    let skill_service_impl =
        SkillServiceImpl::new(deps.skill_repo.clone(), deps.world_repo.clone());
    let player_character_service_impl = PlayerCharacterServiceImpl::new(
        deps.player_character_repo.clone(),
        deps.location_crud.clone(),
        deps.world_repo.clone(),
        deps.clock.clone(),
    );

    let challenge_resolution_service = Arc::new(ChallengeResolutionService::new(
        Arc::new(challenge_service_impl.clone()),
        Arc::new(skill_service_impl),
        Arc::new(player_character_service_impl),
        deps.dm_approval_queue_service.clone(),
        challenge_outcome_approval_service.clone(),
        deps.clock.clone(),
        deps.rng.clone(),
    ));
    let challenge_resolution_service_port: Arc<dyn ChallengeResolutionServicePort> =
        challenge_resolution_service;

    // ==========================================================================
    // Narrative Event Approval Service
    // ==========================================================================
    let narrative_event_approval_service = Arc::new(NarrativeEventApprovalService::new(
        Arc::new(narrative_event_service_impl.clone()),
        deps.story_event_service_app.clone(),
    ));
    let narrative_event_approval_service_port: Arc<dyn NarrativeEventApprovalServicePort> =
        narrative_event_approval_service;

    // ==========================================================================
    // Region Service
    // ==========================================================================
    let region_service: Arc<dyn wrldbldr_engine_app::application::services::RegionService> =
        Arc::new(RegionServiceImpl::new(
            deps.region_ports.crud.clone(),
            deps.region_ports.connection.clone(),
            deps.region_ports.exit.clone(),
            deps.region_ports.npc.clone(),
            deps.location_crud.clone(),
            deps.location_map.clone(),
        ));

    // ==========================================================================
    // Prompt Context Service
    // ==========================================================================
    let prompt_context_service_impl = Arc::new(PromptContextServiceImpl::new(
        deps.world_service.clone(),
        deps.world_state.clone(),
        deps.challenge_service_app.clone(),
        deps.skill_service.clone(),
        deps.narrative_event_service_app.clone(),
        deps.character_crud.clone(),
        deps.player_character_repo.clone(),
        deps.region_item.clone(),
        Arc::new(disposition_service_impl) as Arc<dyn DispositionService>,
        Arc::new(actantial_context_service_impl) as Arc<dyn ActantialContextService>,
    ));
    let prompt_context_service_port: Arc<dyn PromptContextServicePort> =
        prompt_context_service_impl;

    // ==========================================================================
    // Assemble Result
    // ==========================================================================
    let ports = GameServicePorts {
        story_event_service,
        challenge_service,
        challenge_resolution_service: challenge_resolution_service_port,
        challenge_outcome_approval_service: challenge_outcome_approval_service_port,
        narrative_event_service,
        narrative_event_approval_service: narrative_event_approval_service_port,
        event_chain_service,
        trigger_evaluation_service: trigger_evaluation_service_port,
        event_effect_executor: event_effect_executor_port,
        disposition_service,
        actantial_context_service,
        staging_service: deps.staging_service, // Use pre-constructed service
        prompt_context_service: prompt_context_service_port,
        region_service,
    };

    GameServicesResult { ports }
}

/// Helper function to create story event ports from a Neo4j repository.
///
/// This is a convenience wrapper around the repository factory for cases
/// where only story event ports are needed.
pub fn create_story_event_ports(repository: &Neo4jRepository) -> StoryEventPorts {
    use super::repositories::coerce_isp;
    use wrldbldr_engine_ports::outbound::{
        StoryEventCrudPort, StoryEventDialoguePort, StoryEventEdgePort, StoryEventQueryPort,
    };

    let story_event_concrete = Arc::new(repository.story_events());
    coerce_isp!(
        story_event_concrete,
        dyn StoryEventCrudPort => crud,
        dyn StoryEventEdgePort => edge,
        dyn StoryEventQueryPort => query,
        dyn StoryEventDialoguePort => dialogue,
    );

    StoryEventPorts {
        crud,
        edge,
        query,
        dialogue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that GameServicePorts has all expected fields.
    ///
    /// This is a compile-time test - if the struct fields don't match,
    /// the code won't compile.
    #[test]
    fn test_game_service_ports_structure() {
        fn _verify_ports(ports: &GameServicePorts) {
            // Core event services
            let _ = &ports.story_event_service;
            let _ = &ports.narrative_event_service;
            let _ = &ports.event_chain_service;

            // Challenge services
            let _ = &ports.challenge_service;
            let _ = &ports.challenge_resolution_service;
            let _ = &ports.challenge_outcome_approval_service;

            // Narrative services
            let _ = &ports.narrative_event_approval_service;
            let _ = &ports.trigger_evaluation_service;
            let _ = &ports.event_effect_executor;

            // Character services
            let _ = &ports.disposition_service;
            let _ = &ports.actantial_context_service;

            // Supporting services
            let _ = &ports.staging_service;
            let _ = &ports.prompt_context_service;
            let _ = &ports.region_service;
        }

        // The existence of this function proves the types are correct at compile time
        let _ = _verify_ports;
    }

    /// Test that GameServiceDependencies has all expected fields.
    ///
    /// This verifies the dependency struct structure at compile time.
    #[test]
    fn test_game_service_dependencies_structure() {
        use wrldbldr_engine_adapters::infrastructure::ollama::OllamaClient;

        fn _verify_deps(deps: &GameServiceDependencies<OllamaClient>) {
            // ISP port groups
            let _ = &deps.story_event_ports;
            let _ = &deps.narrative_event_ports;
            let _ = &deps.challenge_ports;
            let _ = &deps.region_ports;

            // Non-ISP repository ports
            let _ = &deps.event_chain_repo;
            let _ = &deps.player_character_repo;
            let _ = &deps.character_crud;
            let _ = &deps.character_want;
            let _ = &deps.character_actantial;
            let _ = &deps.character_disposition;
            let _ = &deps.relationship_repo;
            let _ = &deps.item_repo;
            let _ = &deps.goal_repo;
            let _ = &deps.want_repo;
            let _ = &deps.skill_repo;
            let _ = &deps.world_repo;
            let _ = &deps.location_crud;
            let _ = &deps.location_map;
            let _ = &deps.region_item;

            // Infrastructure services
            let _ = &deps.event_bus;
            let _ = &deps.clock;
            let _ = &deps.rng;
            let _ = &deps.llm_port;

            // Service dependencies
            let _ = &deps.settings_service;
            let _ = &deps.prompt_template_service;

            // Queue services
            let _ = &deps.challenge_outcome_queue;
            let _ = &deps.dm_approval_queue_service;

            // World state
            let _ = &deps.world_state;

            // Pre-constructed services
            let _ = &deps.staging_service;

            // App-layer services
            let _ = &deps.world_service;
            let _ = &deps.skill_service;
            let _ = &deps.challenge_service_app;
            let _ = &deps.narrative_event_service_app;
            let _ = &deps.story_event_service_app;

            // Event channels (from event_infra)
            let _ = &deps.challenge_approval_tx;
        }

        // The existence of this function proves the types are correct at compile time
        let _ = _verify_deps;
    }

    /// Test that GameServicesResult has all expected fields.
    #[test]
    fn test_game_services_result_structure() {
        fn _verify_result(result: &GameServicesResult) {
            let _ = &result.ports;
            // Note: challenge_approval_rx is now in EventInfrastructure, not here
        }

        let _ = _verify_result;
    }
}
