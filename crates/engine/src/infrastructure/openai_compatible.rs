// OpenAI-compatible client - constructor variants for future use
#![allow(dead_code)]

//! OpenAI-compatible LLM client
//!
//! Works with any OpenAI-compatible API server including:
//! - Ollama
//! - MLX LM server (serve-llm.sh)
//! - llama.cpp server
//! - vLLM
//! - LocalAI
//! - LM Studio

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::infrastructure::ports::{
    FinishReason, LlmError, LlmModelDescriptor, LlmModelsPort, LlmPort, LlmRequest, LlmResponse,
    MessageRole, TokenUsage,
};

/// Client for OpenAI-compatible API servers
#[derive(Clone)]
pub struct OpenAICompatibleClient {
    client: Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
}

/// Default base URL (Ollama default).
pub const DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// Default model.
pub const DEFAULT_MODEL: &str = "mlx-community/gpt-oss-20b-MXFP4-Q8";

// Legacy aliases for backward compatibility during migration
pub const DEFAULT_OLLAMA_BASE_URL: &str = DEFAULT_BASE_URL;
pub const DEFAULT_OLLAMA_MODEL: &str = DEFAULT_MODEL;

impl OpenAICompatibleClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self::with_api_key(base_url, model, None)
    }

    /// Create client with API key for authenticated requests.
    pub fn with_api_key(base_url: &str, model: &str, api_key: Option<String>) -> Self {
        // Use 120 second timeout for LLM requests (they can be slow)
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key,
        }
    }

    /// Create client with custom timeout (for testing).
    pub fn with_timeout(base_url: &str, model: &str, timeout_secs: u64) -> Self {
        Self::with_timeout_and_key(base_url, model, timeout_secs, None)
    }

    /// Create client with custom timeout and API key (for testing).
    pub fn with_timeout_and_key(
        base_url: &str,
        model: &str,
        timeout_secs: u64,
        api_key: Option<String>,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key,
        }
    }

    /// Create client from environment variables.
    ///
    /// Uses `OLLAMA_BASE_URL` and `OLLAMA_MODEL` environment variables,
    /// falling back to defaults if not set.
    ///
    /// Also reads `OLLAMA_API_KEY` for authenticated APIs.
    ///
    /// Note: Environment variable names kept as OLLAMA_* for backward compatibility.
    pub fn from_env() -> Self {
        let base_url =
            std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        let api_key = std::env::var("OLLAMA_API_KEY").ok().filter(|s| !s.is_empty());
        Self::with_api_key(&base_url, &model, api_key)
    }
}

impl Default for OpenAICompatibleClient {
    fn default() -> Self {
        Self::new(DEFAULT_BASE_URL, DEFAULT_MODEL)
    }
}

impl OpenAICompatibleClient {
    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the model.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the API key (if configured).
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    /// Create a request builder with Authorization header if API key is present.
    fn build_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method, &url);

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        request
    }
}

#[async_trait]
impl LlmPort for OpenAICompatibleClient {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Use request model if provided, otherwise fall back to client's default
        let model = request.model.clone().unwrap_or_else(|| self.model.clone());
        let api_request = OpenAIChatRequest {
            model,
            messages: build_messages(&request),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let response = self
            .build_request(reqwest::Method::POST, "/v1/chat/completions")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .map_err(|e| LlmError::RequestFailed(e.to_string()))?;
            return Err(LlmError::RequestFailed(error_text));
        }

        let api_response: OpenAIChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        convert_response(api_response)
    }
}

#[async_trait]
impl LlmModelsPort for OpenAICompatibleClient {
    async fn list_models(&self) -> Result<Vec<LlmModelDescriptor>, LlmError> {
        let response = self
            .build_request(reqwest::Method::GET, "/v1/models")
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .map_err(|e| LlmError::RequestFailed(e.to_string()))?;
            return Err(LlmError::RequestFailed(format!(
                "Failed to list models: {}",
                error_text
            )));
        }

        let models_response: OpenAIModelsResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        let descriptors = models_response
            .data
            .into_iter()
            .map(|model| LlmModelDescriptor {
                id: model.id,
                owned_by: model.owned_by,
                object: model.object,
            })
            .collect();

        Ok(descriptors)
    }
}

fn build_messages(request: &LlmRequest) -> Vec<OpenAIMessage> {
    let mut messages = Vec::new();

    if let Some(system) = &request.system_prompt {
        messages.push(OpenAIMessage {
            role: "system".to_string(),
            content: Some(system.clone()),
        });
    }

    for msg in &request.messages {
        messages.push(OpenAIMessage {
            role: match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => "system",
                MessageRole::Unknown => "user", // Default unknown roles to user
            }
            .to_string(),
            content: Some(msg.content.clone()),
        });
    }

    messages
}

fn convert_response(response: OpenAIChatResponse) -> Result<LlmResponse, LlmError> {
    let choice = response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| LlmError::InvalidResponse("No choices in LLM response".to_string()))?;

    let finish_reason = match choice.finish_reason.as_deref() {
        Some("stop") => FinishReason::Stop,
        Some("length") => FinishReason::Length,
        Some("content_filter") => FinishReason::ContentFilter,
        _ => FinishReason::Stop,
    };

    Ok(LlmResponse {
        content: choice.message.content.unwrap_or_default(),
        finish_reason,
        usage: response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
    })
}

// =============================================================================
// OpenAI API types
// =============================================================================

#[derive(Debug, Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// =============================================================================
// OpenAI Models API types
// =============================================================================

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
    object: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
    object: Option<String>,
    #[serde(default)]
    owned_by: Option<String>,
}
