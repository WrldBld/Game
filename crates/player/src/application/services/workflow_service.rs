//! Workflow Service - Application service for ComfyUI workflow configuration
//!
//! This service provides use case implementations for managing workflow slots,
//! uploading and configuring workflows, and testing workflow execution.

use serde::{Deserialize, Serialize};

use crate::ports::outbound::{ApiError, ApiPort};

/// Summary of a workflow slot with its configuration status
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkflowSlotStatus {
    pub slot: String,
    pub display_name: String,
    pub default_width: u32,
    pub default_height: u32,
    pub configured: bool,
    pub config: Option<WorkflowConfigBrief>,
}

/// Brief workflow configuration info
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkflowConfigBrief {
    pub name: String,
}

/// A category of workflow slots
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkflowSlotCategory {
    pub name: String,
    pub slots: Vec<WorkflowSlotStatus>,
}

/// Response from listing workflow slots
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowSlotsResponse {
    pub categories: Vec<WorkflowSlotCategory>,
}

/// ControlNet mapping for pose injection
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControlNetMapping {
    pub node_id: String,
    pub input_key: String,
    pub enabled: bool,
}

/// Full workflow configuration data
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub id: String,
    pub slot: String,
    pub slot_display_name: String,
    pub name: String,
    pub analysis: WorkflowAnalysis,
    pub prompt_mappings: Vec<PromptMapping>,
    pub input_defaults: Vec<InputDefault>,
    pub locked_inputs: Vec<String>,
    /// Model ID override for this workflow
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Prompt prefix override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_prefix: Option<String>,
    /// Prompt suffix override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_suffix: Option<String>,
    /// Negative prompt override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_prompt_override: Option<String>,
    /// Whether SCORE tokens are enabled for this workflow
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_tokens_enabled: Option<bool>,
    /// Pose asset ID for ControlNet
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controlnet_pose_asset_id: Option<String>,
    /// ControlNet node mappings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controlnet_mappings: Option<Vec<ControlNetMapping>>,
    pub created_at: String,
    pub updated_at: String,
}

/// Workflow analysis data
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkflowAnalysis {
    pub node_count: usize,
    pub inputs: Vec<WorkflowInput>,
    pub text_inputs: Vec<WorkflowInput>,
}

/// Workflow input information
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub node_id: String,
    pub node_type: String,
    pub node_title: Option<String>,
    pub input_name: String,
    pub input_type: String,
    pub current_value: serde_json::Value,
}

/// Prompt mapping configuration
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PromptMapping {
    pub node_id: String,
    pub input_name: String,
    pub mapping_type: String, // "primary" or "negative"
}

/// Input default value
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputDefault {
    pub node_id: String,
    pub input_name: String,
    pub default_value: serde_json::Value,
}

/// Workflow analysis result from analyze endpoint
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeWorkflowResponse {
    pub is_valid: bool,
    pub analysis: WorkflowAnalysis,
    pub suggested_prompt_mappings: Vec<PromptMapping>,
    #[serde(default)]
    pub errors: Vec<String>,
}

/// Request to save a workflow configuration
#[derive(Clone, Debug, Serialize)]
pub struct SaveWorkflowRequest {
    pub name: String,
    pub workflow_json: serde_json::Value,
    pub prompt_mappings: Vec<serde_json::Value>,
    pub input_defaults: Vec<InputDefault>,
    pub locked_inputs: Vec<String>,
}

/// Request to test a workflow
#[derive(Clone, Debug, Serialize)]
pub struct TestWorkflowRequest {
    pub prompt: String,
}

/// Response from workflow test
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct TestWorkflowResponse {
    pub image_url: String,
    #[serde(default)]
    pub duration_ms: u64,
}

/// Workflow service for managing ComfyUI workflows
///
/// This service provides methods for workflow-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct WorkflowService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> WorkflowService<A> {
    /// Create a new WorkflowService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// List all workflow slots organized by category
    pub async fn list_workflows(&self) -> Result<WorkflowSlotsResponse, ApiError> {
        self.api.get("/api/workflows").await
    }

    /// Get workflow configuration for a specific slot
    ///
    /// Returns None if the slot is not configured
    pub async fn get_workflow_config(
        &self,
        slot_id: &str,
    ) -> Result<Option<WorkflowConfig>, ApiError> {
        let path = format!("/api/workflows/{}", slot_id);
        self.api.get_optional(&path).await
    }

    /// Save workflow configuration to a slot
    ///
    /// # Arguments
    /// * `slot_id` - The slot identifier
    /// * `name` - Workflow name
    /// * `workflow_json` - The ComfyUI workflow JSON
    /// * `prompt_mappings` - List of prompt mapping configurations
    /// * `input_defaults` - List of input default values
    /// * `locked_inputs` - List of locked input identifiers
    pub async fn save_workflow_config(
        &self,
        slot_id: &str,
        name: &str,
        workflow_json: serde_json::Value,
        prompt_mappings: Vec<serde_json::Value>,
        input_defaults: Vec<InputDefault>,
        locked_inputs: Vec<String>,
    ) -> Result<(), ApiError> {
        let path = format!("/api/workflows/{}", slot_id);
        let request = SaveWorkflowRequest {
            name: name.to_string(),
            workflow_json,
            prompt_mappings,
            input_defaults,
            locked_inputs,
        };
        self.api.post_no_response(&path, &request).await
    }

    /// Delete workflow configuration from a slot
    pub async fn delete_workflow_config(&self, slot_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/workflows/{}", slot_id);
        self.api.delete(&path).await
    }

    /// Test a workflow with a prompt
    ///
    /// # Arguments
    /// * `slot_id` - The slot identifier
    /// * `prompt` - Test prompt to use
    ///
    /// # Returns
    /// Test result with generated image URL and duration
    pub async fn test_workflow(
        &self,
        slot_id: &str,
        prompt: &str,
    ) -> Result<TestWorkflowResponse, ApiError> {
        let path = format!("/api/workflows/{}/test", slot_id);
        let body = TestWorkflowRequest {
            prompt: prompt.to_string(),
        };
        self.api.post(&path, &body).await
    }

    /// Analyze a workflow JSON to extract inputs and suggest mappings
    ///
    /// # Arguments
    /// * `workflow_json` - The ComfyUI workflow JSON to analyze
    ///
    /// # Returns
    /// Analysis result with node count, inputs, and suggested prompt mappings
    pub async fn analyze_workflow(
        &self,
        workflow_json: serde_json::Value,
    ) -> Result<AnalyzeWorkflowResponse, ApiError> {
        let body = serde_json::json!({ "workflow_json": workflow_json });
        self.api.post("/api/workflows/analyze", &body).await
    }

    /// Update just the defaults of a workflow (without re-uploading the workflow JSON)
    ///
    /// # Arguments
    /// * `slot_id` - The slot identifier
    /// * `input_defaults` - New default values for inputs
    /// * `locked_inputs` - Optional list of locked input identifiers
    /// * `model_id` - Optional model ID override
    /// * `prompt_prefix` - Optional prompt prefix override
    /// * `prompt_suffix` - Optional prompt suffix override
    /// * `negative_prompt_override` - Optional negative prompt override
    /// * `score_tokens_enabled` - Optional SCORE tokens toggle
    /// * `controlnet_pose_asset_id` - Optional pose asset ID for ControlNet
    /// * `controlnet_mappings` - Optional ControlNet node mappings
    #[allow(clippy::too_many_arguments)]
    pub async fn update_workflow_defaults(
        &self,
        slot_id: &str,
        input_defaults: Vec<InputDefault>,
        locked_inputs: Option<Vec<String>>,
        model_id: Option<String>,
        prompt_prefix: Option<String>,
        prompt_suffix: Option<String>,
        negative_prompt_override: Option<String>,
        score_tokens_enabled: Option<bool>,
        controlnet_pose_asset_id: Option<String>,
        controlnet_mappings: Option<Vec<ControlNetMapping>>,
    ) -> Result<WorkflowConfig, ApiError> {
        let path = format!("/api/workflows/{}/defaults", slot_id);
        let body = serde_json::json!({
            "input_defaults": input_defaults,
            "locked_inputs": locked_inputs,
            "model_id": model_id,
            "prompt_prefix": prompt_prefix,
            "prompt_suffix": prompt_suffix,
            "negative_prompt_override": negative_prompt_override,
            "score_tokens_enabled": score_tokens_enabled,
            "controlnet_pose_asset_id": controlnet_pose_asset_id,
            "controlnet_mappings": controlnet_mappings,
        });
        self.api.patch(&path, &body).await
    }
}

impl<A: ApiPort + Clone> Clone for WorkflowService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}
