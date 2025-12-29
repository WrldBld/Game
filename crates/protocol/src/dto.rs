//! Data Transfer Objects (DTOs)
//!
//! Wire-format types for serialization/deserialization that are shared
//! between engine and player. These types use raw UUIDs and primitive types
//! for transport, rather than domain ID types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ARCHITECTURE EXCEPTION: [APPROVED 2025-12-28]
// Uses domain ID types for DTO conversion methods only.
// Wire format uses raw Uuid; these imports enable to_domain() conversion.
use wrldbldr_domain::entities::{BatchStatus, GalleryAsset, GenerationBatch};
use wrldbldr_domain::value_objects::{DispositionLevel, NpcDispositionState, RelationshipLevel};
use wrldbldr_domain::{CharacterId, PlayerCharacterId};

// =============================================================================
// NPC Disposition DTOs
// =============================================================================

/// Wire-format disposition state for protocol serialization
///
/// This DTO is used to transfer NPC disposition state over WebSocket/HTTP.
/// It uses raw UUIDs instead of domain ID types for serialization compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcDispositionStateDto {
    /// The NPC's UUID
    pub npc_id: Uuid,
    /// The PC's UUID
    pub pc_id: Uuid,
    /// Current emotional stance
    pub disposition: DispositionLevel,
    /// Long-term relationship level
    pub relationship: RelationshipLevel,
    /// Fine-grained sentiment score (-1.0 to 1.0)
    pub sentiment: f32,
    /// When this state was last updated (RFC 3339)
    pub updated_at: String,
    /// Reason for the last disposition change
    pub disposition_reason: Option<String>,
    /// Accumulated relationship points
    pub relationship_points: i32,
}

impl From<&NpcDispositionState> for NpcDispositionStateDto {
    fn from(state: &NpcDispositionState) -> Self {
        Self {
            npc_id: state.npc_id.to_uuid(),
            pc_id: state.pc_id.to_uuid(),
            disposition: state.disposition,
            relationship: state.relationship,
            sentiment: state.sentiment,
            updated_at: state.updated_at.to_rfc3339(),
            disposition_reason: state.disposition_reason.clone(),
            relationship_points: state.relationship_points,
        }
    }
}

impl NpcDispositionStateDto {
    /// Convert back to domain type
    pub fn to_domain(&self) -> NpcDispositionState {
        use chrono::Utc;
        NpcDispositionState {
            npc_id: CharacterId::from_uuid(self.npc_id),
            pc_id: PlayerCharacterId::from_uuid(self.pc_id),
            disposition: self.disposition,
            relationship: self.relationship,
            sentiment: self.sentiment,
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            disposition_reason: self.disposition_reason.clone(),
            relationship_points: self.relationship_points,
        }
    }
}

// =============================================================================
// Asset DTOs (REST API)
// =============================================================================

/// Request DTO for uploading an asset
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadAssetRequestDto {
    pub asset_type: String,
    pub file_path: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub set_active: bool,
}

/// Request DTO for updating an asset's label
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetLabelRequestDto {
    pub label: Option<String>,
}

/// Response DTO for gallery assets
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryAssetResponseDto {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub asset_type: String,
    pub file_path: String,
    pub is_active: bool,
    pub label: Option<String>,
    pub is_generated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_reference_id: Option<String>,
    pub created_at: String,
}

impl From<GalleryAsset> for GalleryAssetResponseDto {
    fn from(a: GalleryAsset) -> Self {
        let is_generated = a.is_generated();
        Self {
            id: a.id.to_string(),
            entity_type: a.entity_type.to_string(),
            entity_id: a.entity_id,
            asset_type: a.asset_type.to_string(),
            file_path: a.file_path,
            is_active: a.is_active,
            label: a.label,
            is_generated,
            style_reference_id: None,
            created_at: a.created_at.to_rfc3339(),
        }
    }
}

/// Request DTO for generating an asset
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateAssetRequestDto {
    /// World this asset belongs to
    pub world_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub asset_type: String,
    pub workflow: String,
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    #[serde(default = "default_generate_count")]
    pub count: u8,
    #[serde(default)]
    pub style_reference_id: Option<String>,
}

fn default_generate_count() -> u8 {
    4
}

/// Response DTO for generation batches
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationBatchResponseDto {
    pub id: String,
    /// World this batch belongs to
    pub world_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub asset_type: String,
    pub workflow: String,
    pub prompt: String,
    pub count: u8,
    pub status: String,
    pub progress: Option<u8>,
    pub asset_count: usize,
    pub requested_at: String,
    pub completed_at: Option<String>,
}

impl From<GenerationBatch> for GenerationBatchResponseDto {
    fn from(b: GenerationBatch) -> Self {
        let (status, progress) = match &b.status {
            BatchStatus::Queued => ("Queued".to_string(), None),
            BatchStatus::Generating { progress } => ("Generating".to_string(), Some(*progress)),
            BatchStatus::ReadyForSelection => ("ReadyForSelection".to_string(), Some(100)),
            BatchStatus::Completed => ("Completed".to_string(), Some(100)),
            BatchStatus::Failed { error } => (format!("Failed: {}", error), None),
        };

        Self {
            id: b.id.to_string(),
            world_id: b.world_id.to_string(),
            entity_type: b.entity_type.to_string(),
            entity_id: b.entity_id,
            asset_type: b.asset_type.to_string(),
            workflow: b.workflow,
            prompt: b.prompt,
            count: b.count,
            status,
            progress,
            asset_count: b.assets.len(),
            requested_at: b.requested_at.to_rfc3339(),
            completed_at: b.completed_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// Request DTO for selecting assets from a batch
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectFromBatchRequestDto {
    pub selected_assets: Vec<String>,
    #[serde(default)]
    pub discard_others: bool,
    #[serde(default)]
    pub labels: Vec<Option<String>>,
}

// =============================================================================
// Export DTOs (REST API)
// =============================================================================

/// Query parameters DTO for export endpoints
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportQueryDto {
    #[serde(default)]
    pub format: Option<String>,
}

// =============================================================================
// Workflow DTOs (REST API)
// =============================================================================

/// DTO for prompt mapping configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMappingDto {
    pub node_id: String,
    pub input_name: String,
    pub mapping_type: PromptMappingTypeDto,
}

/// DTO for prompt mapping type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptMappingTypeDto {
    Primary,
    Negative,
}

/// Response DTO for workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowConfigResponseDto {
    pub id: String,
    pub slot: String,
    pub slot_display_name: String,
    pub name: String,
    pub node_count: usize,
    pub input_count: usize,
    pub prompt_mappings: Vec<PromptMappingDto>,
    pub has_primary_prompt: bool,
    pub has_negative_prompt: bool,
    pub created_at: String,
    pub updated_at: String,
}
