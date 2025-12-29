//! Generation service port - Interface for asset generation operations
//!
//! This port abstracts asset generation business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

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

/// Port for generation service operations
///
/// This trait defines the application use cases for asset generation,
/// including queueing generation requests, tracking batches, and
/// selecting from completed batches.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait GenerationServicePort: Send + Sync {
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
}
