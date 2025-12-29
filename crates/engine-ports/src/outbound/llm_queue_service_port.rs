//! LLM Queue Service Port - Interface for LLM request queue operations
//!
//! This port defines the interface for managing LLM processing queue operations,
//! including enqueueing requests, dequeuing for processing, and completing/failing items.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(any(test, feature = "testing"))]
use mockall::automock;

use crate::outbound::queue_port::QueueItemStatus;
use wrldbldr_domain::value_objects::GamePromptRequest;

// ============================================================================
// Request/Response Types
// ============================================================================

/// LLM queue request - what gets enqueued for LLM processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmQueueRequest {
    /// Type of LLM request
    pub request_type: LlmRequestType,
    /// World ID for routing responses
    pub world_id: Uuid,
    /// Player character ID (for challenge targeting)
    #[serde(default)]
    pub pc_id: Option<Uuid>,
    /// The prompt for NPC response generation
    #[serde(default)]
    pub prompt: Option<GamePromptRequest>,
    /// Context for suggestion generation
    #[serde(default)]
    pub suggestion_context: Option<SuggestionContext>,
    /// Callback ID for correlating responses
    pub callback_id: String,
}

/// LLM request type discriminator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LlmRequestType {
    /// Generate NPC response to player action
    NpcResponse { action_item_id: Uuid },
    /// Generate suggestions for entity fields
    Suggestion {
        field_type: String,
        entity_id: Option<String>,
    },
}

/// Context for generating suggestions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SuggestionContext {
    /// Type of entity (e.g., "character", "location")
    pub entity_type: Option<String>,
    /// Name of the entity (if already set)
    pub entity_name: Option<String>,
    /// World/setting name or type
    pub world_setting: Option<String>,
    /// Hints or keywords to guide generation
    pub hints: Option<String>,
    /// Additional context from other fields
    pub additional_context: Option<String>,
    /// World ID for per-world template resolution
    #[serde(default)]
    pub world_id: Option<String>,
}

/// LLM queue item - wraps a request with queue metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmQueueItem {
    /// Unique item ID
    pub id: Uuid,
    /// The request payload
    pub payload: LlmQueueRequest,
    /// Priority (higher = more urgent)
    pub priority: u8,
    /// Callback ID from the request
    pub callback_id: String,
}

/// LLM response - result of processing an LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// NPC dialogue text
    pub npc_dialogue: String,
    /// Internal reasoning (shown to DM only)
    pub internal_reasoning: String,
    /// Proposed tool calls
    pub proposed_tool_calls: Vec<ProposedToolCall>,
    /// Optional challenge suggestion
    #[serde(default)]
    pub challenge_suggestion: Option<ChallengeSuggestion>,
    /// Optional narrative event suggestion
    #[serde(default)]
    pub narrative_event_suggestion: Option<NarrativeEventSuggestion>,
    /// Topics discussed (for dialogue tracking)
    #[serde(default)]
    pub topics: Vec<String>,
}

/// A proposed tool call from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedToolCall {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

/// Challenge suggestion from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSuggestion {
    pub challenge_id: String,
    pub confidence: ConfidenceLevel,
    pub reasoning: String,
}

/// Narrative event suggestion from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEventSuggestion {
    pub event_id: String,
    pub confidence: ConfidenceLevel,
    pub reasoning: String,
    pub matched_triggers: Vec<String>,
}

/// Confidence level for suggestions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    Low,
    Medium,
    High,
}

// ============================================================================
// Port Trait
// ============================================================================

/// Port for LLM queue service operations
///
/// This trait defines the interface for managing the LLM processing queue.
/// Implementations handle the actual storage and retrieval of queue items.
#[cfg_attr(any(test, feature = "testing"), automock)]
#[async_trait]
pub trait LlmQueueServicePort: Send + Sync {
    /// Enqueue an LLM request for processing
    ///
    /// Returns the unique ID assigned to the queue item.
    async fn enqueue(&self, request: LlmQueueRequest) -> anyhow::Result<Uuid>;

    /// Dequeue the next item for processing
    ///
    /// Returns None if the queue is empty. The item is marked as "processing"
    /// and should be completed or failed after processing.
    async fn dequeue(&self) -> anyhow::Result<Option<LlmQueueItem>>;

    /// Mark an item as successfully completed
    ///
    /// The result is provided for logging/auditing purposes.
    async fn complete(&self, id: Uuid, result: LlmResponse) -> anyhow::Result<()>;

    /// Mark an item as failed
    ///
    /// Failed items may be retried depending on the implementation.
    async fn fail(&self, id: Uuid, error: String) -> anyhow::Result<()>;

    /// Cancel a suggestion request by callback ID
    ///
    /// Returns true if a matching request was found and cancelled.
    async fn cancel_suggestion(&self, callback_id: &str) -> anyhow::Result<bool>;

    /// Get the current queue depth (pending items)
    async fn depth(&self) -> anyhow::Result<usize>;

    /// Get the number of items currently being processed
    async fn processing_count(&self) -> anyhow::Result<usize>;

    /// Get all items with a given status
    async fn list_by_status(&self, status: QueueItemStatus) -> anyhow::Result<Vec<LlmQueueItem>>;

    /// Clean up old completed/failed items beyond retention period
    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64>;
}
