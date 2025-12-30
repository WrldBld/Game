//! Workflow Service - Parsing and managing ComfyUI workflow configurations
//!
//! This service handles:
//! - Parsing ComfyUI API format workflow JSON
//! - Extracting configurable inputs from workflows
//! - Applying prompts and overrides to workflows
//! - Managing workflow configurations
//!
//! Note: Core workflow analysis functions are now in `wrldbldr_domain_types`.
//! This module provides thin wrappers for backwards compatibility.

use anyhow::{anyhow, Result};
use std::sync::Arc;

use crate::application::dto::{workflow_config_from_export_dto, workflow_config_to_export_dto};
use wrldbldr_domain::entities::{InputDefault, PromptMappingType, WorkflowConfiguration};
use wrldbldr_domain_types::{PromptMapping, WorkflowAnalysis};
use wrldbldr_engine_ports::outbound::RandomPort;
use wrldbldr_protocol::WorkflowConfigExportDto;

/// Service for working with ComfyUI workflows
pub struct WorkflowService {
    random_port: Arc<dyn RandomPort>,
}

impl WorkflowService {
    /// Create a new WorkflowService with the given random port.
    pub fn new(random_port: Arc<dyn RandomPort>) -> Self {
        Self { random_port }
    }
}

impl WorkflowService {
    /// Analyze a ComfyUI API format workflow JSON
    ///
    /// Extracts all configurable inputs (non-connection values) from the workflow.
    /// Delegates to `wrldbldr_domain_types::analyze_workflow`.
    pub fn analyze_workflow(workflow_json: &serde_json::Value) -> WorkflowAnalysis {
        wrldbldr_domain_types::analyze_workflow(workflow_json)
    }

    /// Validate a workflow JSON is in ComfyUI API format
    ///
    /// Delegates to `wrldbldr_domain_types::validate_workflow`.
    pub fn validate_workflow(workflow_json: &serde_json::Value) -> Result<()> {
        wrldbldr_domain_types::validate_workflow(workflow_json).map_err(|e| anyhow!(e))
    }

    /// Prepare a workflow for execution by applying prompt and overrides
    ///
    /// Returns a modified copy of the workflow with:
    /// - Prompts injected into mapped text fields
    /// - Default values applied
    /// - Override values applied
    /// - Random seed generated (if seed input exists)
    pub fn prepare_workflow(
        &self,
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

            Self::set_input(
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
                Self::set_input(
                    &mut workflow,
                    &default.node_id,
                    &default.input_name,
                    default.default_value.clone(),
                )?;
            }
        }

        // Apply overrides
        for override_val in overrides {
            Self::set_input(
                &mut workflow,
                &override_val.node_id,
                &override_val.input_name,
                override_val.default_value.clone(),
            )?;
        }

        // Randomize seed inputs
        self.randomize_seeds(&mut workflow);

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
    fn randomize_seeds(&self, workflow: &mut serde_json::Value) {
        if let Some(nodes) = workflow.as_object_mut() {
            for (_node_id, node) in nodes {
                if let Some(inputs) = node.get_mut("inputs").and_then(|i| i.as_object_mut()) {
                    // Look for common seed input names
                    for seed_name in ["seed", "noise_seed", "random_seed"] {
                        if inputs.contains_key(seed_name) {
                            // Generate a random i64 seed
                            let new_seed: i64 = self.random_port.random_i64();
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

    /// Find common node types in a workflow
    ///
    /// Delegates to `wrldbldr_domain_types::find_nodes_by_type`.
    pub fn find_nodes_by_type(
        workflow: &serde_json::Value,
        class_type: &str,
    ) -> Vec<(String, serde_json::Value)> {
        wrldbldr_domain_types::find_nodes_by_type(workflow, class_type)
    }

    /// Auto-detect prompt mappings from common node types
    ///
    /// Delegates to `wrldbldr_domain_types::auto_detect_prompt_mappings`.
    pub fn auto_detect_prompt_mappings(workflow: &serde_json::Value) -> Vec<PromptMapping> {
        wrldbldr_domain_types::auto_detect_prompt_mappings(workflow)
    }

    /// Export all workflow configurations to a single JSON for backup
    ///
    /// # Arguments
    /// * `configs` - The workflow configurations to export
    /// * `exported_at` - The timestamp to record in the export (use clock.now() from caller)
    pub fn export_configs(
        configs: &[WorkflowConfiguration],
        exported_at: chrono::DateTime<chrono::Utc>,
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
    pub fn import_configs(json: &serde_json::Value) -> Result<Vec<WorkflowConfiguration>> {
        let workflows = json
            .get("workflows")
            .ok_or_else(|| anyhow!("Missing 'workflows' field in import data"))?;

        let dtos: Vec<WorkflowConfigExportDto> = serde_json::from_value(workflows.clone())
            .map_err(|e| anyhow!("Failed to parse workflow configurations: {}", e))?;

        let configs: Vec<WorkflowConfiguration> = dtos
            .into_iter()
            .map(workflow_config_from_export_dto)
            .collect::<Result<Vec<_>>>()?;

        Ok(configs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain_types::InputType;

    fn sample_workflow() -> serde_json::Value {
        serde_json::json!({
            "3": {
                "class_type": "KSampler",
                "_meta": {"title": "KSampler"},
                "inputs": {
                    "seed": 12345,
                    "steps": 20,
                    "cfg": 7.5,
                    "sampler_name": "euler_ancestral",
                    "scheduler": "normal",
                    "denoise": 1.0,
                    "model": ["4", 0],
                    "positive": ["6", 0],
                    "negative": ["7", 0],
                    "latent_image": ["5", 0]
                }
            },
            "5": {
                "class_type": "EmptyLatentImage",
                "_meta": {"title": "Empty Latent Image"},
                "inputs": {
                    "width": 512,
                    "height": 512,
                    "batch_size": 1
                }
            },
            "6": {
                "class_type": "CLIPTextEncode",
                "_meta": {"title": "Positive Prompt"},
                "inputs": {
                    "text": "a beautiful landscape",
                    "clip": ["4", 1]
                }
            },
            "7": {
                "class_type": "CLIPTextEncode",
                "_meta": {"title": "Negative Prompt"},
                "inputs": {
                    "text": "ugly, blurry",
                    "clip": ["4", 1]
                }
            }
        })
    }

    #[test]
    fn test_analyze_workflow() {
        let workflow = sample_workflow();
        let analysis = WorkflowService::analyze_workflow(&workflow);

        assert_eq!(analysis.node_count, 4);
        assert!(analysis.errors.is_empty());

        // Should have extracted non-connection inputs
        assert!(analysis.inputs.len() > 0);

        // Should have found prompt text inputs (positive and negative prompts)
        let prompt_text_inputs: Vec<_> = analysis
            .text_inputs
            .iter()
            .filter(|i| i.node_type == "CLIPTextEncode" && i.input_name == "text")
            .collect();
        assert_eq!(prompt_text_inputs.len(), 2);

        // Should not include connection inputs (arrays)
        assert!(!analysis
            .inputs
            .iter()
            .any(|i| i.input_name == "model" || i.input_name == "positive"));
    }

    #[test]
    fn test_validate_workflow() {
        let workflow = sample_workflow();
        assert!(WorkflowService::validate_workflow(&workflow).is_ok());

        // Invalid: not an object
        let invalid = serde_json::json!([1, 2, 3]);
        assert!(WorkflowService::validate_workflow(&invalid).is_err());

        // Invalid: empty object
        let empty = serde_json::json!({});
        assert!(WorkflowService::validate_workflow(&empty).is_err());
    }

    #[test]
    fn test_auto_detect_prompt_mappings() {
        let workflow = sample_workflow();
        let mappings = WorkflowService::auto_detect_prompt_mappings(&workflow);

        assert_eq!(mappings.len(), 2);

        // Should have detected primary prompt
        assert!(mappings
            .iter()
            .any(|m| m.mapping_type == PromptMappingType::Primary));

        // Should have detected negative prompt
        assert!(mappings
            .iter()
            .any(|m| m.mapping_type == PromptMappingType::Negative));
    }

    #[test]
    fn test_input_type_detection() {
        assert_eq!(
            InputType::from_value(&serde_json::json!("hello")),
            InputType::Text
        );
        assert_eq!(
            InputType::from_value(&serde_json::json!(42)),
            InputType::Integer
        );
        assert_eq!(
            InputType::from_value(&serde_json::json!(3.14)),
            InputType::Float
        );
        assert_eq!(
            InputType::from_value(&serde_json::json!(true)),
            InputType::Boolean
        );
    }
}
