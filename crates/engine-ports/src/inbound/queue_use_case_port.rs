//! Queue use case ports - Inbound interfaces for queue operations
//!
//! These ports define the inbound interfaces that adapters/handlers call.
//! The signatures match the internal service ports in engine-app exactly,
//! enabling adapters to delegate directly to the underlying services.
//!
//! # Architecture Note
//!
//! Each *UseCasePort trait mirrors a corresponding *ServicePort in
//! `engine-app::application::services::internal`. The types are duplicated
//! here to maintain the hexagonal architecture boundary (ports don't depend
//! on app layer).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use wrldbldr_domain::value_objects::GamePromptRequest;
use wrldbldr_domain::WorldId;
use wrldbldr_engine_dto::{
    ChallengeSuggestionInfo, NarrativeEventSuggestionInfo, ProposedToolInfo, SuggestionContext,
};

// Import QueueItemStatus from outbound ports (single source of truth)
pub use crate::outbound::QueueItemStatus;

// ============================================================================
// Asset Generation Queue Types
// ============================================================================

/// Asset generation request - what gets enqueued for ComfyUI processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGenerationRequest {
    /// World ID (optional for some generation types)
    pub world_id: Option<Uuid>,
    /// Type of entity (e.g., "character", "location", "item")
    pub entity_type: String,
    /// ID of the entity to generate assets for
    pub entity_id: String,
    /// ComfyUI workflow ID to use
    pub workflow_id: String,
    /// Prompt for generation
    pub prompt: String,
    /// Number of images to generate
    pub count: u32,
    /// Optional negative prompt
    #[serde(default)]
    pub negative_prompt: Option<String>,
    /// Optional style reference asset ID
    #[serde(default)]
    pub style_reference_id: Option<Uuid>,
}

/// Asset generation queue item - wraps a request with queue metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGenerationQueueItem {
    /// Unique item ID
    pub id: Uuid,
    /// The request payload
    pub payload: AssetGenerationRequest,
    /// Priority (higher = more urgent)
    pub priority: u8,
    /// When the item was enqueued
    pub enqueued_at: DateTime<Utc>,
}

/// Result of a successful asset generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    /// Generated asset IDs
    pub asset_ids: Vec<Uuid>,
    /// File paths of generated images
    pub file_paths: Vec<String>,
    /// Generation metadata
    pub metadata: GenerationMetadata,
}

/// Metadata about the generation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Workflow used
    pub workflow: String,
    /// Prompt used
    pub prompt: String,
    /// Negative prompt (if any)
    #[serde(default)]
    pub negative_prompt: Option<String>,
    /// Seed used (if available)
    #[serde(default)]
    pub seed: Option<i64>,
    /// Time taken in milliseconds
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

// ============================================================================
// Generation Queue Projection Types
// ============================================================================

/// Snapshot of a suggestion task's current state
#[derive(Debug, Clone, Serialize)]
pub struct SuggestionTaskSnapshot {
    /// Unique request identifier
    pub request_id: String,
    /// Type of field being suggested (e.g., "name", "description")
    pub field_type: String,
    /// Entity ID if applicable
    pub entity_id: Option<String>,
    /// Current status: "queued", "processing", "ready", "failed"
    pub status: String,
    /// Generated suggestions (when status is "ready")
    pub suggestions: Option<Vec<String>>,
    /// Error message (when status is "failed")
    pub error: Option<String>,
    /// Whether the user has marked this as read
    pub is_read: bool,
}

/// Snapshot of a generation batch with read state
#[derive(Debug, Clone, Serialize)]
pub struct GenerationBatchSnapshot {
    /// Batch identifier
    pub id: String,
    /// World this batch belongs to
    pub world_id: String,
    /// Entity type being generated
    pub entity_type: String,
    /// Entity ID being generated for
    pub entity_id: Option<String>,
    /// Current status
    pub status: String,
    /// Number of items in the batch
    pub item_count: usize,
    /// Number of completed items
    pub completed_count: usize,
    /// Whether the user has marked this as read
    pub is_read: bool,
}

/// Unified snapshot of the generation queue state
#[derive(Debug, Clone, Serialize)]
pub struct GenerationQueueSnapshot {
    /// Image generation batches
    pub batches: Vec<GenerationBatchSnapshot>,
    /// Text suggestion tasks
    pub suggestions: Vec<SuggestionTaskSnapshot>,
}

// ============================================================================
// Player Action Queue Types
// ============================================================================

/// Player action - what gets enqueued when a player takes an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerAction {
    /// World where the action occurs
    pub world_id: Uuid,
    /// Player who initiated the action
    pub player_id: String,
    /// Player character performing the action (for challenge targeting)
    #[serde(default)]
    pub pc_id: Option<Uuid>,
    /// Type of action (e.g., "talk", "examine", "move")
    pub action_type: String,
    /// Target of the action (e.g., NPC ID, item ID)
    pub target: Option<String>,
    /// Dialogue text (for talk actions)
    pub dialogue: Option<String>,
    /// When the action was submitted
    pub timestamp: DateTime<Utc>,
}

/// Player action queue item - wraps an action with queue metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerActionQueueItem {
    /// Unique item ID
    pub id: Uuid,
    /// The action payload
    pub payload: PlayerAction,
    /// Priority (higher = more urgent)
    pub priority: u8,
    /// When the item was enqueued
    pub enqueued_at: DateTime<Utc>,
}

// ============================================================================
// LLM Queue Types
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

/// LLM queue response - result of processing an LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmQueueResponse {
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
// DM Approval Queue Types
// ============================================================================

/// Approval request - what gets enqueued for DM review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// World where the approval is needed
    pub world_id: Uuid,
    /// ID of the source action that generated this approval
    pub source_action_id: Uuid,
    /// Type of decision required
    pub decision_type: ApprovalDecisionType,
    /// Urgency level
    pub urgency: ApprovalUrgency,
    /// Player character ID (for SPOKE_TO edge creation)
    #[serde(default)]
    pub pc_id: Option<Uuid>,
    /// NPC character ID
    #[serde(default)]
    pub npc_id: Option<String>,
    /// NPC name (for display)
    pub npc_name: String,
    /// Proposed dialogue text
    pub proposed_dialogue: String,
    /// Internal reasoning (shown to DM only)
    pub internal_reasoning: String,
    /// Proposed tool calls
    pub proposed_tools: Vec<ProposedToolInfo>,
    /// Number of times this has been rejected and retried
    pub retry_count: u32,
    /// Optional challenge suggestion
    #[serde(default)]
    pub challenge_suggestion: Option<ChallengeSuggestionInfo>,
    /// Optional narrative event suggestion
    #[serde(default)]
    pub narrative_event_suggestion: Option<NarrativeEventSuggestionInfo>,
    // Context for dialogue persistence
    /// Player's dialogue text
    #[serde(default)]
    pub player_dialogue: Option<String>,
    /// Scene ID where dialogue occurred
    #[serde(default)]
    pub scene_id: Option<String>,
    /// Location ID where dialogue occurred
    #[serde(default)]
    pub location_id: Option<String>,
    /// Game time when dialogue occurred
    #[serde(default)]
    pub game_time: Option<String>,
    /// Topics discussed
    #[serde(default)]
    pub topics: Vec<String>,
}

/// Type of decision required
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecisionType {
    NpcResponse,
    ToolUsage,
    ChallengeSuggestion,
    SceneTransition,
    ChallengeOutcome,
}

/// Urgency level for approval requests
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalUrgency {
    Normal = 0,
    AwaitingPlayer = 1,
    SceneCritical = 2,
}

/// Approval queue item - wraps a request with queue metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalQueueItem {
    /// Unique item ID
    pub id: Uuid,
    /// The request payload
    pub payload: ApprovalRequest,
    /// Priority (based on urgency)
    pub priority: u8,
    /// When the item was enqueued
    pub enqueued_at: DateTime<Utc>,
    /// When the item was last updated
    pub updated_at: DateTime<Utc>,
}

// Re-export DmApprovalDecision from engine-dto (single source of truth)
pub use wrldbldr_engine_dto::DmApprovalDecision;

// ============================================================================
// Asset Generation Queue Use Case Port
// ============================================================================

/// Port for asset generation queue use case operations
///
/// This trait mirrors `AssetGenerationQueueServicePort` from engine-app.
/// Adapters implement this to delegate to the underlying service.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait AssetGenerationQueueUseCasePort: Send + Sync {
    /// Enqueue an asset generation request
    ///
    /// Returns the unique ID assigned to the queue item.
    async fn enqueue(&self, request: AssetGenerationRequest) -> anyhow::Result<Uuid>;

    /// Dequeue the next item for processing
    ///
    /// Returns None if the queue is empty. The item is marked as "processing"
    /// and should be completed or failed after processing.
    async fn dequeue(&self) -> anyhow::Result<Option<AssetGenerationQueueItem>>;

    /// Mark an item as successfully completed with the generation result
    async fn complete(&self, id: Uuid, result: GenerationResult) -> anyhow::Result<()>;

    /// Mark an item as failed
    async fn fail(&self, id: Uuid, error: String) -> anyhow::Result<()>;

    /// Get the current queue depth (pending items)
    async fn depth(&self) -> anyhow::Result<usize>;

    /// Get the number of items currently being processed
    async fn processing_count(&self) -> anyhow::Result<usize>;

    /// Check if the queue has capacity for more work
    async fn has_capacity(&self) -> anyhow::Result<bool>;

    /// Get a specific item by ID
    async fn get(&self, id: Uuid) -> anyhow::Result<Option<AssetGenerationQueueItem>>;

    /// Get all items with a given status
    async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> anyhow::Result<Vec<AssetGenerationQueueItem>>;

    /// Clean up old completed/failed items beyond retention period
    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64>;
}

// ============================================================================
// Generation Queue Projection Use Case Port
// ============================================================================

/// Port for projecting generation queue state
///
/// This trait mirrors `GenerationQueueProjectionServicePort` from engine-app.
/// Adapters implement this to delegate to the underlying service.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait GenerationQueueProjectionUseCasePort: Send + Sync {
    /// Build a generation queue snapshot for a user and world
    ///
    /// # Arguments
    ///
    /// * `user_id` - Optional user ID for applying read markers.
    ///               If `None`, all items are treated as unread.
    /// * `world_id` - The world to project queue state for.
    ///
    /// # Returns
    ///
    /// A unified snapshot containing all active batches and suggestion tasks.
    async fn project_queue(
        &self,
        user_id: Option<String>,
        world_id: WorldId,
    ) -> anyhow::Result<GenerationQueueSnapshot>;
}

// ============================================================================
// Player Action Queue Use Case Port
// ============================================================================

/// Port for player action queue use case operations
///
/// This trait mirrors `PlayerActionQueueServicePort` from engine-app.
/// Adapters implement this to delegate to the underlying service.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait PlayerActionQueueUseCasePort: Send + Sync {
    /// Enqueue a player action for processing
    ///
    /// Returns the unique ID assigned to the queue item.
    async fn enqueue(&self, action: PlayerAction) -> anyhow::Result<Uuid>;

    /// Dequeue the next action for processing
    ///
    /// Returns None if the queue is empty. The item is marked as "processing"
    /// and should be completed after processing.
    async fn dequeue(&self) -> anyhow::Result<Option<PlayerActionQueueItem>>;

    /// Mark an action as successfully completed
    async fn complete(&self, id: Uuid) -> anyhow::Result<()>;

    /// Get the current queue depth (pending items)
    async fn depth(&self) -> anyhow::Result<usize>;

    /// Get a specific action by ID
    async fn get(&self, id: Uuid) -> anyhow::Result<Option<PlayerActionQueueItem>>;

    /// Get all items with a given status
    async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> anyhow::Result<Vec<PlayerActionQueueItem>>;

    /// Clean up old completed/failed items beyond retention period
    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64>;
}

// ============================================================================
// LLM Queue Use Case Port
// ============================================================================

/// Port for LLM queue use case operations
///
/// This trait mirrors `LlmQueueServicePort` from engine-app.
/// Adapters implement this to delegate to the underlying service.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait LlmQueueUseCasePort: Send + Sync {
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
    async fn complete(&self, id: Uuid, result: LlmQueueResponse) -> anyhow::Result<()>;

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

// ============================================================================
// DM Approval Queue Use Case Port
// ============================================================================

/// Port for DM approval queue use case operations
///
/// This trait mirrors `DmApprovalQueueServicePort` from engine-app.
/// Adapters implement this to delegate to the underlying service.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait DmApprovalQueueUseCasePort: Send + Sync {
    /// Enqueue an approval request for DM review
    ///
    /// Returns the unique ID assigned to the queue item.
    async fn enqueue(&self, approval: ApprovalRequest) -> anyhow::Result<Uuid>;

    /// Dequeue the next item for processing
    ///
    /// Returns None if the queue is empty.
    async fn dequeue(&self) -> anyhow::Result<Option<ApprovalQueueItem>>;

    /// Process a DM decision on an approval item
    async fn complete(&self, id: Uuid, decision: DmApprovalDecision) -> anyhow::Result<()>;

    /// Get all pending approvals for a world
    async fn get_pending(&self, world_id: WorldId) -> anyhow::Result<Vec<ApprovalQueueItem>>;

    /// Get an approval item by ID
    async fn get(&self, id: Uuid) -> anyhow::Result<Option<ApprovalQueueItem>>;

    /// Get approval history for a world
    async fn get_history(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> anyhow::Result<Vec<ApprovalQueueItem>>;

    /// Delay an approval for later
    async fn delay(&self, id: Uuid, until: DateTime<Utc>) -> anyhow::Result<()>;

    /// Discard a challenge suggestion from an approval
    async fn discard_challenge(&self, request_id: &str) -> anyhow::Result<()>;

    /// Get the current queue depth (pending items)
    async fn depth(&self) -> anyhow::Result<usize>;

    /// Get all items with a given status
    async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> anyhow::Result<Vec<ApprovalQueueItem>>;

    /// Clean up old completed/failed items beyond retention period
    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64>;

    /// Expire approval items older than the specified timeout
    async fn expire_old(&self, timeout: std::time::Duration) -> anyhow::Result<u64>;
}
