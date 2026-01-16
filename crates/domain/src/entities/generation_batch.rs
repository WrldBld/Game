//! GenerationBatch entity - Tracks batches of AI-generated assets

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::gallery_asset::{AssetType, EntityType};
use crate::{AssetId, BatchId, WorldId};

// Re-export BatchStatus from types module
pub use crate::types::BatchStatus;

/// A batch of assets being generated together
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationBatch {
    id: BatchId,
    /// World this batch belongs to
    world_id: WorldId,
    /// Type of entity this batch is for
    entity_type: EntityType,
    /// ID of the entity (Character, Location, or Item)
    entity_id: String,
    /// Type of asset being generated
    asset_type: AssetType,
    /// ComfyUI workflow to use
    workflow: String,
    /// Prompt for generation
    prompt: String,
    /// Negative prompt (if any)
    negative_prompt: Option<String>,
    /// Number of variations to generate
    count: u8,
    /// Current status
    status: BatchStatus,
    /// Generated asset IDs (populated when complete)
    assets: Vec<AssetId>,
    /// Style reference asset ID (if using consistent style)
    style_reference_id: Option<AssetId>,
    /// When the batch was requested
    requested_at: DateTime<Utc>,
    /// When the batch completed (success or failure)
    completed_at: Option<DateTime<Utc>>,
}

impl GenerationBatch {
    /// Create a new generation batch request
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world_id: WorldId,
        entity_type: EntityType,
        entity_id: impl Into<String>,
        asset_type: AssetType,
        workflow: impl Into<String>,
        prompt: impl Into<String>,
        count: u8,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: BatchId::new(),
            world_id,
            entity_type,
            entity_id: entity_id.into(),
            asset_type,
            workflow: workflow.into(),
            prompt: prompt.into(),
            negative_prompt: None,
            count,
            status: BatchStatus::Queued,
            assets: Vec::new(),
            style_reference_id: None,
            requested_at: now,
            completed_at: None,
        }
    }

    // --- Accessors ---

    pub fn id(&self) -> BatchId {
        self.id
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn entity_type(&self) -> EntityType {
        self.entity_type
    }

    pub fn entity_id(&self) -> &str {
        &self.entity_id
    }

    pub fn asset_type(&self) -> AssetType {
        self.asset_type
    }

    pub fn workflow(&self) -> &str {
        &self.workflow
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn negative_prompt(&self) -> Option<&str> {
        self.negative_prompt.as_deref()
    }

    pub fn count(&self) -> u8 {
        self.count
    }

    pub fn status(&self) -> &BatchStatus {
        &self.status
    }

    pub fn assets(&self) -> &[AssetId] {
        &self.assets
    }

    pub fn style_reference_id(&self) -> Option<AssetId> {
        self.style_reference_id
    }

    pub fn requested_at(&self) -> DateTime<Utc> {
        self.requested_at
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    // --- Builder methods ---

    pub fn with_negative_prompt(mut self, negative_prompt: impl Into<String>) -> Self {
        self.negative_prompt = Some(negative_prompt.into());
        self
    }

    pub fn with_style_reference(mut self, style_reference_id: AssetId) -> Self {
        self.style_reference_id = Some(style_reference_id);
        self
    }

    // --- Mutation methods ---

    /// Start generating this batch
    pub fn start_generating(&mut self) {
        self.status = BatchStatus::Generating { progress: 0 };
    }

    /// Update generation progress (0-100)
    pub fn update_progress(&mut self, progress: u8) {
        self.status = BatchStatus::Generating {
            progress: progress.min(100),
        };
    }

    /// Mark generation as complete, ready for selection
    pub fn complete_generation(&mut self, asset_ids: Vec<AssetId>, now: DateTime<Utc>) {
        self.assets = asset_ids;
        self.status = BatchStatus::ReadyForSelection;
        self.completed_at = Some(now);
    }

    /// Mark batch as fully completed (user has selected assets)
    pub fn finalize(&mut self, now: DateTime<Utc>) {
        self.status = BatchStatus::Completed;
        if self.completed_at.is_none() {
            self.completed_at = Some(now);
        }
    }

    /// Mark batch as failed
    pub fn fail(&mut self, error: impl Into<String>, now: DateTime<Utc>) {
        self.status = BatchStatus::Failed {
            error: error.into(),
        };
        self.completed_at = Some(now);
    }

    /// Get position in queue (for display purposes)
    pub fn queue_position(&self) -> Option<u32> {
        // This would be set by the queue manager
        None
    }
}

/// Request to create a new generation batch
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRequest {
    world_id: WorldId,
    entity_type: EntityType,
    entity_id: String,
    asset_type: AssetType,
    workflow: String,
    prompt: String,
    negative_prompt: Option<String>,
    count: u8,
    style_reference_id: Option<AssetId>,
}

impl GenerationRequest {
    /// Create a new generation request
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world_id: WorldId,
        entity_type: EntityType,
        entity_id: impl Into<String>,
        asset_type: AssetType,
        workflow: impl Into<String>,
        prompt: impl Into<String>,
        count: u8,
    ) -> Self {
        Self {
            world_id,
            entity_type,
            entity_id: entity_id.into(),
            asset_type,
            workflow: workflow.into(),
            prompt: prompt.into(),
            negative_prompt: None,
            count,
            style_reference_id: None,
        }
    }

    // --- Accessors ---

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn entity_type(&self) -> EntityType {
        self.entity_type
    }

    pub fn entity_id(&self) -> &str {
        &self.entity_id
    }

    pub fn asset_type(&self) -> AssetType {
        self.asset_type
    }

    pub fn workflow(&self) -> &str {
        &self.workflow
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn negative_prompt(&self) -> Option<&str> {
        self.negative_prompt.as_deref()
    }

    pub fn count(&self) -> u8 {
        self.count
    }

    pub fn style_reference_id(&self) -> Option<AssetId> {
        self.style_reference_id
    }

    // --- Builder methods ---

    pub fn with_negative_prompt(mut self, negative_prompt: impl Into<String>) -> Self {
        self.negative_prompt = Some(negative_prompt.into());
        self
    }

    pub fn with_style_reference(mut self, style_reference_id: AssetId) -> Self {
        self.style_reference_id = Some(style_reference_id);
        self
    }

    // --- Conversion ---

    pub fn into_batch(self, now: DateTime<Utc>) -> GenerationBatch {
        let mut batch = GenerationBatch::new(
            self.world_id,
            self.entity_type,
            self.entity_id,
            self.asset_type,
            self.workflow,
            self.prompt,
            self.count,
            now,
        );
        if let Some(neg) = self.negative_prompt {
            batch = batch.with_negative_prompt(neg);
        }
        if let Some(style_ref) = self.style_reference_id {
            batch = batch.with_style_reference(style_ref);
        }
        batch
    }
}
