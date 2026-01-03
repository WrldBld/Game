//! Ollama LLM client - stub implementation.

use async_trait::async_trait;
use crate::infrastructure::ports::{LlmPort, LlmRequest, LlmResponse, LlmError, ToolDefinition};

pub struct OllamaClient {
    #[allow(dead_code)]
    base_url: String,
}

impl OllamaClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

#[async_trait]
impl LlmPort for OllamaClient {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        todo!("Ollama: generate")
    }

    async fn generate_with_tools(
        &self,
        _request: LlmRequest,
        _tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        todo!("Ollama: generate_with_tools")
    }
}
