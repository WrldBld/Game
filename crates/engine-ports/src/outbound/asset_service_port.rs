//! Asset service port - Interface for asset gallery operations
//!
//! This port abstracts asset gallery business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::entities::{AssetType, EntityType, GalleryAsset};
use wrldbldr_domain::AssetId;

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
/// including listing, creating, and retrieving assets.
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
}
