// Asset generation use cases - methods for future image generation
#![allow(dead_code)]

//! Asset generation use cases.
//!
//! Handles image generation for game entities (characters, locations, items).

pub mod expression_sheet;

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{
    AssetId, AssetPath, AssetType, BatchId, EntityType, GalleryAsset, GenerationMetadata, WorldId,
};

use crate::queue_types::AssetGenerationData;

use crate::infrastructure::ports::{ClockPort, ImageGenError, ImageRequest, QueuePort, RepoError};
use crate::repositories::AssetsRepository;

// Type aliases for old names to maintain compatibility
type QueueService = dyn QueuePort;
type ClockService = dyn ClockPort;

pub use expression_sheet::GenerateExpressionSheet;

/// Container for asset use cases.
pub struct AssetUseCases {
    pub generate: Arc<GenerateAsset>,
    pub expression_sheet: Arc<GenerateExpressionSheet>,
}

impl AssetUseCases {
    pub fn new(
        generate: Arc<GenerateAsset>,
        expression_sheet: Arc<GenerateExpressionSheet>,
    ) -> Self {
        Self {
            generate,
            expression_sheet,
        }
    }
}

/// Result of asset generation.
#[derive(Debug)]
pub struct GenerateResult {
    /// The generated asset ID
    pub asset_id: AssetId,
    /// The image data (bytes)
    pub image_data: Vec<u8>,
    /// The image format (e.g., "png")
    pub format: String,
}

/// Generate asset use case.
///
/// Orchestrates image generation for game entities.
pub struct GenerateAsset {
    assets: Arc<AssetsRepository>,
    queue: Arc<QueueService>,
    clock: Arc<ClockService>,
}

impl GenerateAsset {
    pub fn new(
        assets: Arc<AssetsRepository>,
        queue: Arc<QueueService>,
        clock: Arc<ClockService>,
    ) -> Self {
        Self {
            assets,
            queue,
            clock,
        }
    }

    /// Generate an image synchronously (blocking until complete).
    ///
    /// # Arguments
    /// * `entity_type` - Type of entity (Character, Location, Item)
    /// * `entity_id` - ID of the entity
    /// * `asset_type` - Type of asset (Portrait, Sprite, etc.)
    /// * `prompt` - Generation prompt
    /// * `workflow` - ComfyUI workflow to use
    ///
    /// # Returns
    /// * `Ok(GenerateResult)` - Image generated successfully
    /// * `Err(GenerateError)` - Generation failed
    pub async fn execute(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
        asset_type: AssetType,
        prompt: &str,
        workflow: &str,
    ) -> Result<GenerateResult, GenerateError> {
        // Check if service is available
        let is_healthy = match self.assets.check_health().await {
            Ok(healthy) => healthy,
            Err(e) => {
                tracing::warn!(error = %e, "Asset generation health check failed, treating as unavailable");
                false
            }
        };
        if !is_healthy {
            return Err(GenerateError::Unavailable);
        }

        // Generate the image
        let request = ImageRequest {
            prompt: prompt.to_string(),
            workflow: workflow.to_string(),
            width: 512,
            height: 512,
        };

        let image_data = self
            .assets
            .generate(request)
            .await
            .map_err(|e| GenerateError::Failed(e.to_string()))?;

        // Create generation metadata
        let batch_id = BatchId::new();
        let seed = rand::random::<i64>().abs(); // Random seed
        let metadata = GenerationMetadata {
            workflow: workflow.to_string(),
            prompt: prompt.to_string(),
            negative_prompt: None,
            seed,
            style_reference_id: None,
            batch_id,
        };

        // Create the asset
        let now = self.clock.now();
        let file_path_str = format!("assets/{:?}/{}.png", entity_type, entity_id);
        let file_path = AssetPath::new(file_path_str)
            .map_err(|e| GenerateError::Failed(format!("Invalid asset path: {}", e)))?;
        let asset = GalleryAsset::new_generated(
            entity_type,
            entity_id.to_string(),
            asset_type,
            file_path,
            metadata,
            now,
        );

        let asset_id = asset.id();

        self.assets
            .save(&asset)
            .await
            .map_err(|e| GenerateError::Failed(e.to_string()))?;

        Ok(GenerateResult {
            asset_id,
            image_data,
            format: "png".to_string(),
        })
    }

    /// Queue an asset generation request for background processing.
    ///
    /// Returns immediately with a queue ID that can be used to track status.
    pub async fn queue_generation(
        &self,
        world_id: Option<WorldId>,
        entity_type: &str,
        entity_id: &str,
        workflow_id: &str,
        prompt: &str,
        count: u32,
    ) -> Result<Uuid, GenerateError> {
        let data = AssetGenerationData {
            world_id,
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            workflow_id: workflow_id.to_string(),
            prompt: prompt.to_string(),
            count,
        };

        self.queue
            .enqueue_asset_generation(&data)
            .await
            .map_err(|e| GenerateError::Failed(e.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("Generation failed: {0}")]
    Failed(String),
    #[error("Service unavailable")]
    Unavailable,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Image generation error: {0}")]
    ImageGen(#[from] ImageGenError),
}
