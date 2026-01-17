//! LLM operations wrapper.

use std::sync::Arc;

use crate::infrastructure::ports::{LlmError, LlmPort, LlmRequest, LlmResponse, ToolDefinition};

/// LLM service wrapper for use cases.
pub struct LlmService {
    llm: Arc<dyn LlmPort>,
}

impl LlmService {
    pub fn new(llm: Arc<dyn LlmPort>) -> Self {
        Self { llm }
    }

    pub async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.llm.generate(request).await
    }

    pub async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        self.llm.generate_with_tools(request, tools).await
    }
}
