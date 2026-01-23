//! Mock queue for testing.

use std::sync::Mutex;

use async_trait::async_trait;
use crate::infrastructure::ports::{
    QueueError, QueueItem, QueueItemId, QueueItemStatus, QueuePort,
};
use crate::queue_types::{
    ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData,
};

/// Simple mock queue for testing.
pub struct MockQueueForTesting;

impl MockQueueForTesting {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockQueueForTesting {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QueuePort for MockQueueForTesting {
    async fn enqueue_player_action(
        &self,
        _data: &PlayerActionData,
    ) -> Result<QueueItemId, QueueError> {
        Ok(QueueItemId::new())
    }

    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_llm_request(
        &self,
        _data: &LlmRequestData,
    ) -> Result<QueueItemId, QueueError> {
        Ok(QueueItemId::new())
    }

    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_dm_approval(
        &self,
        _data: &ApprovalRequestData,
    ) -> Result<QueueItemId, QueueError> {
        Ok(QueueItemId::new())
    }

    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_asset_generation(
        &self,
        _data: &AssetGenerationData,
    ) -> Result<QueueItemId, QueueError> {
        Ok(QueueItemId::new())
    }

    async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn mark_complete(&self, _id: QueueItemId) -> Result<(), QueueError> {
        Ok(())
    }

    async fn mark_failed(&self, _id: QueueItemId, _error: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn get_pending_count(&self, _queue_type: &str) -> Result<usize, QueueError> {
        Ok(0)
    }

    async fn list_by_type(
        &self,
        _queue_type: &str,
        _limit: usize,
    ) -> Result<Vec<QueueItem>, QueueError> {
        Ok(vec![])
    }

    async fn set_result_json(
        &self,
        _id: QueueItemId,
        _result_json: &str,
    ) -> Result<(), QueueError> {
        Ok(())
    }

    async fn cancel_pending_llm_request_by_callback_id(
        &self,
        _callback_id: &str,
    ) -> Result<bool, QueueError> {
        Ok(false)
    }

    async fn get_approval_request(
        &self,
        _id: QueueItemId,
    ) -> Result<Option<ApprovalRequestData>, QueueError> {
        Ok(None)
    }

    async fn get_generation_read_state(
        &self,
        _user_id: &str,
        _world_id: wrldbldr_domain::WorldId,
    ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
        Ok(None)
    }

    async fn upsert_generation_read_state(
        &self,
        _user_id: &str,
        _world_id: wrldbldr_domain::WorldId,
        _read_batches: &[String],
        _read_suggestions: &[String],
    ) -> Result<(), QueueError> {
        Ok(())
    }

    async fn delete_by_callback_id(&self, _callback_id: &str) -> Result<bool, QueueError> {
        Ok(false)
    }
}
