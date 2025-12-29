//! Asset service port - Interface for asset gallery operations
//!
//! This port abstracts asset gallery business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::entities::{AssetType, BatchStatus, EntityType, GalleryAsset, GenerationBatch};
use wrldbldr_domain::{AssetId, BatchId, WorldId};

/// Request to create a new asset
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssetRequest {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub asset_type: AssetType,
    pub file_path: String,
    pub label: Option<String>,
}

/// Port for asset service operations
///
/// This trait defines the application use cases for asset gallery management,
/// including listing, creating, and retrieving assets, as well as generation
/// batch management.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait AssetServicePort: Send + Sync {
    /// Get a single asset by ID
    async fn get_asset(&self, asset_id: AssetId) -> Result<Option<GalleryAsset>>;

    /// List all assets for an entity
    async fn list_by_entity(
        &self,
        entity_type: EntityType,
        entity_id: &str,
    ) -> Result<Vec<GalleryAsset>>;

    /// Create a new asset
    async fn create_asset(&self, request: CreateAssetRequest) -> Result<GalleryAsset>;

    /// Update an asset's label
    async fn update_asset_label(&self, asset_id: AssetId, label: Option<String>) -> Result<()>;

    /// Delete an asset
    async fn delete_asset(&self, asset_id: AssetId) -> Result<()>;

    /// Activate an asset (set as current for its entity/type slot)
    async fn activate_asset(&self, asset_id: AssetId) -> Result<()>;

    /// Create a generation batch
    async fn create_batch(&self, batch: GenerationBatch) -> Result<GenerationBatch>;

    /// Get a batch by ID
    async fn get_batch(&self, batch_id: BatchId) -> Result<Option<GenerationBatch>>;

    /// List all active batches (queued or generating) for a specific world
    async fn list_active_batches_by_world(&self, world_id: WorldId)
        -> Result<Vec<GenerationBatch>>;

    /// List batches ready for selection
    async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>>;

    /// Update batch status
    async fn update_batch_status(&self, batch_id: BatchId, status: BatchStatus) -> Result<()>;

    /// Update batch assets
    async fn update_batch_assets(&self, batch_id: BatchId, assets: Vec<AssetId>) -> Result<()>;

    /// Delete a batch
    async fn delete_batch(&self, batch_id: BatchId) -> Result<()>;
}
