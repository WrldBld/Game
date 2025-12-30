//! Application State Construction
//!
//! This module provides the construction logic for AppState.
//! The AppState struct is defined in engine-composition, but the construction
//! (dependency injection and wiring) is done here in the composition root.

use std::sync::Arc;

use anyhow::Result;

use wrldbldr_engine_adapters::infrastructure::config::AppConfig;
use wrldbldr_engine_adapters::infrastructure::export::Neo4jWorldExporter;
// Import port adapters for use case construction (replaces OldUseCases::new())
use wrldbldr_engine_adapters::infrastructure::ports::{
    ChallengeDmApprovalQueueAdapter, ChallengeOutcomeApprovalAdapter, ChallengeResolutionAdapter,
    ConnectionDirectorialContextAdapter, ConnectionManagerAdapter, ConnectionWorldStateAdapter,
    DirectorialContextAdapter, DmActionQueuePlaceholder, DmNotificationAdapter,
    InteractionServiceAdapter, PlayerActionQueueAdapter, PlayerCharacterServiceAdapter,
    SceneServiceAdapter, SceneWorldStateAdapter, StagingServiceAdapter, StagingStateAdapter,
    WorldServiceAdapter,
};
use wrldbldr_engine_adapters::infrastructure::websocket::WebSocketBroadcastAdapter;
use wrldbldr_engine_adapters::infrastructure::suggestion_enqueue_adapter::SuggestionEnqueueAdapter;
use wrldbldr_engine_adapters::infrastructure::world_connection_manager::SharedWorldConnectionManager;
use wrldbldr_engine_adapters::infrastructure::TokioFileStorageAdapter;

use wrldbldr_engine_app::application::handlers::AppRequestHandler;
use wrldbldr_engine_app::application::services::generation_service::{
    GenerationEvent, GenerationService,
};
use wrldbldr_engine_app::application::services::{
    challenge_resolution_service::ChallengeResolutionService, staging_service::StagingService,
    ActantialContextServiceImpl, AssetGenerationQueueService, AssetServiceImpl,
    ChallengeApprovalEvent, ChallengeOutcomeApprovalService, ChallengeServiceImpl,
    CharacterServiceImpl, DMApprovalQueueService, DispositionServiceImpl, DmActionProcessorService,
    DmActionQueueService, EventChainServiceImpl, EventEffectExecutor,
    GenerationQueueProjectionService, InteractionServiceImpl, ItemServiceImpl, LLMQueueService,
    LocationServiceImpl, NarrativeEventApprovalService, NarrativeEventServiceImpl,
    OutcomeTriggerService, PlayerActionQueueService, PlayerCharacterServiceImpl,
    PromptContextServiceImpl, RegionServiceImpl, RelationshipServiceImpl,
    SceneResolutionServiceImpl, SceneServiceImpl, SheetTemplateService,
    SkillServiceImpl, StoryEventServiceImpl, TriggerEvaluationService, WorkflowConfigService,
    WorldServiceImpl,
};
use wrldbldr_engine_app::application::use_cases::{
    ChallengeUseCase, ConnectionUseCase, InventoryUseCase, MovementUseCase, NarrativeEventUseCase,
    ObservationUseCase, PlayerActionUseCase, SceneBuilder, SceneUseCase, StagingApprovalUseCase,
};

// Import composition layer types
use wrldbldr_engine_composition::{
    AppConfig as CompositionAppConfig, AppState, AssetServices, CoreServices, EventInfra,
    GameServices, LlmPortDyn, PlayerServices, QueueServices, UseCases,
};

use wrldbldr_engine_ports::inbound::{
    ChallengeUseCasePort, ConnectionUseCasePort, InventoryUseCasePort, MovementUseCasePort,
    NarrativeEventUseCasePort, ObservationUseCasePort, PlayerActionUseCasePort, RequestHandler,
    SceneUseCasePort, StagingUseCasePort,
};
use wrldbldr_engine_ports::outbound::{
    ActantialContextServicePort,
    AssetGenerationQueueServicePort,
    AssetServicePort,
    BroadcastPort,
    ChallengeOutcomeApprovalServicePort,
    ChallengeResolutionServicePort,
    ChallengeServicePort,
    CharacterServicePort,
    ComfyUIPort,
    DispositionServicePort,
    DmActionProcessorPort,
    DmActionQueueServicePort,
    DmApprovalQueueServicePort,
    DomainEventRepositoryPort,
    EventBusPort,
    EventChainServicePort,
    EventEffectExecutorPort,
    EventNotifierPort,
    GenerationQueueProjectionServicePort,
    GenerationReadStatePort,
    GenerationServicePort,
    InteractionServicePort,
    ItemServicePort,
    LlmQueueServicePort,
    LocationServicePort,
    NarrativeEventApprovalServicePort,
    NarrativeEventServicePort,
    PlayerActionQueueServicePort,
    PlayerCharacterServicePort,
    PromptContextServicePort,
    PromptTemplateServicePort,
    QueuePort,
    RegionItemPort,
    RelationshipServicePort,
    SceneResolutionServicePort,
    SceneServicePort,
    SettingsServicePort,
    SheetTemplateServicePort,
    SkillServicePort,
    StagingServicePort,
    StoryEventServicePort,
    TriggerEvaluationServicePort,
    WorkflowServicePort,
    ConnectionBroadcastPort, ConnectionContextPort, ConnectionLifecyclePort, ConnectionQueryPort,
    WorldApprovalPort, WorldConversationPort, WorldDirectorialPort, WorldLifecyclePort,
    WorldScenePort, WorldServicePort, WorldTimePort,
};

// Re-export AppStatePort for server.rs
pub use wrldbldr_engine_ports::inbound::AppStatePort;

use wrldbldr_domain::value_objects::{
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
    pub llm_queue_service: Arc<
        LLMQueueService<
            QueueBackendEnum<LlmRequestData>,
            OllamaClientType,
            InProcessNotifier,
        >,
    >,

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
    let repository = infra.neo4j.clone();
    let llm_client = infra.llm_client.clone();
    let comfyui_client = infra.comfyui_client.clone();
    let settings_service = infra.settings_service_concrete.clone();
    let prompt_template_service = infra.prompt_template_service_concrete.clone();
    let directorial_context_repo = infra.directorial_context_repo.clone();
    let world_connection_manager = infra.world_connection_manager.clone();
    let world_state = infra.world_state.clone();

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
    let scene_repo = repos.scene_repo.clone();

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

    // EventChain ISP ports - extract god trait for now (services use full interface)
    let event_chain_repo: Arc<dyn wrldbldr_engine_ports::outbound::EventChainRepositoryPort> =
        Arc::new(repository.event_chains());

    // PlayerCharacter - extract god trait (services use full interface)
    let player_character_repo = repos.player_character.god.clone();

    // Create world exporter
    let world_exporter: Arc<dyn wrldbldr_engine_ports::outbound::WorldExporterPort> =
        Arc::new(Neo4jWorldExporter::new(repository.clone()));

    // Initialize application services as Arc<dyn Trait> for shared ownership
    // This allows services to be shared between grouped service structs and AppRequestHandler
    //
    // Each service is created with two Arc versions:
    // - Arc<dyn *Service> for the old adapter-layer containers (OldCoreServices, etc.)
    // - Arc<dyn *ServicePort> for the composition-layer containers (CoreServices, etc.)
    // This is needed because trait objects can't be cast between different trait types.

    let world_service_impl = WorldServiceImpl::new(
        world_repo.clone(),
        world_exporter,
        settings_service.clone(),
        clock.clone(),
    );
    let world_service: Arc<dyn wrldbldr_engine_app::application::services::WorldService> =
        Arc::new(world_service_impl.clone());
    let world_service_port: Arc<dyn WorldServicePort> = Arc::new(world_service_impl);

    let character_service_impl = CharacterServiceImpl::new(
        world_repo.clone(),
        character_crud.clone(),
        character_want.clone(),
        relationship_repo.clone(),
        settings_service.clone(),
        clock.clone(),
    );
    let character_service: Arc<dyn wrldbldr_engine_app::application::services::CharacterService> =
        Arc::new(character_service_impl.clone());
    let character_service_port: Arc<dyn CharacterServicePort> = Arc::new(character_service_impl);

    let location_service_impl = LocationServiceImpl::new(
        world_repo.clone(),
        location_crud.clone(),
        location_hierarchy.clone(),
        location_connection.clone(),
        location_map.clone(),
    );
    let location_service: Arc<dyn wrldbldr_engine_app::application::services::LocationService> =
        Arc::new(location_service_impl.clone());
    let location_service_port: Arc<dyn LocationServicePort> = Arc::new(location_service_impl);

    let relationship_repo_for_effects = relationship_repo.clone();
    let relationship_service_impl = RelationshipServiceImpl::new(relationship_repo);
    let relationship_service: Arc<
        dyn wrldbldr_engine_app::application::services::RelationshipService,
    > = Arc::new(relationship_service_impl.clone());
    let relationship_service_port: Arc<dyn RelationshipServicePort> =
        Arc::new(relationship_service_impl);

    let scene_repo_for_resolution = scene_repo.clone();

    let scene_service_impl = SceneServiceImpl::new(
        scene_repo.clone(),
        location_crud.clone(),
        character_crud.clone(),
    );
    let scene_service: Arc<dyn wrldbldr_engine_app::application::services::SceneService> =
        Arc::new(scene_service_impl.clone());
    let scene_service_port: Arc<dyn SceneServicePort> = Arc::new(scene_service_impl);

    let skill_service_impl_for_port = SkillServiceImpl::new(skill_repo.clone(), world_repo.clone());
    let skill_service: Arc<dyn wrldbldr_engine_app::application::services::SkillService> =
        Arc::new(skill_service_impl_for_port.clone());
    let skill_service_port: Arc<dyn SkillServicePort> = Arc::new(skill_service_impl_for_port);

    let interaction_service_impl = InteractionServiceImpl::new(interaction_repo);
    let interaction_service: Arc<
        dyn wrldbldr_engine_app::application::services::InteractionService,
    > = Arc::new(interaction_service_impl.clone());
    let interaction_service_port: Arc<dyn InteractionServicePort> =
        Arc::new(interaction_service_impl);

    // StoryEvent ISP ports needed for services (will be used after event_bus is created)
    let story_event_crud_for_service = story_event_crud.clone();
    let story_event_edge_for_service = story_event_edge.clone();
    let story_event_query_for_service = story_event_query.clone();
    let story_event_dialogue_for_service = story_event_dialogue.clone();

    let challenge_service_for_port = ChallengeServiceImpl::new(
        challenge_crud.clone(),
        challenge_skill.clone(),
        challenge_scene.clone(),
        challenge_prerequisite.clone(),
        challenge_availability.clone(),
    );
    let challenge_service: Arc<dyn wrldbldr_engine_app::application::services::ChallengeService> =
        Arc::new(challenge_service_for_port.clone());
    let challenge_service_port: Arc<dyn ChallengeServicePort> =
        Arc::new(challenge_service_for_port);
    // Keep concrete version for ChallengeResolutionService generics
    let challenge_service_impl = ChallengeServiceImpl::new(
        challenge_crud.clone(),
        challenge_skill.clone(),
        challenge_scene.clone(),
        challenge_prerequisite.clone(),
        challenge_availability.clone(),
    );

    // NarrativeEvent ISP ports needed for services (will be created after event_bus)
    let narrative_event_crud_for_service = narrative_event_crud.clone();
    let narrative_event_tie_for_service = narrative_event_tie.clone();
    let narrative_event_npc_for_service = narrative_event_npc.clone();
    let narrative_event_query_for_service = narrative_event_query.clone();
    // Repos needed for trigger evaluation service (Phase 2) - uses only specific ISP traits
    let narrative_event_crud_for_triggers = narrative_event_crud.clone();
    let story_event_query_for_triggers = story_event_query.clone();
    let story_event_edge_for_triggers = story_event_edge.clone();
    // Repos needed for event effect executor (Phase 2) - uses only NarrativeEventCrudPort and ChallengeCrudPort
    let narrative_event_crud_for_effects = narrative_event_crud.clone();
    let challenge_crud_for_effects = challenge_crud.clone();

    // Clone event_chain_repo for port version before it's moved
    let _event_chain_repo_for_port = event_chain_repo.clone();
    let event_chain_service_impl_for_port = EventChainServiceImpl::new(event_chain_repo);
    let event_chain_service: Arc<
        dyn wrldbldr_engine_app::application::services::EventChainService,
    > = Arc::new(event_chain_service_impl_for_port.clone());
    let event_chain_service_port: Arc<dyn EventChainServicePort> =
        Arc::new(event_chain_service_impl_for_port);

    let asset_repo_for_service = asset_repo.clone();
    let asset_service = AssetServiceImpl::new(asset_repo_for_service, clock.clone());
    // Clone workflow_repo before creating service (we'll need it for composition layer too)
    let workflow_repo_for_composition = workflow_repo.clone();
    let _workflow_config_service = WorkflowConfigService::new(workflow_repo, clock.clone());
    let sheet_template_service = Arc::new(SheetTemplateService::new(sheet_template_repo));

    let item_service_for_port = ItemServiceImpl::new(
        item_repo.clone(),
        player_character_repo.clone(),
        region_item.clone(),
    );
    let item_service: Arc<dyn wrldbldr_engine_app::application::services::ItemService> =
        Arc::new(item_service_for_port.clone());
    let item_service_port: Arc<dyn ItemServicePort> = Arc::new(item_service_for_port);
    // Keep concrete version for DMApprovalQueueService
    let item_service_impl = ItemServiceImpl::new(
        item_repo.clone(),
        player_character_repo.clone(),
        region_item.clone(),
    );

    let player_character_repo_for_triggers = player_character_repo.clone();
    let player_character_repo_for_actantial = player_character_repo.clone();
    let player_character_repo_for_handler = player_character_repo.clone();

    let player_character_service_for_port = PlayerCharacterServiceImpl::new(
        player_character_repo.clone(),
        location_crud.clone(),
        world_repo.clone(),
        clock.clone(),
    );
    let player_character_service: Arc<
        dyn wrldbldr_engine_app::application::services::PlayerCharacterService,
    > = Arc::new(player_character_service_for_port.clone());
    let player_character_service_port: Arc<dyn PlayerCharacterServicePort> =
        Arc::new(player_character_service_for_port);
    // Keep concrete version for ChallengeResolutionService generics
    let player_character_service_impl = PlayerCharacterServiceImpl::new(
        player_character_repo.clone(),
        location_crud.clone(),
        world_repo.clone(),
        clock.clone(),
    );

    // Keep concrete skill service for ChallengeResolutionService generics
    let skill_service_impl = SkillServiceImpl::new(skill_repo.clone(), world_repo.clone());

    // Create flag repository for scene condition evaluation
    let flag_repo: Arc<dyn wrldbldr_engine_ports::outbound::FlagRepositoryPort> =
        Arc::new(repository.flags());
    // Create observation repository for KnowsCharacter scene condition
    let observation_repo: Arc<dyn wrldbldr_engine_ports::outbound::ObservationRepositoryPort> =
        Arc::new(repository.observations());
    // Clone for request handler (scene_resolution_service consumes the original)
    let observation_repo_for_handler = observation_repo.clone();

    // Clone repos for port version before they're moved
    let _player_character_repo_for_scene_port = player_character_repo.clone();
    let _scene_repo_for_port = scene_repo_for_resolution.clone();
    let _flag_repo_for_port = flag_repo.clone();
    let _observation_repo_for_port = observation_repo.clone();

    let scene_resolution_service_impl = SceneResolutionServiceImpl::new(
        player_character_repo,
        scene_repo_for_resolution,
        flag_repo,
        observation_repo,
    );
    let _scene_resolution_service: Arc<
        dyn wrldbldr_engine_app::application::services::SceneResolutionService,
    > = Arc::new(scene_resolution_service_impl.clone());
    let scene_resolution_service_port: Arc<dyn SceneResolutionServicePort> =
        Arc::new(scene_resolution_service_impl);

    // Create outcome trigger service for challenge resolution (Phase 22D)
    // Uses ISP: ChallengeCrudPort only
    let outcome_trigger_service = Arc::new(OutcomeTriggerService::new(challenge_crud.clone()));

    // Note: world_connection_manager and world_state are already extracted from infra above

    // ===========================================================================
    // Level 2: Event Infrastructure + Queue Backends (parallel)
    // ===========================================================================
    let (event_infra, queue_backends) = tokio::try_join!(
        super::factories::create_event_infrastructure(&config),
        super::factories::queue_services::create_queue_backends(&config),
    )?;

    // Extract event infrastructure components
    let event_bus = event_infra.event_bus;
    let event_notifier = event_infra.event_notifier_concrete;
    let domain_event_repository = event_infra.domain_event_repository;
    let generation_read_state_repository = event_infra.generation_read_state_repository;
    let generation_event_tx = event_infra.generation_event_tx;
    let generation_event_rx = event_infra.generation_event_rx;
    let challenge_approval_tx = event_infra.challenge_approval_tx;
    let challenge_approval_rx = event_infra.challenge_approval_rx;

    // Extract queue backends
    let player_action_queue = queue_backends.player_action_queue;
    let llm_queue = queue_backends.llm_queue;
    let dm_action_queue = queue_backends.dm_action_queue;
    let asset_generation_queue = queue_backends.asset_generation_queue;
    let approval_queue = queue_backends.approval_queue;
    let challenge_outcome_queue = queue_backends.challenge_outcome_queue;
    let queue_factory = queue_backends.queue_factory;

    // Create story event service with event bus
    // Uses ISP sub-traits: crud, edge, query, dialogue
    let story_event_service_impl = Arc::new(StoryEventServiceImpl::new(
        story_event_crud_for_service.clone(),
        story_event_edge_for_service.clone(),
        story_event_query_for_service.clone(),
        story_event_dialogue_for_service.clone(),
        event_bus.clone(),
        clock.clone(),
    ));
    // Cast to various ISP ports - StoryEventServiceImpl implements all of them
    let story_event_service: Arc<
        dyn wrldbldr_engine_app::application::services::StoryEventService,
    > = story_event_service_impl.clone();
    let story_event_service_port: Arc<dyn StoryEventServicePort> =
        story_event_service_impl.clone();
    let dialogue_context_service: Arc<dyn wrldbldr_engine_ports::outbound::DialogueContextServicePort> =
        story_event_service_impl.clone();

    // Create narrative event service with event bus
    // Uses ISP sub-traits: crud, tie, npc, query
    // Create both trait object and concrete impl (impl needed for NarrativeEventApprovalService generics)
    let narrative_event_service_impl = NarrativeEventServiceImpl::new(
        narrative_event_crud_for_service.clone(),
        narrative_event_tie_for_service.clone(),
        narrative_event_npc_for_service.clone(),
        narrative_event_query_for_service.clone(),
        event_bus.clone(),
    );
    // Clone for port version
    let narrative_event_service_impl_for_port = NarrativeEventServiceImpl::new(
        narrative_event_crud_for_service.clone(),
        narrative_event_tie_for_service.clone(),
        narrative_event_npc_for_service.clone(),
        narrative_event_query_for_service.clone(),
        event_bus.clone(),
    );
    let narrative_event_service: Arc<
        dyn wrldbldr_engine_app::application::services::NarrativeEventService,
    > = Arc::new(narrative_event_service_impl_for_port.clone());
    let narrative_event_service_port: Arc<dyn NarrativeEventServicePort> =
        Arc::new(narrative_event_service_impl_for_port);

    // Initialize queue services
    // Services take Arc<Q>, so we pass Arc<QueueBackendEnum<T>> directly
    let player_action_queue_service = Arc::new(PlayerActionQueueService::new(
        player_action_queue.clone(),
        llm_queue.clone(),
        clock.clone(),
    ));

    let dm_action_queue_service = Arc::new(DmActionQueueService::new(
        dm_action_queue.clone(),
        clock.clone(),
    ));

    // generation_event_tx/rx already extracted from event_infra above
    let generation_event_tx_for_llm = generation_event_tx.clone();

    let llm_client_arc = Arc::new(llm_client.clone());
    // LLMQueueService uses ISP: ChallengeCrudPort + ChallengeSkillPort, NarrativeEventCrudPort
    let llm_queue_service = Arc::new(LLMQueueService::new(
        llm_queue.clone(),
        llm_client_arc,
        approval_queue.clone(),
        challenge_crud.clone(),
        challenge_skill.clone(),
        skill_repo.clone(),
        narrative_event_crud.clone(), // ISP: Uses only NarrativeEventCrudPort
        queue_factory.config().llm_batch_size,
        queue_factory.llm_notifier(),
        generation_event_tx_for_llm,
        prompt_template_service.clone(),
    ));

    let asset_repo_for_queue = asset_repo.clone();
    let file_storage_for_asset_queue: Arc<dyn wrldbldr_engine_ports::outbound::FileStoragePort> =
        Arc::new(TokioFileStorageAdapter::new());
    let asset_generation_queue_service = Arc::new(AssetGenerationQueueService::new(
        asset_generation_queue.clone(),
        Arc::new(comfyui_client.clone()),
        asset_repo_for_queue,
        clock.clone(),
        file_storage_for_asset_queue,
        config.generated_assets_path.clone(),
        queue_factory.config().asset_batch_size,
        queue_factory.asset_generation_notifier(),
    ));

    let dm_approval_queue_service = Arc::new(DMApprovalQueueService::new(
        approval_queue.clone(),
        dialogue_context_service.clone(),
        Arc::new(item_service_impl.clone()),
        clock.clone(),
    ));

    // Create DM action processor service for dm_action_worker
    // This service handles the business logic for DM actions (approval decisions,
    // direct NPC control, event triggering, scene transitions)
    let dm_action_processor: Arc<dyn DmActionProcessorPort> =
        Arc::new(DmActionProcessorService::new(
            dm_approval_queue_service.clone(),
            narrative_event_service.clone(),
            scene_service.clone(),
            interaction_service.clone(),
            clock.clone(),
        ));
    tracing::info!("Initialized DM action processor service");

    // Create file storage adapter for generation service
    let file_storage: Arc<dyn wrldbldr_engine_ports::outbound::FileStoragePort> =
        Arc::new(TokioFileStorageAdapter::new());

    // Create generation service (generation_event_tx already created above)
    let generation_service = Arc::new(GenerationService::new(
        Arc::new(comfyui_client.clone()) as Arc<dyn wrldbldr_engine_ports::outbound::ComfyUIPort>,
        asset_repo.clone(),
        clock.clone(),
        file_storage,
        config.asset_base_path.clone(),
        config.workflow_path.clone(),
        generation_event_tx,
    ));

    // Create challenge outcome approval service (P3.3) - must be created before resolution service
    // Wire LLM port for suggestion generation, settings service for branch count,
    // and persistent queue for challenge outcomes
    //
    // The service uses an event channel instead of WorldConnectionPort for hexagonal compliance.
    // Events are published by ChallengeApprovalEventPublisher (started in server.rs).
    // challenge_approval_tx/rx already extracted from event_infra above
    let llm_for_suggestions = Arc::new(llm_client.clone());
    let challenge_outcome_approval_service = Arc::new(ChallengeOutcomeApprovalService::new(
        challenge_approval_tx,
        outcome_trigger_service.clone(),
        player_character_repo_for_triggers.clone(),
        item_repo.clone(),
        prompt_template_service.clone(),
        challenge_outcome_queue.clone(),
        llm_for_suggestions,
        settings_service.clone(),
        clock.clone(),
    ));

    // Create challenge resolution service with required approval service
    // Uses concrete service impls for generics compatibility
    // Note: No longer takes WorldConnectionPort - returns typed results for use case layer to broadcast
    // Note: rng is already extracted from infra above
    let challenge_resolution_service = Arc::new(ChallengeResolutionService::new(
        Arc::new(challenge_service_impl.clone()),
        Arc::new(skill_service_impl.clone()),
        Arc::new(player_character_service_impl.clone()),
        dm_approval_queue_service.clone(),
        challenge_outcome_approval_service.clone(),
        clock.clone(),
        rng.clone(),
    ));

    // Create narrative event approval service
    // Uses concrete service impls for generics compatibility
    let narrative_event_approval_service = Arc::new(NarrativeEventApprovalService::new(
        Arc::new(narrative_event_service_impl.clone()),
        story_event_service.clone(),
    ));

    // Create trigger evaluation service (Phase 2)
    // Uses ISP: NarrativeEventCrudPort, StoryEventQueryPort, StoryEventEdgePort
    let trigger_evaluation_service = Arc::new(TriggerEvaluationService::new(
        narrative_event_crud_for_triggers,
        player_character_repo_for_triggers,
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

    // Create generation queue projection service
    let generation_queue_projection_service = Arc::new(GenerationQueueProjectionService::new(
        asset_service.clone(),
        domain_event_repository.clone(),
        generation_read_state_repository.clone(),
    ));

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
    // Uses ISP: CharacterCrudPort, CharacterWantPort, CharacterActantialPort
    let actantial_context_service = Arc::new(ActantialContextServiceImpl::new(
        character_crud.clone(),
        character_want.clone(),
        character_actantial.clone(),
        player_character_repo_for_actantial.clone(),
        goal_repo,
        want_repo,
        clock.clone(),
    ));

    // Create prompt context service for building LLM prompts from player actions
    // Uses ISP: CharacterCrudPort, RegionItemPort
    // PromptContextServiceImpl now implements both app-layer trait and port trait directly
    let prompt_context_service_impl = Arc::new(PromptContextServiceImpl::new(
        world_service.clone(),
        world_state.clone() as Arc<dyn wrldbldr_engine_ports::outbound::WorldStatePort>,
        challenge_service.clone(),
        skill_service.clone(),
        narrative_event_service.clone(),
        character_crud.clone(),
        player_character_repo_for_actantial,
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

    // Create suggestion enqueue adapter for AI suggestions
    // Pass world_repo for auto-enrichment of suggestion context
    // Cast LLMQueueService to LlmQueueServicePort for hexagonal architecture compliance
    let llm_queue_service_port: Arc<dyn wrldbldr_engine_ports::outbound::LlmQueueServicePort> =
        llm_queue_service.clone();
    let suggestion_enqueue_adapter: Arc<
        dyn wrldbldr_engine_ports::outbound::SuggestionEnqueuePort,
    > = Arc::new(SuggestionEnqueueAdapter::new(
        llm_queue_service_port,
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
    // Create use cases directly (replaces OldUseCases::new())
    // This moves the use case construction from engine-adapters to engine-runner
    // ===========================================================================

    // Create broadcast adapter for all use cases to share
    let broadcast: Arc<dyn BroadcastPort> =
        Arc::new(WebSocketBroadcastAdapter::new(world_connection_manager.clone()));

    // Create DM notification adapter (clone connection_manager since we'll use it again)
    let dm_notification = Arc::new(DmNotificationAdapter::new(world_connection_manager.clone()));

    // Create staging adapters
    // Note: StagingStateAdapter implements both StagingStatePort and StagingStateExtPort
    // Note: StagingServiceAdapter implements both StagingServicePort and StagingServiceExtPort
    let staging_state_adapter = Arc::new(StagingStateAdapter::new(world_state.clone()));
    let staging_service_adapter = Arc::new(StagingServiceAdapter::new(staging_service.clone()));

    // Create shared scene builder
    // Uses ISP: RegionCrudPort, RegionConnectionPort, RegionExitPort, RegionItemPort
    let scene_builder = Arc::new(SceneBuilder::new(
        region_crud.clone(),
        region_connection.clone(),
        region_exit.clone(),
        region_item.clone(),
        location_crud.clone(),
    ));

    // Create movement use case
    // Uses ISP: RegionCrudPort, RegionConnectionPort
    let movement_use_case = Arc::new(MovementUseCase::new(
        player_character_repo_for_handler.clone(),
        region_crud.clone(),
        region_connection.clone(),
        location_crud.clone(),
        location_map.clone(),
        staging_service_adapter.clone(),
        staging_state_adapter.clone(),
        broadcast.clone(),
        scene_builder.clone(),
        clock.clone(),
    ));

    // Create inventory use case
    // Uses ISP: RegionItemPort
    let inventory_use_case = Arc::new(InventoryUseCase::new(
        player_character_repo_for_handler.clone(),
        region_item.clone(),
        broadcast.clone(),
    ));

    // Create staging approval use case
    // Uses ISP: CharacterCrudPort, RegionCrudPort
    let staging_approval_use_case = Arc::new(StagingApprovalUseCase::new(
        staging_service_adapter,
        staging_state_adapter,
        character_crud_for_use_cases.clone(),
        region_crud.clone(),
        location_crud.clone(),
        broadcast.clone(),
        scene_builder,
        clock.clone(),
    ));

    // Create player action use case
    let player_action_queue_adapter =
        Arc::new(PlayerActionQueueAdapter::new(player_action_queue_service.clone()));
    let player_action_use_case = Arc::new(PlayerActionUseCase::new(
        movement_use_case.clone(),
        player_action_queue_adapter,
        dm_notification,
    ));

    // Create observation use case
    // Uses ISP: CharacterCrudPort for character lookups
    // Uses BroadcastPort for NpcApproach and LocationEvent notifications
    let observation_use_case = Arc::new(ObservationUseCase::new(
        player_character_repo_for_handler.clone(),
        character_crud_for_use_cases,
        observation_repo_for_use_cases,
        broadcast.clone(),
        clock.clone(),
    ));

    // =========================================================================
    // Challenge Use Case
    // =========================================================================
    // Adapter wraps the ChallengeResolutionService to implement ChallengeResolutionPort
    let challenge_resolution_adapter = Arc::new(ChallengeResolutionAdapter::new(
        challenge_resolution_service.clone(),
    ));
    let challenge_outcome_adapter = Arc::new(ChallengeOutcomeApprovalAdapter::new(
        challenge_outcome_approval_service.clone(),
    ));
    let challenge_dm_queue_adapter = Arc::new(ChallengeDmApprovalQueueAdapter::new(
        dm_approval_queue_service.clone(),
    ));

    let challenge_use_case = Arc::new(ChallengeUseCase::new(
        challenge_resolution_adapter,
        challenge_outcome_adapter,
        challenge_dm_queue_adapter,
        broadcast.clone(),
    ));

    // =========================================================================
    // Scene Use Case
    // =========================================================================
    let scene_service_adapter = Arc::new(SceneServiceAdapter::new(scene_service_port.clone()));
    let interaction_service_adapter =
        Arc::new(InteractionServiceAdapter::new(interaction_service_port.clone()));
    let scene_world_state_adapter = Arc::new(SceneWorldStateAdapter::new(world_state.clone()));
    let scene_directorial_adapter = Arc::new(DirectorialContextAdapter::new(
        directorial_context_repo.clone(),
    ));
    let dm_action_queue_placeholder = Arc::new(DmActionQueuePlaceholder::new());

    let scene_use_case = Arc::new(SceneUseCase::new(
        scene_service_adapter,
        interaction_service_adapter,
        scene_world_state_adapter,
        scene_directorial_adapter,
        dm_action_queue_placeholder,
    ));

    // =========================================================================
    // Connection Use Case
    // =========================================================================
    let connection_manager_adapter =
        Arc::new(ConnectionManagerAdapter::new(world_connection_manager.clone()));
    let world_service_adapter = Arc::new(WorldServiceAdapter::new(world_service_port.clone()));
    let pc_service_adapter = Arc::new(PlayerCharacterServiceAdapter::new(player_character_service_port.clone()));
    let connection_directorial_adapter = Arc::new(ConnectionDirectorialContextAdapter::new(
        directorial_context_repo.clone(),
    ));
    let connection_world_state_adapter =
        Arc::new(ConnectionWorldStateAdapter::new(world_state.clone()));

    let connection_use_case = Arc::new(ConnectionUseCase::new(
        connection_manager_adapter,
        world_service_adapter,
        pc_service_adapter,
        connection_directorial_adapter,
        connection_world_state_adapter,
        broadcast.clone(),
    ));

    // =========================================================================
    // Narrative Event Use Case
    // =========================================================================
    let narrative_event_use_case = Arc::new(NarrativeEventUseCase::new(
        narrative_event_approval_service.clone(),
        broadcast.clone(),
    ));

    tracing::info!("Initialized use cases container with all 9 use cases");

    // ===========================================================================
    // Create composition-layer UseCases with port traits (casting from concrete)
    // ===========================================================================
    let composition_use_cases = UseCases::new(
        broadcast.clone(),
        movement_use_case as Arc<dyn MovementUseCasePort>,
        staging_approval_use_case as Arc<dyn StagingUseCasePort>,
        inventory_use_case as Arc<dyn InventoryUseCasePort>,
        player_action_use_case as Arc<dyn PlayerActionUseCasePort>,
        observation_use_case as Arc<dyn ObservationUseCasePort>,
        challenge_use_case as Arc<dyn ChallengeUseCasePort>,
        scene_use_case as Arc<dyn SceneUseCasePort>,
        connection_use_case as Arc<dyn ConnectionUseCasePort>,
        narrative_event_use_case as Arc<dyn NarrativeEventUseCasePort>,
    );

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
    let composition_queues = QueueServices::new(
        player_action_queue_service.clone() as Arc<dyn PlayerActionQueueServicePort>,
        dm_action_queue_service.clone() as Arc<dyn DmActionQueueServicePort>,
        llm_queue_service.clone() as Arc<dyn LlmQueueServicePort>,
        asset_generation_queue_service.clone() as Arc<dyn AssetGenerationQueueServicePort>,
        dm_approval_queue_service.clone() as Arc<dyn DmApprovalQueueServicePort>,
        challenge_outcome_queue.clone()
            as Arc<dyn QueuePort<wrldbldr_domain::value_objects::ChallengeOutcomeData>>,
    );

    // ===========================================================================
    // Create composition-layer AssetServices (port-based)
    // ===========================================================================
    // Create a new WorkflowConfigService for composition layer (uses cloned repo)
    let workflow_config_service_for_composition =
        Arc::new(WorkflowConfigService::new(workflow_repo_for_composition, clock.clone()));
    let composition_assets = AssetServices::new(
        Arc::new(asset_service.clone()) as Arc<dyn AssetServicePort>,
        workflow_config_service_for_composition as Arc<dyn WorkflowServicePort>,
        generation_service.clone() as Arc<dyn GenerationServicePort>,
        generation_queue_projection_service.clone()
            as Arc<dyn GenerationQueueProjectionServicePort>,
    );

    // ===========================================================================
    // Create composition-layer PlayerServices (port-based)
    // ===========================================================================
    // Use port-typed versions
    let composition_player = PlayerServices::new(
        sheet_template_service.clone() as Arc<dyn SheetTemplateServicePort>,
        player_character_service_port.clone(),
        scene_resolution_service_port.clone(),
    );

    // ===========================================================================
    // Create composition-layer EventInfra (port-based)
    // ===========================================================================
    let composition_events = EventInfra::new(
        event_bus.clone() as Arc<dyn EventBusPort>,
        Arc::new(event_notifier.clone()) as Arc<dyn EventNotifierPort>,
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
        staging_service.clone() as Arc<dyn StagingServicePort>,
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
