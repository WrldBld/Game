//! Queue operations wrapper.

use std::sync::Arc;

use crate::infrastructure::ports::{QueueError, QueueItem, QueuePort};
use crate::queue_types::{
    ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData,
};
use uuid::Uuid;
use wrldbldr_domain::WorldId;

/// Queue wrapper for use cases.
pub struct Queue {
    queue: Arc<dyn QueuePort>,
}

impl Queue {
    pub fn new(queue: Arc<dyn QueuePort>) -> Self {
        Self { queue }
    }

    pub async fn enqueue_player_action(&self, data: &PlayerActionData) -> Result<Uuid, QueueError> {
        self.queue.enqueue_player_action(data).await
    }

    pub async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        self.queue.dequeue_player_action().await
    }

    pub async fn enqueue_llm_request(&self, data: &LlmRequestData) -> Result<Uuid, QueueError> {
        self.queue.enqueue_llm_request(data).await
    }

    pub async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        self.queue.dequeue_llm_request().await
    }

    pub async fn enqueue_dm_approval(
        &self,
        data: &ApprovalRequestData,
    ) -> Result<Uuid, QueueError> {
        self.queue.enqueue_dm_approval(data).await
    }

    pub async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        self.queue.dequeue_dm_approval().await
    }

    pub async fn enqueue_asset_generation(
        &self,
        data: &AssetGenerationData,
    ) -> Result<Uuid, QueueError> {
        self.queue.enqueue_asset_generation(data).await
    }

    pub async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
        self.queue.dequeue_asset_generation().await
    }

    pub async fn mark_complete(&self, id: Uuid) -> Result<(), QueueError> {
        self.queue.mark_complete(id).await
    }

    pub async fn mark_failed(&self, id: Uuid, error: &str) -> Result<(), QueueError> {
        self.queue.mark_failed(id, error).await
    }

    pub async fn get_pending_count(&self, queue_type: &str) -> Result<usize, QueueError> {
        self.queue.get_pending_count(queue_type).await
    }

    pub async fn list_by_type(
        &self,
        queue_type: &str,
        limit: usize,
    ) -> Result<Vec<QueueItem>, QueueError> {
        self.queue.list_by_type(queue_type, limit).await
    }

    pub async fn set_result_json(&self, id: Uuid, result_json: &str) -> Result<(), QueueError> {
        self.queue.set_result_json(id, result_json).await
    }

    pub async fn cancel_pending_llm_request_by_callback_id(
        &self,
        callback_id: &str,
    ) -> Result<bool, QueueError> {
        self.queue
            .cancel_pending_llm_request_by_callback_id(callback_id)
            .await
    }

    pub async fn get_approval_request(
        &self,
        id: Uuid,
    ) -> Result<Option<ApprovalRequestData>, QueueError> {
        self.queue.get_approval_request(id).await
    }

    pub async fn get_generation_read_state(
        &self,
        user_id: &str,
        world_id: WorldId,
    ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
        self.queue
            .get_generation_read_state(user_id, world_id)
            .await
    }

    pub async fn upsert_generation_read_state(
        &self,
        user_id: &str,
        world_id: WorldId,
        read_batches: &[String],
        read_suggestions: &[String],
    ) -> Result<(), QueueError> {
        self.queue
            .upsert_generation_read_state(user_id, world_id, read_batches, read_suggestions)
            .await
    }

    pub async fn delete_by_callback_id(&self, callback_id: &str) -> Result<bool, QueueError> {
        self.queue.delete_by_callback_id(callback_id).await
    }
}
