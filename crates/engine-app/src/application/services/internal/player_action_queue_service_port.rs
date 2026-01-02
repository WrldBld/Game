//! Player Action Queue Service Port - Interface for player action queue operations
//!
//! This port defines the interface for managing player action queue operations,
//! including enqueueing actions, dequeuing for processing, and completing items.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(any(test, feature = "testing"))]
use mockall::automock;

use wrldbldr_engine_ports::outbound::QueueItemStatus;

// ============================================================================
// Request/Response Types
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
// Port Trait
// ============================================================================

/// Port for player action queue service operations
///
/// This trait defines the interface for managing the player action queue.
/// Player actions are enqueued here and then processed to generate LLM requests.
#[cfg_attr(any(test, feature = "testing"), automock)]
#[async_trait]
pub trait PlayerActionQueueServicePort: Send + Sync {
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
