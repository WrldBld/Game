//! Workflow helper functions for HTTP handlers
//!
//! These functions handle workflow preparation, export, and import.
//! They are orchestration functions that live in the adapters layer.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rand::Rng;

use wrldbldr_domain::entities::{InputDefault, PromptMappingType, WorkflowConfiguration};
use wrldbldr_protocol::WorkflowConfigExportDto;

use crate::infrastructure::dto_conversions::{
    workflow_config_from_export_dto, workflow_config_to_export_dto,
};

/// Prepare a workflow for execution by applying prompt and overrides
///
/// Returns a modified copy of the workflow with:
/// - Prompts injected into mapped text fields
/// - Default values applied
/// - Override values applied
/// - Random seed generated (if seed input exists)
pub fn prepare_workflow(
    config: &WorkflowConfiguration,
    prompt: &str,
    negative_prompt: Option<&str>,
    overrides: &[InputDefault],
) -> Result<serde_json::Value> {
    let mut workflow = config.workflow_json.clone();

    // Apply prompt mappings
    for mapping in &config.prompt_mappings {
        let text = match mapping.mapping_type {
            PromptMappingType::Primary => prompt.to_string(),
            PromptMappingType::Negative => negative_prompt.unwrap_or("").to_string(),
        };

        set_input(
            &mut workflow,
            &mapping.node_id,
            &mapping.input_name,
            text.into(),
        )?;
    }

    // Apply defaults (for inputs not in overrides)
    for default in &config.input_defaults {
        let is_overridden = overrides
            .iter()
            .any(|o| o.node_id == default.node_id && o.input_name == default.input_name);

        if !is_overridden {
            set_input(
                &mut workflow,
                &default.node_id,
                &default.input_name,
                default.default_value.clone(),
            )?;
        }
    }

    // Apply overrides
    for override_val in overrides {
        set_input(
            &mut workflow,
            &override_val.node_id,
            &override_val.input_name,
            override_val.default_value.clone(),
        )?;
    }

    // Randomize seed inputs
    randomize_seeds(&mut workflow);

    Ok(workflow)
}

/// Set an input value in the workflow
fn set_input(
    workflow: &mut serde_json::Value,
    node_id: &str,
    input_name: &str,
    value: serde_json::Value,
) -> Result<()> {
    let node = workflow
        .get_mut(node_id)
        .ok_or_else(|| anyhow!("Node '{}' not found in workflow", node_id))?;

    let inputs = node
        .get_mut("inputs")
        .ok_or_else(|| anyhow!("Node '{}' has no inputs", node_id))?;

    inputs[input_name] = value;
    Ok(())
}

/// Randomize all seed inputs in the workflow
///
/// ComfyUI won't generate new images if the seed hasn't changed,
/// so we randomize seeds to ensure unique outputs.
fn randomize_seeds(workflow: &mut serde_json::Value) {
    let mut rng = rand::thread_rng();

    if let Some(nodes) = workflow.as_object_mut() {
        for (_node_id, node) in nodes {
            if let Some(inputs) = node.get_mut("inputs").and_then(|i| i.as_object_mut()) {
                // Look for common seed input names
                for seed_name in ["seed", "noise_seed", "random_seed"] {
                    if inputs.contains_key(seed_name) {
                        // Generate a random i64 seed
                        let new_seed: i64 = rng.gen();
                        inputs.insert(
                            seed_name.to_string(),
                            serde_json::Value::Number(new_seed.into()),
                        );
                    }
                }
            }
        }
    }
}

/// Export all workflow configurations to a single JSON for backup
///
/// # Arguments
/// * `configs` - The workflow configurations to export
/// * `exported_at` - The timestamp to record in the export
pub fn export_configs(
    configs: &[WorkflowConfiguration],
    exported_at: DateTime<Utc>,
) -> serde_json::Value {
    let exported: Vec<WorkflowConfigExportDto> = configs
        .iter()
        .cloned()
        .map(workflow_config_to_export_dto)
        .collect();
    serde_json::json!({
        "version": "1.0",
        "exported_at": exported_at.to_rfc3339(),
        "workflows": exported,
    })
}

/// Import workflow configurations from exported JSON
///
/// # Arguments
/// * `json` - The JSON value containing exported workflow configurations
/// * `fallback_time` - Fallback timestamp if datetime parsing fails (typically from ClockPort)
pub fn import_configs(
    json: &serde_json::Value,
    fallback_time: DateTime<Utc>,
) -> Result<Vec<WorkflowConfiguration>> {
    let workflows = json
        .get("workflows")
        .ok_or_else(|| anyhow!("Missing 'workflows' field in import data"))?;

    let dtos: Vec<WorkflowConfigExportDto> = serde_json::from_value(workflows.clone())
        .map_err(|e| anyhow!("Failed to parse workflow configurations: {}", e))?;

    let configs: Vec<WorkflowConfiguration> = dtos
        .into_iter()
        .map(|dto| workflow_config_from_export_dto(dto, fallback_time))
        .collect::<Result<Vec<_>>>()?;

    Ok(configs)
}
