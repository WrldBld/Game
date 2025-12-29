//! Player Action Use Case Adapters
//!
//! Implements player action-related ports by wrapping existing services.

use std::sync::Arc;

use chrono::Utc;
use wrldbldr_domain::{ActionId, PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::inbound::{DmNotificationPort, PlayerActionQueuePort};
use wrldbldr_engine_ports::outbound::{PlayerAction, PlayerActionQueueServicePort};
use wrldbldr_protocol::ServerMessage;

use crate::infrastructure::world_connection_manager::SharedWorldConnectionManager;

/// Adapter for PlayerActionQueueServicePort (outbound) implementing PlayerActionQueuePort (inbound)
pub struct PlayerActionQueueAdapter {
    service: Arc<dyn PlayerActionQueueServicePort>,
}

impl PlayerActionQueueAdapter {
    pub fn new(service: Arc<dyn PlayerActionQueueServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl PlayerActionQueuePort for PlayerActionQueueAdapter {
    async fn enqueue_action(
        &self,
        world_id: &WorldId,
        player_id: String,
        pc_id: Option<PlayerCharacterId>,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    ) -> Result<ActionId, String> {
        let action = PlayerAction {
            world_id: *world_id.as_uuid(),
            player_id,
            pc_id: pc_id.map(|id| *id.as_uuid()),
            action_type,
            target,
            dialogue,
            timestamp: Utc::now(),
        };

        self.service
            .enqueue(action)
            .await
            .map(|id| ActionId::from_uuid(id))
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
