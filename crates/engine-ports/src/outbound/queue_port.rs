//! Queue port - Interface for queue operations
//!
//! This port provides a storage-agnostic interface for queue operations,
//! supporting multiple backends (InMemory, SQLite, Redis) and different
//! queue types (standard, approval, processing).
//!
//! In proper hexagonal architecture, ports work with domain types.
//! Serialization/deserialization is an adapter concern, not a port concern.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use wrldbldr_domain::WorldId;

/// Unique identifier for queue items
pub type QueueItemId = Uuid;

/// Status of a queue item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueItemStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Delayed,
    Expired,
}

impl QueueItemStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueueItemStatus::Pending => "pending",
            QueueItemStatus::Processing => "processing",
            QueueItemStatus::Completed => "completed",
            QueueItemStatus::Failed => "failed",
            QueueItemStatus::Delayed => "delayed",
            QueueItemStatus::Expired => "expired",
        }
    }
}

/// Queue item wrapper - domain representation
///
/// This is a domain-layer wrapper for queue payloads. It contains
/// metadata about the queue item's lifecycle without any serialization
/// concerns. Adapters are responsible for converting this to/from
/// their storage format.
#[derive(Debug, Clone)]
pub struct QueueItem<T> {
    pub id: QueueItemId,
    pub payload: T,
    pub status: QueueItemStatus,
    pub priority: u8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl<T> QueueItem<T> {
    /// Create a new queue item with explicit timestamps
    ///
    /// This is the preferred constructor for testable code. Use with an injected clock.
    pub fn new_with_time(payload: T, priority: u8, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            payload,
            status: QueueItemStatus::Pending,
            priority,
            created_at: now,
            updated_at: now,
            scheduled_at: None,
            attempts: 0,
            max_attempts: 3,
            error_message: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new queue item with current time
    ///
    /// Note: For testable code, prefer `new_with_time()` with an injected clock.
    pub fn new(payload: T, priority: u8) -> Self {
        Self::new_with_time(payload, priority, Utc::now())
    }

    /// Create a queue item with a specific ID and explicit timestamps
    ///
    /// This is the preferred constructor for reconstruction from storage or testable code.
    pub fn with_id_and_time(id: QueueItemId, payload: T, priority: u8, now: DateTime<Utc>) -> Self {
        Self {
            id,
            payload,
            status: QueueItemStatus::Pending,
            priority,
            created_at: now,
            updated_at: now,
            scheduled_at: None,
            attempts: 0,
            max_attempts: 3,
            error_message: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a queue item with a specific ID (for reconstruction from storage)
    ///
    /// Note: For testable code, prefer `with_id_and_time()` with an injected clock.
    pub fn with_id(id: QueueItemId, payload: T, priority: u8) -> Self {
        Self::with_id_and_time(id, payload, priority, Utc::now())
    }
}

/// Queue errors
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Queue is full")]
    QueueFull,

    #[error("Invalid status transition")]
    InvalidStatus,

    #[error("Max attempts exceeded")]
    MaxAttemptsExceeded,

    #[error("Database error: {0}")]
    Database(String),
}

/// Core queue port - storage-agnostic interface for queue operations.
///
/// The generic type T represents the domain payload type.
/// Adapters are responsible for serialization/deserialization.
///
/// # Type Bounds
///
/// - `Send + Sync`: Required for async trait methods across thread boundaries
/// - `Clone`: Required for returning copies of queue items
///
/// Note: Serialization bounds (`Serialize + DeserializeOwned`) are intentionally
/// omitted. In hexagonal architecture, ports should not concern themselves with
/// serialization - that's an adapter implementation detail.
#[async_trait]
pub trait QueuePort<T>: Send + Sync
where
    T: Send + Sync + Clone,
{
    /// Add item to queue with given priority (higher = more urgent)
    async fn enqueue(&self, payload: T, priority: u8) -> Result<QueueItemId, QueueError>;

    /// Get next item for processing (marks as Processing)
    async fn dequeue(&self) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Peek at next item without removing or changing status
    async fn peek(&self) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Mark item as completed
    async fn complete(&self, id: QueueItemId) -> Result<(), QueueError>;

    /// Mark item as failed (may retry based on attempts)
    async fn fail(&self, id: QueueItemId, error: &str) -> Result<(), QueueError>;

    /// Delay item for later processing
    async fn delay(&self, id: QueueItemId, until: DateTime<Utc>) -> Result<(), QueueError>;

    /// Get item by ID
    async fn get(&self, id: QueueItemId) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Get all items with given status
    async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> Result<Vec<QueueItem<T>>, QueueError>;

    /// Get queue depth (pending items count)
    async fn depth(&self) -> Result<usize, QueueError>;

    /// Clear completed/failed items older than duration
    async fn cleanup(&self, older_than: Duration) -> Result<usize, QueueError>;
}

/// Extended port for approval queues with human-facing features.
///
/// Approval queues are designed for DM review workflows where items
/// need to be organized by world and have expiration handling.
#[async_trait]
pub trait ApprovalQueuePort<T>: QueuePort<T>
where
    T: Send + Sync + Clone,
{
    /// Get all pending items for a specific world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<QueueItem<T>>, QueueError>;

    /// Get history (completed/failed/expired items) for a world
    async fn get_history_by_world(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<QueueItem<T>>, QueueError>;

    /// Expire items older than duration (marks as Expired rather than deleting)
    async fn expire_old(&self, older_than: Duration) -> Result<usize, QueueError>;
}

/// Port for processing queues with concurrency control.
///
/// Processing queues are designed for background job processing
/// where we need to limit concurrent operations.
#[async_trait]
pub trait ProcessingQueuePort<T>: QueuePort<T>
where
    T: Send + Sync + Clone,
{
    /// Get configured batch size for this queue
    fn batch_size(&self) -> usize;

    /// Get number of items currently in Processing status
    async fn processing_count(&self) -> Result<usize, QueueError>;

    /// Check if queue can accept more work (processing_count < batch_size)
    async fn has_capacity(&self) -> Result<bool, QueueError>;
}
