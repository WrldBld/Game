// Port traits define the full contract - many methods are for future use
#![allow(dead_code)]

//! External service port traits (LLM, Image Generation, Queues).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub use wrldbldr_domain::QueueItemId;
use wrldbldr_domain::WorldId;

use super::error::QueueError;
use crate::queue_types::{
    ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData, QueueType,
};

use super::error::{ImageGenError, LlmError};

// =============================================================================
// LLM Types
// =============================================================================

/// LLM request/response types
#[derive(Debug, Clone)]
pub struct LlmRequest {
    /// The conversation history
    pub messages: Vec<ChatMessage>,
    /// System prompt / context
    pub system_prompt: Option<String>,
    /// Temperature for response generation (0.0 - 2.0)
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Optional images for multimodal models
    pub images: Vec<ImageData>,
    /// Optional model override (uses client's default if None)
    pub model: Option<String>,
    /// Optional fingerprint metadata for multi-phase pipeline (VCR fingerprinting)
    /// This is ignored by the actual LLM client but used by VCR for stable matching
    pub fingerprint_meta: Option<super::PhaseFingerprintMeta>,
}

impl LlmRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            images: Vec::new(),
            model: None,
            fingerprint_meta: None,
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: Option<u32>) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_fingerprint_meta(mut self, meta: super::PhaseFingerprintMeta) -> Self {
        self.fingerprint_meta = Some(meta);
        self
    }
}

/// A message in the conversation
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }
}

/// Role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Unknown,
}

/// Image data for multimodal requests
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Base64-encoded image data
    pub data: String,
    /// MIME type (e.g., "image/png")
    pub media_type: String,
}

/// Response from the LLM
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// The generated text content
    pub content: String,
    /// Finish reason
    pub finish_reason: FinishReason,
    /// Token usage
    pub usage: Option<TokenUsage>,
}

// =============================================================================
// VCR Fingerprint Types (for multi-phase LLM pipeline)
// =============================================================================

/// Metadata for multi-phase LLM requests to include in fingerprinting.
#[derive(Debug, Clone, Default)]
pub struct PhaseFingerprintMeta {
    /// Phase short ID (e.g., "chal", "evt", "tool")
    pub phase_id: Option<String>,
    /// Prompt template key used for this phase
    pub template_key: Option<String>,
    /// Template version for cache-busting when prompts change
    pub template_version: Option<String>,
}

/// Reason the generation finished
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    /// Fallback for unknown/legacy finish reasons (e.g., old "ToolCalls" cassettes)
    #[serde(other)]
    Unknown,
}

/// Token usage information
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
}

// =============================================================================
// LLM Model Discovery Types
// =============================================================================

/// Descriptor for an available LLM model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmModelDescriptor {
    /// Model identifier (e.g., "llama3.2", "gpt-4")
    pub id: String,
    /// Provider/owner of the model (e.g., "meta", "openai")
    pub owned_by: Option<String>,
    /// Optional capabilities or metadata
    pub object: Option<String>,
}

/// Port for LLM model discovery and validation.
///
/// This is separate from `LlmPort` because it queries the provider's
/// available models rather than generating content.
#[async_trait]
pub trait LlmModelsPort: Send + Sync {
    /// List all available models from the provider.
    async fn list_models(&self) -> Result<Vec<LlmModelDescriptor>, LlmError>;

    /// Validate that a specific model ID is available.
    ///
    /// Default implementation queries list_models and checks for presence.
    async fn validate_model(&self, model_id: &str) -> Result<(), LlmError> {
        let models = self.list_models().await?;
        if models.iter().any(|m| m.id == model_id) {
            Ok(())
        } else {
            Err(LlmError::InvalidResponse(format!(
                "Model '{}' not found in available models",
                model_id
            )))
        }
    }
}

// =============================================================================
// Image Generation Types
// =============================================================================

/// Image generation request/response types
#[derive(Debug, Clone)]
pub struct ImageRequest {
    pub prompt: String,
    pub workflow: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct ImageResult {
    pub image_data: Vec<u8>,
    pub format: String,
}

#[async_trait]
pub trait ImageGenPort: Send + Sync {
    async fn generate(&self, request: ImageRequest) -> Result<ImageResult, ImageGenError>;
    async fn check_health(&self) -> Result<bool, ImageGenError>;
}

// =============================================================================
// Queue Port
// =============================================================================

/// Queue item wrapper with metadata.
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub id: QueueItemId,
    pub data: QueueItemData,
    pub created_at: DateTime<Utc>,
    pub status: QueueItemStatus,
    pub error_message: Option<String>,
    /// Optional JSON result payload for completed items.
    ///
    /// Used for queued suggestion results so that Creator UI can hydrate after reload.
    pub result_json: Option<String>,
}

/// Concrete queue item data - avoids generics for dyn compatibility.
#[derive(Debug, Clone)]
pub enum QueueItemData {
    PlayerAction(PlayerActionData),
    LlmRequest(LlmRequestData),
    DmApproval(ApprovalRequestData),
    AssetGeneration(AssetGenerationData),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueItemStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[async_trait]
pub trait QueuePort: Send + Sync {
    // Player action queue
    async fn enqueue_player_action(
        &self,
        data: &PlayerActionData,
    ) -> Result<QueueItemId, QueueError>;
    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError>;

    // LLM request queue
    async fn enqueue_llm_request(&self, data: &LlmRequestData) -> Result<QueueItemId, QueueError>;
    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError>;

    // DM approval queue
    async fn enqueue_dm_approval(
        &self,
        data: &ApprovalRequestData,
    ) -> Result<QueueItemId, QueueError>;
    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError>;

    // Asset generation queue
    async fn enqueue_asset_generation(
        &self,
        data: &AssetGenerationData,
    ) -> Result<QueueItemId, QueueError>;
    async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError>;

    // Common operations
    async fn mark_complete(&self, id: QueueItemId) -> Result<(), QueueError>;
    async fn mark_failed(&self, id: QueueItemId, error: &str) -> Result<(), QueueError>;
    async fn get_pending_count(&self, queue_type: QueueType) -> Result<usize, QueueError>;

    /// List queue items by type (newest first).
    ///
    /// This is used by WebSocket Creator UI to hydrate a unified generation queue.
    async fn list_by_type(
        &self,
        queue_type: QueueType,
        limit: usize,
    ) -> Result<Vec<QueueItem>, QueueError>;

    /// Persist a JSON result payload for a queue item.
    async fn set_result_json(&self, id: QueueItemId, result_json: &str) -> Result<(), QueueError>;

    /// Cancel a pending LLM request by callback_id.
    ///
    /// Returns true if a matching pending request was found and cancelled.
    async fn cancel_pending_llm_request_by_callback_id(
        &self,
        callback_id: &str,
    ) -> Result<bool, QueueError>;

    /// Get an approval request by ID (for extracting NPC info when processing decision)
    async fn get_approval_request(
        &self,
        id: QueueItemId,
    ) -> Result<Option<ApprovalRequestData>, QueueError>;

    /// Get the persisted generation queue read-state for a user in a world.
    ///
    /// Returns (read_batches, read_suggestions) if present.
    async fn get_generation_read_state(
        &self,
        user_id: &str,
        world_id: WorldId,
    ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError>;

    /// Upsert the persisted generation queue read-state for a user in a world.
    async fn upsert_generation_read_state(
        &self,
        user_id: &str,
        world_id: WorldId,
        read_batches: &[String],
        read_suggestions: &[String],
    ) -> Result<(), QueueError>;

    /// Delete a queue item by callback_id (used for dismissing suggestions).
    ///
    /// This scans pending/completed LLM requests to find one with matching callback_id.
    async fn delete_by_callback_id(&self, callback_id: &str) -> Result<bool, QueueError>;

    /// Atomically mark a queue item complete and enqueue the next phased LLM request.
    ///
    /// This operation is idempotent: if called multiple times with the same `phase_key`,
    /// only one next-phase item will be enqueued. This prevents duplicate phase execution
    /// when transient failures occur between phase transitions.
    ///
    /// # Arguments
    /// * `item_id` - The queue item to mark complete
    /// * `next_request` - The next phase's LLM request data
    /// * `phase_key` - Unique key for idempotency (typically includes phase number and content hash)
    ///
    /// # Returns
    /// The ID of the enqueued next-phase item (or the existing item if already enqueued)
    async fn advance_phased_llm_request(
        &self,
        item_id: QueueItemId,
        next_request: &LlmRequestData,
        phase_key: &str,
    ) -> Result<QueueItemId, QueueError>;
}
