//! Workflow service port - Interface for workflow configuration operations
//!
//! This port abstracts workflow configuration business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use wrldbldr_domain::entities::{
    InputDefault, PromptMapping, WorkflowAnalysis, WorkflowConfiguration, WorkflowSlot,
};
use wrldbldr_domain::{WorkflowConfigId, WorldId};

/// Port for workflow service operations
///
/// This trait defines the application use cases for workflow configuration
/// management, including listing, retrieving, saving, deleting, and finding
/// active workflows.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait WorkflowServicePort: Send + Sync {
    /// Get a workflow configuration by ID
    async fn get_workflow(&self, id: WorkflowConfigId) -> Result<Option<WorkflowConfiguration>>;

    /// List all workflow configurations
    async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>>;

    /// List all workflow configurations for a slot
    async fn list_by_slot(&self, slot: WorkflowSlot) -> Result<Vec<WorkflowConfiguration>>;

    /// Get a workflow configuration by slot
    ///
    /// Returns the workflow configuration for the given slot, if one exists.
    async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>>;

    /// Save a workflow configuration
    ///
    /// Creates a new configuration or updates an existing one based on the slot.
    async fn save(&self, config: &WorkflowConfiguration) -> Result<()>;

    /// Delete a workflow configuration by slot
    ///
    /// Returns true if a configuration was deleted, false if none existed.
    async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool>;

    /// Get the active workflow configuration for a world and slot
    ///
    /// Returns the configured workflow for the given slot, falling back to
    /// a default configuration if none is explicitly set for the world.
    async fn get_active_for_slot(
        &self,
        world_id: WorldId,
        slot: WorkflowSlot,
    ) -> Result<Option<WorkflowConfiguration>>;
}

// ============================================================================
// Workflow Utility Functions
// ============================================================================

/// Analyze a ComfyUI API format workflow JSON
///
/// Extracts all configurable inputs (non-connection values) from the workflow.
pub fn analyze_workflow(workflow_json: &serde_json::Value) -> WorkflowAnalysis {
    use wrldbldr_domain::entities::{InputType, WorkflowInput};

    let mut inputs = Vec::new();
    let mut text_inputs = Vec::new();
    let mut errors = Vec::new();
    let mut node_count = 0;

    let nodes = match workflow_json.as_object() {
        Some(nodes) => nodes,
        None => {
            errors.push("Workflow JSON must be an object with node IDs as keys".to_string());
            return WorkflowAnalysis {
                node_count: 0,
                inputs,
                text_inputs,
                errors,
            };
        }
    };

    for (node_id, node) in nodes {
        node_count += 1;

        let class_type = node
            .get("class_type")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let node_title = node
            .get("_meta")
            .and_then(|m| m.get("title"))
            .and_then(|t| t.as_str())
            .map(String::from);

        let node_inputs = match node.get("inputs").and_then(|v| v.as_object()) {
            Some(inputs) => inputs,
            None => continue,
        };

        for (input_name, value) in node_inputs {
            if value.is_array() {
                continue;
            }

            let input_type = InputType::from_value(value);

            let workflow_input = WorkflowInput {
                node_id: node_id.clone(),
                node_type: class_type.clone(),
                node_title: node_title.clone(),
                input_name: input_name.clone(),
                input_type: input_type.clone(),
                current_value: value.clone(),
            };

            if input_type == InputType::Text {
                text_inputs.push(workflow_input.clone());
            }

            inputs.push(workflow_input);
        }
    }

    inputs.sort_by(|a, b| a.node_id.cmp(&b.node_id).then(a.input_name.cmp(&b.input_name)));
    text_inputs.sort_by(|a, b| a.node_id.cmp(&b.node_id).then(a.input_name.cmp(&b.input_name)));

    WorkflowAnalysis {
        node_count,
        inputs,
        text_inputs,
        errors,
    }
}

/// Validate a workflow JSON is in ComfyUI API format
pub fn validate_workflow(workflow_json: &serde_json::Value) -> Result<()> {
    use anyhow::anyhow;

    let nodes = workflow_json
        .as_object()
        .ok_or_else(|| anyhow!("Workflow must be a JSON object"))?;

    if nodes.is_empty() {
        return Err(anyhow!("Workflow has no nodes"));
    }

    let mut valid_nodes = 0;
    for (_node_id, node) in nodes {
        if !node.is_object() {
            continue;
        }
        if node.get("class_type").is_some() {
            valid_nodes += 1;
        }
    }

    if valid_nodes == 0 {
        return Err(anyhow!(
            "No valid ComfyUI nodes found. Make sure you're using the API format"
        ));
    }

    Ok(())
}

/// Prepare a workflow for execution by applying prompt and overrides
pub fn prepare_workflow(
    config: &WorkflowConfiguration,
    prompt: &str,
    negative_prompt: Option<&str>,
    overrides: &[InputDefault],
) -> Result<serde_json::Value> {
    use anyhow::anyhow;
    use rand::Rng;
    use wrldbldr_domain::entities::PromptMappingType;

    let mut workflow = config.workflow_json.clone();

    for mapping in &config.prompt_mappings {
        let text = match mapping.mapping_type {
            PromptMappingType::Primary => prompt.to_string(),
            PromptMappingType::Negative => negative_prompt.unwrap_or("").to_string(),
        };

        set_workflow_input(&mut workflow, &mapping.node_id, &mapping.input_name, text.into())?;
    }

    for default in &config.input_defaults {
        let is_overridden = overrides
            .iter()
            .any(|o| o.node_id == default.node_id && o.input_name == default.input_name);

        if !is_overridden {
            set_workflow_input(
                &mut workflow,
                &default.node_id,
                &default.input_name,
                default.default_value.clone(),
            )?;
        }
    }

    for override_val in overrides {
        set_workflow_input(
            &mut workflow,
            &override_val.node_id,
            &override_val.input_name,
            override_val.default_value.clone(),
        )?;
    }

    if let Some(nodes) = workflow.as_object_mut() {
        let mut rng = rand::thread_rng();
        for (_node_id, node) in nodes {
            if let Some(inputs) = node.get_mut("inputs").and_then(|i| i.as_object_mut()) {
                for seed_name in ["seed", "noise_seed", "random_seed"] {
                    if inputs.contains_key(seed_name) {
                        let new_seed: i64 = rng.gen();
                        inputs.insert(seed_name.to_string(), serde_json::Value::Number(new_seed.into()));
                    }
                }
            }
        }
    }

    Ok(workflow)
}

fn set_workflow_input(
    workflow: &mut serde_json::Value,
    node_id: &str,
    input_name: &str,
    value: serde_json::Value,
) -> Result<()> {
    use anyhow::anyhow;

    let node = workflow
        .get_mut(node_id)
        .ok_or_else(|| anyhow!("Node '{}' not found in workflow", node_id))?;

    let inputs = node
        .get_mut("inputs")
        .ok_or_else(|| anyhow!("Node '{}' has no inputs", node_id))?;

    inputs[input_name] = value;
    Ok(())
}

/// Auto-detect prompt mappings from common node types
pub fn auto_detect_prompt_mappings(workflow: &serde_json::Value) -> Vec<PromptMapping> {
    use wrldbldr_domain::entities::PromptMappingType;

    let mut mappings = Vec::new();
    let clip_nodes = find_nodes_by_type(workflow, "CLIPTextEncode");

    for (node_id, node) in clip_nodes {
        let title = node
            .get("_meta")
            .and_then(|m| m.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        let is_negative = title.to_lowercase().contains("negative") || title.to_lowercase().contains("neg");

        let mapping_type = if is_negative {
            PromptMappingType::Negative
        } else {
            if mappings.iter().any(|m: &PromptMapping| m.mapping_type == PromptMappingType::Primary) {
                continue;
            }
            PromptMappingType::Primary
        };

        mappings.push(PromptMapping {
            node_id: node_id.clone(),
            input_name: "text".to_string(),
            mapping_type,
        });
    }

    mappings
}

fn find_nodes_by_type(workflow: &serde_json::Value, class_type: &str) -> Vec<(String, serde_json::Value)> {
    let mut found = Vec::new();

    if let Some(nodes) = workflow.as_object() {
        for (node_id, node) in nodes {
            if let Some(ct) = node.get("class_type").and_then(|v| v.as_str()) {
                if ct == class_type {
                    found.push((node_id.clone(), node.clone()));
                }
            }
        }
    }

    found
}

/// Export all workflow configurations to a single JSON for backup
pub fn export_workflow_configs(configs: &[WorkflowConfiguration], exported_at: DateTime<Utc>) -> serde_json::Value {
    use wrldbldr_protocol::WorkflowConfigExportDto;

    let exported: Vec<WorkflowConfigExportDto> = configs.iter().cloned().map(Into::into).collect();
    serde_json::json!({
        "version": "1.0",
        "exported_at": exported_at.to_rfc3339(),
        "workflows": exported,
    })
}

/// Import workflow configurations from exported JSON
pub fn import_workflow_configs(json: &serde_json::Value) -> Result<Vec<WorkflowConfiguration>> {
    use anyhow::anyhow;
    use wrldbldr_protocol::WorkflowConfigExportDto;

    let workflows = json
        .get("workflows")
        .ok_or_else(|| anyhow!("Missing 'workflows' field in import data"))?;

    let dtos: Vec<WorkflowConfigExportDto> = serde_json::from_value(workflows.clone())
        .map_err(|e| anyhow!("Failed to parse workflow configurations: {}", e))?;

    let configs: Vec<WorkflowConfiguration> = dtos
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>>>()?;

    Ok(configs)
}
