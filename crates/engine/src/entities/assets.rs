//! Asset entity operations.

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{self as domain, AssetId};

use crate::infrastructure::ports::{AssetRepo, ImageGenPort, ImageRequest, RepoError};

/// Asset entity operations.
///
/// Handles gallery assets and image generation.
pub struct Assets {
    repo: Arc<dyn AssetRepo>,
    image_gen: Arc<dyn ImageGenPort>,
}

impl Assets {
    pub fn new(repo: Arc<dyn AssetRepo>, image_gen: Arc<dyn ImageGenPort>) -> Self {
        Self { repo, image_gen }
    }

    pub async fn get(&self, id: AssetId) -> Result<Option<domain::GalleryAsset>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, asset: &domain::GalleryAsset) -> Result<(), RepoError> {
        self.repo.save(asset).await
    }

    pub async fn list_for_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
    ) -> Result<Vec<domain::GalleryAsset>, RepoError> {
        self.repo.list_for_entity(entity_type, entity_id).await
    }

    pub async fn set_active(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        asset_id: AssetId,
    ) -> Result<(), RepoError> {
        self.repo.set_active(entity_type, entity_id, asset_id).await
    }

    /// Generate a new image asset.
    pub async fn generate(&self, request: ImageRequest) -> Result<Vec<u8>, crate::infrastructure::ports::ImageGenError> {
        let result = self.image_gen.generate(request).await?;
        Ok(result.image_data)
    }

    /// Check if image generation service is available.
    pub async fn check_health(&self) -> Result<bool, crate::infrastructure::ports::ImageGenError> {
        self.image_gen.check_health().await
    }
}
