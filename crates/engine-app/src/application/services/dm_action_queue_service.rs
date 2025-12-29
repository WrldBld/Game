//! DM Action Queue Service - Enqueues and processes DM actions
//!
//! This service manages the DMActionQueue, which receives DM actions
//! (approval decisions, direct NPC control, event triggers, scene transitions)
//! and processes them immediately.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use wrldbldr_domain::value_objects::{DmActionData, DmActionType};
use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::outbound::{ClockPort, QueueError, QueueItem, QueueItemId, QueuePort};

/// Service for managing the DM action queue
pub struct DmActionQueueService<Q: QueuePort<DmActionData>> {
    pub(crate) queue: Arc<Q>,
    /// Clock for time operations (required for testability)
    clock: Arc<dyn ClockPort>,
}

impl<Q: QueuePort<DmActionData>> DmActionQueueService<Q> {
    pub fn queue(&self) -> &Arc<Q> {
        &self.queue
    }

    /// Create a new DM action queue service
    ///
    /// # Arguments
    /// * `clock` - Clock for time operations. Use `SystemClock` in production,
    ///             `MockClockPort` in tests for deterministic behavior.
    pub fn new(queue: Arc<Q>, clock: Arc<dyn ClockPort>) -> Self {
        Self { queue, clock }
    }

    /// Get the current time
    fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Enqueue a DM action for processing
    ///
    /// DM actions have high priority (1) to ensure they are processed
    /// before player actions.
    pub async fn enqueue_action(
        &self,
        world_id: &WorldId,
        dm_id: String,
        action: DmActionType,
    ) -> Result<QueueItemId, QueueError> {
        let item = DmActionData {
            world_id: *world_id,
            dm_id,
            action,
            timestamp: self.now(),
        };

        // High priority for DM actions
        self.queue.enqueue(item, 1).await
    }

    /// Process the next DM action from the queue
    ///
    /// Returns the action item ID if processed, None if queue was empty
    pub async fn process_next<F, Fut>(
        &self,
        process_action: F,
    ) -> Result<Option<QueueItemId>, QueueError>
    where
        F: FnOnce(DmActionData) -> Fut,
        Fut: std::future::Future<Output = Result<(), QueueError>>,
    {
        let Some(item) = self.queue.dequeue().await? else {
            return Ok(None);
        };

        // Clone payload before passing to callback (item.payload is already Clone)
        match process_action(item.payload.clone()).await {
            Ok(()) => {
                self.queue.complete(item.id).await?;
                Ok(Some(item.id))
            }
            Err(e) => {
                // Mark as failed
                self.queue.fail(item.id, &e.to_string()).await?;
                Err(e)
            }
        }
    }

    /// Get queue depth (number of pending actions)
    pub async fn depth(&self) -> Result<usize, QueueError> {
        self.queue.depth().await
    }

    /// Get a specific action item by ID
    pub async fn get_action(
        &self,
        id: QueueItemId,
    ) -> Result<Option<QueueItem<DmActionData>>, QueueError> {
        self.queue.get(id).await
    }
}
