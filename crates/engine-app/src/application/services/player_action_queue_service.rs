//! Player Action Queue Service - Enqueues and processes player actions
//!
//! This service manages the PlayerActionQueue, which receives player actions
//! and routes them to the LLMReasoningQueue for processing.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use wrldbldr_domain::value_objects::{
    GamePromptRequest, LlmRequestData, LlmRequestType, PlayerActionData,
};
use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::outbound::{
    ClockPort, ProcessingQueuePort, QueueError, QueueItem, QueueItemId, QueuePort,
};

/// Service for managing the player action queue
pub struct PlayerActionQueueService<
    Q: QueuePort<PlayerActionData>,
    LQ: ProcessingQueuePort<LlmRequestData>,
> {
    pub(crate) queue: Arc<Q>,
    llm_queue: Arc<LQ>,
    /// Clock for time operations (required for testability)
    clock: Arc<dyn ClockPort>,
}

impl<Q: QueuePort<PlayerActionData>, LQ: ProcessingQueuePort<LlmRequestData>>
    PlayerActionQueueService<Q, LQ>
{
    pub fn queue(&self) -> &Arc<Q> {
        &self.queue
    }

    /// Create a new player action queue service
    ///
    /// # Arguments
    /// * `clock` - Clock for time operations. Use `SystemClock` in production,
    ///             `MockClockPort` in tests for deterministic behavior.
    pub fn new(queue: Arc<Q>, llm_queue: Arc<LQ>, clock: Arc<dyn ClockPort>) -> Self {
        Self {
            queue,
            llm_queue,
            clock,
        }
    }

    /// Get the current time
    fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Enqueue a player action for processing
    pub async fn enqueue_action(
        &self,
        world_id: &WorldId,
        player_id: String,
        pc_id: Option<PlayerCharacterId>,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    ) -> Result<QueueItemId, QueueError> {
        let item = PlayerActionData {
            world_id: *world_id,
            player_id,
            pc_id,
            action_type,
            target,
            dialogue,
            timestamp: self.now(),
        };

        // Normal priority for player actions
        self.queue.enqueue(item, 0).await
    }

    /// Process the next player action from the queue
    ///
    /// This method:
    /// 1. Dequeues a player action
    /// 2. Builds the LLM request context
    /// 3. Enqueues it to the LLM queue
    /// 4. Marks the action as completed
    ///
    /// Returns the action item ID if processed, None if queue was empty
    pub async fn process_next<F, Fut>(
        &self,
        build_prompt: F,
    ) -> Result<Option<QueueItemId>, QueueError>
    where
        F: FnOnce(PlayerActionData) -> Fut,
        Fut: std::future::Future<Output = Result<GamePromptRequest, QueueError>>,
    {
        let Some(item) = self.queue.dequeue().await? else {
            return Ok(None);
        };

        // Clone payload before passing to callback (item.payload is already Clone)
        let payload = item.payload.clone();
        let world_id = payload.world_id;
        let pc_id = payload.pc_id;
        let item_id = item.id;

        // Build the prompt request from the action (async)
        let prompt = build_prompt(payload).await?;

        // Create LLM request item
        let llm_request = LlmRequestData {
            request_type: LlmRequestType::NpcResponse {
                action_item_id: item_id,
            },
            world_id,
            pc_id,
            prompt: Some(prompt),
            suggestion_context: None,
            callback_id: item_id.to_string(),
        };

        // Enqueue to LLM queue (normal priority)
        self.llm_queue.enqueue(llm_request, 0).await?;

        // Mark action as completed (LLM queue handles the rest)
        self.queue.complete(item_id).await?;

        Ok(Some(item_id))
    }

    /// Get queue depth (number of pending actions)
    pub async fn depth(&self) -> Result<usize, QueueError> {
        self.queue.depth().await
    }

    /// Get a specific action item by ID
    pub async fn get_action(
        &self,
        id: QueueItemId,
    ) -> Result<Option<QueueItem<PlayerActionData>>, QueueError> {
        self.queue.get(id).await
    }
}
