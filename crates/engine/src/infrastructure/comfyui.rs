// ComfyUI client - prepared for future image generation integration
#![allow(dead_code)]

//! ComfyUI image generation client
//!
//! Implements the ImageGenPort trait for AI asset generation using ComfyUI's API.
//!
//! ## Workflow System
//!
//! Workflows are stored as JSON files in `resources/workflows/` and embedded at compile time
//! using `include_str!`. This ensures stability and avoids runtime file system dependencies.
//!
//! ### Available Workflows
//!
//! - `backdrop_v1`: Landscape-oriented backdrop generation (1024x768)
//!   - Node 6: Positive prompt (prepends SCORE_9, SCORE_8, SCORE_8_UP, SCORE_7 if missing)
//!   - Node 7: Negative prompt (uses default from workflow)
//!   - Node 5: Width/Height (overridden if provided in request)
//!   - Node 3: KSampler seed (randomized per request)
//!
//! - `portrait_v1`: Portrait-oriented character generation (768x1024)
//!   - Same node structure as backdrop_v1, different default dimensions
//!   - Intended for NPC portraits with optional ControlNet integration
//!
//! ### Request Processing
//!
//! When a workflow ID is specified in `ImageRequest.workflow`:
//! 1. Load the embedded workflow JSON
//! 2. Inject the request prompt into node 6
//! 3. Randomize the seed in node 3
//! 4. Override width/height in node 5 if request provides non-zero values
//! 5. Queue the prompt and poll for completion
//!
//! If the workflow ID is unknown, returns `ImageGenError::GenerationFailed` with a clear message.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

use crate::infrastructure::ports::{ImageGenError, ImageGenPort, ImageRequest, ImageResult};

/// Embedded workflow JSON for backdrop generation (landscape, 1024x768).
const BACKDROP_V1_WORKFLOW: &str = include_str!("../../resources/workflows/backdrop_v1.json");

/// Embedded workflow JSON for portrait generation (portrait, 768x1024).
const PORTRAIT_V1_WORKFLOW: &str = include_str!("../../resources/workflows/portrait_v1.json");

/// Default negative prompt used when none is provided.
const DEFAULT_NEGATIVE_PROMPT: &str = "text, watermark, people, crowd,";

/// Node ID for the positive prompt CLIPTextEncode node.
const NODE_POSITIVE_PROMPT: &str = "6";

/// Node ID for the negative prompt CLIPTextEncode node.
const NODE_NEGATIVE_PROMPT: &str = "7";

/// Node ID for the EmptyLatentImage node (dimensions).
const NODE_LATENT_IMAGE: &str = "5";

/// Node ID for the KSampler node (seed).
const NODE_KSAMPLER: &str = "3";

/// Client for ComfyUI API
#[derive(Clone)]
pub struct ComfyUIClient {
    client: Client,
    base_url: String,
}

impl ComfyUIClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minute timeout for generation
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Load a workflow by ID and prepare it for execution.
    ///
    /// # Arguments
    /// * `request` - The image generation request with all parameters
    ///
    /// # Returns
    /// The prepared workflow as a JSON value, ready for queuing.
    ///
    /// # Errors
    /// Returns `ImageGenError::GenerationFailed` if the workflow ID is unknown.
    fn load_and_prepare_workflow(
        &self,
        request: &ImageRequest,
    ) -> Result<serde_json::Value, ImageGenError> {
        // Load the base workflow JSON
        let workflow_json = match request.workflow.as_str() {
            "backdrop_v1" => BACKDROP_V1_WORKFLOW,
            "portrait_v1" => PORTRAIT_V1_WORKFLOW,
            _ => {
                // For custom workflow slots, try to use the request's workflow as a workflow JSON directly
                // This allows dynamic workflow configurations from the database
                return self.load_and_prepare_dynamic_workflow(request);
            }
        };

        // Parse the workflow
        let mut workflow: serde_json::Value = serde_json::from_str(workflow_json)
            .map_err(|e| ImageGenError::GenerationFailed(format!("Failed to parse workflow: {}", e)))?;

        // Ensure it's an object
        let workflow_obj = workflow
            .as_object_mut()
            .ok_or_else(|| ImageGenError::GenerationFailed("Workflow is not a JSON object".to_string()))?;

        // Build effective prompt with prefix/suffix and SCORE tokens
        let effective_prompt = request.effective_prompt();
        let formatted_prompt = if request.score_tokens_enabled {
            Self::format_prompt_with_score_tokens(&effective_prompt)
        } else {
            effective_prompt
        };
        Self::update_node_text(workflow_obj, NODE_POSITIVE_PROMPT, "text", &formatted_prompt)?;

        // Inject negative prompt if provided
        if let Some(ref negative) = request.negative_prompt {
            Self::update_node_text(workflow_obj, NODE_NEGATIVE_PROMPT, "text", negative)?;
        }

        // Apply model selection if provided (node 4 is the checkpoint loader)
        if let Some(ref model_id) = request.model_id {
            Self::update_node_input(workflow_obj, "4", "ckpt_name", model_id)?;
        }

        // Randomize seed in node 3
        let random_seed = rand::random::<u64>();
        Self::update_node_input(workflow_obj, NODE_KSAMPLER, "seed", random_seed)?;

        // Override width/height in node 5 if provided
        if request.width > 0 {
            Self::update_node_input(workflow_obj, NODE_LATENT_IMAGE, "width", request.width)?;
        }
        if request.height > 0 {
            Self::update_node_input(workflow_obj, NODE_LATENT_IMAGE, "height", request.height)?;
        }

        // Apply ControlNet pose injection if configured
        if let Some(ref mappings) = request.controlnet_mappings {
            for mapping in mappings {
                if mapping.enabled {
                    if let Some(ref pose_id) = request.pose_asset_id {
                        // Inject pose asset path into the mapped node input
                        let pose_path = format!("wrldbldr/poses/{}.png", pose_id);
                        Self::update_node_text(workflow_obj, &mapping.node_id, &mapping.input_key, &pose_path)?;
                    }
                }
            }
        }

        Ok(serde_json::Value::Object(workflow_obj.clone()))
    }

    /// Load and prepare a dynamic workflow from the request.
    ///
    /// This handles workflows that are passed as JSON directly (from database)
    /// rather than embedded workflows.
    fn load_and_prepare_dynamic_workflow(
        &self,
        request: &ImageRequest,
    ) -> Result<serde_json::Value, ImageGenError> {
        // For now, return an error indicating dynamic workflows need to be looked up
        // The actual implementation would fetch from WorkflowConfigRepo
        Err(ImageGenError::GenerationFailed(format!(
            "Unknown workflow ID: '{}'. Use 'backdrop_v1' or 'portrait_v1'.",
            request.workflow
        )))
    }

    /// Format a prompt with SCORE tokens for ComfyUI Pony/SDXL models.
    ///
    /// If the prompt already contains SCORE_9, returns it unchanged.
    /// Otherwise, prepends the standard SCORE token sequence.
    fn format_prompt_with_score_tokens(prompt: &str) -> String {
        let trimmed = prompt.trim();
        if trimmed.contains("SCORE_9") {
            trimmed.to_string()
        } else {
            format!("SCORE_9, SCORE_8, SCORE_8_UP, SCORE_7, {}", trimmed)
        }
    }

    /// Update a text field in a node's inputs.
    fn update_node_text(
        workflow: &mut serde_json::Map<String, serde_json::Value>,
        node_id: &str,
        field: &str,
        value: &str,
    ) -> Result<(), ImageGenError> {
        let node = workflow
            .get_mut(node_id)
            .ok_or_else(|| ImageGenError::GenerationFailed(format!("Node {} not found in workflow", node_id)))?;

        let inputs = node
            .get_mut("inputs")
            .and_then(|i| i.as_object_mut())
            .ok_or_else(|| ImageGenError::GenerationFailed(format!("Node {} has no inputs", node_id)))?;

        inputs.insert(field.to_string(), serde_json::Value::String(value.to_string()));
        Ok(())
    }

    /// Update a numeric field in a node's inputs.
    fn update_node_input<T: Serialize>(
        workflow: &mut serde_json::Map<String, serde_json::Value>,
        node_id: &str,
        field: &str,
        value: T,
    ) -> Result<(), ImageGenError> {
        let node = workflow
            .get_mut(node_id)
            .ok_or_else(|| ImageGenError::GenerationFailed(format!("Node {} not found in workflow", node_id)))?;

        let inputs = node
            .get_mut("inputs")
            .and_then(|i| i.as_object_mut())
            .ok_or_else(|| ImageGenError::GenerationFailed(format!("Node {} has no inputs", node_id)))?;

        let json_value = serde_json::to_value(value)
            .map_err(|e| ImageGenError::GenerationFailed(format!("Failed to serialize value: {}", e)))?;

        inputs.insert(field.to_string(), json_value);
        Ok(())
    }

    /// Build a simple fallback workflow for image generation.
    ///
    /// This is used when no specific workflow ID is provided, maintaining
    /// backward compatibility with existing code.
    fn build_fallback_workflow(request: &ImageRequest) -> serde_json::Value {
        // Build effective prompt with prefix/suffix
        let effective_prompt = request.effective_prompt();
        let formatted_prompt = if request.score_tokens_enabled {
            Self::format_prompt_with_score_tokens(&effective_prompt)
        } else {
            effective_prompt
        };

        // Use provided negative prompt or default
        let negative_prompt = request.negative_prompt.clone()
            .unwrap_or_else(|| DEFAULT_NEGATIVE_PROMPT.to_string());

        // Use provided model or default
        let model_name = request.model_id.clone()
            .unwrap_or_else(|| "v1-5-pruned-emaonly.ckpt".to_string());

        serde_json::json!({
            "3": {
                "inputs": {
                    "seed": rand::random::<u32>(),
                    "steps": 20,
                    "cfg": 8.0,
                    "sampler_name": "euler",
                    "scheduler": "normal",
                    "denoise": 1.0,
                    "model": ["4", 0],
                    "positive": ["6", 0],
                    "negative": ["7", 0],
                    "latent_image": ["5", 0]
                },
                "class_type": "KSampler"
            },
            "4": {
                "inputs": {
                    "ckpt_name": model_name
                },
                "class_type": "CheckpointLoaderSimple"
            },
            "5": {
                "inputs": {
                    "width": request.width,
                    "height": request.height,
                    "batch_size": 1
                },
                "class_type": "EmptyLatentImage"
            },
            "6": {
                "inputs": {
                    "text": formatted_prompt,
                    "clip": ["4", 1]
                },
                "class_type": "CLIPTextEncode"
            },
            "7": {
                "inputs": {
                    "text": negative_prompt,
                    "clip": ["4", 1]
                },
                "class_type": "CLIPTextEncode"
            },
            "8": {
                "inputs": {
                    "samples": ["3", 0],
                    "vae": ["4", 2]
                },
                "class_type": "VAEDecode"
            },
            "9": {
                "inputs": {
                    "filename_prefix": "wrldbldr",
                    "images": ["8", 0]
                },
                "class_type": "SaveImage"
            }
        })
    }

    /// Queue a workflow for execution
    async fn queue_prompt(
        &self,
        workflow: serde_json::Value,
    ) -> Result<QueueResponse, ImageGenError> {
        let client_id = uuid::Uuid::new_v4().to_string();
        let request = QueuePromptRequest {
            prompt: workflow,
            client_id,
        };

        let response = self
            .client
            .post(format!("{}/prompt", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| ImageGenError::GenerationFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ImageGenError::GenerationFailed(error_text));
        }

        response
            .json()
            .await
            .map_err(|e| ImageGenError::GenerationFailed(e.to_string()))
    }

    /// Get the history of a completed prompt
    async fn get_history(&self, prompt_id: &str) -> Result<HistoryResponse, ImageGenError> {
        let response = self
            .client
            .get(format!("{}/history/{}", self.base_url, prompt_id))
            .send()
            .await
            .map_err(|e| ImageGenError::GenerationFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ImageGenError::GenerationFailed(error_text));
        }

        response
            .json()
            .await
            .map_err(|e| ImageGenError::GenerationFailed(e.to_string()))
    }

    /// Download a generated image
    async fn get_image(
        &self,
        filename: &str,
        subfolder: &str,
        folder_type: &str,
    ) -> Result<Vec<u8>, ImageGenError> {
        let response = self
            .client
            .get(format!("{}/view", self.base_url))
            .query(&[
                ("filename", filename),
                ("subfolder", subfolder),
                ("type", folder_type),
            ])
            .send()
            .await
            .map_err(|e| ImageGenError::GenerationFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ImageGenError::GenerationFailed(error_text));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| ImageGenError::GenerationFailed(e.to_string()))
    }

    /// Wait for a prompt to complete and return the first image
    async fn wait_for_completion(&self, prompt_id: &str) -> Result<ImageOutput, ImageGenError> {
        let max_attempts = Self::get_poll_timeout_secs();
        const POLL_INTERVAL: Duration = Duration::from_secs(1);

        let mut prompt_appeared = false;

        for _ in 0..max_attempts {
            let history = self.get_history(prompt_id).await?;

            if let Some(prompt_history) = history.prompts.get(prompt_id) {
                prompt_appeared = true;
                if prompt_history.status.completed {
                    // Find the first image output
                    for output in prompt_history.outputs.values() {
                        if let Some(images) = &output.images {
                            if let Some(image) = images.first() {
                                return Ok(image.clone());
                            }
                        }
                    }
                    return Err(ImageGenError::GenerationFailed(
                        "No images in output".to_string(),
                    ));
                }
            }

            sleep(POLL_INTERVAL).await;
        }

        // Distinguish between "prompt never appeared" vs "prompt did not complete"
        if prompt_appeared {
            Err(ImageGenError::GenerationFailed(format!(
                "Prompt {} was queued but did not complete within {} seconds",
                prompt_id, max_attempts
            )))
        } else {
            Err(ImageGenError::GenerationFailed(format!(
                "Prompt {} never appeared in history within {} seconds",
                prompt_id, max_attempts
            )))
        }
    }

    /// Get the poll timeout in seconds from environment variable or use default.
    ///
    /// Environment variable: `COMFYUI_POLL_TIMEOUT_SECS`
    /// Default: 120 seconds
    fn get_poll_timeout_secs() -> u32 {
        std::env::var("COMFYUI_POLL_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(120)
    }
}

#[async_trait]
impl ImageGenPort for ComfyUIClient {
    async fn generate(&self, request: ImageRequest) -> Result<ImageResult, ImageGenError> {
        // Build workflow from request
        // If a specific workflow ID is provided and non-empty, use the workflow system
        let workflow = if !request.workflow.is_empty() && request.workflow != "default" {
            self.load_and_prepare_workflow(&request)?
        } else {
            Self::build_fallback_workflow(&request)
        };

        // Queue the prompt
        let queue_response = self.queue_prompt(workflow).await?;

        // Wait for completion
        let image_output = self.wait_for_completion(&queue_response.prompt_id).await?;

        // Download the image
        let image_data = self
            .get_image(
                &image_output.filename,
                &image_output.subfolder,
                &image_output.r#type,
            )
            .await?;

        // Determine format from filename
        let format = if image_output.filename.ends_with(".png") {
            "png"
        } else if image_output.filename.ends_with(".jpg")
            || image_output.filename.ends_with(".jpeg")
        {
            "jpeg"
        } else {
            "png"
        }
        .to_string();

        Ok(ImageResult { image_data, format })
    }

    async fn check_health(&self) -> Result<bool, ImageGenError> {
        let response = self
            .client
            .get(format!("{}/system_stats", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "ComfyUI health check failed");
                ImageGenError::Unavailable
            })?;

        Ok(response.status().is_success())
    }
}

// =============================================================================
// ComfyUI API types
// =============================================================================

#[derive(Debug, Serialize)]
struct QueuePromptRequest {
    prompt: serde_json::Value,
    client_id: String,
}

#[derive(Debug, Deserialize)]
struct QueueResponse {
    prompt_id: String,
    #[allow(dead_code)]
    number: u32,
}

#[derive(Debug, Deserialize)]
struct HistoryResponse {
    #[serde(flatten)]
    prompts: HashMap<String, PromptHistory>,
}

#[derive(Debug, Deserialize)]
struct PromptHistory {
    outputs: HashMap<String, NodeOutput>,
    status: PromptStatus,
}

#[derive(Debug, Deserialize)]
struct NodeOutput {
    images: Option<Vec<ImageOutput>>,
}

#[derive(Debug, Clone, Deserialize)]
struct ImageOutput {
    filename: String,
    subfolder: String,
    r#type: String,
}

#[derive(Debug, Deserialize)]
struct PromptStatus {
    #[allow(dead_code)]
    status_str: String,
    completed: bool,
}
