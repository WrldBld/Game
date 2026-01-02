//! Application State Construction
//!
//! This module provides the construction logic for AppState.
//! The AppState struct is defined in engine-composition, but the construction
//! (dependency injection and wiring) is done here in the composition root.

use std::sync::Arc;

use anyhow::Result;

use wrldbldr_engine_adapters::infrastructure::config::AppConfig;
use wrldbldr_engine_adapters::infrastructure::suggestion_enqueue_adapter::SuggestionEnqueueAdapter;
use wrldbldr_engine_adapters::infrastructure::world_connection_manager::{
    new_shared_manager, SharedWorldConnectionManager,
};

use super::factories::{
    create_asset_services, create_core_services, create_use_cases, AssetServiceDependencies,
    CoreServiceDependencies, UseCaseDependencies,
};
use wrldbldr_engine_app::application::handlers::AppRequestHandler;
use wrldbldr_engine_app::application::services::generation_service::GenerationEvent;
use wrldbldr_engine_app::application::services::{
    challenge_resolution_service::ChallengeResolutionService, staging_service::StagingService,
    ActantialContextServiceImpl, AssetGenerationQueueService, ChallengeApprovalEvent,
    ChallengeOutcomeApprovalService, DMApprovalQueueService, DispositionServiceImpl,
    DmActionQueueService, EventEffectExecutor, ItemServiceImpl, LLMQueueService,
    NarrativeEventApprovalService, OutcomeTriggerService, PlayerActionQueueService,
    PromptContextServiceImpl, RegionServiceImpl, SheetTemplateService, TriggerEvaluationService,
};

// Import composition layer types
use wrldbldr_engine_composition::{
    AppConfig as CompositionAppConfig, AppState, AssetServices, CoreServices, EventInfra,
    GameServices, LlmPortDyn, LlmSuggestionQueueAdapter, PlayerServices, QueueServices,
};

use wrldbldr_engine_ports::inbound::RequestHandler;
// Internal service traits (NOT ports - internal app-layer contracts)
use wrldbldr_engine_app::application::services::internal::{
    ActantialContextServicePort, ChallengeOutcomeApprovalServicePort, ChallengeResolutionServicePort,
    DispositionServicePort, NarrativeEventApprovalServicePort, OutcomeTriggerServicePort,
    PromptContextServicePort, TriggerEvaluationServicePort,
};
// Internal service traits (NOT ports - internal app-layer contracts)
use wrldbldr_engine_app::application::services::internal::{
    PromptTemplateServicePort, SettingsServicePort,
};
// True outbound ports (adapter-implemented infrastructure)
use wrldbldr_engine_ports::outbound::{
    ApprovalRequestLookupPort, BroadcastPort, ComfyUIPort, ConnectionBroadcastPort,
    ConnectionContextPort, ConnectionLifecyclePort, ConnectionManagerPort, ConnectionQueryPort,
    ConnectionUnicastPort, DmActionProcessorPort, DmNotificationPort, DomainEventRepositoryPort,
    EventBusPort, EventEffectExecutorPort, EventNotifierPort, GenerationReadStatePort,
    RegionItemPort, StagingUseCaseServiceExtPort, WorldApprovalPort, WorldConversationPort,
    WorldDirectorialPort, WorldLifecyclePort, WorldScenePort, WorldTimePort,
};

// Re-export AppStatePort for server.rs
pub use wrldbldr_engine_ports::inbound::AppStatePort;

use wrldbldr_engine_ports::outbound::{
    ApprovalRequestData, AssetGenerationData, ChallengeOutcomeData, DmActionData, LlmRequestData,
    PlayerActionData,
};
use wrldbldr_engine_adapters::infrastructure::comfyui::ComfyUIClient as ComfyUIClientType;
use wrldbldr_engine_adapters::infrastructure::ollama::OllamaClient as OllamaClientType;
use wrldbldr_engine_adapters::infrastructure::queues::{InProcessNotifier, QueueBackendEnum};

/// Worker services - concrete queue service types for background workers.
///
/// Background workers need access to concrete service implementations because they
/// call infrastructure-specific methods like `run_worker()` and `process_next()` that
/// are not part of the port traits. These methods are worker-specific infrastructure
/// code that legitimately needs concrete types.
///
/// This struct is separate from `AppState` (which uses port traits) to maintain clean
/// hexagonal architecture: handlers use `AppState` with ports, workers use `WorkerServices`
/// with concrete types.
#[derive(Clone)]
pub struct WorkerServices {
    /// LLM queue service with `run_worker()` method
    pub llm_queue_service:
        Arc<LLMQueueService<QueueBackendEnum<LlmRequestData>, OllamaClientType, InProcessNotifier>>,

    /// Asset generation queue service with `run_worker()` method
    pub asset_generation_queue_service: Arc<
        AssetGenerationQueueService<
            QueueBackendEnum<AssetGenerationData>,
            ComfyUIClientType,
            InProcessNotifier,
        >,
    >,

    /// Player action queue service with `process_next()` method
    pub player_action_queue_service: Arc<
        PlayerActionQueueService<
            QueueBackendEnum<PlayerActionData>,
            QueueBackendEnum<LlmRequestData>,
        >,
    >,

    /// DM action queue service with `process_next()` method
    pub dm_action_queue_service: Arc<DmActionQueueService<QueueBackendEnum<DmActionData>>>,

    /// DM approval queue service with `queue()` accessor for `expire_old()`
    pub dm_approval_queue_service:
        Arc<DMApprovalQueueService<QueueBackendEnum<ApprovalRequestData>, ItemServiceImpl>>,

    /// DM action processor service for processing DM actions (via port trait)
    /// Uses port trait since dm_action_worker delegates business logic via DmActionProcessorPort
    pub dm_action_processor: Arc<dyn DmActionProcessorPort>,

    /// Challenge outcome queue (concrete backend type)
    pub challenge_outcome_queue: Arc<QueueBackendEnum<ChallengeOutcomeData>>,

    /// Prompt context service (app-layer trait, not port trait)
    /// Workers need the app-layer trait which has `build_prompt_from_action(&action)`
    pub prompt_context_service:
        Arc<dyn wrldbldr_engine_app::application::services::PromptContextService>,

    /// World connection manager (concrete type for workers that need broadcasting)
    pub world_connection_manager: SharedWorldConnectionManager,

    /// Event bus for publishing generation events
    pub event_bus: Arc<dyn EventBusPort>,

    /// Broadcast port for publishing game events
    pub broadcast: Arc<dyn BroadcastPort>,
}

/// Creates a new AppState with all services initialized.
///
/// Returns:
/// - `AppState`: The main application state with port-typed services
/// - `WorkerServices`: Concrete queue services for background workers
/// - `generation_event_rx`: GenerationEvent receiver for GenerationEventPublisher
/// - `challenge_approval_rx`: ChallengeApprovalEvent receiver for ChallengeApprovalEventPublisher
pub async fn new_app_state(
    config: AppConfig,
) -> Result<(
    AppState,
    WorkerServices,
    tokio::sync::mpsc::Receiver<GenerationEvent>,
    tokio::sync::mpsc::Receiver<ChallengeApprovalEvent>,
)> {
    // ===========================================================================
    // Level 0: Infrastructure
    // ===========================================================================
    // Use infrastructure factory to create all foundational dependencies
    let infra = super::factories::create_infrastructure(&config).await?;

    // Extract fields for use in remaining code
    let clock = infra.clock.clone();
    let rng = infra.rng.clone();
    let repository = infra.neo4j().clone();
    let settings_service = infra.settings_service.clone();
    let prompt_template_service = infra.prompt_template_service.clone();
    let directorial_context_repo = infra.directorial_context_repo.clone();
    let world_connection_manager = new_shared_manager();
    let world_state = infra.world_state.clone();
    let world_state_update = infra.world_state_update.clone();
    let staging_state = infra.staging_state.clone();

    // ==========================================================================
    // Worker-only external clients (not stored in InfrastructureContext)
    // ==========================================================================
    let llm_client = OllamaClientType::new(&config.ollama_base_url, &config.ollama_model);
    let comfyui_client = ComfyUIClientType::new(&config.comfyui_base_url, clock.clone());
    tracing::info!("Initialized LLM and ComfyUI clients");

    // ===========================================================================
    // Level 1: Repository Ports
    // ===========================================================================
    // Use repository factory to create all ISP-compliant repository ports
    let repos = super::factories::create_repository_ports(&repository);

    // Extract commonly used ports from the RepositoryPorts struct
    // Non-ISP repositories
    let world_repo = repos.world.clone();
    let relationship_repo = repos.relationship.clone();
    let skill_repo = repos.skill.clone();
    let interaction_repo = repos.interaction.clone();
    let asset_repo = repos.asset.clone();
    let workflow_repo = repos.workflow.clone();
    let sheet_template_repo = repos.sheet_template.clone();
    let item_repo = repos.item.clone();
    let goal_repo = repos.goal.clone();
    let want_repo = repos.want.clone();
    // Scene ISP ports
    let scene_crud = repos.scene.crud.clone();
    let scene_query = repos.scene.query.clone();
    let scene_location = repos.scene.location.clone();
    let scene_featured_character = repos.scene.featured_character.clone();
    let scene_completion = repos.scene.completion.clone();

    // Character ISP ports
    let character_crud = repos.character.crud.clone();
    let character_want = repos.character.want.clone();
    let character_actantial = repos.character.actantial.clone();
    let character_location = repos.character.location.clone();
    let character_disposition = repos.character.disposition.clone();

    // Location ISP ports
    let location_crud = repos.location.crud.clone();
    let location_hierarchy = repos.location.hierarchy.clone();
    let location_connection = repos.location.connection.clone();
    let location_map = repos.location.map.clone();

    // Region ISP ports
    let region_crud = repos.region.crud.clone();
    let region_connection = repos.region.connection.clone();
    let region_exit = repos.region.exit.clone();
    let region_npc = repos.region.npc.clone();
    let region_item = repos.region.item.clone();

    // Challenge ISP ports
    let challenge_crud = repos.challenge.crud.clone();
    let challenge_skill = repos.challenge.skill.clone();
    let challenge_scene = repos.challenge.scene.clone();
    let challenge_prerequisite = repos.challenge.prerequisite.clone();
    let challenge_availability = repos.challenge.availability.clone();

    // StoryEvent ISP ports
    let story_event_crud = repos.story_event.crud.clone();
    let story_event_edge = repos.story_event.edge.clone();
    let story_event_query = repos.story_event.query.clone();
    let story_event_dialogue = repos.story_event.dialogue.clone();

    // NarrativeEvent ISP ports
    let narrative_event_crud = repos.narrative_event.crud.clone();
    let narrative_event_tie = repos.narrative_event.tie.clone();
    let narrative_event_npc = repos.narrative_event.npc.clone();
    let narrative_event_query = repos.narrative_event.query.clone();

    // EventChain ISP ports
    let event_chain_crud = repos.event_chain.crud.clone();
    let event_chain_query = repos.event_chain.query.clone();
    let event_chain_membership = repos.event_chain.membership.clone();
    let event_chain_state = repos.event_chain.state.clone();

    // PlayerCharacter ISP ports
    let pc_crud = repos.player_character.crud.clone();
    let pc_query = repos.player_character.query.clone();
    let pc_position = repos.player_character.position.clone();
    let pc_inventory = repos.player_character.inventory.clone();

    // World exporter from infrastructure
    let world_exporter = infra.world_exporter.clone();

    // Create flag repository for scene condition evaluation
    let flag_repo: Arc<dyn wrldbldr_engine_ports::outbound::FlagRepositoryPort> =
        Arc::new(repository.flags());
    // Create observation repository for KnowsCharacter scene condition
    let observation_repo: Arc<dyn wrldbldr_engine_ports::outbound::ObservationRepositoryPort> =
        Arc::new(repository.observations());

    // ===========================================================================
    // Level 1b: Core Services (using factory for port traits)
    // ===========================================================================
    // The factory creates all core services and returns Arc<dyn *ServicePort> for each.
    // These port traits are used by the composition layer (CoreServices, PlayerServices).
    //
    // We also need Arc<dyn *Service> (app-layer traits) for AppRequestHandler, which
    // requires the full service interface. These are created inline below.
    // Note: scene_repo available if scene resolution service needs it in the future
    let core_service_ports = create_core_services(CoreServiceDependencies {
        world_repo: world_repo.clone(),
        world_exporter: world_exporter.clone(),
        settings_service: settings_service.clone(),
        clock: clock.clone(),
        character_crud: character_crud.clone(),
        character_want: character_want.clone(),
        relationship_repo: relationship_repo.clone(),
        location_crud: location_crud.clone(),
        location_hierarchy: location_hierarchy.clone(),
        location_connection: location_connection.clone(),
        location_map: location_map.clone(),
        scene_crud: scene_crud.clone(),
        scene_query: scene_query.clone(),
        scene_location: scene_location.clone(),
        scene_featured_character: scene_featured_character.clone(),
        scene_completion: scene_completion.clone(),
        skill_repo: skill_repo.clone(),
        interaction_repo: interaction_repo.clone(),
        item_repo: item_repo.clone(),
        region_item: region_item.clone(),
        pc_crud: pc_crud.clone(),
        pc_query: pc_query.clone(),
        pc_position: pc_position.clone(),
        pc_inventory: pc_inventory.clone(),
        flag_repo: flag_repo.clone(),
        observation_repo: observation_repo.clone(),
        sheet_template_repo: sheet_template_repo.clone(),
    });

    // Extract port traits for use in composition layer and adapters
    let world_service_port = core_service_ports.world_service_port;
    let character_service_port = core_service_ports.character_service_port;
    let location_service_port = core_service_ports.location_service_port;
    let scene_service_port = core_service_ports.scene_service_port;
    let skill_service_port = core_service_ports.skill_service_port;
    let interaction_service_port = core_service_ports.interaction_service_port;
    let relationship_service_port = core_service_ports.relationship_service_port;
    let item_service_port = core_service_ports.item_service_port;
    let player_character_service_port = core_service_ports.player_character_service_port;
    let scene_resolution_service_port = core_service_ports.scene_resolution_service_port;
    let sheet_template_service_port = core_service_ports.sheet_template_service_port;

    // Extract app-layer traits for use in AppRequestHandler
    // These point to the SAME instances as the port traits (no duplication)
    let world_service = core_service_ports.world_service;
    let character_service = core_service_ports.character_service;
    let location_service = core_service_ports.location_service;
    let scene_service = core_service_ports.scene_service;
    let skill_service = core_service_ports.skill_service;
    let interaction_service = core_service_ports.interaction_service;
    let relationship_service = core_service_ports.relationship_service;
    let item_service = core_service_ports.item_service;
    let player_character_service = core_service_ports.player_character_service;

    // AppRequestHandler needs Arc<SheetTemplateService> (concrete type), not trait object
    let sheet_template_service = Arc::new(SheetTemplateService::new(sheet_template_repo));

    // Keep relationship_repo for effects (EventEffectExecutor needs it)
    let relationship_repo_for_effects = relationship_repo.clone();

    // Repos needed for trigger evaluation service (Phase 2) - uses only specific ISP traits
    let narrative_event_crud_for_triggers = narrative_event_crud.clone();
    let story_event_query_for_triggers = story_event_query.clone();
    let story_event_edge_for_triggers = story_event_edge.clone();
    // Repos needed for event effect executor (Phase 2) - uses only NarrativeEventCrudPort and ChallengeCrudPort
    let narrative_event_crud_for_effects = narrative_event_crud.clone();
    let challenge_crud_for_effects = challenge_crud.clone();

    // Note: Game services (Challenge, EventChain, StoryEvent, NarrativeEvent) are created
    // after event_infra using the game_services factory (Level 3b)
    //
    // ChallengeResolutionService and NarrativeEventApprovalService now use port traits
    // (ChallengeServicePort, SkillServicePort, PlayerCharacterServicePort, etc.)
    // so no duplicate service instantiations are needed here. (Phase 2A.3 complete)

    let pc_crud_for_triggers = pc_crud.clone();
    let pc_crud_for_actantial = pc_crud.clone();
    let pc_crud_for_handler = pc_crud.clone();

    // Clone for request handler
    let observation_repo_for_handler = observation_repo.clone();

    // Create outcome trigger service for challenge resolution (Phase 22D)
    // Uses ISP: ChallengeCrudPort only
    let outcome_trigger_service = Arc::new(OutcomeTriggerService::new(challenge_crud.clone()));

    // Note: world_connection_manager and world_state are already extracted from infra above

    // ===========================================================================
    // Level 2: Event Infrastructure + Queue Backends (parallel)
    // ===========================================================================
    let (event_infra, queue_backends) = tokio::try_join!(
        super::factories::create_event_infrastructure(&config, clock.clone()),
        super::factories::queue_services::create_queue_backends(&config, clock.clone()),
    )?;

    // Extract event infrastructure components
    let event_bus = event_infra.event_bus;
    let event_notifier = event_infra.event_notifier;
    let domain_event_repository = event_infra.domain_event_repository;
    let generation_read_state_repository = event_infra.generation_read_state_repository;
    let generation_event_tx = event_infra.generation_event_tx;
    let generation_event_rx = event_infra.generation_event_rx;
    let challenge_approval_tx = event_infra.challenge_approval_tx;
    let challenge_approval_rx = event_infra.challenge_approval_rx;

    // Note: queue_backends is passed to create_queue_services() by reference,
    // then individual queues are extracted from QueueServiceContext as needed

    // ===========================================================================
    // Level 3b: Game Services (using factory)
    // ===========================================================================
    let game_service_ports =
        super::factories::create_game_services(super::factories::GameServiceDependencies {
            // Challenge ISP ports
            challenge_crud: challenge_crud.clone(),
            challenge_skill: challenge_skill.clone(),
            challenge_scene: challenge_scene.clone(),
            challenge_prerequisite: challenge_prerequisite.clone(),
            challenge_availability: challenge_availability.clone(),
            // EventChain ISP ports
            event_chain_crud,
            event_chain_query,
            event_chain_membership,
            event_chain_state,
            // StoryEvent ISP ports
            story_event_crud: story_event_crud.clone(),
            story_event_edge: story_event_edge.clone(),
            story_event_query: story_event_query.clone(),
            story_event_dialogue: story_event_dialogue.clone(),
            // NarrativeEvent ISP ports
            narrative_event_crud: narrative_event_crud.clone(),
            narrative_event_tie: narrative_event_tie.clone(),
            narrative_event_npc: narrative_event_npc.clone(),
            narrative_event_query: narrative_event_query.clone(),
            // Shared dependencies
            event_bus: event_bus.clone(),
            clock: clock.clone(),
        });

    // Extract game service ports and app-layer traits
    let challenge_service_port = game_service_ports.challenge_service_port;
    let challenge_service = game_service_ports.challenge_service;
    let event_chain_service_port = game_service_ports.event_chain_service_port;
    let event_chain_service = game_service_ports.event_chain_service;
    let story_event_service_port = game_service_ports.story_event_service_port;
    let story_event_service = game_service_ports.story_event_service;
    let dialogue_context_service = game_service_ports.dialogue_context_service;
    let narrative_event_service_port = game_service_ports.narrative_event_service_port;
    let narrative_event_service = game_service_ports.narrative_event_service;
    // StoryEventRecordingServicePort for NarrativeEventApprovalService (Phase 2A.3)
    let story_event_recording_service = game_service_ports.story_event_recording_service;

    // ===========================================================================
    // Level 2b: Queue Services (using factory)
    // ===========================================================================
    let (queue_service_ports, queue_worker_services) =
        super::factories::create_queue_services(super::factories::QueueServiceDependencies {
            config: &config,
            clock: clock.clone(),
            llm_client: llm_client.clone(),
            comfyui_client: comfyui_client.clone(),
            prompt_template_service: prompt_template_service.clone(),
            repos: &repos,
            queue_backends: &queue_backends,
            dialogue_context_service: dialogue_context_service.clone(),
            generation_event_tx: generation_event_tx.clone(),
            narrative_event_service: narrative_event_service.clone(),
            scene_service: scene_service.clone(),
            interaction_service: interaction_service.clone(),
        })?;

    // Extract port versions (for AppState)
    let player_action_queue_service_port = queue_service_ports.player_action_queue_service_port;
    let dm_action_queue_service_port = queue_service_ports.dm_action_queue_service_port;
    let llm_queue_service_port = queue_service_ports.llm_queue_service_port;
    let asset_generation_queue_service_port =
        queue_service_ports.asset_generation_queue_service_port;
    let dm_approval_queue_service_port = queue_service_ports.dm_approval_queue_service_port;
    let challenge_outcome_queue_port = queue_service_ports.challenge_outcome_queue_port;

    // Extract concrete versions (for WorkerServices and adapters)
    let player_action_queue_service = queue_worker_services.player_action_queue_service;
    let dm_action_queue_service = queue_worker_services.dm_action_queue_service;
    let llm_queue_service = queue_worker_services.llm_queue_service;
    let asset_generation_queue_service = queue_worker_services.asset_generation_queue_service;
    let dm_approval_queue_service = queue_worker_services.dm_approval_queue_service;
    let challenge_outcome_queue = queue_worker_services.challenge_outcome_queue;

    // DM action processor
    let dm_action_processor = queue_worker_services.dm_action_processor;

    // ===========================================================================
    // Level 2c: Asset Services (using factory)
    // ===========================================================================
    // Create all asset services using the factory
    let asset_services = create_asset_services(AssetServiceDependencies {
        clock: clock.clone(),
        comfyui: Arc::new(comfyui_client.clone()) as Arc<dyn ComfyUIPort>,
        settings_service: settings_service.clone() as Arc<dyn SettingsServicePort>,
        asset_repo: asset_repo.clone(),
        workflow_repo: workflow_repo.clone(),
        generation_event_tx,
        domain_event_repository: domain_event_repository.clone(),
        generation_read_state_repository: generation_read_state_repository.clone(),
        asset_base_path: config.asset_base_path.clone(),
        workflow_path: config.workflow_path.clone(),
    });

    // Extract port versions for composition layer
    let asset_service_port = asset_services.asset_service.clone();
    let workflow_config_service_port = asset_services.workflow_config_service.clone();
    let generation_service = asset_services.generation_service.clone();
    let generation_queue_projection_service =
        asset_services.generation_queue_projection_service.clone();
    // AppRequestHandler now depends on the projection port.

    // Create challenge outcome approval service (P3.3) - must be created before resolution service
    // Wire LLM port for suggestion generation, settings service for branch count,
    // and persistent queue for challenge outcomes
    //
    // The service uses an event channel instead of WorldConnectionPort for hexagonal compliance.
    // Events are published by ChallengeApprovalEventPublisher (started in server.rs).
    // challenge_approval_tx/rx already extracted from event_infra above
    let llm_for_suggestions = Arc::new(llm_client.clone());
    let challenge_outcome_pending: Arc<dyn wrldbldr_engine_ports::outbound::ChallengeOutcomePendingPort> =
        Arc::new(
            wrldbldr_engine_adapters::infrastructure::in_memory::InMemoryChallengeOutcomePendingStore::new(),
        );
    let challenge_outcome_approval_service = Arc::new(ChallengeOutcomeApprovalService::new(
        challenge_approval_tx,
        outcome_trigger_service.clone() as Arc<dyn OutcomeTriggerServicePort>,
        pc_crud_for_triggers.clone(),
        pc_inventory.clone(),
        item_repo.clone(),
        prompt_template_service.clone(),
        challenge_outcome_pending,
        challenge_outcome_queue.clone(),
        llm_for_suggestions,
        settings_service.clone() as Arc<dyn SettingsServicePort>,
        clock.clone(),
    ));

    // Create challenge resolution service with required approval service
    // Now uses port traits (Phase 2A.3 - no more generics, single service instances)
    // Note: No longer takes WorldConnectionPort - returns typed results for use case layer to broadcast
    let challenge_resolution_service = Arc::new(ChallengeResolutionService::new(
        challenge_service_port.clone(),
        skill_service_port.clone(),
        player_character_service_port.clone(),
        dm_approval_queue_service.clone() as Arc<dyn ApprovalRequestLookupPort>,
        challenge_outcome_approval_service.clone() as Arc<dyn ChallengeOutcomeApprovalServicePort>,
        clock.clone(),
        rng.clone(),
    ));

    // Create narrative event approval service
    // Now uses port traits (Phase 2A.3 - no more generics, single service instances)
    let narrative_event_approval_service = Arc::new(NarrativeEventApprovalService::new(
        narrative_event_service_port.clone(),
        story_event_recording_service.clone(),
    ));

    // Create trigger evaluation service (Phase 2)
    // Uses ISP: NarrativeEventCrudPort, PlayerCharacterCrudPort, StoryEventQueryPort, StoryEventEdgePort
    let trigger_evaluation_service = Arc::new(TriggerEvaluationService::new(
        narrative_event_crud_for_triggers,
        pc_crud_for_triggers,
        story_event_query_for_triggers,
        story_event_edge_for_triggers,
    ));

    // Create event effect executor (Phase 2)
    // Uses ISP: ChallengeCrudPort, NarrativeEventCrudPort only
    let event_effect_executor = Arc::new(EventEffectExecutor::new(
        challenge_crud_for_effects,
        narrative_event_crud_for_effects,
        relationship_repo_for_effects,
    ));

    // Create staging service (Staging System)
    // Note: StagingService is generic over concrete types, so we need concrete Arc<...>
    // Uses ISP: RegionCrudPort, RegionNpcPort, NarrativeEventCrudPort
    let staging_repo = Arc::new(repository.stagings());
    let region_crud_for_staging = Arc::new(repository.regions());
    let region_npc_for_staging = Arc::new(repository.regions());
    // Create a fresh concrete narrative_event_repo for staging (it only needs CrudPort)
    let narrative_event_repo_for_staging = Arc::new(repository.narrative_events());
    let llm_for_staging = Arc::new(llm_client.clone());
    let staging_service = Arc::new(StagingService::new(
        staging_repo,
        region_crud_for_staging,
        region_npc_for_staging,
        narrative_event_repo_for_staging,
        story_event_service.clone(),
        llm_for_staging,
        prompt_template_service.clone(),
        clock.clone(),
    ));

    // Note: generation_queue_projection_service already created by asset_services factory above

    // Create disposition service (P1.4)
    // Uses ISP: CharacterDispositionPort only
    let disposition_service = Arc::new(DispositionServiceImpl::new(
        character_disposition.clone(),
        clock.clone(),
    ));

    // Create region service
    // Uses ISP: RegionCrudPort, RegionConnectionPort, RegionExitPort, RegionNpcPort, LocationCrudPort, LocationMapPort
    let region_service: Arc<dyn wrldbldr_engine_app::application::services::RegionService> =
        Arc::new(RegionServiceImpl::new(
            region_crud.clone(),
            region_connection.clone(),
            region_exit.clone(),
            region_npc.clone(),
            location_crud.clone(),
            location_map.clone(),
        ));

    // Create actantial context service (P1.5)
    // Uses ISP: CharacterCrudPort, CharacterWantPort, CharacterActantialPort, PlayerCharacterCrudPort
    let actantial_context_service = Arc::new(ActantialContextServiceImpl::new(
        character_crud.clone(),
        character_want.clone(),
        character_actantial.clone(),
        pc_crud_for_actantial.clone(),
        goal_repo,
        want_repo,
        clock.clone(),
    ));

    // Create prompt context service for building LLM prompts from player actions
    // Uses ISP: CharacterCrudPort, PlayerCharacterCrudPort, RegionItemPort
    // PromptContextServiceImpl now implements both app-layer trait and port trait directly
    let prompt_context_service_impl = Arc::new(PromptContextServiceImpl::new(
        world_service.clone(),
        world_state.clone(),
        challenge_service.clone(),
        skill_service.clone(),
        narrative_event_service.clone(),
        character_crud.clone(),
        pc_crud_for_actantial,
        region_item.clone(),
        disposition_service.clone(),
        actantial_context_service.clone()
            as Arc<dyn wrldbldr_engine_app::application::services::ActantialContextService>,
    ));
    // Cast to app-layer trait for WorkerServices (which needs the app-layer signature)
    let prompt_context_service: Arc<
        dyn wrldbldr_engine_app::application::services::PromptContextService,
    > = prompt_context_service_impl.clone();
    // Cast to port trait for AppState (no adapter shim needed - impl is direct now)
    let prompt_context_service_port: Arc<dyn PromptContextServicePort> =
        prompt_context_service_impl;

    // Clone generation services for use in request handler
    let generation_queue_projection_for_handler = generation_queue_projection_service.clone();
    let generation_read_state_for_handler = generation_read_state_repository.clone();

    // Create LLM suggestion queue adapter (bridges port to internal service)
    // This adapter implements LlmSuggestionQueuePort by delegating to the internal LlmQueueServicePort
    let llm_suggestion_queue_adapter: Arc<dyn wrldbldr_engine_ports::outbound::LlmSuggestionQueuePort> =
        Arc::new(LlmSuggestionQueueAdapter::new(llm_queue_service_port.clone()));
    tracing::debug!("Created LLM suggestion queue adapter bridge");

    // Create suggestion enqueue adapter for AI suggestions
    // Pass world_repo for auto-enrichment of suggestion context
    // Uses llm_suggestion_queue_adapter (port) instead of internal service
    let suggestion_enqueue_adapter: Arc<
        dyn wrldbldr_engine_ports::outbound::SuggestionEnqueuePort,
    > = Arc::new(SuggestionEnqueueAdapter::new(
        llm_suggestion_queue_adapter,
        world_repo.clone(),
    ));
    tracing::info!("Initialized suggestion enqueue adapter with world auto-enrichment");

    // Create request handler for WebSocket-first architecture
    // Services are already Arc<dyn Trait>, so just clone them
    // Clone ISP sub-trait ports for the handler and use cases
    // AppRequestHandler needs CharacterLocationPort, use cases need CharacterCrudPort
    let character_location_for_handler = character_location.clone();
    let character_crud_for_use_cases = character_crud.clone();
    let observation_repo_for_use_cases = observation_repo_for_handler.clone();

    let request_handler: Arc<dyn RequestHandler> = Arc::new(AppRequestHandler::new(
        world_service.clone(),
        character_service.clone(),
        location_service.clone(),
        skill_service.clone(),
        scene_service.clone(),
        interaction_service.clone(),
        challenge_service.clone(),
        narrative_event_service.clone(),
        event_chain_service.clone(),
        player_character_service.clone(),
        relationship_service.clone(),
        actantial_context_service.clone()
            as Arc<dyn wrldbldr_engine_app::application::services::ActantialContextService>,
        disposition_service.clone()
            as Arc<dyn wrldbldr_engine_app::application::services::DispositionService>,
        story_event_service.clone(),
        item_service.clone(),
        region_service,
        sheet_template_service.clone(),
        character_location_for_handler, // ISP: Uses CharacterLocationPort
        observation_repo_for_handler,
        region_crud.clone(), // ISP: Uses RegionCrudPort
        suggestion_enqueue_adapter,
        generation_queue_projection_for_handler,
        generation_read_state_for_handler,
        clock.clone(),
    ));
    tracing::info!("Initialized request handler for WebSocket-first architecture");

    // ===========================================================================
    // Create use cases (using factory)
    // ===========================================================================
    let use_case_ctx = create_use_cases(UseCaseDependencies {
        // Infrastructure
        connection_manager: world_connection_manager.clone() as Arc<dyn ConnectionManagerPort>,
        connection_broadcast: world_connection_manager.clone() as Arc<dyn ConnectionBroadcastPort>,
        connection_unicast: world_connection_manager.clone() as Arc<dyn ConnectionUnicastPort>,
        dm_notification: world_connection_manager.clone() as Arc<dyn DmNotificationPort>,
        staging_state: staging_state.clone(),
        world_state_update: world_state_update.clone(),
        clock: clock.clone(),
        // Repository ports (ISP-split PC traits)
        pc_crud: pc_crud_for_handler.clone(),
        pc_position: pc_position.clone(),
        pc_inventory: pc_inventory.clone(),
        region_crud: region_crud.clone(),
        region_connection: region_connection.clone(),
        region_exit: region_exit.clone(),
        region_item: region_item.clone(),
        location_crud: location_crud.clone(),
        location_map: location_map.clone(),
        character_crud: character_crud_for_use_cases.clone(),
        observation_repo: observation_repo_for_use_cases.clone(),
        directorial_context_repo: directorial_context_repo.clone(),
        // Service ports (inbound)
        scene_service_port: scene_service_port.clone(),
        interaction_service_port: interaction_service_port.clone(),
        world_service_port: world_service_port.clone(),
        player_character_service_port: player_character_service_port.clone(),
        // Service ports (outbound)
        staging_service: staging_service.clone(),
        player_action_queue_service_port: player_action_queue_service_port.clone(),
        challenge_resolution_service_port: challenge_resolution_service.clone(),
        challenge_outcome_approval_service_port: challenge_outcome_approval_service.clone(),
        dm_approval_queue_service_port: dm_approval_queue_service_port.clone(),
        dm_action_queue_service_port: dm_action_queue_service_port.clone(),
        narrative_event_approval_service: narrative_event_approval_service.clone()
            as Arc<dyn NarrativeEventApprovalServicePort>,
        // Request handler
        request_handler: request_handler.clone(),
    });

    // Extract use cases container and broadcast for use in composition layer
    let composition_use_cases = use_case_ctx.use_cases;
    let broadcast = use_case_ctx.broadcast;

    // ===========================================================================
    // Create composition-layer CoreServices (port-based)
    // ===========================================================================
    // Use the port-typed Arc versions created earlier (not app service trait versions)
    let composition_core = CoreServices::new(
        world_service_port.clone(),
        character_service_port.clone(),
        location_service_port.clone(),
        scene_service_port.clone(),
        skill_service_port.clone(),
        interaction_service_port.clone(),
        relationship_service_port.clone(),
        item_service_port.clone(),
    );

    // ===========================================================================
    // Create composition-layer GameServices (port-based)
    // ===========================================================================
    // Use port-typed versions for services, cast for services that are already Arc<concrete>
    let composition_game = GameServices::new(
        story_event_service_port.clone(),
        challenge_service_port.clone(),
        challenge_resolution_service.clone() as Arc<dyn ChallengeResolutionServicePort>,
        challenge_outcome_approval_service.clone() as Arc<dyn ChallengeOutcomeApprovalServicePort>,
        narrative_event_service_port.clone(),
        narrative_event_approval_service.clone() as Arc<dyn NarrativeEventApprovalServicePort>,
        event_chain_service_port.clone(),
        trigger_evaluation_service.clone() as Arc<dyn TriggerEvaluationServicePort>,
        event_effect_executor.clone() as Arc<dyn EventEffectExecutorPort>,
        disposition_service.clone() as Arc<dyn DispositionServicePort>,
        actantial_context_service.clone() as Arc<dyn ActantialContextServicePort>,
    );

    // ===========================================================================
    // Create composition-layer QueueServices (port-based)
    // ===========================================================================
    // Use port versions directly from queue_service_ctx (no casting needed)
    let composition_queues = QueueServices::new(
        player_action_queue_service_port.clone(),
        dm_action_queue_service_port.clone(),
        llm_queue_service_port.clone(),
        asset_generation_queue_service_port.clone(),
        dm_approval_queue_service_port.clone(),
        challenge_outcome_queue_port.clone(),
    );

    // ===========================================================================
    // Create composition-layer AssetServices (port-based)
    // ===========================================================================
    // Use port versions from the asset_services factory
    let composition_assets = AssetServices::new(
        asset_service_port,
        workflow_config_service_port,
        generation_service.clone(),
        generation_queue_projection_service.clone(),
    );

    // ===========================================================================
    // Create composition-layer PlayerServices (port-based)
    // ===========================================================================
    // Use port-typed versions from factory
    let composition_player = PlayerServices::new(
        sheet_template_service_port.clone(),
        player_character_service_port.clone(),
        scene_resolution_service_port.clone(),
    );

    // ===========================================================================
    // Create composition-layer EventInfra (port-based)
    // ===========================================================================
    let composition_events = EventInfra::new(
        event_bus.clone() as Arc<dyn EventBusPort>,
        event_notifier.clone() as Arc<dyn EventNotifierPort>,
        domain_event_repository.clone() as Arc<dyn DomainEventRepositoryPort>,
        generation_read_state_repository.clone() as Arc<dyn GenerationReadStatePort>,
    );

    // ===========================================================================
    // Create composition-layer AppConfig
    // ===========================================================================
    let composition_config = CompositionAppConfig::new(
        config.server_host.clone(),
        config.server_port,
        config.neo4j_uri.clone(),
        config.ollama_base_url.clone(),
        config.comfyui_base_url.clone(),
    );

    // ===========================================================================
    // Create composition-layer AppState (pure ports)
    // ===========================================================================
    let composition_app_state = AppState::new(
        composition_config,
        Arc::new(llm_client.clone()) as Arc<dyn LlmPortDyn>,
        Arc::new(comfyui_client.clone()) as Arc<dyn ComfyUIPort>,
        region_item.clone() as Arc<dyn RegionItemPort>,
        composition_core,
        composition_game,
        composition_queues,
        composition_assets,
        composition_player,
        composition_events,
        settings_service.clone() as Arc<dyn SettingsServicePort>,
        prompt_template_service.clone() as Arc<dyn PromptTemplateServicePort>,
        staging_service.clone() as Arc<dyn StagingUseCaseServiceExtPort>,
        world_connection_manager.clone() as Arc<dyn ConnectionQueryPort>,
        world_connection_manager.clone() as Arc<dyn ConnectionContextPort>,
        world_connection_manager.clone() as Arc<dyn ConnectionBroadcastPort>,
        world_connection_manager.clone() as Arc<dyn ConnectionLifecyclePort>,
        world_state.clone() as Arc<dyn WorldTimePort>,
        world_state.clone() as Arc<dyn WorldConversationPort>,
        world_state.clone() as Arc<dyn WorldApprovalPort>,
        world_state.clone() as Arc<dyn WorldScenePort>,
        world_state.clone() as Arc<dyn WorldDirectorialPort>,
        world_state.clone() as Arc<dyn WorldLifecyclePort>,
        request_handler.clone(),
        directorial_context_repo.clone(),
        composition_use_cases,
        prompt_context_service_port,
        clock.clone(),
    );

    // ===========================================================================
    // Create WorkerServices (concrete queue types for background workers)
    // ===========================================================================
    // Workers need concrete types because they call infrastructure-specific methods
    // like run_worker() and process_next() that aren't part of the port traits
    let worker_services = WorkerServices {
        llm_queue_service: llm_queue_service.clone(),
        asset_generation_queue_service: asset_generation_queue_service.clone(),
        player_action_queue_service: player_action_queue_service.clone(),
        dm_action_queue_service: dm_action_queue_service.clone(),
        dm_approval_queue_service: dm_approval_queue_service.clone(),
        dm_action_processor: dm_action_processor.clone(),
        challenge_outcome_queue: challenge_outcome_queue.clone(),
        prompt_context_service: prompt_context_service.clone(),
        world_connection_manager: world_connection_manager.clone(),
        event_bus: event_bus.clone(),
        broadcast: broadcast.clone(),
    };

    Ok((
        composition_app_state,
        worker_services,
        generation_event_rx,
        challenge_approval_rx,
    ))
}

/// Alias for `new_app_state` for backward compatibility.
///
/// This is the preferred entry point for creating application state.
pub async fn new_adapter_state(
    config: AppConfig,
) -> Result<(
    AppState,
    WorkerServices,
    tokio::sync::mpsc::Receiver<GenerationEvent>,
    tokio::sync::mpsc::Receiver<ChallengeApprovalEvent>,
)> {
    new_app_state(config).await
}
