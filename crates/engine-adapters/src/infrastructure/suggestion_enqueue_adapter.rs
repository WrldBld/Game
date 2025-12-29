//! Suggestion Enqueue Adapter - Bridges SuggestionEnqueuePort to LLMQueueService
//!
//! This adapter implements the `SuggestionEnqueuePort` trait using the concrete
//! `LLMQueueService`, hiding the complex generics from consumers.
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
use wrldbldr_domain::value_objects::{LlmRequestData, LlmRequestType};
use wrldbldr_engine_app::application::services::{LLMQueueService, SuggestionContext};
use wrldbldr_engine_ports::outbound::{
    LlmPort, ProcessingQueuePort, QueueError, QueueNotificationPort, SuggestionEnqueuePort,
    SuggestionEnqueueRequest, SuggestionEnqueueResponse, WorldRepositoryPort,
};

/// Adapter that implements SuggestionEnqueuePort for LLMQueueService
///
/// This adapter wraps the generic LLMQueueService and exposes a simple
/// async interface for enqueuing suggestion requests. It also handles
/// auto-enrichment of suggestion context with world data.
pub struct SuggestionEnqueueAdapter<Q, L, N>
where
    Q: ProcessingQueuePort<LlmRequestData> + 'static,
    L: LlmPort + Clone + 'static,
    N: QueueNotificationPort + 'static,
{
    llm_queue_service: Arc<LLMQueueService<Q, L, N>>,
    world_repository: Arc<dyn WorldRepositoryPort>,
}

impl<Q, L, N> SuggestionEnqueueAdapter<Q, L, N>
where
    Q: ProcessingQueuePort<LlmRequestData> + 'static,
    L: LlmPort + Clone + 'static,
    N: QueueNotificationPort + 'static,
{
    /// Create a new adapter wrapping an LLMQueueService
    ///
    /// # Arguments
    /// * `llm_queue_service` - The LLM queue service to delegate to
    /// * `world_repository` - Repository for fetching world data for auto-enrichment
    pub fn new(
        llm_queue_service: Arc<LLMQueueService<Q, L, N>>,
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
        mut context: SuggestionContext,
        world_id: Option<uuid::Uuid>,
    ) -> SuggestionContext {
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
impl<Q, L, N> SuggestionEnqueuePort for SuggestionEnqueueAdapter<Q, L, N>
where
    Q: ProcessingQueuePort<LlmRequestData> + 'static,
    L: LlmPort + Clone + 'static,
    N: QueueNotificationPort + 'static,
{
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

        // Convert context
        let suggestion_context = SuggestionContext {
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

        // Convert service SuggestionContext to domain SuggestionContext
        let domain_context = wrldbldr_domain::value_objects::SuggestionContext {
            entity_type: enriched_context.entity_type,
            entity_name: enriched_context.entity_name,
            world_setting: enriched_context.world_setting,
            hints: enriched_context.hints,
            additional_context: enriched_context.additional_context,
            world_id: enriched_context
                .world_id
                .and_then(|s| uuid::Uuid::parse_str(&s).ok())
                .map(WorldId::from_uuid),
        };

        // Create LLM request data
        let llm_request = LlmRequestData {
            request_type: LlmRequestType::Suggestion {
                field_type: request.field_type,
                entity_id: request.entity_id,
            },
            world_id: WorldId::from_uuid(world_id),
            pc_id: None,
            prompt: None,
            suggestion_context: Some(domain_context),
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
