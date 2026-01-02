//! Asset Services Container - Port-based abstraction for asset generation services
//!
//! This module provides `AssetServices`, a grouped structure for asset management
//! and generation services using **port traits** from `wrldbldr-engine-ports`.
//!
//! # Architecture
//!
//! This struct groups all services related to asset management and generation,
//! including workflow configuration and queue projections. All fields use port
//! traits for clean hexagonal architecture boundaries.
//!
//! # Services Included
//!
//! - **Asset Service**: CRUD operations for managed assets (images, etc.)
//! - **Workflow Config Service**: ComfyUI workflow configuration management
//! - **Generation Service**: Orchestrates asset generation requests
//! - **Generation Queue Projection**: Read-side projection for generation queue state
//!
//! # Usage
//!
//! ```ignore
//! use wrldbldr_engine_composition::AssetServices;
//!
//! let asset_services = AssetServices::new(
//!     asset_service,
//!     workflow_config_service,
//!     generation_service,
//!     generation_queue_projection_service,
//! );
//!
//! // Access via port traits
//! let asset = asset_services.asset_service.get_asset(asset_id).await?;
//! ```

use std::sync::Arc;

// Internal service traits (NOT ports - internal app-layer contracts)
use wrldbldr_engine_app::application::services::internal::{
    AssetUseCasePort, GenerationQueueProjectionUseCasePort, GenerationUseCasePort,
    WorkflowUseCasePort,
};

/// Container for asset management and generation services.
///
/// This struct groups all services related to asset lifecycle management,
/// from workflow configuration through generation to storage and retrieval.
///
/// All fields are `Arc<dyn ...Port>` for:
/// - Shared ownership across handlers and workers
/// - Dynamic dispatch enabling mock injection for tests
/// - No generic type parameters for simpler composition
///
/// # Service Categories
///
/// ## Asset Management
/// - `asset_service`: CRUD operations for stored assets (portraits, scene images, etc.)
///
/// ## Workflow Configuration
/// - `workflow_config_service`: ComfyUI workflow definitions and parameters
///
/// ## Generation Pipeline
/// - `generation_service`: Orchestrates the asset generation process
/// - `generation_queue_projection_service`: Read-side view of generation queue state
#[derive(Clone)]
pub struct AssetServices {
    /// Service for asset CRUD operations.
    ///
    /// Manages stored assets including character portraits, location images,
    /// and other visual content. Handles metadata and file references.
    pub asset_service: Arc<dyn AssetUseCasePort>,

    /// Service for workflow configuration management.
    ///
    /// Provides access to ComfyUI workflow definitions that control how
    /// assets are generated. Includes workflow templates and parameter schemas.
    pub workflow_config_service: Arc<dyn WorkflowUseCasePort>,

    /// Service for orchestrating asset generation.
    ///
    /// Coordinates the generation pipeline from request through ComfyUI
    /// execution to final asset storage. Handles generation batches and
    /// individual asset requests.
    pub generation_service: Arc<dyn GenerationUseCasePort>,

    /// Read-side projection service for generation queue state.
    ///
    /// Provides current state views of the generation queue including
    /// pending, processing, and completed items. Used for UI status
    /// displays and monitoring.
    pub generation_queue_projection_service: Arc<dyn GenerationQueueProjectionUseCasePort>,
}

impl AssetServices {
    /// Creates a new `AssetServices` instance with all asset-related services.
    ///
    /// # Arguments
    ///
    /// All arguments are `Arc<dyn ...Port>` to allow any implementation:
    ///
    /// * `asset_service` - For asset CRUD operations
    /// * `workflow_config_service` - For workflow configuration access
    /// * `generation_service` - For orchestrating asset generation
    /// * `generation_queue_projection_service` - For generation queue state queries
    ///
    /// # Example
    ///
    /// ```ignore
    /// let asset_services = AssetServices::new(
    ///     Arc::new(asset_service_impl) as Arc<dyn AssetUseCasePort>,
    ///     Arc::new(workflow_service_impl) as Arc<dyn WorkflowUseCasePort>,
    ///     Arc::new(generation_service_impl) as Arc<dyn GenerationUseCasePort>,
    ///     Arc::new(projection_service_impl) as Arc<dyn GenerationQueueProjectionUseCasePort>,
    /// );
    /// ```
    pub fn new(
        asset_service: Arc<dyn AssetUseCasePort>,
        workflow_config_service: Arc<dyn WorkflowUseCasePort>,
        generation_service: Arc<dyn GenerationUseCasePort>,
        generation_queue_projection_service: Arc<dyn GenerationQueueProjectionUseCasePort>,
    ) -> Self {
        Self {
            asset_service,
            workflow_config_service,
            generation_service,
            generation_queue_projection_service,
        }
    }
}

impl std::fmt::Debug for AssetServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetServices")
            .field("asset_service", &"Arc<dyn AssetUseCasePort>")
            .field("workflow_config_service", &"Arc<dyn WorkflowUseCasePort>")
            .field("generation_service", &"Arc<dyn GenerationUseCasePort>")
            .field(
                "generation_queue_projection_service",
                &"Arc<dyn GenerationQueueProjectionUseCasePort>",
            )
            .finish()
    }
}
