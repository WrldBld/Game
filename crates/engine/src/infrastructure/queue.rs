//! SQLite queue implementation - stub.

use async_trait::async_trait;
use uuid::Uuid;
use wrldbldr_domain::{ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData};

use crate::infrastructure::ports::{QueueError, QueueItem, QueuePort};

pub struct SqliteQueue {
    #[allow(dead_code)]
    db_path: String,
}

impl SqliteQueue {
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }
}

#[async_trait]
impl QueuePort for SqliteQueue {
    async fn enqueue_player_action(&self, _data: &PlayerActionData) -> Result<Uuid, QueueError> {
        todo!("SQLite queue: enqueue_player_action")
    }

    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        todo!("SQLite queue: dequeue_player_action")
    }

    async fn enqueue_llm_request(&self, _data: &LlmRequestData) -> Result<Uuid, QueueError> {
        todo!("SQLite queue: enqueue_llm_request")
    }

    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        todo!("SQLite queue: dequeue_llm_request")
    }

    async fn enqueue_dm_approval(&self, _data: &ApprovalRequestData) -> Result<Uuid, QueueError> {
        todo!("SQLite queue: enqueue_dm_approval")
    }

    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        todo!("SQLite queue: dequeue_dm_approval")
    }

    async fn enqueue_asset_generation(
        &self,
        _data: &AssetGenerationData,
    ) -> Result<Uuid, QueueError> {
        todo!("SQLite queue: enqueue_asset_generation")
    }

    async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
        todo!("SQLite queue: dequeue_asset_generation")
    }

    async fn mark_complete(&self, _id: Uuid) -> Result<(), QueueError> {
        todo!("SQLite queue: mark_complete")
    }

    async fn mark_failed(&self, _id: Uuid, _error: &str) -> Result<(), QueueError> {
        todo!("SQLite queue: mark_failed")
    }

    async fn get_pending_count(&self, _queue_type: &str) -> Result<usize, QueueError> {
        todo!("SQLite queue: get_pending_count")
    }
}
