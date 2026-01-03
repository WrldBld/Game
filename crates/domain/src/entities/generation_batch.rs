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
    pub id: BatchId,
    /// World this batch belongs to
    pub world_id: WorldId,
    /// Type of entity this batch is for
    pub entity_type: EntityType,
    /// ID of the entity (Character, Location, or Item)
    pub entity_id: String,
    /// Type of asset being generated
    pub asset_type: AssetType,
    /// ComfyUI workflow to use
    pub workflow: String,
    /// Prompt for generation
    pub prompt: String,
    /// Negative prompt (if any)
    pub negative_prompt: Option<String>,
    /// Number of variations to generate
    pub count: u8,
    /// Current status
    pub status: BatchStatus,
    /// Generated asset IDs (populated when complete)
    pub assets: Vec<AssetId>,
    /// Style reference asset ID (if using consistent style)
    pub style_reference_id: Option<AssetId>,
    /// When the batch was requested
    pub requested_at: DateTime<Utc>,
    /// When the batch completed (success or failure)
    pub completed_at: Option<DateTime<Utc>>,
}

impl GenerationBatch {
    /// Create a new generation batch request
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

    pub fn with_negative_prompt(mut self, negative_prompt: impl Into<String>) -> Self {
        self.negative_prompt = Some(negative_prompt.into());
        self
    }

    pub fn with_style_reference(mut self, style_reference_id: AssetId) -> Self {
        self.style_reference_id = Some(style_reference_id);
        self
    }

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
    pub world_id: WorldId,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub asset_type: AssetType,
    pub workflow: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub count: u8,
    pub style_reference_id: Option<AssetId>,
}

impl GenerationRequest {
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
