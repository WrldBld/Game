//! LLM Suggestion Queue Adapter - Bridges LlmSuggestionQueuePort to LlmQueueServicePort
//!
//! This adapter implements the `LlmSuggestionQueuePort` outbound port by delegating
//! to the internal `LlmQueueServicePort` service trait.
//!
//! # Architecture
//!
//! This adapter lives in `engine-composition` (not `engine-adapters`) because:
//! - It needs to bridge between a port (`LlmSuggestionQueuePort`) and an internal
//!   service trait (`LlmQueueServicePort`)
//! - `engine-composition` is allowed to depend on `engine-app` for DI wiring
//! - `engine-adapters` should NOT depend on `engine-app`
//!
//! The composition root creates this adapter and provides it to other adapters
//! that need to submit LLM suggestion requests.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use wrldbldr_engine_app::application::services::internal::{
    LlmQueueRequest, LlmQueueServicePort, LlmRequestType,
    LlmSuggestionContext as InternalSuggestionContext,
};
use wrldbldr_engine_ports::outbound::{
    LlmSuggestionQueuePort, LlmSuggestionQueueRequest, QueueError,
};

/// Adapter that implements LlmSuggestionQueuePort by delegating to LlmQueueServicePort
pub struct LlmSuggestionQueueAdapter {
    llm_queue_service: Arc<dyn LlmQueueServicePort>,
}

impl LlmSuggestionQueueAdapter {
    /// Create a new adapter wrapping an LlmQueueServicePort
    pub fn new(llm_queue_service: Arc<dyn LlmQueueServicePort>) -> Self {
        Self { llm_queue_service }
    }
}

#[async_trait]
impl LlmSuggestionQueuePort for LlmSuggestionQueueAdapter {
    async fn enqueue(&self, request: LlmSuggestionQueueRequest) -> Result<Uuid, QueueError> {
        // Convert port DTO to internal DTO
        let internal_context = InternalSuggestionContext {
            entity_type: request.suggestion_context.entity_type,
            entity_name: request.suggestion_context.entity_name,
            world_setting: request.suggestion_context.world_setting,
            hints: request.suggestion_context.hints,
            additional_context: request.suggestion_context.additional_context,
            world_id: request.suggestion_context.world_id,
        };

        // Create internal LLM queue request
        let internal_request = LlmQueueRequest {
            request_type: LlmRequestType::Suggestion {
                field_type: request.field_type,
                entity_id: request.entity_id,
            },
            world_id: request.world_id,
            pc_id: None,
            prompt: None,
            suggestion_context: Some(internal_context),
            callback_id: request.callback_id,
        };

        // Delegate to internal service
        self.llm_queue_service
            .enqueue(internal_request)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))
    }

    async fn cancel(&self, callback_id: &str) -> Result<bool, QueueError> {
        self.llm_queue_service
            .cancel_suggestion(callback_id)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))
    }
}
