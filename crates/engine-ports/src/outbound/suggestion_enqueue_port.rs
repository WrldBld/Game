//! Suggestion Enqueue Port - Interface for queuing AI suggestion requests
//!
//! This port provides a way to enqueue suggestion requests to the LLM queue
//! without exposing the full LLMQueueService with its complex generics.

use async_trait::async_trait;

use super::QueueError;

/// Request for an AI suggestion
#[derive(Debug, Clone)]
pub struct SuggestionEnqueueRequest {
    /// Type of suggestion (e.g., "deflection_behavior", "behavioral_tells")
    pub field_type: String,
    /// Entity ID for context (e.g., character_id, want_id)
    pub entity_id: Option<String>,
    /// World ID for routing responses
    pub world_id: Option<uuid::Uuid>,
    /// Context for the suggestion
    pub context: SuggestionEnqueueContext,
}

/// Context information for generating a suggestion
#[derive(Debug, Clone, Default)]
pub struct SuggestionEnqueueContext {
    /// Type of entity (e.g., "character", "npc")
    pub entity_type: Option<String>,
    /// Name of the entity
    pub entity_name: Option<String>,
    /// World/setting name or type
    pub world_setting: Option<String>,
    /// Hints or keywords to guide generation
    pub hints: Option<String>,
    /// Additional context from other fields
    pub additional_context: Option<String>,
    /// World ID for per-world template resolution
    pub world_id: Option<String>,
}

/// Response from enqueuing a suggestion
#[derive(Debug, Clone)]
pub struct SuggestionEnqueueResponse {
    /// Request ID for tracking
    pub request_id: String,
}

/// Port for enqueuing suggestion requests
///
/// This abstracts away the LLMQueueService's complex generics,
/// allowing the AppRequestHandler to enqueue suggestions without
/// knowing about the concrete queue implementation.
#[async_trait]
pub trait SuggestionEnqueuePort: Send + Sync {
    /// Enqueue a suggestion request
    ///
    /// Returns a request_id that can be used to track the suggestion.
    /// Results are delivered via WebSocket events (SuggestionCompleted, SuggestionFailed).
    async fn enqueue_suggestion(
        &self,
        request: SuggestionEnqueueRequest,
    ) -> Result<SuggestionEnqueueResponse, QueueError>;

    /// Cancel a pending suggestion request
    ///
    /// Returns true if the request was found and cancelled.
    async fn cancel_suggestion(&self, request_id: &str) -> Result<bool, QueueError>;
}
