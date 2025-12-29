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

use wrldbldr_domain::entities::{PromptMapping, PromptMappingType, WorkflowAnalysis};

/// DTO for prompt mapping configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMappingDto {
    pub node_id: String,
    pub input_name: String,
    pub mapping_type: PromptMappingTypeDto,
}

impl From<PromptMapping> for PromptMappingDto {
    fn from(value: PromptMapping) -> Self {
        Self {
            node_id: value.node_id,
            input_name: value.input_name,
            mapping_type: value.mapping_type.into(),
        }
    }
}

impl From<PromptMappingDto> for PromptMapping {
    fn from(value: PromptMappingDto) -> Self {
        Self {
            node_id: value.node_id,
            input_name: value.input_name,
            mapping_type: value.mapping_type.into(),
        }
    }
}

/// DTO for prompt mapping type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptMappingTypeDto {
    Primary,
    Negative,
}

impl From<PromptMappingType> for PromptMappingTypeDto {
    fn from(value: PromptMappingType) -> Self {
        match value {
            PromptMappingType::Primary => Self::Primary,
            PromptMappingType::Negative => Self::Negative,
        }
    }
}

impl From<PromptMappingTypeDto> for PromptMappingType {
    fn from(value: PromptMappingTypeDto) -> Self {
        match value {
            PromptMappingTypeDto::Primary => Self::Primary,
            PromptMappingTypeDto::Negative => Self::Negative,
        }
    }
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

// =============================================================================
// Rule System DTOs (REST API)
// =============================================================================

use wrldbldr_domain::value_objects::{RuleSystemConfig, RuleSystemType, RuleSystemVariant};

/// Summary of a preset for browsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSystemPresetSummaryDto {
    pub variant: RuleSystemVariant,
    pub name: String,
    pub description: String,
}

/// Summary of a rule system type for browsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSystemSummaryDto {
    pub system_type: RuleSystemType,
    pub name: String,
    pub description: String,
    pub dice_notation: String,
    pub presets: Vec<RuleSystemPresetSummaryDto>,
}

/// Full preset details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSystemPresetDetailsDto {
    pub variant: RuleSystemVariant,
    pub config: RuleSystemConfig,
}

/// Details about a rule system type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSystemTypeDetailsDto {
    pub system_type: RuleSystemType,
    pub name: String,
    pub description: String,
    pub dice_notation: String,
    pub presets: Vec<RuleSystemPresetSummaryDto>,
}

/// Parse a string into a RuleSystemType.
pub fn parse_system_type(s: &str) -> Result<RuleSystemType, String> {
    match s.to_lowercase().as_str() {
        "d20" => Ok(RuleSystemType::D20),
        "d100" => Ok(RuleSystemType::D100),
        "narrative" => Ok(RuleSystemType::Narrative),
        "custom" => Ok(RuleSystemType::Custom),
        _ => Err(format!(
            "Unknown rule system type: {}. Valid types: d20, d100, narrative, custom",
            s
        )),
    }
}

/// Parse a string into a RuleSystemVariant.
pub fn parse_variant(s: &str) -> Result<RuleSystemVariant, String> {
    match s.to_lowercase().replace("-", "_").as_str() {
        "dnd5e" | "dnd_5e" => Ok(RuleSystemVariant::Dnd5e),
        "pathfinder2e" | "pathfinder_2e" => Ok(RuleSystemVariant::Pathfinder2e),
        "generic_d20" | "genericd20" => Ok(RuleSystemVariant::GenericD20),
        "coc7e" | "coc_7e" | "callofcthulhu7e" | "call_of_cthulhu_7e" => {
            Ok(RuleSystemVariant::CallOfCthulhu7e)
        }
        "runequest" | "rune_quest" => Ok(RuleSystemVariant::RuneQuest),
        "generic_d100" | "genericd100" => Ok(RuleSystemVariant::GenericD100),
        "kidsonbikes" | "kids_on_bikes" => Ok(RuleSystemVariant::KidsOnBikes),
        "fatecore" | "fate_core" | "fate" => Ok(RuleSystemVariant::FateCore),
        "pbta" | "poweredbyapocalypse" | "powered_by_apocalypse" => {
            Ok(RuleSystemVariant::PoweredByApocalypse)
        }
        _ => Err(format!(
            "Unknown variant: {}. Valid variants: dnd5e, pathfinder2e, generic_d20, coc7e, runequest, generic_d100, kidsonbikes, fatecore, pbta",
            s
        )),
    }
}

// =============================================================================
// Extended Workflow DTOs (REST API)
// =============================================================================

use std::str::FromStr;
use wrldbldr_domain::entities::{InputDefault, InputType, WorkflowInput, WorkflowSlot};

/// Response for workflow slot status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSlotStatusDto {
    pub slot: String,
    pub display_name: String,
    pub default_width: u32,
    pub default_height: u32,
    pub configured: bool,
    pub config: Option<WorkflowConfigResponseDto>,
}

/// A category of workflow slots (e.g., "Character Assets").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSlotCategoryDto {
    pub name: String,
    pub slots: Vec<WorkflowSlotStatusDto>,
}

/// Response containing all workflow slots grouped by category.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSlotsResponseDto {
    pub categories: Vec<WorkflowSlotCategoryDto>,
}

/// DTO for input default values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDefaultDto {
    pub node_id: String,
    pub input_name: String,
    pub default_value: serde_json::Value,
}

impl From<InputDefault> for InputDefaultDto {
    fn from(value: InputDefault) -> Self {
        Self {
            node_id: value.node_id,
            input_name: value.input_name,
            default_value: value.default_value,
        }
    }
}

impl From<InputDefaultDto> for InputDefault {
    fn from(value: InputDefaultDto) -> Self {
        Self {
            node_id: value.node_id,
            input_name: value.input_name,
            default_value: value.default_value,
        }
    }
}

/// DTO for input type classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputTypeDto {
    Text,
    Integer,
    Float,
    Boolean,
    Select(Vec<String>),
    Unknown,
}

impl From<InputType> for InputTypeDto {
    fn from(value: InputType) -> Self {
        match value {
            InputType::Text => Self::Text,
            InputType::Integer => Self::Integer,
            InputType::Float => Self::Float,
            InputType::Boolean => Self::Boolean,
            InputType::Select(opts) => Self::Select(opts),
            InputType::Unknown => Self::Unknown,
        }
    }
}

impl From<InputTypeDto> for InputType {
    fn from(value: InputTypeDto) -> Self {
        match value {
            InputTypeDto::Text => Self::Text,
            InputTypeDto::Integer => Self::Integer,
            InputTypeDto::Float => Self::Float,
            InputTypeDto::Boolean => Self::Boolean,
            InputTypeDto::Select(opts) => Self::Select(opts),
            InputTypeDto::Unknown => Self::Unknown,
        }
    }
}

/// DTO for workflow input information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowInputDto {
    pub node_id: String,
    pub node_type: String,
    pub node_title: Option<String>,
    pub input_name: String,
    pub input_type: InputTypeDto,
    pub current_value: serde_json::Value,
}

impl From<WorkflowInput> for WorkflowInputDto {
    fn from(value: WorkflowInput) -> Self {
        Self {
            node_id: value.node_id,
            node_type: value.node_type,
            node_title: value.node_title,
            input_name: value.input_name,
            input_type: value.input_type.into(),
            current_value: value.current_value,
        }
    }
}

/// DTO for workflow analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowAnalysisDto {
    pub node_count: usize,
    pub inputs: Vec<WorkflowInputDto>,
    pub text_inputs: Vec<WorkflowInputDto>,
    pub errors: Vec<String>,
}

impl From<WorkflowAnalysis> for WorkflowAnalysisDto {
    fn from(value: WorkflowAnalysis) -> Self {
        Self {
            node_count: value.node_count,
            inputs: value.inputs.into_iter().map(WorkflowInputDto::from).collect(),
            text_inputs: value.text_inputs.into_iter().map(WorkflowInputDto::from).collect(),
            errors: value.errors,
        }
    }
}

/// Full configuration response (includes workflow JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowConfigFullResponseDto {
    pub id: String,
    pub slot: String,
    pub slot_display_name: String,
    pub name: String,
    pub workflow_json: serde_json::Value,
    pub prompt_mappings: Vec<PromptMappingDto>,
    pub input_defaults: Vec<InputDefaultDto>,
    pub locked_inputs: Vec<String>,
    pub analysis: WorkflowAnalysisDto,
    pub created_at: String,
    pub updated_at: String,
}

/// Response for workflow analysis endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowAnalysisResponseDto {
    pub is_valid: bool,
    pub analysis: WorkflowAnalysisDto,
    pub suggested_prompt_mappings: Vec<PromptMappingDto>,
}

/// Response for workflow import operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkflowsResponseDto {
    pub imported: usize,
    pub skipped: usize,
}

/// Response for workflow test operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestWorkflowResponseDto {
    pub prompt_id: String,
    pub queue_position: u32,
}

/// Request to create/update a workflow configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkflowConfigRequestDto {
    pub name: String,
    pub workflow_json: serde_json::Value,
    #[serde(default)]
    pub prompt_mappings: Vec<PromptMappingDto>,
    #[serde(default)]
    pub input_defaults: Vec<InputDefaultDto>,
    #[serde(default)]
    pub locked_inputs: Vec<String>,
}

/// Request to update just the defaults of a workflow (without re-uploading the workflow JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkflowDefaultsRequestDto {
    #[serde(default)]
    pub input_defaults: Vec<InputDefaultDto>,
    #[serde(default)]
    pub locked_inputs: Option<Vec<String>>,
}

/// Request to analyze a workflow (without saving).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeWorkflowRequestDto {
    pub workflow_json: serde_json::Value,
}

/// Import workflow configurations request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkflowsRequestDto {
    pub data: serde_json::Value,
    #[serde(default)]
    pub replace_existing: bool,
}

/// Test a workflow configuration request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestWorkflowRequestDto {
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: Option<String>,
}

/// Parse a workflow slot string into a WorkflowSlot enum.
pub fn parse_workflow_slot(slot: &str) -> Result<WorkflowSlot, String> {
    WorkflowSlot::from_str(slot)
}
