//! Shared application state
//!
//! This module provides a modular application state structure that composes
//! several sub-structures for better organization and maintainability.

mod asset_services;
mod core_services;
mod event_infra;
mod game_services;
mod player_services;
mod queue_services;

pub use asset_services::AssetServices;
pub use core_services::CoreServices;
pub use event_infra::EventInfrastructure;
pub use game_services::GameServices;
pub use player_services::PlayerServices;
pub use queue_services::QueueServices;

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use wrldbldr_engine_ports::inbound::RequestHandler;
use wrldbldr_engine_ports::outbound::AsyncSessionPort;
use wrldbldr_engine_app::application::handlers::AppRequestHandler;
use wrldbldr_engine_app::application::services::{
    ActantialContextServiceImpl, AssetGenerationQueueService, AssetServiceImpl,
    challenge_resolution_service::ChallengeResolutionService, ChallengeOutcomeApprovalService,
    ChallengeServiceImpl, CharacterServiceImpl, DMActionQueueService, DMApprovalQueueService,
    EventChainServiceImpl, InteractionServiceImpl, ItemServiceImpl, LLMQueueService, LocationServiceImpl,
    MoodServiceImpl, NarrativeEventApprovalService, NarrativeEventServiceImpl, PlayerActionQueueService,
    PlayerCharacterServiceImpl, PromptTemplateService, SceneResolutionServiceImpl, SceneServiceImpl,
    SettingsService, SheetTemplateService, SkillServiceImpl, StoryEventServiceImpl, RelationshipServiceImpl,
    WorkflowConfigService, WorldServiceImpl, GenerationQueueProjectionService, SessionJoinService,
    OutcomeTriggerService, TriggerEvaluationService, EventEffectExecutor,
    staging_service::StagingService,
};
use wrldbldr_engine_app::application::services::generation_service::{GenerationService, GenerationEvent};
use wrldbldr_protocol::AppEvent;
use wrldbldr_engine_ports::outbound::{
    AppEventRepositoryPort, EventBusPort, GenerationReadStatePort,
};
use crate::infrastructure::comfyui::ComfyUIClient;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::event_bus::{InProcessEventNotifier, SqliteEventBus};
use crate::infrastructure::export::Neo4jWorldExporter;
use crate::infrastructure::ollama::OllamaClient;
use crate::infrastructure::persistence::{
    Neo4jNarrativeEventRepository, Neo4jRegionRepository, Neo4jRepository, 
    Neo4jStagingRepository, SqlitePromptTemplateRepository, SqliteSettingsRepository,
};
use crate::infrastructure::queues::QueueFactory;
use crate::infrastructure::repositories::{
    SqliteAppEventRepository, SqliteGenerationReadStateRepository,
};
use crate::infrastructure::session::SessionManager;
use crate::infrastructure::session_adapter::SessionManagerAdapter;
use crate::infrastructure::world_connection_manager::{
    SharedWorldConnectionManager, new_shared_manager,
};

/// Shared application state
///
/// This struct composes several sub-structures that group related services
/// for better organization and maintainability.
pub struct AppState {
    pub config: AppConfig,
    /// Neo4j repository - direct access for specialized operations
    ///
    /// While most data access should go through service layers, some operations
    /// (like region management) may need direct repository access.
    pub repository: Neo4jRepository,
    pub llm_client: OllamaClient,
    pub comfyui_client: ComfyUIClient,
    /// Active WebSocket sessions
    pub sessions: Arc<RwLock<SessionManager>>,
    /// Async session port used by application services (hexagonal boundary over SessionManager)
    pub async_session_port: Arc<dyn AsyncSessionPort>,

    // Grouped services
    pub core: CoreServices,
    pub game: GameServices<OllamaClient>,
    pub queues: QueueServices,
    pub assets: AssetServices,
    pub player: PlayerServices,
    pub events: EventInfrastructure,
    pub settings_service: Arc<SettingsService>,
    /// Prompt template service for configurable LLM prompts
    pub prompt_template_service: Arc<PromptTemplateService>,
    /// Staging service for NPC presence management
    pub staging_service: Arc<StagingService<
        OllamaClient,
        Neo4jRegionRepository,
        Neo4jNarrativeEventRepository,
        Neo4jStagingRepository,
    >>,

    /// World connection manager for WebSocket-first architecture
    ///
    /// Manages world-scoped connections, replacing session-based model.
    /// Handles JoinWorld/LeaveWorld, role enforcement, and connection tracking.
    pub world_connection_manager: SharedWorldConnectionManager,

    /// Request handler for WebSocket-first architecture
    ///
    /// Handles all Request payloads, routing them to appropriate services.
    pub request_handler: Arc<dyn RequestHandler>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<(Self, tokio::sync::mpsc::UnboundedReceiver<GenerationEvent>)> {
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
        let settings_service = Arc::new(SettingsService::new(settings_repository));

        // Initialize prompt template service (uses same pool as settings - they share the DB file)
        let prompt_template_repository = SqlitePromptTemplateRepository::new(settings_pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize prompt template repository: {}", e))?;
        let prompt_template_repository: Arc<dyn wrldbldr_engine_ports::outbound::PromptTemplateRepositoryPort> =
            Arc::new(prompt_template_repository);
        let prompt_template_service = Arc::new(PromptTemplateService::new(prompt_template_repository));
        tracing::info!("Initialized prompt template service");

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
        let narrative_event_repo: Arc<dyn wrldbldr_engine_ports::outbound::NarrativeEventRepositoryPort> =
            Arc::new(repository.narrative_events());
        let event_chain_repo: Arc<dyn wrldbldr_engine_ports::outbound::EventChainRepositoryPort> =
            Arc::new(repository.event_chains());
        let player_character_repo: Arc<dyn wrldbldr_engine_ports::outbound::PlayerCharacterRepositoryPort> =
            Arc::new(repository.player_characters());
        let item_repo: Arc<dyn wrldbldr_engine_ports::outbound::ItemRepositoryPort> =
            Arc::new(repository.items());
        let goal_repo: Arc<dyn wrldbldr_engine_ports::outbound::GoalRepositoryPort> =
            Arc::new(repository.goals());

        // Create world exporter
        let world_exporter: Arc<dyn wrldbldr_engine_ports::outbound::WorldExporterPort> =
            Arc::new(Neo4jWorldExporter::new(repository.clone()));

        // Initialize application services as Arc<dyn Trait> for shared ownership
        // This allows services to be shared between grouped service structs and AppRequestHandler
        
        let world_service: Arc<dyn wrldbldr_engine_app::application::services::WorldService> = 
            Arc::new(WorldServiceImpl::new(world_repo.clone(), world_exporter, settings_service.clone()));
        
        let character_service: Arc<dyn wrldbldr_engine_app::application::services::CharacterService> = 
            Arc::new(CharacterServiceImpl::new(
                world_repo.clone(),
                character_repo.clone(),
                relationship_repo.clone(),
                settings_service.clone(),
            ));
        
        let location_service: Arc<dyn wrldbldr_engine_app::application::services::LocationService> = 
            Arc::new(LocationServiceImpl::new(world_repo.clone(), location_repo.clone()));
        
        let relationship_repo_for_effects = relationship_repo.clone();
        let relationship_service: Arc<dyn wrldbldr_engine_app::application::services::RelationshipService> = 
            Arc::new(RelationshipServiceImpl::new(relationship_repo));
        
        let scene_repo_for_resolution = scene_repo.clone();
        let character_repo_for_triggers = character_repo.clone();
        
        let scene_service: Arc<dyn wrldbldr_engine_app::application::services::SceneService> = 
            Arc::new(SceneServiceImpl::new(scene_repo.clone(), location_repo.clone(), character_repo.clone()));
        
        let skill_service: Arc<dyn wrldbldr_engine_app::application::services::SkillService> = 
            Arc::new(SkillServiceImpl::new(skill_repo.clone(), world_repo.clone()));
        
        let interaction_service: Arc<dyn wrldbldr_engine_app::application::services::InteractionService> = 
            Arc::new(InteractionServiceImpl::new(interaction_repo));
        
        // Temporarily create a simple story event service without event_bus, will update after event_bus is created
        let story_event_repo_for_service = story_event_repo.clone();
        
        let challenge_service: Arc<dyn wrldbldr_engine_app::application::services::ChallengeService> = 
            Arc::new(ChallengeServiceImpl::new(challenge_repo.clone()));
        // Keep concrete version for ChallengeResolutionService generics
        let challenge_service_impl = ChallengeServiceImpl::new(challenge_repo.clone());
        
        // Narrative event service will be created after event_bus
        let narrative_event_repo_for_service = narrative_event_repo.clone();
        // Repos needed for trigger evaluation service (Phase 2)
        let narrative_event_repo_for_triggers = narrative_event_repo.clone();
        let challenge_repo_for_triggers = challenge_repo.clone();
        let story_event_repo_for_triggers = story_event_repo.clone();
        // Repos needed for event effect executor (Phase 2)
        let narrative_event_repo_for_effects = narrative_event_repo.clone();
        let challenge_repo_for_effects = challenge_repo.clone();
        
        let event_chain_service: Arc<dyn wrldbldr_engine_app::application::services::EventChainService> = 
            Arc::new(EventChainServiceImpl::new(event_chain_repo));
        
        let asset_repo_for_service = asset_repo.clone();
        let asset_service = AssetServiceImpl::new(asset_repo_for_service);
        let workflow_config_service = WorkflowConfigService::new(workflow_repo);
        let sheet_template_service = SheetTemplateService::new(sheet_template_repo);
        
        let item_service: Arc<dyn wrldbldr_engine_app::application::services::ItemService> = 
            Arc::new(ItemServiceImpl::new(item_repo.clone(), player_character_repo.clone()));
        // Keep concrete version for DMApprovalQueueService
        let item_service_impl = ItemServiceImpl::new(item_repo.clone(), player_character_repo.clone());
        
        let player_character_repo_for_triggers = player_character_repo.clone();
        let player_character_repo_for_actantial = player_character_repo.clone();
        
        let player_character_service: Arc<dyn wrldbldr_engine_app::application::services::PlayerCharacterService> = 
            Arc::new(PlayerCharacterServiceImpl::new(
                player_character_repo.clone(),
                location_repo.clone(),
                world_repo.clone(),
            ));
        // Keep concrete version for ChallengeResolutionService generics
        let player_character_service_impl = PlayerCharacterServiceImpl::new(
            player_character_repo.clone(),
            location_repo.clone(),
            world_repo.clone(),
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
        
        let scene_resolution_service: Arc<dyn wrldbldr_engine_app::application::services::SceneResolutionService> = 
            Arc::new(SceneResolutionServiceImpl::new(
                player_character_repo,
                scene_repo_for_resolution,
                character_repo.clone(),
                flag_repo,
                observation_repo,
            ));

        // Create outcome trigger service for challenge resolution (Phase 22D)
        let outcome_trigger_service = Arc::new(OutcomeTriggerService::new(
            challenge_repo.clone(),
        ));

        // Initialize queue infrastructure using factory
        let queue_factory = QueueFactory::new(config.queue.clone()).await?;
        tracing::info!("Queue backend: {}", queue_factory.config().backend);

        let player_action_queue = queue_factory.create_player_action_queue().await?;
        let llm_queue = queue_factory.create_llm_queue().await?;
        let dm_action_queue = queue_factory.create_dm_action_queue().await?;
        let asset_generation_queue = queue_factory.create_asset_generation_queue().await?;
        let approval_queue = queue_factory.create_approval_queue().await?;

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

        let app_event_repository_impl = SqliteAppEventRepository::new(event_pool).await
            .map_err(|e| anyhow::anyhow!("Failed to initialize event repository: {}", e))?;
        // Generation read-state repository shares the same SQLite pool as app events
        let generation_read_state_repository =
            SqliteGenerationReadStateRepository::new(app_event_repository_impl.pool().clone());
        generation_read_state_repository.init_schema().await?;
        let generation_read_state_repository: Arc<dyn GenerationReadStatePort> =
            Arc::new(generation_read_state_repository);

        let app_event_repository: Arc<dyn AppEventRepositoryPort> =
            Arc::new(app_event_repository_impl);

        let event_notifier = InProcessEventNotifier::new();
        let event_bus: Arc<dyn EventBusPort<AppEvent>> = Arc::new(SqliteEventBus::new(
            app_event_repository.clone(),
            event_notifier.clone(),
        ));

        // Create story event service with event bus
        let story_event_service: Arc<dyn wrldbldr_engine_app::application::services::StoryEventService> = 
            Arc::new(StoryEventServiceImpl::new(story_event_repo_for_service, event_bus.clone()));
        
        // Create narrative event service with event bus
        // Create both trait object and concrete impl (impl needed for NarrativeEventApprovalService generics)
        let narrative_event_service_impl = NarrativeEventServiceImpl::new(narrative_event_repo_for_service.clone(), event_bus.clone());
        let narrative_event_service: Arc<dyn wrldbldr_engine_app::application::services::NarrativeEventService> = 
            Arc::new(NarrativeEventServiceImpl::new(narrative_event_repo_for_service, event_bus.clone()));

        // Initialize session manager (must be before async_session_port which uses it)
        let sessions = Arc::new(RwLock::new(SessionManager::new(
            config.session.max_conversation_history,
        )));

        // Create async session port adapter for application services
        let async_session_port: Arc<dyn AsyncSessionPort> =
            Arc::new(SessionManagerAdapter::new(sessions.clone()));

        // Initialize queue services
        // Services take Arc<Q>, so we pass Arc<QueueBackendEnum<T>> directly
        let player_action_queue_service = Arc::new(PlayerActionQueueService::new(
            player_action_queue.clone(),
            llm_queue.clone(),
        ));

        let dm_action_queue_service = Arc::new(DMActionQueueService::new(dm_action_queue.clone()));

        // Create event channel for generation service (needed for LLMQueueService suggestions)
        let (generation_event_tx, generation_event_rx) = tokio::sync::mpsc::unbounded_channel();
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
        let asset_generation_queue_service = Arc::new(AssetGenerationQueueService::new(
            asset_generation_queue.clone(),
            Arc::new(comfyui_client.clone()),
            asset_repo_for_queue,
            queue_factory.config().asset_batch_size,
            queue_factory.asset_generation_notifier(),
        ));

        let dm_approval_queue_service = Arc::new(DMApprovalQueueService::new(
            approval_queue.clone(),
            story_event_service.clone(),
            Arc::new(item_service_impl.clone()),
        ));

        // Create generation service (generation_event_tx already created above)
        let generation_service = Arc::new(GenerationService::new(
            Arc::new(comfyui_client.clone()) as Arc<dyn wrldbldr_engine_ports::outbound::ComfyUIPort>,
            asset_repo.clone(),
            std::path::PathBuf::from("./data/assets"),
            std::path::PathBuf::from("./workflows"),
            generation_event_tx,
        ));

        // Create challenge outcome approval service (P3.3) - must be created before resolution service
        // Wire LLM port for suggestion generation and settings service for branch count
        let llm_for_suggestions = Arc::new(llm_client.clone());
        let challenge_outcome_approval_service = Arc::new(
            ChallengeOutcomeApprovalService::new(
                async_session_port.clone(),
                outcome_trigger_service.clone(),
                player_character_repo_for_triggers.clone(),
                item_repo.clone(),
                prompt_template_service.clone(),
            )
            .with_llm_port(llm_for_suggestions)
            .with_settings_service(settings_service.clone()),
        );

        // Create challenge resolution service with approval service wired in
        // Uses concrete service impls for generics compatibility
        let challenge_resolution_service = Arc::new(
            ChallengeResolutionService::new(
                async_session_port.clone(),
                Arc::new(challenge_service_impl.clone()),
                Arc::new(skill_service_impl.clone()),
                Arc::new(player_character_service_impl.clone()),
                event_bus.clone(),
                dm_approval_queue_service.clone(),
                outcome_trigger_service,
            )
            .with_outcome_approval_service(challenge_outcome_approval_service.clone()),
        );

        // Create narrative event approval service
        // Uses concrete service impls for generics compatibility
        let narrative_event_approval_service = Arc::new(NarrativeEventApprovalService::new(
            async_session_port.clone(),
            Arc::new(narrative_event_service_impl.clone()),
            story_event_service.clone(),
        ));

        // Create trigger evaluation service (Phase 2)
        let trigger_evaluation_service = Arc::new(TriggerEvaluationService::new(
            narrative_event_repo_for_triggers,
            challenge_repo_for_triggers,
            character_repo_for_triggers,
            player_character_repo_for_triggers,
            story_event_repo_for_triggers,
        ));

        // Create event effect executor (Phase 2)
        let event_effect_executor = Arc::new(EventEffectExecutor::new(
            async_session_port.clone(),
            challenge_repo_for_effects,
            narrative_event_repo_for_effects,
            relationship_repo_for_effects,
        ));

        // Create staging service (Staging System)
        let staging_repo = Arc::new(repository.stagings());
        let region_repo_for_staging = Arc::new(repository.regions());
        // Create region repo for request handler
        let region_repo_for_handler: Arc<dyn wrldbldr_engine_ports::outbound::RegionRepositoryPort> = 
            Arc::new(repository.regions());
        let narrative_event_repo_for_staging = Arc::new(repository.narrative_events());
        let llm_for_staging = Arc::new(llm_client.clone());
        let staging_service = Arc::new(StagingService::new(
            staging_repo,
            region_repo_for_staging,
            narrative_event_repo_for_staging,
            story_event_service.clone(),
            llm_for_staging,
            prompt_template_service.clone(),
        ));

        // Create session join service
        let session_join_service = Arc::new(SessionJoinService::new(
            async_session_port.clone(),
            world_service.clone(),
        ));

        // Create generation queue projection service
        let generation_queue_projection_service = Arc::new(GenerationQueueProjectionService::new(
            asset_service.clone(),
            app_event_repository.clone(),
            generation_read_state_repository.clone(),
        ));

        // Create mood service (P1.4)
        let mood_service = Arc::new(MoodServiceImpl::new(character_repo.clone()));

        // Create actantial context service (P1.5)
        let actantial_context_service = Arc::new(ActantialContextServiceImpl::new(
            character_repo.clone(),
            player_character_repo_for_actantial,
            goal_repo,
            item_repo.clone(),
        ));

        // Build grouped services
        // Services are already Arc<dyn Trait>, so we clone them for shared ownership
        let core = CoreServices::new(
            world_service.clone(),
            character_service.clone(),
            location_service.clone(),
            scene_service.clone(),
            skill_service.clone(),
            interaction_service.clone(),
            relationship_service.clone(),
            item_service.clone(),
        );

        let game = GameServices::new(
            story_event_service,
            challenge_service.clone(),
            challenge_resolution_service,
            challenge_outcome_approval_service,
            narrative_event_service.clone(),
            narrative_event_approval_service,
            event_chain_service.clone(),
            trigger_evaluation_service,
            event_effect_executor,
            mood_service.clone() as Arc<dyn wrldbldr_engine_app::application::services::MoodService>,
            actantial_context_service.clone() as Arc<dyn wrldbldr_engine_app::application::services::ActantialContextService>,
        );

        let queues = QueueServices::new(
            player_action_queue_service,
            dm_action_queue_service,
            llm_queue_service,
            asset_generation_queue_service,
            dm_approval_queue_service,
        );

        let assets = AssetServices::new(
            asset_service,
            workflow_config_service,
            generation_service,
            generation_queue_projection_service,
        );

        let player = PlayerServices::new(
            sheet_template_service,
            player_character_service.clone(),
            scene_resolution_service.clone(),
            session_join_service,
        );

        let events = EventInfrastructure::new(
            event_bus,
            event_notifier,
            app_event_repository,
            generation_read_state_repository,
        );

        // Create world connection manager for WebSocket-first architecture
        let world_connection_manager = new_shared_manager();
        tracing::info!("Initialized world connection manager for WebSocket-first architecture");

        // Create request handler for WebSocket-first architecture
        // Services are already Arc<dyn Trait>, so just clone them
        let request_handler: Arc<dyn RequestHandler> = Arc::new(AppRequestHandler::new(
            core.world_service.clone(),
            core.character_service.clone(),
            core.location_service.clone(),
            core.skill_service.clone(),
            core.scene_service.clone(),
            core.interaction_service.clone(),
            game.challenge_service.clone(),
            game.narrative_event_service.clone(),
            game.event_chain_service.clone(),
            player.player_character_service.clone(),
            core.relationship_service.clone(),
            game.actantial_context_service.clone(),
            game.mood_service.clone(),
            game.story_event_service.clone(),
            observation_repo_for_handler,
            region_repo_for_handler,
        ));
        tracing::info!("Initialized request handler for WebSocket-first architecture");

        Ok((Self {
            config: config.clone(),
            repository,
            llm_client,
            comfyui_client,
            sessions: Arc::clone(&sessions),
            async_session_port,
            core,
            game,
            queues,
            assets,
            player,
            events,
            settings_service,
            prompt_template_service,
            staging_service,
            world_connection_manager,
            request_handler,
        }, generation_event_rx))
    }
}
