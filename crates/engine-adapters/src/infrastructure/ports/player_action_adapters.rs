//! Player Action Use Case Adapters
//!
//! Implements player action-related ports by wrapping existing services.

use std::sync::Arc;

use wrldbldr_domain::{ActionId, PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::outbound::{
    ClockPort, PlayerAction, PlayerActionQueuePort, PlayerActionQueueServicePort,
};

/// Adapter for PlayerActionQueueServicePort implementing PlayerActionQueuePort.
pub struct PlayerActionQueueAdapter {
    service: Arc<dyn PlayerActionQueueServicePort>,
    clock: Arc<dyn ClockPort>,
}

impl PlayerActionQueueAdapter {
    pub fn new(service: Arc<dyn PlayerActionQueueServicePort>, clock: Arc<dyn ClockPort>) -> Self {
        Self { service, clock }
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
            timestamp: self.clock.now(),
        };

        self.service
            .enqueue(action)
            .await
            .map(ActionId::from_uuid)
            .map_err(|e| e.to_string())
    }

    async fn depth(&self) -> Result<usize, String> {
        self.service.depth().await.map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    // Tests would require mock services
}
