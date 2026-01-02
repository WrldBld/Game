//! Asset Services Factory
//!
//! Creates asset management and generation services for the composition root.
//! This is Level 4 in the composition hierarchy - parallel with game_services.
//!
//! # Services Created
//!
//! - **AssetServiceImpl**: CRUD operations for stored assets (portraits, scene images)
//! - **WorkflowConfigService**: ComfyUI workflow configuration management
//! - **GenerationService**: Orchestrates asset generation pipeline
//! - **GenerationQueueProjectionService**: Read-side view of generation queue state
//!
//! # Dependencies
//!
//! This factory requires:
//! - Level 0: clock
//! - ComfyUI port (worker-only adapter passed from composition root)
//! - Level 1: Repository ports (asset_repo, workflow_repo)
//! - Level 2a: `EventInfrastructure` (generation_event_tx, domain_event_repository, generation_read_state_repository)
//! - Config: asset_base_path, workflow_path
//!
//! # Architecture
//!
//! Asset services handle the lifecycle of visual assets:
//! 1. Workflow configuration defines how assets are generated
//! 2. Generation service queues and orchestrates ComfyUI requests
//! 3. Asset service stores and retrieves generated assets
//! 4. Projection service provides queue state for UI

use std::sync::Arc;

use tokio::sync::mpsc;

use wrldbldr_engine_adapters::infrastructure::{
    in_memory::InMemoryActiveGenerationBatches, TokioFileStorageAdapter,
};
use wrldbldr_engine_app::application::services::generation_service::{
    GenerationEvent, GenerationService,
};
use wrldbldr_engine_app::application::services::internal::{
    AssetUseCasePort, GenerationQueueProjectionUseCasePort, GenerationUseCasePort,
    WorkflowUseCasePort,
};
use wrldbldr_engine_app::application::services::{
    AssetServiceImpl, GenerationQueueProjectionService, WorkflowConfigService,
};
use wrldbldr_engine_ports::inbound::SettingsUseCasePort;
use wrldbldr_engine_ports::outbound::{
    ActiveGenerationBatchesPort, AssetRepositoryPort, ClockPort, ComfyUIPort,
    DomainEventRepositoryPort, FileStoragePort, GenerationReadStatePort, WorkflowRepositoryPort,
};

/// Dependencies required for creating asset services.
///
/// This struct encapsulates all external dependencies needed to construct
/// asset management and generation services.
///
/// # Dependency Categories
///
/// ## Infrastructure (from Level 0)
/// - `clock`: Time operations for timestamps
/// - `comfyui_client`: ComfyUI client for image generation
///
/// ## Repositories (from Level 1)
/// - `asset_repo`: Asset CRUD operations
/// - `workflow_repo`: Workflow configuration storage
///
/// ## Event Infrastructure (from Level 2a)
/// - `generation_event_tx`: Channel for generation events
/// - `domain_event_repository`: Domain event storage
/// - `generation_read_state_repository`: Read state tracking
///
/// ## Configuration
/// - `asset_base_path`: Directory for storing generated assets
/// - `workflow_path`: Directory containing workflow definitions
pub struct AssetServiceDependencies {
    // Infrastructure (Level 0)
    /// Clock for timestamps
    pub clock: Arc<dyn ClockPort>,

    /// ComfyUI port for image generation
    pub comfyui: Arc<dyn ComfyUIPort>,

    /// Settings service for resolving per-world behavior
    pub settings_service: Arc<dyn SettingsUseCasePort>,

    // Repositories (Level 1)
    /// Asset repository for CRUD operations
    pub asset_repo: Arc<dyn AssetRepositoryPort>,

    /// Workflow repository for configuration storage
    pub workflow_repo: Arc<dyn WorkflowRepositoryPort>,

    // Event infrastructure (Level 2a)
    /// Sender for generation events
    pub generation_event_tx: mpsc::Sender<GenerationEvent>,

    /// Domain event repository for event sourcing
    pub domain_event_repository: Arc<dyn DomainEventRepositoryPort>,

    /// Generation read state repository for tracking read markers
    pub generation_read_state_repository: Arc<dyn GenerationReadStatePort>,

    // Configuration
    /// Base path for storing generated assets
    pub asset_base_path: String,

    /// Path to workflow definition files
    pub workflow_path: String,
}

/// Container for all asset service port trait objects.
///
/// This struct groups services related to asset management and generation,
/// using port traits for clean hexagonal architecture boundaries.
///
/// # Service Categories
///
/// ## Asset Management
/// - `asset_service`: CRUD operations for stored assets
///
/// ## Workflow Configuration
/// - `workflow_config_service`: ComfyUI workflow configuration
///
/// ## Generation Pipeline
/// - `generation_service`: Orchestrates asset generation
/// - `generation_queue_projection_service`: Queue state projection
///
#[derive(Clone)]
pub struct AssetUseCasePorts {
    /// Asset service for CRUD operations (port trait)
    pub asset_service: Arc<dyn AssetUseCasePort>,

    /// Workflow configuration service (port trait)
    pub workflow_config_service: Arc<dyn WorkflowUseCasePort>,

    /// Generation service for asset generation (port trait)
    pub generation_service: Arc<dyn GenerationUseCasePort>,

    /// Generation queue projection service (port trait)
    pub generation_queue_projection_service: Arc<dyn GenerationQueueProjectionUseCasePort>,
}

/// Creates all asset service port trait objects from their dependencies.
///
/// This function:
/// 1. Creates file storage adapter for generation service
/// 2. Instantiates all asset services with their dependencies
/// 3. Coerces concrete implementations to port trait objects
/// 4. Returns `AssetUseCasePorts` with all services
///
/// # Arguments
///
/// * `deps` - The dependencies required for asset service construction
///
/// # Returns
///
/// `AssetUseCasePorts` containing all port trait objects and concrete types needed
/// for composition.
///
/// # Example
///
/// ```ignore
/// let deps = AssetServiceDependencies {
///     clock: infra.clock.clone(),
///     comfyui: comfyui_port.clone(),
///     asset_repo: repos.asset_repo.clone(),
///     workflow_repo: repos.workflow_repo.clone(),
///     generation_event_tx: event_infra.generation_event_tx.clone(),
///     domain_event_repository: event_infra.domain_event_repository.clone(),
///     generation_read_state_repository: event_infra.generation_read_state_repository.clone(),
///     asset_base_path: config.asset_base_path.clone(),
///     workflow_path: config.workflow_path.clone(),
/// };
///
/// let asset_services = create_asset_services(deps);
///
/// // Use ports for asset operations
/// let assets = asset_services.asset_service.list_assets(entity_type, entity_id).await?;
/// ```
pub fn create_asset_services(deps: AssetServiceDependencies) -> AssetUseCasePorts {
    // =========================================================================
    // Asset Service
    // =========================================================================
    let asset_service_concrete = AssetServiceImpl::new(deps.asset_repo.clone(), deps.clock.clone());
    let asset_service: Arc<dyn AssetUseCasePort> = Arc::new(asset_service_concrete.clone());
    tracing::debug!("Created asset service");

    // =========================================================================
    // Workflow Config Service
    // =========================================================================
    let workflow_config_service_concrete =
        WorkflowConfigService::new(deps.workflow_repo.clone(), deps.clock.clone());
    let workflow_config_service: Arc<dyn WorkflowUseCasePort> =
        Arc::new(workflow_config_service_concrete);
    tracing::debug!("Created workflow config service");

    // =========================================================================
    // File Storage Adapter
    // =========================================================================
    let file_storage: Arc<dyn FileStoragePort> = Arc::new(TokioFileStorageAdapter::new());

    // =========================================================================
    // Generation Service
    // =========================================================================
    let active_batches: Arc<dyn ActiveGenerationBatchesPort> =
        Arc::new(InMemoryActiveGenerationBatches::new());
    let generation_service_concrete = GenerationService::new(
        deps.comfyui,
        deps.asset_repo.clone(),
        deps.clock.clone(),
        file_storage,
        deps.asset_base_path,
        deps.workflow_path,
        deps.generation_event_tx,
        active_batches,
        deps.settings_service,
    );
    let generation_service: Arc<dyn GenerationUseCasePort> = Arc::new(generation_service_concrete);
    tracing::debug!("Created generation service");

    // =========================================================================
    // Generation Queue Projection Service
    // =========================================================================
    let generation_queue_projection_service_concrete =
        Arc::new(GenerationQueueProjectionService::new(
            asset_service_concrete.clone(),
            deps.domain_event_repository,
            deps.generation_read_state_repository,
        ));
    let generation_queue_projection_service: Arc<dyn GenerationQueueProjectionUseCasePort> =
        generation_queue_projection_service_concrete;
    tracing::debug!("Created generation queue projection service");

    tracing::info!("Asset services factory completed");

    AssetUseCasePorts {
        asset_service,
        workflow_config_service,
        generation_service,
        generation_queue_projection_service,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that AssetUseCasePorts has all expected fields.
    ///
    /// This is a compile-time test - if the struct fields don't match,
    /// the code won't compile.
    #[test]
    fn test_asset_service_ports_structure() {
        fn _verify_ports(ports: &AssetUseCasePorts) {
            // Port traits
            let _: &Arc<dyn AssetUseCasePort> = &ports.asset_service;
            let _: &Arc<dyn WorkflowUseCasePort> = &ports.workflow_config_service;
            let _: &Arc<dyn GenerationUseCasePort> = &ports.generation_service;
            let _: &Arc<dyn GenerationQueueProjectionUseCasePort> =
                &ports.generation_queue_projection_service;
        }

        // The existence of this function proves the types are correct at compile time
        let _ = _verify_ports;
    }

    /// Test that AssetServiceDependencies has all expected fields.
    ///
    /// This verifies the dependency struct structure at compile time.
    #[test]
    fn test_asset_service_dependencies_structure() {
        fn _verify_deps(deps: &AssetServiceDependencies) {
            // Infrastructure
            let _: &Arc<dyn ClockPort> = &deps.clock;
            let _: &Arc<dyn ComfyUIPort> = &deps.comfyui;

            // Repositories
            let _: &Arc<dyn AssetRepositoryPort> = &deps.asset_repo;
            let _: &Arc<dyn WorkflowRepositoryPort> = &deps.workflow_repo;

            // Event infrastructure
            let _: &mpsc::Sender<GenerationEvent> = &deps.generation_event_tx;
            let _: &Arc<dyn DomainEventRepositoryPort> = &deps.domain_event_repository;
            let _: &Arc<dyn GenerationReadStatePort> = &deps.generation_read_state_repository;

            // Configuration
            let _: &String = &deps.asset_base_path;
            let _: &String = &deps.workflow_path;
        }

        // The existence of this function proves the types are correct at compile time
        let _ = _verify_deps;
    }

    /// Test that AssetUseCasePorts implements Clone.
    #[test]
    fn test_asset_service_ports_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<AssetUseCasePorts>();
    }
}
