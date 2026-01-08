//! Workflow Configuration Entity
//!
//! Stores ComfyUI workflow configurations for each asset generation slot.
//! Includes the workflow JSON, prompt mappings, and default values.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::WorkflowConfigId;

// Re-export shared workflow types from types module
pub use crate::types::{
    InputDefault, InputType, PromptMapping, PromptMappingType, WorkflowAnalysis, WorkflowInput,
    WorkflowSlot,
};

/// Workflow configuration for a specific asset generation slot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowConfiguration {
    pub id: WorkflowConfigId,
    /// The slot this workflow is configured for
    pub slot: WorkflowSlot,
    /// User-friendly name for this workflow
    pub name: String,
    /// The raw ComfyUI API workflow JSON
    pub workflow_json: serde_json::Value,
    /// Which text inputs should receive the generation prompt
    pub prompt_mappings: Vec<PromptMapping>,
    /// Default values for workflow inputs
    pub input_defaults: Vec<InputDefault>,
    /// Input paths that should always use defaults (never shown in UI)
    pub locked_inputs: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkflowConfiguration {
    /// Create a new workflow configuration
    pub fn new(
        slot: WorkflowSlot,
        name: impl Into<String>,
        workflow_json: serde_json::Value,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: WorkflowConfigId::new(),
            slot,
            name: name.into(),
            workflow_json,
            prompt_mappings: Vec::new(),
            input_defaults: Vec::new(),
            locked_inputs: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a prompt mapping
    pub fn with_prompt_mapping(mut self, mapping: PromptMapping) -> Self {
        self.prompt_mappings.push(mapping);
        self
    }

    /// Add a default value for an input
    pub fn with_default(mut self, default: InputDefault) -> Self {
        self.input_defaults.push(default);
        self
    }

    /// Lock an input (always use default, hide from UI)
    pub fn with_locked_input(mut self, input_path: impl Into<String>) -> Self {
        self.locked_inputs.push(input_path.into());
        self
    }

    /// Get the default value for a specific input, if set
    pub fn get_default(&self, node_id: &str, input_name: &str) -> Option<&serde_json::Value> {
        self.input_defaults
            .iter()
            .find(|d| d.node_id == node_id && d.input_name == input_name)
            .map(|d| &d.default_value)
    }

    /// Check if an input is locked
    pub fn is_locked(&self, node_id: &str, input_name: &str) -> bool {
        let path = format!("{}.{}", node_id, input_name);
        self.locked_inputs.contains(&path)
    }

    /// Get the primary prompt mapping
    pub fn primary_prompt_mapping(&self) -> Option<&PromptMapping> {
        self.prompt_mappings
            .iter()
            .find(|m| m.mapping_type == PromptMappingType::Primary)
    }

    /// Get the negative prompt mapping
    pub fn negative_prompt_mapping(&self) -> Option<&PromptMapping> {
        self.prompt_mappings
            .iter()
            .find(|m| m.mapping_type == PromptMappingType::Negative)
    }

    /// Update the workflow JSON
    pub fn update_workflow(&mut self, workflow_json: serde_json::Value, now: DateTime<Utc>) {
        self.workflow_json = workflow_json;
        self.updated_at = now;
    }

    /// Update prompt mappings
    pub fn set_prompt_mappings(&mut self, mappings: Vec<PromptMapping>, now: DateTime<Utc>) {
        self.prompt_mappings = mappings;
        self.updated_at = now;
    }

    /// Update input defaults
    pub fn set_input_defaults(&mut self, defaults: Vec<InputDefault>, now: DateTime<Utc>) {
        self.input_defaults = defaults;
        self.updated_at = now;
    }

    /// Update locked inputs
    pub fn set_locked_inputs(&mut self, locked: Vec<String>, now: DateTime<Utc>) {
        self.locked_inputs = locked;
        self.updated_at = now;
    }
}
