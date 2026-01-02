//! Infrastructure Factory
//!
//! Creates foundational infrastructure components with zero or minimal dependencies.
//! This is Level 0 in the composition hierarchy - nothing depends on prior factories.
//!
//! # Components Created
//!
//! - Clock, Environment, RNG adapters (zero dependencies)
//! - Neo4j repository connection
//! - SQLite pool for settings/templates
//! - Settings, PromptTemplate, DirectorialContext services
//! - World state manager

use std::sync::Arc;

use anyhow::Result;
use sqlx::SqlitePool;

use wrldbldr_engine_adapters::infrastructure::{
    clock::SystemClock,
    config::AppConfig,
    environment_adapter::SystemEnvironmentAdapter,
    export::Neo4jWorldExporter,
    in_memory::{InMemoryPromptTemplateCache, InMemorySettingsCache},
    persistence::{
        Neo4jRepository, SqliteDirectorialContextRepository, SqlitePromptTemplateRepository,
        SqliteSettingsRepository,
    },
    random_adapter::ThreadRngAdapter,
    settings_loader::load_settings_from_env,
    WorldStateManager,
};
use wrldbldr_engine_app::application::services::{PromptTemplateService, SettingsService};
use wrldbldr_engine_ports::inbound::PromptTemplateUseCasePort;
use wrldbldr_engine_ports::inbound::SettingsUseCasePort;
use wrldbldr_engine_ports::outbound::{
    ClockPort, DirectorialContextRepositoryPort, EnvironmentPort, PromptTemplateCachePort,
    PromptTemplateRepositoryPort, RandomPort, SettingsCachePort, SettingsRepositoryPort,
    StagingStateExtPort, WorldExporterPort, WorldStatePort, WorldStateUpdatePort,
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

    /// Random number generator for dice rolls
    pub rng: Arc<dyn RandomPort>,

    // =========================================================================
    // Database & External Clients
    // =========================================================================
    /// Neo4j graph database repository
    neo4j: Neo4jRepository,

    /// World exporter for snapshots
    pub world_exporter: Arc<dyn WorldExporterPort>,

    // =========================================================================
    // Settings & Configuration Services
    // =========================================================================
    /// Settings service (port version)
    pub settings_service: Arc<dyn SettingsUseCasePort>,

    /// Prompt template service (port version)
    pub prompt_template_service: Arc<dyn PromptTemplateUseCasePort>,

    /// Directorial context repository
    pub directorial_context_repo: Arc<dyn DirectorialContextRepositoryPort>,

    // =========================================================================
    // Connection & State Infrastructure
    // =========================================================================
    /// World state port for in-memory game state (time/conversation/scene/etc)
    pub world_state: Arc<dyn WorldStatePort>,

    /// World state update port (used by scene/connection use cases)
    pub world_state_update: Arc<dyn WorldStateUpdatePort>,

    /// Staging state port (used by movement + staging use cases)
    pub staging_state: Arc<dyn StagingStateExtPort>,
}

impl InfrastructureContext {
    pub fn neo4j(&self) -> &Neo4jRepository {
        &self.neo4j
    }
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
        clock.clone(),
    )
    .await?;
    tracing::info!("Neo4j connection established");

    let world_exporter: Arc<dyn WorldExporterPort> =
        Arc::new(Neo4jWorldExporter::new(neo4j.clone()));

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

    let settings_cache: Arc<dyn SettingsCachePort> = Arc::new(InMemorySettingsCache::new());

    let settings_loader: wrldbldr_engine_app::application::services::SettingsLoaderFn =
        Arc::new(load_settings_from_env);
    let settings_service: Arc<dyn SettingsUseCasePort> = Arc::new(SettingsService::new(
        settings_repository.clone(),
        settings_loader,
        settings_cache,
    ));

    // =========================================================================
    // Prompt template service
    // =========================================================================
    let prompt_template_repository_impl =
        SqlitePromptTemplateRepository::new(settings_pool.clone())
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to initialize prompt template repository: {}", e)
            })?;
    let prompt_template_repository: Arc<dyn PromptTemplateRepositoryPort> =
        Arc::new(prompt_template_repository_impl);
    let prompt_template_cache: Arc<dyn PromptTemplateCachePort> =
        Arc::new(InMemoryPromptTemplateCache::new());
    let prompt_template_service: Arc<dyn PromptTemplateUseCasePort> =
        Arc::new(PromptTemplateService::new(
            prompt_template_repository.clone(),
            environment.clone(),
            prompt_template_cache,
        ));
    tracing::info!("Initialized prompt template service");

    // =========================================================================
    // Directorial context repository
    // =========================================================================
    let directorial_context_repo_impl =
        SqliteDirectorialContextRepository::new(settings_pool.clone())
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
    let world_state_manager = Arc::new(WorldStateManager::new(clock.clone()));
    let world_state: Arc<dyn WorldStatePort> = world_state_manager.clone();
    let world_state_update: Arc<dyn WorldStateUpdatePort> = world_state_manager.clone();
    let staging_state: Arc<dyn StagingStateExtPort> = world_state_manager;
    tracing::info!("Initialized world connection and state managers");

    Ok(InfrastructureContext {
        clock,
        rng,
        neo4j,
        world_exporter,
        settings_service,
        prompt_template_service,
        directorial_context_repo,
        world_state,
        world_state_update,
        staging_state,
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
