//! Application State Construction
//!
//! This module provides the construction logic for AppState.
//! The AppState struct is defined in engine-composition, but the construction
//! (dependency injection and wiring) is done here in the composition root.

use std::sync::Arc;

use anyhow::Result;

use wrldbldr_engine_adapters::infrastructure::clock::SystemClock;
use wrldbldr_engine_adapters::infrastructure::comfyui::ComfyUIClient;
use wrldbldr_engine_adapters::infrastructure::config::AppConfig;
use wrldbldr_engine_adapters::infrastructure::event_bus::{InProcessEventNotifier, SqliteEventBus};
use wrldbldr_engine_adapters::infrastructure::export::Neo4jWorldExporter;
use wrldbldr_engine_adapters::infrastructure::ollama::OllamaClient;
use wrldbldr_engine_adapters::infrastructure::persistence::{
    Neo4jRepository, SqliteDirectorialContextRepository, SqlitePromptTemplateRepository,
    SqliteSettingsRepository,
};
use wrldbldr_engine_adapters::infrastructure::queues::QueueFactory;
use wrldbldr_engine_adapters::infrastructure::repositories::{
    SqliteDomainEventRepository, SqliteGenerationReadStateRepository,
};
use wrldbldr_engine_adapters::infrastructure::settings_loader::load_settings_from_env;
// Import port adapters for use case construction (replaces OldUseCases::new())
use wrldbldr_engine_adapters::infrastructure::ports::{
    ChallengeDmApprovalQueueAdapter, ChallengeOutcomeApprovalAdapter, ChallengeResolutionAdapter,
    ConnectionDirectorialContextAdapter, ConnectionManagerAdapter, ConnectionWorldStateAdapter,
    DirectorialContextAdapter, DmActionQueuePlaceholder, DmNotificationAdapter,
    InteractionServiceAdapter, PlayerActionQueueAdapter, PlayerCharacterServiceAdapter,
    SceneServiceAdapter, SceneWorldStateAdapter, StagingServiceAdapter, StagingStateAdapter,
    WorldMessageAdapter, WorldServiceAdapter,
};
use wrldbldr_engine_adapters::infrastructure::websocket::WebSocketBroadcastAdapter;
use wrldbldr_engine_adapters::infrastructure::suggestion_enqueue_adapter::SuggestionEnqueueAdapter;
use wrldbldr_engine_adapters::infrastructure::world_connection_manager::new_shared_manager;
use wrldbldr_engine_adapters::infrastructure::SystemEnvironmentAdapter;
use wrldbldr_engine_adapters::infrastructure::TokioFileStorageAdapter;
use wrldbldr_engine_adapters::infrastructure::WorldStateManager;

use wrldbldr_engine_app::application::handlers::AppRequestHandler;
use wrldbldr_engine_app::application::services::generation_service::{
    GenerationEvent, GenerationService,
};
use wrldbldr_engine_app::application::services::{
    challenge_resolution_service::ChallengeResolutionService, staging_service::StagingService,
    ActantialContextServiceImpl, AssetGenerationQueueService, AssetServiceImpl,
    ChallengeApprovalEvent, ChallengeOutcomeApprovalService, ChallengeServiceImpl,
    CharacterServiceImpl, DMApprovalQueueService, DispositionServiceImpl, DmActionQueueService,
    EventChainServiceImpl, EventEffectExecutor, GenerationQueueProjectionService,
    InteractionServiceImpl, ItemServiceImpl, LLMQueueService, LocationServiceImpl,
    NarrativeEventApprovalService, NarrativeEventServiceImpl, OutcomeTriggerService,
    PlayerActionQueueService, PlayerCharacterServiceImpl, PromptContextServiceImpl,
    PromptTemplateService, RegionServiceImpl, RelationshipServiceImpl, SceneResolutionServiceImpl,
    SceneServiceImpl, SettingsService, SheetTemplateService, SkillServiceImpl,
    StoryEventServiceImpl, TriggerEvaluationService, WorkflowConfigService, WorldServiceImpl,
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
    CharacterRepositoryPort,
    CharacterServicePort,
    ClockPort,
    ComfyUIPort,
    DispositionServicePort,
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
    LocationRepositoryPort,
    LocationServicePort,
    NarrativeEventApprovalServicePort,
    NarrativeEventServicePort,
    ObservationRepositoryPort,
    PlayerCharacterRepositoryPort,
    // Queue service ports
    PlayerActionQueueServicePort,
    PlayerCharacterServicePort,
    PromptContextServicePort,
    PromptTemplateServicePort,
    QueuePort,
    RegionRepositoryPort,
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
    WorldConnectionManagerPort,
    WorldServicePort,
    WorldStatePort,
};

// Re-export AdapterState for server.rs
pub use wrldbldr_engine_adapters::infrastructure::AdapterState;

use wrldbldr_domain::value_objects::{
    ApprovalRequestData, AssetGenerationData, ChallengeOutcomeData, DmActionData,
    GamePromptRequest, LlmRequestData, PlayerActionData,
};
use wrldbldr_engine_adapters::infrastructure::comfyui::ComfyUIClient as ComfyUIClientType;
use wrldbldr_engine_adapters::infrastructure::ollama::OllamaClient as OllamaClientType;
use wrldbldr_engine_adapters::infrastructure::queues::{InProcessNotifier, QueueBackendEnum};

/// Adapter that implements PromptContextServicePort by delegating to PromptContextService.
///
/// This adapter bridges the app-layer PromptContextService trait to the port-layer
/// PromptContextServicePort trait. The port trait has a different method signature
/// (individual fields vs struct), so this adapter does the conversion.
///
/// Note: This is a workaround for the incomplete trait migration. In the future,
/// PromptContextServiceImpl should directly implement PromptContextServicePort.
struct PromptContextServicePortAdapter {
    inner: Arc<dyn wrldbldr_engine_app::application::services::PromptContextService>,
}

#[async_trait::async_trait]
impl PromptContextServicePort for PromptContextServicePortAdapter {
    async fn build_prompt_from_action(
        &self,
        world_id: wrldbldr_domain::WorldId,
        pc_id: wrldbldr_domain::PlayerCharacterId,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
        _region_id: Option<wrldbldr_domain::RegionId>,
    ) -> Result<GamePromptRequest, wrldbldr_engine_ports::outbound::PromptContextError> {
        use wrldbldr_domain::value_objects::PlayerActionData;
        use wrldbldr_engine_ports::outbound::PromptContextError;

        // Build the action struct from individual fields
        let action = PlayerActionData {
            world_id,
            player_id: "adapter".to_string(), // Not used in prompt building
            pc_id: Some(pc_id),
            action_type,
            target,
            dialogue,
            timestamp: chrono::Utc::now(),
        };

        // Delegate to the inner service
        self.inner
            .build_prompt_from_action(world_id, &action)
            .await
            .map_err(|e| PromptContextError::Internal(e.to_string()))
    }

    async fn find_responding_character(
        &self,
        _world_id: wrldbldr_domain::WorldId,
        _scene_id: wrldbldr_domain::SceneId,
        _target: Option<String>,
    ) -> Result<Option<wrldbldr_domain::CharacterId>, wrldbldr_engine_ports::outbound::PromptContextError> {
        // Not used by any current code paths through AppState
        Ok(None)
    }
}

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

    /// Challenge outcome queue (concrete backend type)
    pub challenge_outcome_queue: Arc<QueueBackendEnum<ChallengeOutcomeData>>,

    /// Prompt context service (app-layer trait, not port trait)
    /// Workers need the app-layer trait which has `build_prompt_from_action(&action)`
    pub prompt_context_service:
        Arc<dyn wrldbldr_engine_app::application::services::PromptContextService>,
}

/// Buffer size for internal event channels
/// Provides backpressure when consumers are slow
const EVENT_CHANNEL_BUFFER: usize = 256;

/// Creates a new AdapterState with all services initialized.
///
/// Returns:
/// - `AdapterState`: The main application state with port-typed services
/// - `WorkerServices`: Concrete queue services for background workers
/// - `generation_event_rx`: GenerationEvent receiver for GenerationEventPublisher
/// - `challenge_approval_rx`: ChallengeApprovalEvent receiver for ChallengeApprovalEventPublisher
pub async fn new_app_state(
    config: AppConfig,
) -> Result<(
    AdapterState,
    WorkerServices,
    tokio::sync::mpsc::Receiver<GenerationEvent>,
    tokio::sync::mpsc::Receiver<ChallengeApprovalEvent>,
)> {
    // Create system clock for all services that need time operations
    let clock: Arc<dyn ClockPort> = Arc::new(SystemClock::new());

    // Create environment adapter for services that need environment variable access
    let environment_adapter: Arc<dyn wrldbldr_engine_ports::outbound::EnvironmentPort> =
        Arc::new(SystemEnvironmentAdapter::new());

    // Initialize Neo4j repository
    let repository = Neo4jRepository::new(
        &config.neo4j_uri,
        &config.neo4j_user,
        &config.neo4j_password,
        &config.neo4j_database,
    )
    .await?;

    // Initialize Ollama client
    let llm_client = OllamaClient::new(&config.ollama_base_url, &config.ollama_model);

    // Initialize ComfyUI client
    let comfyui_client = ComfyUIClient::new(&config.comfyui_base_url);

    // Initialize settings service early (needed by other services for validation)
    let settings_db_path = config.queue.sqlite_path.replace(".db", "_settings.db");
    if let Some(parent) = std::path::Path::new(&settings_db_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("Failed to create settings database directory: {}", e))?;
    }
    let settings_pool = sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", settings_db_path))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to settings database: {}", e))?;
    tracing::info!("Connected to settings database: {}", settings_db_path);

    let settings_repository = SqliteSettingsRepository::new(settings_pool.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize settings repository: {}", e))?;
    let settings_repository: Arc<dyn wrldbldr_engine_ports::outbound::SettingsRepositoryPort> =
        Arc::new(settings_repository);
    // Inject the settings loader function (from adapters layer) into the service
    let settings_loader: wrldbldr_engine_app::application::services::SettingsLoaderFn =
        Arc::new(load_settings_from_env);
    let settings_service = Arc::new(SettingsService::new(settings_repository, settings_loader));

    // Initialize prompt template service (uses same pool as settings - they share the DB file)
    let prompt_template_repository = SqlitePromptTemplateRepository::new(settings_pool.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize prompt template repository: {}", e))?;
    let prompt_template_repository: Arc<
        dyn wrldbldr_engine_ports::outbound::PromptTemplateRepositoryPort,
    > = Arc::new(prompt_template_repository);
    let prompt_template_service = Arc::new(PromptTemplateService::new(
        prompt_template_repository,
        environment_adapter.clone(),
    ));
    tracing::info!("Initialized prompt template service");

    // Initialize directorial context repository (shares same SQLite pool)
    let directorial_context_repo = SqliteDirectorialContextRepository::new(settings_pool)
        .await
        .map_err(|e| {
            anyhow::anyhow!("Failed to initialize directorial context repository: {}", e)
        })?;
    let directorial_context_repo: Arc<
        dyn wrldbldr_engine_ports::outbound::DirectorialContextRepositoryPort,
    > = Arc::new(directorial_context_repo);
    tracing::info!("Initialized directorial context repository");

    // Create individual repository ports as Arc'd trait objects
    let world_repo: Arc<dyn wrldbldr_engine_ports::outbound::WorldRepositoryPort> =
        Arc::new(repository.worlds());
    let character_repo: Arc<dyn wrldbldr_engine_ports::outbound::CharacterRepositoryPort> =
        Arc::new(repository.characters());
    let location_repo: Arc<dyn wrldbldr_engine_ports::outbound::LocationRepositoryPort> =
        Arc::new(repository.locations());
    let scene_repo: Arc<dyn wrldbldr_engine_ports::outbound::SceneRepositoryPort> =
        Arc::new(repository.scenes());
    let relationship_repo: Arc<dyn wrldbldr_engine_ports::outbound::RelationshipRepositoryPort> =
        Arc::new(repository.relationships());
    let skill_repo: Arc<dyn wrldbldr_engine_ports::outbound::SkillRepositoryPort> =
        Arc::new(repository.skills());
    let interaction_repo: Arc<dyn wrldbldr_engine_ports::outbound::InteractionRepositoryPort> =
        Arc::new(repository.interactions());
    let story_event_repo: Arc<dyn wrldbldr_engine_ports::outbound::StoryEventRepositoryPort> =
        Arc::new(repository.story_events());
    let challenge_repo: Arc<dyn wrldbldr_engine_ports::outbound::ChallengeRepositoryPort> =
        Arc::new(repository.challenges());
    let asset_repo: Arc<dyn wrldbldr_engine_ports::outbound::AssetRepositoryPort> =
        Arc::new(repository.assets());
    let workflow_repo: Arc<dyn wrldbldr_engine_ports::outbound::WorkflowRepositoryPort> =
        Arc::new(repository.workflows());
    let sheet_template_repo: Arc<dyn wrldbldr_engine_ports::outbound::SheetTemplateRepositoryPort> =
        Arc::new(repository.sheet_templates());
    let narrative_event_repo: Arc<
        dyn wrldbldr_engine_ports::outbound::NarrativeEventRepositoryPort,
    > = Arc::new(repository.narrative_events());
    let event_chain_repo: Arc<dyn wrldbldr_engine_ports::outbound::EventChainRepositoryPort> =
        Arc::new(repository.event_chains());
    let player_character_repo: Arc<
        dyn wrldbldr_engine_ports::outbound::PlayerCharacterRepositoryPort,
    > = Arc::new(repository.player_characters());
    let item_repo: Arc<dyn wrldbldr_engine_ports::outbound::ItemRepositoryPort> =
        Arc::new(repository.items());
    let goal_repo: Arc<dyn wrldbldr_engine_ports::outbound::GoalRepositoryPort> =
        Arc::new(repository.goals());
    let want_repo: Arc<dyn wrldbldr_engine_ports::outbound::WantRepositoryPort> =
        Arc::new(repository.wants());
    let region_repo: Arc<dyn wrldbldr_engine_ports::outbound::RegionRepositoryPort> =
        Arc::new(repository.regions());

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
        character_repo.clone(),
        relationship_repo.clone(),
        settings_service.clone(),
        clock.clone(),
    );
    let character_service: Arc<dyn wrldbldr_engine_app::application::services::CharacterService> =
        Arc::new(character_service_impl.clone());
    let character_service_port: Arc<dyn CharacterServicePort> = Arc::new(character_service_impl);

    let location_service_impl = LocationServiceImpl::new(world_repo.clone(), location_repo.clone());
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
        location_repo.clone(),
        character_repo.clone(),
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

    // Temporarily create a simple story event service without event_bus, will update after event_bus is created
    let story_event_repo_for_service = story_event_repo.clone();

    let challenge_service_for_port = ChallengeServiceImpl::new(challenge_repo.clone());
    let challenge_service: Arc<dyn wrldbldr_engine_app::application::services::ChallengeService> =
        Arc::new(challenge_service_for_port.clone());
    let challenge_service_port: Arc<dyn ChallengeServicePort> =
        Arc::new(challenge_service_for_port);
    // Keep concrete version for ChallengeResolutionService generics
    let challenge_service_impl = ChallengeServiceImpl::new(challenge_repo.clone());

    // Narrative event service will be created after event_bus
    let narrative_event_repo_for_service = narrative_event_repo.clone();
    // Repos needed for trigger evaluation service (Phase 2)
    let narrative_event_repo_for_triggers = narrative_event_repo.clone();
    let story_event_repo_for_triggers = story_event_repo.clone();
    // Repos needed for event effect executor (Phase 2)
    let narrative_event_repo_for_effects = narrative_event_repo.clone();
    let challenge_repo_for_effects = challenge_repo.clone();

    // Clone event_chain_repo for port version before it's moved
    let event_chain_repo_for_port = event_chain_repo.clone();
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
    let workflow_config_service = WorkflowConfigService::new(workflow_repo);
    let sheet_template_service = Arc::new(SheetTemplateService::new(sheet_template_repo));

    let item_service_for_port = ItemServiceImpl::new(
        item_repo.clone(),
        player_character_repo.clone(),
        region_repo.clone(),
    );
    let item_service: Arc<dyn wrldbldr_engine_app::application::services::ItemService> =
        Arc::new(item_service_for_port.clone());
    let item_service_port: Arc<dyn ItemServicePort> = Arc::new(item_service_for_port);
    // Keep concrete version for DMApprovalQueueService
    let item_service_impl = ItemServiceImpl::new(
        item_repo.clone(),
        player_character_repo.clone(),
        region_repo.clone(),
    );

    let player_character_repo_for_triggers = player_character_repo.clone();
    let player_character_repo_for_actantial = player_character_repo.clone();
    let player_character_repo_for_handler = player_character_repo.clone();

    let player_character_service_for_port = PlayerCharacterServiceImpl::new(
        player_character_repo.clone(),
        location_repo.clone(),
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
        location_repo.clone(),
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
    let player_character_repo_for_scene_port = player_character_repo.clone();
    let scene_repo_for_port = scene_repo_for_resolution.clone();
    let flag_repo_for_port = flag_repo.clone();
    let observation_repo_for_port = observation_repo.clone();

    let scene_resolution_service_impl = SceneResolutionServiceImpl::new(
        player_character_repo,
        scene_repo_for_resolution,
        flag_repo,
        observation_repo,
    );
    let scene_resolution_service: Arc<
        dyn wrldbldr_engine_app::application::services::SceneResolutionService,
    > = Arc::new(scene_resolution_service_impl.clone());
    let scene_resolution_service_port: Arc<dyn SceneResolutionServicePort> =
        Arc::new(scene_resolution_service_impl);

    // Create outcome trigger service for challenge resolution (Phase 22D)
    let outcome_trigger_service = Arc::new(OutcomeTriggerService::new(challenge_repo.clone()));

    // Create world connection manager for WebSocket-first architecture
    let world_connection_manager = new_shared_manager();
    tracing::info!("Initialized world connection manager for WebSocket-first architecture");

    // Create world state manager for per-world state (game time, conversation, approvals)
    let world_state = Arc::new(WorldStateManager::new());
    tracing::info!("Initialized world state manager for per-world state");

    // Initialize queue infrastructure using factory
    let queue_factory = QueueFactory::new(config.queue.clone()).await?;
    tracing::info!("Queue backend: {}", queue_factory.config().backend);

    let player_action_queue = queue_factory.create_player_action_queue().await?;
    let llm_queue = queue_factory.create_llm_queue().await?;
    let dm_action_queue = queue_factory.create_dm_action_queue().await?;
    let asset_generation_queue = queue_factory.create_asset_generation_queue().await?;
    let approval_queue = queue_factory.create_approval_queue().await?;
    let challenge_outcome_queue = queue_factory.create_challenge_outcome_queue().await?;

    // Initialize event bus infrastructure
    // For now, use a separate SQLite database for events
    // In production, this could share the queue pool or use Redis
    let event_db_path = config.queue.sqlite_path.replace(".db", "_events.db");
    if let Some(parent) = std::path::Path::new(&event_db_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("Failed to create event database directory: {}", e))?;
    }
    let event_pool = sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", event_db_path))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to event database: {}", e))?;
    tracing::info!("Connected to event database: {}", event_db_path);

    // Note: settings_service was initialized earlier (needed by services for validation)

    // Domain event repository - the only event repository needed
    // Handles conversion to/from wire format (AppEvent) internally
    let domain_event_repository_impl = SqliteDomainEventRepository::new(event_pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize domain event repository: {}", e))?;

    // Generation read-state repository shares the same SQLite pool as domain events
    let generation_read_state_repository =
        SqliteGenerationReadStateRepository::new(domain_event_repository_impl.pool().clone());
    generation_read_state_repository.init_schema().await?;
    let generation_read_state_repository: Arc<dyn GenerationReadStatePort> =
        Arc::new(generation_read_state_repository);

    let domain_event_repository: Arc<dyn DomainEventRepositoryPort> =
        Arc::new(domain_event_repository_impl);

    let event_notifier = InProcessEventNotifier::new();
    let event_bus: Arc<dyn EventBusPort> = Arc::new(SqliteEventBus::new(
        domain_event_repository.clone(),
        event_notifier.clone(),
    ));

    // Create story event service with event bus
    // Clone repo before it's moved so we can create port version
    let story_event_repo_for_port = story_event_repo_for_service.clone();
    let story_event_service_impl_for_port = StoryEventServiceImpl::new(
        story_event_repo_for_service,
        event_bus.clone(),
        clock.clone(),
    );
    let story_event_service: Arc<
        dyn wrldbldr_engine_app::application::services::StoryEventService,
    > = Arc::new(story_event_service_impl_for_port.clone());
    let story_event_service_port: Arc<dyn StoryEventServicePort> =
        Arc::new(story_event_service_impl_for_port);

    // Create narrative event service with event bus
    // Create both trait object and concrete impl (impl needed for NarrativeEventApprovalService generics)
    let narrative_event_service_impl =
        NarrativeEventServiceImpl::new(narrative_event_repo_for_service.clone(), event_bus.clone());
    // Clone for port version
    let narrative_event_service_impl_for_port =
        NarrativeEventServiceImpl::new(narrative_event_repo_for_service.clone(), event_bus.clone());
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

    // Create event channel for generation service (needed for LLMQueueService suggestions)
    // Uses bounded channel with backpressure when consumers are slow
    let (generation_event_tx, generation_event_rx) =
        tokio::sync::mpsc::channel(EVENT_CHANNEL_BUFFER);
    let generation_event_tx_for_llm = generation_event_tx.clone();

    let llm_client_arc = Arc::new(llm_client.clone());
    let llm_queue_service = Arc::new(LLMQueueService::new(
        llm_queue.clone(),
        llm_client_arc,
        approval_queue.clone(),
        challenge_repo.clone(),
        skill_repo.clone(),
        narrative_event_repo.clone(),
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
        "./data/generated_assets".to_string(),
        queue_factory.config().asset_batch_size,
        queue_factory.asset_generation_notifier(),
    ));

    let dm_approval_queue_service = Arc::new(DMApprovalQueueService::new(
        approval_queue.clone(),
        story_event_service.clone(),
        Arc::new(item_service_impl.clone()),
        clock.clone(),
    ));

    // Create file storage adapter for generation service
    let file_storage: Arc<dyn wrldbldr_engine_ports::outbound::FileStoragePort> =
        Arc::new(TokioFileStorageAdapter::new());

    // Create generation service (generation_event_tx already created above)
    let generation_service = Arc::new(GenerationService::new(
        Arc::new(comfyui_client.clone()) as Arc<dyn wrldbldr_engine_ports::outbound::ComfyUIPort>,
        asset_repo.clone(),
        clock.clone(),
        file_storage,
        "./data/assets".to_string(),
        "./workflows".to_string(),
        generation_event_tx,
    ));

    // Create challenge outcome approval service (P3.3) - must be created before resolution service
    // Wire LLM port for suggestion generation, settings service for branch count,
    // and persistent queue for challenge outcomes
    //
    // The service uses an event channel instead of WorldConnectionPort for hexagonal compliance.
    // Events are published by ChallengeApprovalEventPublisher (started in server.rs).
    let llm_for_suggestions = Arc::new(llm_client.clone());
    // Create event channel for challenge outcome approval (P3.3)
    // Uses bounded channel with backpressure when consumers are slow
    let (challenge_approval_tx, challenge_approval_rx) =
        tokio::sync::mpsc::channel(EVENT_CHANNEL_BUFFER);
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
    let challenge_resolution_service = Arc::new(ChallengeResolutionService::new(
        Arc::new(challenge_service_impl.clone()),
        Arc::new(skill_service_impl.clone()),
        Arc::new(player_character_service_impl.clone()),
        dm_approval_queue_service.clone(),
        challenge_outcome_approval_service.clone(),
        clock.clone(),
    ));

    // Create narrative event approval service
    // Uses concrete service impls for generics compatibility
    let narrative_event_approval_service = Arc::new(NarrativeEventApprovalService::new(
        Arc::new(narrative_event_service_impl.clone()),
        story_event_service.clone(),
    ));

    // Create trigger evaluation service (Phase 2)
    let trigger_evaluation_service = Arc::new(TriggerEvaluationService::new(
        narrative_event_repo_for_triggers,
        player_character_repo_for_triggers,
        story_event_repo_for_triggers,
    ));

    // Create event effect executor (Phase 2)
    let event_effect_executor = Arc::new(EventEffectExecutor::new(
        challenge_repo_for_effects,
        narrative_event_repo_for_effects,
        relationship_repo_for_effects,
    ));

    // Create staging service (Staging System)
    // Note: StagingService is generic over concrete types, so we need concrete Arc<...>
    let staging_repo = Arc::new(repository.stagings());
    let region_repo_for_staging = Arc::new(repository.regions());
    let narrative_event_repo_for_staging = Arc::new(repository.narrative_events());
    let llm_for_staging = Arc::new(llm_client.clone());
    let staging_service = Arc::new(StagingService::new(
        staging_repo,
        region_repo_for_staging,
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
    let disposition_service = Arc::new(DispositionServiceImpl::new(
        character_repo.clone(),
        clock.clone(),
    ));

    // Create region service
    let region_service: Arc<dyn wrldbldr_engine_app::application::services::RegionService> =
        Arc::new(RegionServiceImpl::new(
            region_repo.clone(),
            location_repo.clone(),
        ));

    // Create actantial context service (P1.5)
    let actantial_context_service = Arc::new(ActantialContextServiceImpl::new(
        character_repo.clone(),
        player_character_repo_for_actantial.clone(),
        goal_repo,
        want_repo,
        clock.clone(),
    ));

    // Create prompt context service for building LLM prompts from player actions
    let prompt_context_service: Arc<
        dyn wrldbldr_engine_app::application::services::PromptContextService,
    > = Arc::new(PromptContextServiceImpl::new(
        world_service.clone(),
        world_state.clone() as Arc<dyn wrldbldr_engine_ports::outbound::WorldStatePort>,
        challenge_service.clone(),
        skill_service.clone(),
        narrative_event_service.clone(),
        character_repo.clone(),
        player_character_repo_for_actantial,
        region_repo.clone(),
        disposition_service.clone(),
        actantial_context_service.clone()
            as Arc<dyn wrldbldr_engine_app::application::services::ActantialContextService>,
    ));

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
    // Clone repos for the handler and use cases
    let character_repo_for_handler = character_repo.clone();
    let character_repo_for_use_cases = character_repo.clone();
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
        character_repo_for_handler,
        observation_repo_for_handler,
        region_repo.clone(),
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
    let scene_builder = Arc::new(SceneBuilder::new(
        region_repo.clone(),
        location_repo.clone(),
    ));

    // Create movement use case
    let movement_use_case = Arc::new(MovementUseCase::new(
        player_character_repo_for_handler.clone(),
        region_repo.clone(),
        location_repo.clone(),
        staging_service_adapter.clone(),
        staging_state_adapter.clone(),
        broadcast.clone(),
        scene_builder.clone(),
    ));

    // Create inventory use case
    let inventory_use_case = Arc::new(InventoryUseCase::new(
        player_character_repo_for_handler.clone(),
        region_repo.clone(),
        broadcast.clone(),
    ));

    // Create staging approval use case
    let staging_approval_use_case = Arc::new(StagingApprovalUseCase::new(
        staging_service_adapter,
        staging_state_adapter,
        character_repo_for_use_cases.clone(),
        region_repo.clone(),
        location_repo.clone(),
        broadcast.clone(),
        scene_builder,
    ));

    // Create player action use case
    let player_action_queue_adapter =
        Arc::new(PlayerActionQueueAdapter::new(player_action_queue_service.clone()));
    let player_action_use_case = Arc::new(PlayerActionUseCase::new(
        movement_use_case.clone(),
        player_action_queue_adapter,
        dm_notification,
        broadcast.clone(),
    ));

    // Create observation adapters
    // Note: observation_repo now directly implements the same ObservationRepositoryPort
    // used by ObservationUseCase (consolidated from engine-ports)
    let world_message_adapter = Arc::new(WorldMessageAdapter::new(world_connection_manager.clone()));

    // Create observation use case
    let observation_use_case = Arc::new(ObservationUseCase::new(
        player_character_repo_for_handler.clone(),
        character_repo_for_use_cases,
        observation_repo_for_use_cases,
        world_message_adapter,
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
        broadcast.clone(),
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
        Arc::new(WorkflowConfigService::new(workflow_repo_for_composition));
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
    // Note: adapter's AppConfig doesn't have server_host, use default "0.0.0.0"
    let composition_config = CompositionAppConfig::new(
        "0.0.0.0".to_string(), // server_host not in adapter config, use default
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
        region_repo.clone() as Arc<dyn RegionRepositoryPort>,
        composition_core,
        composition_game,
        composition_queues,
        composition_assets,
        composition_player,
        composition_events,
        settings_service.clone() as Arc<dyn SettingsServicePort>,
        prompt_template_service.clone() as Arc<dyn PromptTemplateServicePort>,
        staging_service.clone() as Arc<dyn StagingServicePort>,
        world_connection_manager.clone() as Arc<dyn WorldConnectionManagerPort>,
        world_state.clone() as Arc<dyn WorldStatePort>,
        request_handler.clone(),
        directorial_context_repo.clone(),
        composition_use_cases,
        Arc::new(PromptContextServicePortAdapter {
            inner: prompt_context_service.clone(),
        }) as Arc<dyn PromptContextServicePort>,
    );

    // ===========================================================================
    // Create AdapterState (wraps composition AppState with infrastructure types)
    // ===========================================================================
    // Pass the full adapter config so server.rs can access queue/CORS settings
    let adapter_state = AdapterState::new(
        composition_app_state,
        config,
        world_connection_manager.clone(),
        comfyui_client.clone(),
        region_repo.clone(),
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
        challenge_outcome_queue: challenge_outcome_queue.clone(),
        prompt_context_service: prompt_context_service.clone(),
    };

    Ok((
        adapter_state,
        worker_services,
        generation_event_rx,
        challenge_approval_rx,
    ))
}

/// Alias for `new_app_state` that returns `AdapterState`.
///
/// This is the preferred entry point for creating application state, as it makes
/// clear that we're returning an `AdapterState` (infrastructure-aware) rather
/// than a composition-layer `AppState`.
pub async fn new_adapter_state(
    config: AppConfig,
) -> Result<(
    AdapterState,
    WorkerServices,
    tokio::sync::mpsc::Receiver<GenerationEvent>,
    tokio::sync::mpsc::Receiver<ChallengeApprovalEvent>,
)> {
    new_app_state(config).await
}
