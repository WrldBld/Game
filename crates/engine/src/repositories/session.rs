//! World session storage wrapper.

use std::sync::Arc;

use uuid::Uuid;

use crate::infrastructure::ports::{ConnectionInfo, SessionError, WorldRole, WorldSessionPort};
use wrldbldr_domain::{PlayerCharacterId, WorldId};

/// World session wrapper for use cases.
pub struct WorldSession {
    session: Arc<dyn WorldSessionPort>,
}

impl WorldSession {
    pub fn new(session: Arc<dyn WorldSessionPort>) -> Self {
        Self { session }
    }

    pub async fn set_user_id(&self, connection_id: Uuid, user_id: String) {
        self.session.set_user_id(connection_id, user_id).await;
    }

    pub async fn join_world(
        &self,
        connection_id: Uuid,
        world_id: WorldId,
        role: WorldRole,
        pc_id: Option<PlayerCharacterId>,
    ) -> Result<(), SessionError> {
        self.session
            .join_world(connection_id, world_id, role, pc_id)
            .await
    }

    pub async fn get_world_connections(&self, world_id: WorldId) -> Vec<ConnectionInfo> {
        self.session.get_world_connections(world_id).await
    }

    pub async fn get_connection(&self, connection_id: Uuid) -> Option<ConnectionInfo> {
        self.session.get_connection(connection_id).await
    }
}
