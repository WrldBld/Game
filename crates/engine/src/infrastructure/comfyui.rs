//! ComfyUI image generation client
//!
//! Implements the ImageGenPort trait for AI asset generation using ComfyUI's API.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

use crate::infrastructure::ports::{ImageGenError, ImageGenPort, ImageRequest, ImageResult};

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
        const MAX_ATTEMPTS: u32 = 120; // 2 minutes with 1 second intervals
        const POLL_INTERVAL: Duration = Duration::from_secs(1);

        for _ in 0..MAX_ATTEMPTS {
            let history = self.get_history(prompt_id).await?;

            if let Some(prompt_history) = history.prompts.get(prompt_id) {
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

        Err(ImageGenError::GenerationFailed(
            "Generation timed out".to_string(),
        ))
    }

    /// Build a simple workflow for image generation
    fn build_workflow(request: &ImageRequest) -> serde_json::Value {
        // This is a simplified workflow template
        // In production, you'd load workflow JSON from files based on request.workflow
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
                    "ckpt_name": "v1-5-pruned-emaonly.ckpt"
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
                    "text": request.prompt,
                    "clip": ["4", 1]
                },
                "class_type": "CLIPTextEncode"
            },
            "7": {
                "inputs": {
                    "text": "bad quality, blurry, ugly",
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
}

#[async_trait]
impl ImageGenPort for ComfyUIClient {
    async fn generate(&self, request: ImageRequest) -> Result<ImageResult, ImageGenError> {
        // Build workflow from request
        let workflow = Self::build_workflow(&request);

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
            .map_err(|_| ImageGenError::Unavailable)?;

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
