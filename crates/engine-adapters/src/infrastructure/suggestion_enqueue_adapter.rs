//! Suggestion Enqueue Adapter - Bridges SuggestionEnqueuePort to LlmQueueServicePort
//!
//! This adapter implements the `SuggestionEnqueuePort` trait using the
//! `LlmQueueServicePort`, hiding complexity from consumers.
//!
//! ## Auto-Enrichment
//!
//! When `world_id` is provided but `world_setting` is not, this adapter will
//! automatically fetch the world from the repository and populate `world_setting`
//! with the world's name and description. This improves suggestion quality by
//! providing the LLM with world context.

use std::sync::Arc;

use async_trait::async_trait;

use wrldbldr_domain::WorldId;
use wrldbldr_engine_app::application::services::internal::{
    LlmQueueRequest, LlmQueueServicePort, LlmRequestType, LlmSuggestionContext,
};
use wrldbldr_engine_ports::outbound::{
    QueueError, SuggestionEnqueuePort, SuggestionEnqueueRequest, SuggestionEnqueueResponse,
    WorldRepositoryPort,
};

/// Adapter that implements SuggestionEnqueuePort using LlmQueueServicePort
///
/// This adapter wraps the LlmQueueServicePort and exposes a simple
/// async interface for enqueuing suggestion requests. It also handles
/// auto-enrichment of suggestion context with world data.
pub struct SuggestionEnqueueAdapter {
    llm_queue_service: Arc<dyn LlmQueueServicePort>,
    world_repository: Arc<dyn WorldRepositoryPort>,
}

impl SuggestionEnqueueAdapter {
    /// Create a new adapter wrapping an LlmQueueServicePort
    ///
    /// # Arguments
    /// * `llm_queue_service` - The LLM queue service port to delegate to
    /// * `world_repository` - Repository for fetching world data for auto-enrichment
    pub fn new(
        llm_queue_service: Arc<dyn LlmQueueServicePort>,
        world_repository: Arc<dyn WorldRepositoryPort>,
    ) -> Self {
        Self {
            llm_queue_service,
            world_repository,
        }
    }

    /// Auto-enrich the suggestion context with world data if not already provided
    ///
    /// If `world_setting` is None and `world_id` is Some, fetches the world
    /// and populates `world_setting` with "{world_name}: {world_description}".
    async fn enrich_context(
        &self,
        mut context: LlmSuggestionContext,
        world_id: Option<uuid::Uuid>,
    ) -> LlmSuggestionContext {
        // Only enrich if world_setting is not already provided
        if context.world_setting.is_some() {
            return context;
        }

        // Try to fetch world data for enrichment
        if let Some(wid) = world_id {
            match self.world_repository.get(WorldId::from_uuid(wid)).await {
                Ok(Some(world)) => {
                    // Build world_setting from world name and description
                    let setting = if world.description.is_empty() {
                        world.name
                    } else {
                        format!("{}: {}", world.name, world.description)
                    };
                    context.world_setting = Some(setting);
                    tracing::debug!(
                        "Auto-enriched suggestion context with world setting for world {}",
                        wid
                    );
                }
                Ok(None) => {
                    tracing::warn!("World {} not found for suggestion context enrichment", wid);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch world {} for suggestion context enrichment: {}",
                        wid,
                        e
                    );
                }
            }
        }

        context
    }
}

#[async_trait]
impl SuggestionEnqueuePort for SuggestionEnqueueAdapter {
    async fn enqueue_suggestion(
        &self,
        request: SuggestionEnqueueRequest,
    ) -> Result<SuggestionEnqueueResponse, QueueError> {
        // Generate request ID
        let request_id = uuid::Uuid::new_v4().to_string();

        // Require world_id for suggestion requests
        let world_id = request.world_id.ok_or_else(|| {
            QueueError::Backend("world_id is required for suggestion requests".to_string())
        })?;

        // Convert context to port's SuggestionContext type
        let suggestion_context = LlmSuggestionContext {
            entity_type: request.context.entity_type,
            entity_name: request.context.entity_name,
            world_setting: request.context.world_setting,
            hints: request.context.hints,
            additional_context: request.context.additional_context,
            world_id: request.context.world_id,
        };

        // Auto-enrich context with world data if needed
        let enriched_context = self
            .enrich_context(suggestion_context, Some(world_id))
            .await;

        // Create LLM queue request using port types
        let llm_request = LlmQueueRequest {
            request_type: LlmRequestType::Suggestion {
                field_type: request.field_type,
                entity_id: request.entity_id,
            },
            world_id,
            pc_id: None,
            prompt: None,
            suggestion_context: Some(enriched_context),
            callback_id: request_id.clone(),
        };

        // Enqueue to LLM queue via port
        self.llm_queue_service
            .enqueue(llm_request)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?;

        Ok(SuggestionEnqueueResponse { request_id })
    }

    async fn cancel_suggestion(&self, request_id: &str) -> Result<bool, QueueError> {
        self.llm_queue_service
            .cancel_suggestion(request_id)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))
    }
}
