//! Infrastructure Factory
//!
//! Creates foundational infrastructure components with zero or minimal dependencies.
//! This is Level 0 in the composition hierarchy - nothing depends on prior factories.
//!
//! # Components Created
//!
//! - Clock, Environment, RNG adapters (zero dependencies)
//! - Neo4j repository connection
//! - Ollama LLM client
//! - ComfyUI image generation client
//! - SQLite pool for settings/templates
//! - Settings, PromptTemplate, DirectorialContext services
//! - World connection manager and state manager

use std::sync::Arc;

use anyhow::Result;
use sqlx::SqlitePool;

use wrldbldr_engine_adapters::infrastructure::{
    clock::SystemClock,
    comfyui::ComfyUIClient,
    config::AppConfig,
    environment_adapter::SystemEnvironmentAdapter,
    export::Neo4jWorldExporter,
    ollama::OllamaClient,
    persistence::{
        Neo4jRepository, SqliteDirectorialContextRepository, SqlitePromptTemplateRepository,
        SqliteSettingsRepository,
    },
    random_adapter::ThreadRngAdapter,
    settings_loader::load_settings_from_env,
    world_connection_manager::{new_shared_manager, SharedWorldConnectionManager},
    WorldStateManager,
};
use wrldbldr_engine_app::application::services::{PromptTemplateService, SettingsService};
use wrldbldr_engine_ports::outbound::{
    ClockPort, DirectorialContextRepositoryPort, EnvironmentPort, PromptTemplateRepositoryPort,
    PromptTemplateServicePort, RandomPort, SettingsRepositoryPort, SettingsServicePort,
    WorldExporterPort,
};

/// Infrastructure context containing all foundational dependencies.
///
/// This struct groups components created at Level 0 of the composition hierarchy:
/// - No external service dependencies (created first)
/// - Used by all subsequent factory levels
#[derive(Clone)]
pub struct InfrastructureContext {
    // =========================================================================
    // Core Adapters (zero dependencies)
    // =========================================================================
    /// System clock for timestamps
    pub clock: Arc<dyn ClockPort>,

    /// Environment variable access
    pub environment: Arc<dyn EnvironmentPort>,

    /// Random number generator for dice rolls
    pub rng: Arc<dyn RandomPort>,

    // =========================================================================
    // Database & External Clients
    // =========================================================================
    /// Neo4j graph database repository
    pub neo4j: Neo4jRepository,

    /// World exporter for snapshots
    pub world_exporter: Arc<dyn WorldExporterPort>,

    /// Ollama LLM client (concrete for workers)
    pub llm_client: OllamaClient,

    /// ComfyUI client (concrete for workers)
    pub comfyui_client: ComfyUIClient,

    /// SQLite pool for settings/templates (shared)
    pub settings_pool: SqlitePool,

    // =========================================================================
    // Settings & Configuration Services
    // =========================================================================
    /// Settings repository
    pub settings_repository: Arc<dyn SettingsRepositoryPort>,

    /// Settings service
    pub settings_service: Arc<dyn SettingsServicePort>,

    /// Prompt template repository
    pub prompt_template_repository: Arc<dyn PromptTemplateRepositoryPort>,

    /// Prompt template service (port version for general use)
    pub prompt_template_service: Arc<dyn PromptTemplateServicePort>,

    /// Prompt template service (concrete version for LLMQueueService)
    pub prompt_template_service_concrete: Arc<PromptTemplateService>,

    /// Directorial context repository
    pub directorial_context_repo: Arc<dyn DirectorialContextRepositoryPort>,

    // =========================================================================
    // Connection & State Infrastructure
    // =========================================================================
    /// World connection manager for WebSocket connections
    pub world_connection_manager: SharedWorldConnectionManager,

    /// World state manager for in-memory game state
    pub world_state: Arc<WorldStateManager>,
}

/// Creates the infrastructure context.
///
/// This is the first factory called in the composition chain.
/// It initializes all foundational infrastructure with no dependencies on other factories.
///
/// # Arguments
/// * `config` - Application configuration
///
/// # Returns
/// * `InfrastructureContext` with all Level 0 dependencies
///
/// # Errors
/// Returns error if Neo4j connection or SQLite initialization fails.
pub async fn create_infrastructure(config: &AppConfig) -> Result<InfrastructureContext> {
    // =========================================================================
    // Zero-dependency adapters
    // =========================================================================
    let clock: Arc<dyn ClockPort> = Arc::new(SystemClock::new());
    let environment: Arc<dyn EnvironmentPort> = Arc::new(SystemEnvironmentAdapter::new());
    let rng: Arc<dyn RandomPort> = Arc::new(ThreadRngAdapter::new());

    // =========================================================================
    // Neo4j connection
    // =========================================================================
    tracing::info!(
        "Connecting to Neo4j at {} (database: {})",
        config.neo4j_uri,
        config.neo4j_database
    );
    let neo4j = Neo4jRepository::new(
        &config.neo4j_uri,
        &config.neo4j_user,
        &config.neo4j_password,
        &config.neo4j_database,
    )
    .await?;
    tracing::info!("Neo4j connection established");

    let world_exporter: Arc<dyn WorldExporterPort> =
        Arc::new(Neo4jWorldExporter::new(neo4j.clone()));

    // =========================================================================
    // External clients
    // =========================================================================
    let llm_client = OllamaClient::new(&config.ollama_base_url, &config.ollama_model);
    let comfyui_client = ComfyUIClient::new(&config.comfyui_base_url);
    tracing::info!("Initialized LLM and ComfyUI clients");

    // =========================================================================
    // SQLite pool for settings
    // =========================================================================
    let settings_db_path = config.queue.sqlite_path.replace(".db", "_settings.db");
    if let Some(parent) = std::path::Path::new(&settings_db_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("Failed to create settings database directory: {}", e))?;
    }
    let settings_pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", settings_db_path))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to settings database: {}", e))?;
    tracing::info!("Connected to settings database: {}", settings_db_path);

    // =========================================================================
    // Settings service
    // =========================================================================
    let settings_repository_impl = SqliteSettingsRepository::new(settings_pool.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize settings repository: {}", e))?;
    let settings_repository: Arc<dyn SettingsRepositoryPort> = Arc::new(settings_repository_impl);

    let settings_loader: wrldbldr_engine_app::application::services::SettingsLoaderFn =
        Arc::new(load_settings_from_env);
    let settings_service: Arc<dyn SettingsServicePort> =
        Arc::new(SettingsService::new(settings_repository.clone(), settings_loader));

    // =========================================================================
    // Prompt template service
    // =========================================================================
    let prompt_template_repository_impl = SqlitePromptTemplateRepository::new(settings_pool.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize prompt template repository: {}", e))?;
    let prompt_template_repository: Arc<dyn PromptTemplateRepositoryPort> =
        Arc::new(prompt_template_repository_impl);
    let prompt_template_service_concrete = Arc::new(PromptTemplateService::new(
        prompt_template_repository.clone(),
        environment.clone(),
    ));
    let prompt_template_service: Arc<dyn PromptTemplateServicePort> =
        prompt_template_service_concrete.clone();
    tracing::info!("Initialized prompt template service");

    // =========================================================================
    // Directorial context repository
    // =========================================================================
    let directorial_context_repo_impl = SqliteDirectorialContextRepository::new(settings_pool.clone())
        .await
        .map_err(|e| {
            anyhow::anyhow!("Failed to initialize directorial context repository: {}", e)
        })?;
    let directorial_context_repo: Arc<dyn DirectorialContextRepositoryPort> =
        Arc::new(directorial_context_repo_impl);
    tracing::info!("Initialized directorial context repository");

    // =========================================================================
    // Connection & state infrastructure
    // =========================================================================
    let world_connection_manager = new_shared_manager();
    let world_state = Arc::new(WorldStateManager::new());
    tracing::info!("Initialized world connection and state managers");

    Ok(InfrastructureContext {
        clock,
        environment,
        rng,
        neo4j,
        world_exporter,
        llm_client,
        comfyui_client,
        settings_pool,
        settings_repository,
        settings_service,
        prompt_template_repository,
        prompt_template_service,
        prompt_template_service_concrete,
        directorial_context_repo,
        world_connection_manager,
        world_state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infrastructure_context_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<InfrastructureContext>();
    }

    #[test]
    fn test_create_infrastructure_signature() {
        // Verify the function signature exists and has the expected shape
        // We can't directly test async function signatures with lifetime bounds,
        // so we just verify the types are constructible
        fn _verify_result_type(_: InfrastructureContext) {}
    }
}
