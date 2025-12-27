//! LLM port - Interface for language model interactions

use async_trait::async_trait;

// Re-export DTOs from engine-dto for convenience
pub use wrldbldr_engine_dto::llm::{
    ChatMessage, FinishReason, ImageData, LlmRequest, LlmResponse, MessageRole, TokenUsage,
    ToolCall, ToolDefinition,
};

/// Port for LLM (Large Language Model) interactions
#[async_trait]
pub trait LlmPort: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Generate a response from the LLM
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, Self::Error>;

    /// Generate a response with tool/function calling support
    async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, Self::Error>;
}
