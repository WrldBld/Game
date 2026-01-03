//! Asset generation use cases.

use std::sync::Arc;

/// Container for asset use cases.
pub struct AssetUseCases {
    pub generate: Arc<GenerateAsset>,
}

impl AssetUseCases {
    pub fn new(generate: Arc<GenerateAsset>) -> Self {
        Self { generate }
    }
}

/// Generate asset use case.
pub struct GenerateAsset {
    #[allow(dead_code)]
    assets: Arc<crate::entities::Assets>,
}

impl GenerateAsset {
    pub fn new(assets: Arc<crate::entities::Assets>) -> Self {
        Self { assets }
    }

    pub async fn execute(
        &self,
        _entity_type: &str,
        _entity_id: uuid::Uuid,
        _prompt: &str,
    ) -> Result<GenerateResult, GenerateError> {
        todo!("Generate asset use case")
    }
}

#[derive(Debug)]
pub struct GenerateResult {
    pub asset_id: wrldbldr_domain::AssetId,
    pub image_url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("Generation failed: {0}")]
    Failed(String),
    #[error("Service unavailable")]
    Unavailable,
}
