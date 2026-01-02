//! DM Action Queue Service - Enqueues and processes DM actions
//!
//! This service manages the DMActionQueue, which receives DM actions
//! (approval decisions, direct NPC control, event triggers, scene transitions)
//! and processes them immediately.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use wrldbldr_domain::value_objects::{DmActionData, DmActionType};
use wrldbldr_domain::WorldId;
use crate::application::services::internal::{
    DmAction, DmActionQueueItem, DmActionQueueServicePort, DmActionType as PortDmActionType,
    DmDecision,
};
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

// ============================================================================
// Port Implementation
// ============================================================================

/// Convert port DmActionType to domain DmActionType
fn convert_port_action_type(action: PortDmActionType) -> DmActionType {
    match action {
        PortDmActionType::ApprovalDecision {
            request_id,
            decision,
        } => {
            // Convert simplified port DmDecision to full domain DmApprovalDecision
            let domain_decision = match decision {
                DmDecision::Accept => wrldbldr_domain::value_objects::DmApprovalDecision::Accept,
                DmDecision::Reject { feedback } => {
                    wrldbldr_domain::value_objects::DmApprovalDecision::Reject { feedback }
                }
                DmDecision::TakeOver { dm_response } => {
                    wrldbldr_domain::value_objects::DmApprovalDecision::TakeOver { dm_response }
                }
            };
            DmActionType::ApprovalDecision {
                request_id,
                decision: domain_decision,
            }
        }
        PortDmActionType::DirectNpcControl { npc_id, dialogue } => {
            // Parse npc_id string to CharacterId
            let char_id = uuid::Uuid::parse_str(&npc_id)
                .map(wrldbldr_domain::CharacterId::from_uuid)
                .unwrap_or_else(|_| wrldbldr_domain::CharacterId::from_uuid(uuid::Uuid::nil()));
            DmActionType::DirectNpcControl {
                npc_id: char_id,
                dialogue,
            }
        }
        PortDmActionType::TriggerEvent { event_id } => DmActionType::TriggerEvent { event_id },
        PortDmActionType::TransitionScene { scene_id } => DmActionType::TransitionScene {
            scene_id: wrldbldr_domain::SceneId::from_uuid(scene_id),
        },
    }
}

/// Convert domain DmActionType to port DmActionType
fn convert_domain_action_type(action: DmActionType) -> PortDmActionType {
    match action {
        DmActionType::ApprovalDecision {
            request_id,
            decision,
        } => {
            // Convert full domain DmApprovalDecision to simplified port DmDecision
            // Note: AcceptWithRecipients and AcceptWithModification map to Accept (lossy)
            let port_decision = match decision {
                wrldbldr_domain::value_objects::DmApprovalDecision::Accept => DmDecision::Accept,
                wrldbldr_domain::value_objects::DmApprovalDecision::AcceptWithRecipients {
                    ..
                } => DmDecision::Accept,
                wrldbldr_domain::value_objects::DmApprovalDecision::AcceptWithModification {
                    ..
                } => DmDecision::Accept,
                wrldbldr_domain::value_objects::DmApprovalDecision::Reject { feedback } => {
                    DmDecision::Reject { feedback }
                }
                wrldbldr_domain::value_objects::DmApprovalDecision::TakeOver { dm_response } => {
                    DmDecision::TakeOver { dm_response }
                }
            };
            PortDmActionType::ApprovalDecision {
                request_id,
                decision: port_decision,
            }
        }
        DmActionType::DirectNpcControl { npc_id, dialogue } => PortDmActionType::DirectNpcControl {
            npc_id: npc_id.to_string(),
            dialogue,
        },
        DmActionType::TriggerEvent { event_id } => PortDmActionType::TriggerEvent { event_id },
        DmActionType::TransitionScene { scene_id } => PortDmActionType::TransitionScene {
            scene_id: scene_id.to_uuid(),
        },
    }
}

#[async_trait]
impl<Q> DmActionQueueServicePort for DmActionQueueService<Q>
where
    Q: QueuePort<DmActionData> + Send + Sync + 'static,
{
    async fn enqueue(&self, action: DmAction) -> anyhow::Result<uuid::Uuid> {
        let item = DmActionData {
            world_id: WorldId::from_uuid(action.world_id),
            dm_id: action.dm_id,
            action: convert_port_action_type(action.action),
            timestamp: action.timestamp,
        };

        // High priority for DM actions
        self.queue
            .enqueue(item, 1)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn dequeue(&self) -> anyhow::Result<Option<DmActionQueueItem>> {
        let item = self
            .queue
            .dequeue()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(item.map(|i| DmActionQueueItem {
            id: i.id,
            payload: DmAction {
                world_id: i.payload.world_id.to_uuid(),
                dm_id: i.payload.dm_id,
                action: convert_domain_action_type(i.payload.action),
                timestamp: i.payload.timestamp,
            },
            priority: i.priority,
            enqueued_at: i.created_at,
        }))
    }

    async fn complete(&self, id: uuid::Uuid) -> anyhow::Result<()> {
        self.queue
            .complete(id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn fail(&self, id: uuid::Uuid, error: String) -> anyhow::Result<()> {
        self.queue
            .fail(id, &error)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn depth(&self) -> anyhow::Result<usize> {
        self.queue
            .depth()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn get(&self, id: uuid::Uuid) -> anyhow::Result<Option<DmActionQueueItem>> {
        let item = self
            .queue
            .get(id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(item.map(|i| DmActionQueueItem {
            id: i.id,
            payload: DmAction {
                world_id: i.payload.world_id.to_uuid(),
                dm_id: i.payload.dm_id,
                action: convert_domain_action_type(i.payload.action),
                timestamp: i.payload.timestamp,
            },
            priority: i.priority,
            enqueued_at: i.created_at,
        }))
    }
}
