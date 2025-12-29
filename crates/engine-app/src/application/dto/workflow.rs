//! Workflow DTOs - Application layer extensions
//!
//! Wire-format types are defined in `wrldbldr_protocol::dto`.
//! This module re-exports them and provides `From` implementations
//! that require application layer services (e.g., WorkflowService).

use std::str::FromStr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::services::WorkflowService;
use wrldbldr_domain::entities::{WorkflowConfiguration, WorkflowSlot};
use wrldbldr_domain::WorkflowConfigId;

// Re-export wire-format types from protocol
pub use wrldbldr_protocol::{
    parse_workflow_slot, AnalyzeWorkflowRequestDto, CreateWorkflowConfigRequestDto,
    ImportWorkflowsRequestDto, ImportWorkflowsResponseDto, InputDefaultDto, InputTypeDto,
    PromptMappingDto, PromptMappingTypeDto, TestWorkflowRequestDto, TestWorkflowResponseDto,
    UpdateWorkflowDefaultsRequestDto, WorkflowAnalysisDto, WorkflowAnalysisResponseDto,
    WorkflowConfigFullResponseDto, WorkflowConfigResponseDto, WorkflowInputDto,
    WorkflowSlotCategoryDto, WorkflowSlotStatusDto, WorkflowSlotsResponseDto,
};

// ============================================================================
// Conversion functions that require application services
// ============================================================================

/// Build a WorkflowConfigResponseDto from a WorkflowConfiguration.
/// This requires WorkflowService to analyze the workflow JSON.
pub fn workflow_config_to_response_dto(
    config: &WorkflowConfiguration,
) -> WorkflowConfigResponseDto {
    let analysis = WorkflowService::analyze_workflow(&config.workflow_json);
    WorkflowConfigResponseDto {
        id: config.id.to_string(),
        slot: config.slot.as_str().to_string(),
        slot_display_name: config.slot.display_name().to_string(),
        name: config.name.clone(),
        node_count: analysis.node_count,
        input_count: analysis.inputs.len(),
        prompt_mappings: config
            .prompt_mappings
            .clone()
            .into_iter()
            .map(Into::into)
            .collect(),
        has_primary_prompt: config.primary_prompt_mapping().is_some(),
        has_negative_prompt: config.negative_prompt_mapping().is_some(),
        created_at: config.created_at.to_rfc3339(),
        updated_at: config.updated_at.to_rfc3339(),
    }
}

/// Build a WorkflowConfigFullResponseDto from a WorkflowConfiguration.
/// This requires WorkflowService to analyze the workflow JSON.
pub fn workflow_config_to_full_response_dto(
    config: &WorkflowConfiguration,
) -> WorkflowConfigFullResponseDto {
    let analysis = WorkflowService::analyze_workflow(&config.workflow_json);
    WorkflowConfigFullResponseDto {
        id: config.id.to_string(),
        slot: config.slot.as_str().to_string(),
        slot_display_name: config.slot.display_name().to_string(),
        name: config.name.clone(),
        workflow_json: config.workflow_json.clone(),
        prompt_mappings: config
            .prompt_mappings
            .clone()
            .into_iter()
            .map(Into::into)
            .collect(),
        input_defaults: config
            .input_defaults
            .clone()
            .into_iter()
            .map(Into::into)
            .collect(),
        locked_inputs: config.locked_inputs.clone(),
        analysis: analysis.into(),
        created_at: config.created_at.to_rfc3339(),
        updated_at: config.updated_at.to_rfc3339(),
    }
}

// ============================================================================
// Export/Import DTO (engine-app specific, uses WorkflowConfigId)
// ============================================================================

use serde::{Deserialize, Serialize};

/// Export/import DTO for workflow configurations (backup/restore).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfigExportDto {
    pub id: String,
    pub slot: String,
    pub name: String,
    pub workflow_json: serde_json::Value,
    #[serde(default)]
    pub prompt_mappings: Vec<PromptMappingDto>,
    #[serde(default)]
    pub input_defaults: Vec<InputDefaultDto>,
    #[serde(default)]
    pub locked_inputs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<WorkflowConfiguration> for WorkflowConfigExportDto {
    fn from(value: WorkflowConfiguration) -> Self {
        Self {
            id: value.id.to_string(),
            slot: value.slot.as_str().to_string(),
            name: value.name,
            workflow_json: value.workflow_json,
            prompt_mappings: value.prompt_mappings.into_iter().map(Into::into).collect(),
            input_defaults: value.input_defaults.into_iter().map(Into::into).collect(),
            locked_inputs: value.locked_inputs,
            created_at: value.created_at.to_rfc3339(),
            updated_at: value.updated_at.to_rfc3339(),
        }
    }
}

impl TryFrom<WorkflowConfigExportDto> for WorkflowConfiguration {
    type Error = anyhow::Error;

    fn try_from(value: WorkflowConfigExportDto) -> anyhow::Result<Self> {
        let id = Uuid::parse_str(&value.id)
            .map(WorkflowConfigId::from_uuid)
            .unwrap_or_else(|_| WorkflowConfigId::new());

        let slot = WorkflowSlot::from_str(&value.slot)
            .map_err(|_| anyhow::anyhow!("Invalid workflow slot: {}", value.slot))?;

        let created_at = DateTime::parse_from_rfc3339(&value.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let updated_at = DateTime::parse_from_rfc3339(&value.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Self {
            id,
            slot,
            name: value.name,
            workflow_json: value.workflow_json,
            prompt_mappings: value.prompt_mappings.into_iter().map(Into::into).collect(),
            input_defaults: value.input_defaults.into_iter().map(Into::into).collect(),
            locked_inputs: value.locked_inputs,
            created_at,
            updated_at,
        })
    }
}
