//! Observation Use Case Adapters
//!
//! Implements observation-related ports by wrapping existing infrastructure.

use std::sync::Arc;
use uuid::Uuid;

use wrldbldr_domain::entities::NpcObservation;
use wrldbldr_engine_app::application::use_cases::{
    ApproachEventData, LocationEventData, ObservationRepositoryPort, WorldMessagePort,
};
use wrldbldr_engine_ports::outbound::ObservationRepositoryPort as PortObservationRepositoryPort;
use wrldbldr_protocol::ServerMessage;

use crate::infrastructure::world_connection_manager::SharedWorldConnectionManager;

/// Adapter for ObservationRepositoryPort
pub struct ObservationRepositoryAdapter {
    repo: Arc<dyn PortObservationRepositoryPort>,
}

impl ObservationRepositoryAdapter {
    pub fn new(repo: Arc<dyn PortObservationRepositoryPort>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl ObservationRepositoryPort for ObservationRepositoryAdapter {
    async fn upsert(&self, observation: &NpcObservation) -> Result<(), String> {
        self.repo.upsert(observation).await
    }
}

/// Adapter for WorldMessagePort
pub struct WorldMessageAdapter {
    manager: SharedWorldConnectionManager,
}

impl WorldMessageAdapter {
    pub fn new(manager: SharedWorldConnectionManager) -> Self {
        Self { manager }
    }
}

#[async_trait::async_trait]
impl WorldMessagePort for WorldMessageAdapter {
    async fn send_to_user(&self, user_id: &str, world_id: Uuid, event: ApproachEventData) {
        let message = ServerMessage::NpcApproach {
            npc_id: event.npc_id,
            npc_name: event.npc_name,
            npc_sprite: event.npc_sprite,
            description: event.description,
            reveal: event.reveal,
        };

        self.manager.send_to_user(user_id, world_id, message).await;
    }

    async fn broadcast_to_world(&self, world_id: Uuid, event: LocationEventData) {
        let message = ServerMessage::LocationEvent {
            region_id: event.region_id,
            description: event.description,
        };

        self.manager.broadcast_to_world(world_id, message).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approach_event_data() {
        let event = ApproachEventData {
            npc_id: "npc-123".to_string(),
            npc_name: "Test NPC".to_string(),
            npc_sprite: Some("sprite.png".to_string()),
            description: "A figure approaches".to_string(),
            reveal: true,
        };

        assert_eq!(event.npc_name, "Test NPC");
        assert!(event.reveal);
    }

    #[test]
    fn test_location_event_data() {
        let event = LocationEventData {
            region_id: "region-456".to_string(),
            description: "A loud noise echoes".to_string(),
        };

        assert!(!event.description.is_empty());
    }
}
