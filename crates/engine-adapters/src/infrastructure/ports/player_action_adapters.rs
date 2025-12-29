//! Player Action Use Case Adapters
//!
//! Implements player action-related ports by wrapping existing services.

use std::sync::Arc;

use wrldbldr_domain::value_objects::{LlmRequestData, PlayerActionData};
use wrldbldr_domain::{ActionId, PlayerCharacterId, WorldId};
use wrldbldr_engine_app::application::services::PlayerActionQueueService;
use wrldbldr_engine_ports::inbound::{DmNotificationPort, PlayerActionQueuePort};
use wrldbldr_engine_ports::outbound::{ProcessingQueuePort, QueuePort};
use wrldbldr_protocol::ServerMessage;

use crate::infrastructure::world_connection_manager::SharedWorldConnectionManager;

/// Adapter for PlayerActionQueueService
///
/// Generic over the queue backend types used by PlayerActionQueueService.
pub struct PlayerActionQueueAdapter<Q, LQ>
where
    Q: QueuePort<PlayerActionData> + Send + Sync + 'static,
    LQ: ProcessingQueuePort<LlmRequestData> + Send + Sync + 'static,
{
    service: Arc<PlayerActionQueueService<Q, LQ>>,
}

impl<Q, LQ> PlayerActionQueueAdapter<Q, LQ>
where
    Q: QueuePort<PlayerActionData> + Send + Sync + 'static,
    LQ: ProcessingQueuePort<LlmRequestData> + Send + Sync + 'static,
{
    pub fn new(service: Arc<PlayerActionQueueService<Q, LQ>>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl<Q, LQ> PlayerActionQueuePort for PlayerActionQueueAdapter<Q, LQ>
where
    Q: QueuePort<PlayerActionData> + Send + Sync + 'static,
    LQ: ProcessingQueuePort<LlmRequestData> + Send + Sync + 'static,
{
    async fn enqueue_action(
        &self,
        world_id: &WorldId,
        player_id: String,
        pc_id: Option<PlayerCharacterId>,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    ) -> Result<ActionId, String> {
        self.service
            .enqueue_action(world_id, player_id, pc_id, action_type, target, dialogue)
            .await
            .map(|id| ActionId::from_uuid(id.into()))
            .map_err(|e| e.to_string())
    }

    async fn depth(&self) -> Result<usize, String> {
        self.service.depth().await.map_err(|e| e.to_string())
    }
}

/// Adapter for DM notification via WorldConnectionManager
pub struct DmNotificationAdapter {
    manager: SharedWorldConnectionManager,
}

impl DmNotificationAdapter {
    pub fn new(manager: SharedWorldConnectionManager) -> Self {
        Self { manager }
    }
}

#[async_trait::async_trait]
impl DmNotificationPort for DmNotificationAdapter {
    async fn notify_action_queued(
        &self,
        world_id: &WorldId,
        action_id: String,
        player_name: String,
        action_type: String,
        queue_depth: usize,
    ) {
        let message = ServerMessage::ActionQueued {
            action_id,
            player_name,
            action_type,
            queue_depth,
        };

        self.manager
            .broadcast_to_dms(*world_id.as_uuid(), message)
            .await;
    }
}

#[cfg(test)]
mod tests {
    // Tests would require mock services
}
