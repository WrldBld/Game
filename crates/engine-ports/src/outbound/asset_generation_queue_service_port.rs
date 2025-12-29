//! Asset Generation Queue Service Port - Interface for asset generation queue operations
//!
//! This port defines the interface for managing asset generation queue operations,
//! including enqueueing ComfyUI requests and processing them with concurrency control.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(any(test, feature = "testing"))]
use mockall::automock;

// ============================================================================
// Request/Response Types
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
// Port Trait
// ============================================================================

/// Port for asset generation queue service operations
///
/// This trait defines the interface for managing the asset generation queue.
/// Requests are processed with concurrency control (typically batch_size=1).
#[cfg_attr(any(test, feature = "testing"), automock)]
#[async_trait]
pub trait AssetGenerationQueueServicePort: Send + Sync {
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
}
