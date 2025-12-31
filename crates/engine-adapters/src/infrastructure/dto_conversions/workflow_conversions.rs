//! Workflow-related DTO conversions
//!
//! Note: PromptMapping, InputDefault, InputType conversions remain in protocol
//! as From impls because they are used bidirectionally and the types are from
//! domain-types (shared vocabulary). This is acceptable since domain-types is
//! specifically designed to be used by both domain and protocol.

use chrono::{DateTime, Utc};
use wrldbldr_common::datetime::parse_datetime_or;
use wrldbldr_domain::entities::WorkflowConfiguration;
use wrldbldr_domain_types::{WorkflowAnalysis, WorkflowInput};
use wrldbldr_protocol::{
    WorkflowAnalysisDto, WorkflowConfigExportDto, WorkflowConfigFullResponseDto,
    WorkflowConfigResponseDto, WorkflowInputDto,
};

/// Convert WorkflowAnalysis to WorkflowAnalysisDto
pub fn workflow_analysis_to_dto(value: WorkflowAnalysis) -> WorkflowAnalysisDto {
    WorkflowAnalysisDto {
        node_count: value.node_count,
        inputs: value.inputs.into_iter().map(workflow_input_to_dto).collect(),
        text_inputs: value
            .text_inputs
            .into_iter()
            .map(workflow_input_to_dto)
            .collect(),
        errors: value.errors,
    }
}

/// Convert WorkflowInput to WorkflowInputDto
fn workflow_input_to_dto(value: WorkflowInput) -> WorkflowInputDto {
    WorkflowInputDto {
        node_id: value.node_id,
        node_type: value.node_type,
        node_title: value.node_title,
        input_name: value.input_name,
        input_type: value.input_type.into(), // Uses From impl from protocol
        current_value: value.current_value,
    }
}

/// Build a WorkflowConfigResponseDto from a WorkflowConfiguration and its analysis.
pub fn workflow_config_to_response_dto(
    config: &WorkflowConfiguration,
    analysis: &WorkflowAnalysis,
) -> WorkflowConfigResponseDto {
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

/// Build a WorkflowConfigFullResponseDto from a WorkflowConfiguration and its analysis.
pub fn workflow_config_to_full_response_dto(
    config: &WorkflowConfiguration,
    analysis: WorkflowAnalysis,
) -> WorkflowConfigFullResponseDto {
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
        analysis: workflow_analysis_to_dto(analysis),
        created_at: config.created_at.to_rfc3339(),
        updated_at: config.updated_at.to_rfc3339(),
    }
}

/// Convert WorkflowConfiguration to WorkflowConfigExportDto for backup/restore
pub fn workflow_config_to_export_dto(value: WorkflowConfiguration) -> WorkflowConfigExportDto {
    WorkflowConfigExportDto {
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

/// Convert WorkflowConfigExportDto to WorkflowConfiguration for import
///
/// # Arguments
/// * `value` - The export DTO to convert
/// * `fallback_time` - Fallback timestamp if parsing fails (typically from ClockPort)
pub fn workflow_config_from_export_dto(
    value: WorkflowConfigExportDto,
    fallback_time: DateTime<Utc>,
) -> anyhow::Result<WorkflowConfiguration> {
    use std::str::FromStr;
    use uuid::Uuid;
    use wrldbldr_domain::WorkflowConfigId;
    use wrldbldr_domain::entities::WorkflowSlot;

    let id = Uuid::parse_str(&value.id)
        .map(WorkflowConfigId::from_uuid)
        .unwrap_or_else(|_| WorkflowConfigId::new());

    let slot = WorkflowSlot::from_str(&value.slot)
        .map_err(|_| anyhow::anyhow!("Invalid workflow slot: {}", value.slot))?;

    let created_at = parse_datetime_or(&value.created_at, fallback_time);
    let updated_at = parse_datetime_or(&value.updated_at, fallback_time);

    Ok(WorkflowConfiguration {
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
