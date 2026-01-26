//! Logging wrapper for QueuePort.
//!
//! Captures all queue operations to the E2E event log for analysis.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use uuid::Uuid;
use wrldbldr_domain::WorldId;

use crate::queue_types::{
    ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData,
};

use crate::infrastructure::ports::{
    ClockPort, QueueError, QueueItem, QueueItemStatus, QueuePort,
};
use wrldbldr_domain::QueueItemId;

use super::event_log::{E2EEvent, E2EEventLog};

/// Queue wrapper that logs all operations to an E2E event log.
pub struct LoggingQueue {
    inner: Arc<dyn QueuePort>,
    event_log: Arc<E2EEventLog>,
}

impl LoggingQueue {
    /// Create a new logging queue wrapper.
    pub fn new(inner: Arc<dyn QueuePort>, event_log: Arc<E2EEventLog>) -> Self {
        Self { inner, event_log }
    }
}

#[async_trait]
impl QueuePort for LoggingQueue {
    // Player action queue
    async fn enqueue_player_action(
        &self,
        data: &PlayerActionData,
    ) -> Result<QueueItemId, QueueError> {
        let id = self.inner.enqueue_player_action(data).await?;

        self.event_log.log(E2EEvent::ActionEnqueued {
            id: id.to_uuid(),
            action_type: data.action_type.clone(),
            target: data.target.clone(),
            dialogue: data.dialogue.clone(),
        });

        Ok(id)
    }

    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        let start = Instant::now();
        let result = self.inner.dequeue_player_action().await?;

        if let Some(ref item) = result {
            self.event_log.log(E2EEvent::ActionProcessed {
                id: item.id.to_uuid(),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        Ok(result)
    }

    // LLM request queue
    async fn enqueue_llm_request(&self, data: &LlmRequestData) -> Result<QueueItemId, QueueError> {
        let id = self.inner.enqueue_llm_request(data).await?;

        self.event_log.log(E2EEvent::LlmRequestEnqueued {
            id: id.to_uuid(),
            request_type: format!("{:?}", data.request_type),
            callback_id: data.callback_id.clone(),
        });

        Ok(id)
    }

    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        self.inner.dequeue_llm_request().await
    }

    // DM approval queue
    async fn enqueue_dm_approval(
        &self,
        data: &ApprovalRequestData,
    ) -> Result<QueueItemId, QueueError> {
        let id = self.inner.enqueue_dm_approval(data).await?;

        self.event_log.log(E2EEvent::ApprovalEnqueued {
            id: id.to_uuid(),
            decision_type: format!("{:?}", data.decision_type),
            urgency: format!("{:?}", data.urgency),
        });

        Ok(id)
    }

    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        self.inner.dequeue_dm_approval().await
    }

    // Asset generation queue
    async fn enqueue_asset_generation(
        &self,
        data: &AssetGenerationData,
    ) -> Result<QueueItemId, QueueError> {
        self.inner.enqueue_asset_generation(data).await
    }

    async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
        self.inner.dequeue_asset_generation().await
    }

    // Common operations (pass through)
    async fn mark_complete(&self, id: QueueItemId) -> Result<(), QueueError> {
        self.inner.mark_complete(id).await
    }

    async fn mark_failed(&self, id: QueueItemId, error: &str) -> Result<(), QueueError> {
        // Log errors
        self.event_log.log(E2EEvent::Error {
            code: "QUEUE_ITEM_FAILED".to_string(),
            message: error.to_string(),
            context: Some(serde_json::json!({ "queue_item_id": id.to_string() })),
        });

        self.inner.mark_failed(id, error).await
    }

    async fn get_pending_count(&self, queue_type: &str) -> Result<usize, QueueError> {
        self.inner.get_pending_count(queue_type).await
    }

    async fn list_by_type(
        &self,
        queue_type: &str,
        limit: usize,
    ) -> Result<Vec<QueueItem>, QueueError> {
        self.inner.list_by_type(queue_type, limit).await
    }

    async fn set_result_json(&self, id: QueueItemId, result_json: &str) -> Result<(), QueueError> {
        self.inner.set_result_json(id, result_json).await
    }

    async fn cancel_pending_llm_request_by_callback_id(
        &self,
        callback_id: &str,
    ) -> Result<bool, QueueError> {
        self.inner
            .cancel_pending_llm_request_by_callback_id(callback_id)
            .await
    }

    async fn get_approval_request(
        &self,
        id: QueueItemId,
    ) -> Result<Option<ApprovalRequestData>, QueueError> {
        self.inner.get_approval_request(id).await
    }

    async fn get_generation_read_state(
        &self,
        user_id: &str,
        world_id: WorldId,
    ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
        self.inner
            .get_generation_read_state(user_id, world_id)
            .await
    }

    async fn upsert_generation_read_state(
        &self,
        user_id: &str,
        world_id: WorldId,
        read_batches: &[String],
        read_suggestions: &[String],
    ) -> Result<(), QueueError> {
        self.inner
            .upsert_generation_read_state(user_id, world_id, read_batches, read_suggestions)
            .await
    }

    async fn delete_by_callback_id(&self, callback_id: &str) -> Result<bool, QueueError> {
        self.inner.delete_by_callback_id(callback_id).await
    }
}
