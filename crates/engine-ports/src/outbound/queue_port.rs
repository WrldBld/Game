//! Queue port - Interface for queue operations
//!
//! This port provides a storage-agnostic interface for queue operations,
//! supporting multiple backends (InMemory, SQLite, Redis) and different
//! queue types (standard, approval, processing).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use wrldbldr_domain::WorldId;

// Re-export DTOs from engine-dto for convenience
pub use wrldbldr_engine_dto::queue::{QueueError, QueueItem, QueueItemId, QueueItemStatus};

/// Core queue port - storage-agnostic interface
#[async_trait]
pub trait QueuePort<T>: Send + Sync
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    /// Add item to queue
    async fn enqueue(&self, payload: T, priority: u8) -> Result<QueueItemId, QueueError>;

    /// Get next item for processing (marks as Processing)
    async fn dequeue(&self) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Peek at next item without removing
    async fn peek(&self) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Mark item as completed
    async fn complete(&self, id: QueueItemId) -> Result<(), QueueError>;

    /// Mark item as failed (may retry based on attempts)
    async fn fail(&self, id: QueueItemId, error: &str) -> Result<(), QueueError>;

    /// Delay item for later processing
    async fn delay(&self, id: QueueItemId, until: DateTime<Utc>) -> Result<(), QueueError>;

    /// Get item by ID
    async fn get(&self, id: QueueItemId) -> Result<Option<QueueItem<T>>, QueueError>;

    /// Get all items with status
    async fn list_by_status(&self, status: QueueItemStatus) -> Result<Vec<QueueItem<T>>, QueueError>;

    /// Get queue depth (pending items)
    async fn depth(&self) -> Result<usize, QueueError>;

    /// Clear completed/failed items older than duration
    async fn cleanup(&self, older_than: Duration) -> Result<usize, QueueError>;
}

/// Extended port for approval queues with human-facing features
#[async_trait]
pub trait ApprovalQueuePort<T>: QueuePort<T>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    /// Get items by world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<QueueItem<T>>, QueueError>;

    /// Get history (completed/failed/expired items)
    async fn get_history_by_world(&self, world_id: WorldId, limit: usize) -> Result<Vec<QueueItem<T>>, QueueError>;

    /// Expire items older than duration
    async fn expire_old(&self, older_than: Duration) -> Result<usize, QueueError>;
}

/// Port for processing queues with concurrency control
#[async_trait]
pub trait ProcessingQueuePort<T>: QueuePort<T>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    /// Get batch size configuration
    fn batch_size(&self) -> usize;

    /// Get number of items currently processing
    async fn processing_count(&self) -> Result<usize, QueueError>;

    /// Check if can accept more work
    async fn has_capacity(&self) -> Result<bool, QueueError>;
}
