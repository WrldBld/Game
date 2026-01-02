//! LLM Suggestion Queue Port - Outbound interface for LLM suggestion queue operations
//!
//! This port defines the outbound interface for enqueuing LLM suggestion requests.
//! It is used by adapters that need to submit suggestions to the LLM processing queue.
//!
//! # Architecture
//!
//! This is an **outbound port**:
//! - **Depended on by**: Application code (handlers in engine-app)
//! - **Implemented by**: Adapters (the actual queue infrastructure)
//!
//! The `SuggestionEnqueueAdapter` implements `SuggestionEnqueuePort` and depends on
//! this `LlmSuggestionQueuePort` to submit requests to the underlying queue.

use async_trait::async_trait;
use uuid::Uuid;

// Re-export SuggestionContext from engine-dto (single source of truth)
pub use wrldbldr_engine_dto::SuggestionContext;

use super::QueueError;

#[cfg(any(test, feature = "testing"))]
use mockall::automock;

// ============================================================================
// DTOs for LLM Suggestion Queue
// ============================================================================

/// Request to enqueue an LLM suggestion
///
/// This is the data structure submitted to the LLM queue for suggestion generation.
#[derive(Debug, Clone)]
pub struct LlmSuggestionQueueRequest {
    /// Type of field to generate (e.g., "deflection_behavior", "behavioral_tells")
    pub field_type: String,
    /// Entity ID if updating an existing entity
    pub entity_id: Option<String>,
    /// World ID for routing responses
    pub world_id: Uuid,
    /// Context for suggestion generation
    pub suggestion_context: SuggestionContext,
    /// Callback ID for correlating responses
    pub callback_id: String,
}

// ============================================================================
// Port Trait
// ============================================================================

/// Port for LLM suggestion queue operations
///
/// This outbound port defines the interface for submitting LLM suggestion
/// requests to the processing queue.
///
/// # Implementation Notes
///
/// Implementations should:
/// - Generate a unique item ID for tracking
/// - Store the request in a persistent queue
/// - Return the item ID for correlation
#[cfg_attr(any(test, feature = "testing"), automock)]
#[async_trait]
pub trait LlmSuggestionQueuePort: Send + Sync {
    /// Enqueue an LLM suggestion request for processing
    ///
    /// Returns the unique ID assigned to the queue item.
    async fn enqueue(&self, request: LlmSuggestionQueueRequest) -> Result<Uuid, QueueError>;

    /// Cancel a pending suggestion request by callback ID
    ///
    /// Returns true if a matching request was found and cancelled.
    async fn cancel(&self, callback_id: &str) -> Result<bool, QueueError>;
}
