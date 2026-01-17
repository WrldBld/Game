//! Ollama LLM client (OpenAI-compatible API)

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::infrastructure::ports::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, MessageRole, TokenUsage, ToolCall,
    ToolDefinition,
};

/// Client for Ollama's OpenAI-compatible API
#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

/// Default Ollama base URL.
pub const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";

/// Default model for Ollama.
pub const DEFAULT_OLLAMA_MODEL: &str = "mlx-community/gpt-oss-20b-MXFP4-Q8";

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        // Use 120 second timeout for LLM requests (they can be slow)
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    /// Create client with custom timeout (for testing).
    pub fn with_timeout(base_url: &str, model: &str, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }

    /// Create client from environment variables.
    ///
    /// Uses `OLLAMA_BASE_URL` and `OLLAMA_MODEL` environment variables,
    /// falling back to defaults if not set.
    pub fn from_env() -> Self {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_OLLAMA_BASE_URL.to_string());
        let model =
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string());
        Self::new(&base_url, &model)
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new(DEFAULT_OLLAMA_BASE_URL, DEFAULT_OLLAMA_MODEL)
    }
}

#[async_trait]
impl LlmPort for OllamaClient {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let api_request = OpenAIChatRequest {
            model: self.model.clone(),
            messages: build_messages(&request),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: None,
        };

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
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

    async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        let api_tools: Vec<OpenAITool> = tools
            .into_iter()
            .map(|t| OpenAITool {
                r#type: "function".to_string(),
                function: OpenAIFunction {
                    name: t.name,
                    description: t.description,
                    parameters: t.parameters,
                },
            })
            .collect();

        let api_request = OpenAIChatRequest {
            model: self.model.clone(),
            messages: build_messages(&request),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: Some(api_tools),
        };

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
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

fn build_messages(request: &LlmRequest) -> Vec<OpenAIMessage> {
    let mut messages = Vec::new();

    if let Some(system) = &request.system_prompt {
        messages.push(OpenAIMessage {
            role: "system".to_string(),
            content: Some(system.clone()),
            tool_calls: None,
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
            tool_calls: None,
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

    let mut tool_calls = Vec::new();
    for tc in choice.message.tool_calls.unwrap_or_default() {
        let arguments: serde_json::Value =
            serde_json::from_str(&tc.function.arguments).map_err(|e| {
                LlmError::InvalidResponse(format!(
                    "Invalid tool call arguments for '{}': {}",
                    tc.function.name, e
                ))
            })?;
        tool_calls.push(ToolCall {
            id: tc.id,
            name: tc.function.name,
            arguments,
        });
    }

    let finish_reason = match choice.finish_reason.as_deref() {
        Some("stop") => FinishReason::Stop,
        Some("length") => FinishReason::Length,
        Some("tool_calls") => FinishReason::ToolCalls,
        Some("content_filter") => FinishReason::ContentFilter,
        _ => FinishReason::Stop,
    };

    Ok(LlmResponse {
        content: choice.message.content.unwrap_or_default(),
        tool_calls,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Serialize)]
struct OpenAITool {
    r#type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Serialize)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    function: OpenAIToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}
