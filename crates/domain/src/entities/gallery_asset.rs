//! GalleryAsset entity - Assets stored in entity galleries with history

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AssetId, BatchId};

// Re-export shared types from types module
pub use crate::types::{AssetType, EntityType};

/// Metadata about how an asset was generated
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationMetadata {
    /// ComfyUI workflow used
    pub workflow: String,
    /// Prompt used for generation
    pub prompt: String,
    /// Negative prompt (if any)
    pub negative_prompt: Option<String>,
    /// Seed used for reproducibility
    pub seed: i64,
    /// Style reference asset (if any)
    pub style_reference_id: Option<AssetId>,
    /// Batch this asset was generated in
    pub batch_id: BatchId,
}

/// An asset stored in an entity's gallery
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryAsset {
    id: AssetId,
    /// Type of entity that owns this asset
    entity_type: EntityType,
    /// ID of the owning entity (Character, Location, or Item)
    entity_id: String,
    /// Type of asset (Portrait, Sprite, Backdrop, etc.)
    asset_type: AssetType,
    /// Path to the stored asset file
    file_path: String,
    /// Whether this is the currently active asset for this slot
    is_active: bool,
    /// User-defined label (e.g., "Angry", "Winter Outfit", "Night")
    label: Option<String>,
    /// Metadata about generation (if AI-generated)
    generation_metadata: Option<GenerationMetadata>,
    /// When the asset was created/uploaded
    created_at: DateTime<Utc>,
}

impl GalleryAsset {
    /// Create a new gallery asset (uploaded, not generated)
    pub fn new(
        entity_type: EntityType,
        entity_id: impl Into<String>,
        asset_type: AssetType,
        file_path: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: AssetId::new(),
            entity_type,
            entity_id: entity_id.into(),
            asset_type,
            file_path: file_path.into(),
            is_active: false,
            label: None,
            generation_metadata: None,
            created_at: now,
        }
    }

    /// Create a new generated asset
    pub fn new_generated(
        entity_type: EntityType,
        entity_id: impl Into<String>,
        asset_type: AssetType,
        file_path: impl Into<String>,
        metadata: GenerationMetadata,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: AssetId::new(),
            entity_type,
            entity_id: entity_id.into(),
            asset_type,
            file_path: file_path.into(),
            is_active: false,
            label: None,
            generation_metadata: Some(metadata),
            created_at: now,
        }
    }

    /// Reconstruct from stored data (e.g., database)
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: AssetId,
        entity_type: EntityType,
        entity_id: String,
        asset_type: AssetType,
        file_path: String,
        is_active: bool,
        label: Option<String>,
        generation_metadata: Option<GenerationMetadata>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            entity_type,
            entity_id,
            asset_type,
            file_path,
            is_active,
            label,
            generation_metadata,
            created_at,
        }
    }

    // --- Accessors ---

    pub fn id(&self) -> AssetId {
        self.id
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

    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn generation_metadata(&self) -> Option<&GenerationMetadata> {
        self.generation_metadata.as_ref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    // --- Builder methods ---

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn set_active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    // --- Mutation methods ---

    /// Activate this asset (mark as current for its slot)
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// Deactivate this asset
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Update the label
    pub fn update_label(&mut self, label: Option<String>) {
        self.label = label;
    }

    /// Check if this asset was AI-generated
    pub fn is_generated(&self) -> bool {
        self.generation_metadata.is_some()
    }
}
