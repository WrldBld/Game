//! Generation use case port - Inbound interface for asset generation operations
//!
//! This port is called by HTTP handlers to trigger asset generation.
//! The implementation lives in engine-app.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::entities::{AssetType, EntityType, GalleryAsset, GenerationBatch};
use wrldbldr_domain::{AssetId, BatchId, WorldId};

/// Request to generate assets
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRequest {
    pub world_id: WorldId,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub asset_type: AssetType,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub count: u8,
    pub style_reference_id: Option<AssetId>,
}

/// Port for generation use case operations
///
/// Called by: HTTP handlers in asset_routes.rs (retry_batch)
/// Implemented by: GenerationService in engine-app
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait GenerationUseCasePort: Send + Sync {
    /// Queue a new asset generation request
    ///
    /// Returns the batch tracking the generation progress.
    async fn generate_asset(&self, request: GenerationRequest) -> Result<GenerationBatch>;

    /// Get a generation batch by ID
    async fn get_batch(&self, id: BatchId) -> Result<Option<GenerationBatch>>;

    /// Select an asset from a completed batch
    ///
    /// Marks the selected asset as active and the batch as completed.
    async fn select_from_batch(
        &self,
        batch_id: BatchId,
        asset_index: usize,
    ) -> Result<GalleryAsset>;

    /// Start processing a generation batch
    ///
    /// This initiates the actual asset generation workflow for a batch,
    /// sending requests to ComfyUI and tracking progress. Used when
    /// retrying failed batches or manually triggering batch processing.
    async fn start_batch_processing(&self, batch: GenerationBatch) -> Result<()>;
}
