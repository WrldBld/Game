//! Suggestion Enqueue Adapter - Bridges SuggestionEnqueuePort to LLMQueueService
//!
//! This adapter implements the `SuggestionEnqueuePort` trait using the concrete
//! `LLMQueueService`, hiding the complex generics from consumers.

use std::sync::Arc;

use async_trait::async_trait;

use wrldbldr_engine_app::application::dto::{LLMRequestItem, LLMRequestType};
use wrldbldr_engine_app::application::services::{LLMQueueService, SuggestionContext};
use wrldbldr_engine_ports::outbound::{
    ProcessingQueuePort, QueueError, QueueNotificationPort, LlmPort, SuggestionEnqueuePort, SuggestionEnqueueRequest,
    SuggestionEnqueueResponse,
};

/// Adapter that implements SuggestionEnqueuePort for LLMQueueService
///
/// This adapter wraps the generic LLMQueueService and exposes a simple
/// async interface for enqueuing suggestion requests.
pub struct SuggestionEnqueueAdapter<Q, L, N>
where
    Q: ProcessingQueuePort<LLMRequestItem> + 'static,
    L: LlmPort + Clone + 'static,
    N: QueueNotificationPort + 'static,
{
    llm_queue_service: Arc<LLMQueueService<Q, L, N>>,
}

impl<Q, L, N> SuggestionEnqueueAdapter<Q, L, N>
where
    Q: ProcessingQueuePort<LLMRequestItem> + 'static,
    L: LlmPort + Clone + 'static,
    N: QueueNotificationPort + 'static,
{
    /// Create a new adapter wrapping an LLMQueueService
    pub fn new(llm_queue_service: Arc<LLMQueueService<Q, L, N>>) -> Self {
        Self { llm_queue_service }
    }
}

#[async_trait]
impl<Q, L, N> SuggestionEnqueuePort for SuggestionEnqueueAdapter<Q, L, N>
where
    Q: ProcessingQueuePort<LLMRequestItem> + 'static,
    L: LlmPort + Clone + 'static,
    N: QueueNotificationPort + 'static,
{
    async fn enqueue_suggestion(
        &self,
        request: SuggestionEnqueueRequest,
    ) -> Result<SuggestionEnqueueResponse, QueueError> {
        // Generate request ID
        let request_id = uuid::Uuid::new_v4().to_string();

        // Convert context
        let suggestion_context = SuggestionContext {
            entity_type: request.context.entity_type,
            entity_name: request.context.entity_name,
            world_setting: request.context.world_setting,
            hints: request.context.hints,
            additional_context: request.context.additional_context,
            world_id: request.context.world_id,
        };

        // Require world_id for suggestion requests
        let world_id = request.world_id.ok_or_else(|| {
            QueueError::Backend("world_id is required for suggestion requests".to_string())
        })?;

        // Create LLM request item
        let llm_request = LLMRequestItem {
            request_type: LLMRequestType::Suggestion {
                field_type: request.field_type,
                entity_id: request.entity_id,
            },
            world_id,
            pc_id: None,
            prompt: None,
            suggestion_context: Some(suggestion_context),
            callback_id: request_id.clone(),
        };

        // Enqueue to LLM queue
        self.llm_queue_service.enqueue(llm_request).await?;

        Ok(SuggestionEnqueueResponse { request_id })
    }

    async fn cancel_suggestion(&self, request_id: &str) -> Result<bool, QueueError> {
        self.llm_queue_service.cancel_suggestion(request_id).await
    }
}
