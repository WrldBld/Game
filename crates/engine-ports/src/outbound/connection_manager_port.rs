use async_trait::async_trait;
use wrldbldr_domain::{ConnectionId, PlayerCharacterId, WorldId};

use super::{ConnectedUser, ConnectionInfo, UserJoinedEvent, WorldRole};

/// Port for connection management.
///
/// Outbound dependency: implemented by adapters, depended on by the application.
#[async_trait]
pub trait ConnectionManagerPort: Send + Sync {
    /// Register a new connection
    async fn register_connection(&self, connection_id: ConnectionId, client_id: String, user_id: String);

    /// Join a world (assumes validation already done by use case)
    async fn join_world(
        &self,
        connection_id: ConnectionId,
        world_id: WorldId,
        role: WorldRole,
        pc_id: Option<PlayerCharacterId>,
        spectate_pc_id: Option<PlayerCharacterId>,
    ) -> Result<Vec<ConnectedUser>, String>;

    /// Leave a world
    async fn leave_world(&self, connection_id: ConnectionId) -> Option<(WorldId, WorldRole)>;

    /// Get connection info
    async fn get_connection(&self, connection_id: ConnectionId) -> Option<ConnectionInfo>;

    /// Set spectate target
    async fn set_spectate_target(&self, connection_id: ConnectionId, pc_id: Option<PlayerCharacterId>);

    /// Get world connections
    async fn get_world_connections(&self, world_id: WorldId) -> Vec<ConnectionId>;

    /// Send to connection
    async fn send_to_connection(&self, connection_id: ConnectionId, user_joined: UserJoinedEvent);

    /// Get the current DM user ID for a world (None if no DM connected)
    async fn get_dm_user_id(&self, world_id: WorldId) -> Option<String>;
}
