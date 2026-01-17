//! World session storage wrapper.

use std::sync::Arc;

use uuid::Uuid;

use crate::api::connections::ConnectionManager;
use crate::infrastructure::ports::{ConnectionInfo, SessionError};
use wrldbldr_domain::{PlayerCharacterId, WorldId, WorldRole};

/// World session wrapper for use cases.
pub struct WorldSession {
    connections: Arc<ConnectionManager>,
}

impl WorldSession {
    pub fn new(connections: Arc<ConnectionManager>) -> Self {
        Self { connections }
    }

    pub async fn set_user_id(&self, connection_id: Uuid, user_id: String) {
        self.connections.set_user_id(connection_id, user_id).await;
    }

    pub async fn join_world(
        &self,
        connection_id: Uuid,
        world_id: WorldId,
        role: WorldRole,
        pc_id: Option<PlayerCharacterId>,
    ) -> Result<(), SessionError> {
        self.connections
            .join_world(connection_id, world_id, role, pc_id)
            .await
            .map_err(SessionError::from)
    }

    pub async fn get_world_connections(&self, world_id: WorldId) -> Vec<ConnectionInfo> {
        self.connections
            .get_world_connections(world_id)
            .await
            .iter()
            .map(ConnectionInfo::from)
            .collect()
    }

    pub async fn get_connection(&self, connection_id: Uuid) -> Option<ConnectionInfo> {
        self.connections
            .get(connection_id)
            .await
            .as_ref()
            .map(ConnectionInfo::from)
    }
}
