//! DM Action Queue Service Port - Interface for DM action queue operations
//!
//! This port defines the interface for managing DM action queue operations,
//! including enqueueing DM actions and processing them with high priority.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(any(test, feature = "testing"))]
use mockall::automock;

// ============================================================================
// Request/Response Types
// ============================================================================

/// DM action - what gets enqueued when a DM takes an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmAction {
    /// World where the action occurs
    pub world_id: Uuid,
    /// DM who initiated the action
    pub dm_id: String,
    /// The specific action to perform
    pub action: DmActionType,
    /// When the action was submitted
    pub timestamp: DateTime<Utc>,
}

/// Types of DM actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DmActionType {
    /// Process an approval decision
    ApprovalDecision {
        request_id: String,
        decision: DmDecision,
    },
    /// Direct control of an NPC
    DirectNpcControl { npc_id: String, dialogue: String },
    /// Trigger a narrative event
    TriggerEvent { event_id: String },
    /// Transition to a new scene
    TransitionScene { scene_id: Uuid },
}

/// Simplified DM decision for action queue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum DmDecision {
    Accept,
    Reject { feedback: String },
    TakeOver { dm_response: String },
}

/// DM action queue item - wraps an action with queue metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmActionQueueItem {
    /// Unique item ID
    pub id: Uuid,
    /// The action payload
    pub payload: DmAction,
    /// Priority (DM actions have high priority)
    pub priority: u8,
    /// When the item was enqueued
    pub enqueued_at: DateTime<Utc>,
}

// ============================================================================
// Port Trait
// ============================================================================

/// Port for DM action queue service operations
///
/// This trait defines the interface for managing the DM action queue.
/// DM actions are enqueued with high priority and processed before player actions.
#[cfg_attr(any(test, feature = "testing"), automock)]
#[async_trait]
pub trait DmActionQueueServicePort: Send + Sync {
    /// Enqueue a DM action for processing
    ///
    /// DM actions have high priority and are processed before player actions.
    /// Returns the unique ID assigned to the queue item.
    async fn enqueue(&self, action: DmAction) -> anyhow::Result<Uuid>;

    /// Dequeue the next DM action for processing
    ///
    /// Returns None if the queue is empty. The item is marked as "processing"
    /// and should be completed after processing.
    async fn dequeue(&self) -> anyhow::Result<Option<DmActionQueueItem>>;

    /// Mark an action as successfully completed
    async fn complete(&self, id: Uuid) -> anyhow::Result<()>;

    /// Mark an action as failed
    async fn fail(&self, id: Uuid, error: String) -> anyhow::Result<()>;

    /// Get the current queue depth (pending items)
    async fn depth(&self) -> anyhow::Result<usize>;

    /// Get a specific action by ID
    async fn get(&self, id: Uuid) -> anyhow::Result<Option<DmActionQueueItem>>;
}
