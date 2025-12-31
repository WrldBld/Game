//! Connection Manager Adapter
//!
//! Implements `ConnectionManagerPort` by wrapping `WorldConnectionManager`.
//! This adapter bridges the use case layer's abstract port interface with the
//! infrastructure's connection management.

use uuid::Uuid;

use tokio::sync::broadcast;
use wrldbldr_domain::PlayerCharacterId;
use wrldbldr_engine_ports::{
    inbound::{ConnectedUser, ConnectionInfo, UserJoinedEvent, WorldRole as UseCaseWorldRole},
    outbound::ConnectionManagerPort,
};
use wrldbldr_protocol::{ServerMessage, WorldRole as ProtocolWorldRole};

use crate::infrastructure::world_connection_manager::{
    ConnectionInfo as InfraConnectionInfo, SharedWorldConnectionManager,
};

/// Adapter that implements ConnectionManagerPort using WorldConnectionManager
pub struct ConnectionManagerAdapter {
    manager: SharedWorldConnectionManager,
}

impl ConnectionManagerAdapter {
    /// Create a new adapter wrapping the given WorldConnectionManager
    pub fn new(manager: SharedWorldConnectionManager) -> Self {
        Self { manager }
    }

    /// Convert use case WorldRole to protocol WorldRole
    fn convert_role_to_protocol(role: UseCaseWorldRole) -> ProtocolWorldRole {
        match role {
            UseCaseWorldRole::DM => ProtocolWorldRole::Dm,
            UseCaseWorldRole::Player => ProtocolWorldRole::Player,
            UseCaseWorldRole::Spectator => ProtocolWorldRole::Spectator,
        }
    }

    /// Convert protocol WorldRole to use case WorldRole
    fn convert_role_from_protocol(role: ProtocolWorldRole) -> UseCaseWorldRole {
        match role {
            ProtocolWorldRole::Dm => UseCaseWorldRole::DM,
            ProtocolWorldRole::Player => UseCaseWorldRole::Player,
            ProtocolWorldRole::Spectator | ProtocolWorldRole::Unknown => {
                UseCaseWorldRole::Spectator // Default unknown to Spectator (least privileged)
            }
        }
    }

    /// Convert infrastructure ConnectionInfo to use case ConnectionInfo
    fn convert_connection_info(info: &InfraConnectionInfo) -> ConnectionInfo {
        ConnectionInfo {
            connection_id: info.connection_id,
            client_id: info.connection_id.to_string(), // Using connection_id as client_id
            user_id: info.user_id.clone(),
            world_id: info.world_id,
            role: info.role.map(Self::convert_role_from_protocol),
            pc_id: info.pc_id,
            spectate_pc_id: info.spectate_pc_id,
        }
    }

    /// Convert protocol ConnectedUser to use case ConnectedUser
    fn convert_connected_user(user: &wrldbldr_protocol::ConnectedUser) -> ConnectedUser {
        ConnectedUser {
            user_id: user.user_id.clone(),
            role: Self::convert_role_from_protocol(user.role),
            pc_id: user
                .pc_id
                .as_ref()
                .and_then(|id| Uuid::parse_str(id).ok().map(PlayerCharacterId::from_uuid)),
            pc_name: None, // Protocol doesn't have pc_name, would need to fetch
        }
    }
}

#[async_trait::async_trait]
impl ConnectionManagerPort for ConnectionManagerAdapter {
    async fn register_connection(&self, connection_id: Uuid, client_id: String, user_id: String) {
        // Create a broadcast channel for this connection
        let (sender, _) = broadcast::channel::<ServerMessage>(256);

        self.manager
            .register_connection(connection_id, client_id, user_id, sender)
            .await;
    }

    async fn join_world(
        &self,
        connection_id: Uuid,
        world_id: Uuid,
        role: UseCaseWorldRole,
        pc_id: Option<Uuid>,
        spectate_pc_id: Option<Uuid>,
    ) -> Result<Vec<ConnectedUser>, String> {
        let protocol_role = Self::convert_role_to_protocol(role);

        self.manager
            .join_world(
                connection_id,
                world_id,
                protocol_role,
                pc_id,
                spectate_pc_id,
            )
            .await
            .map(|users| users.iter().map(Self::convert_connected_user).collect())
            .map_err(|e| format!("{:?}", e))
    }

    async fn leave_world(&self, connection_id: Uuid) -> Option<(Uuid, UseCaseWorldRole)> {
        self.manager
            .leave_world(connection_id)
            .await
            .map(|(world_id, role)| (world_id, Self::convert_role_from_protocol(role)))
    }

    async fn get_connection(&self, connection_id: Uuid) -> Option<ConnectionInfo> {
        self.manager
            .get_connection(connection_id)
            .await
            .map(|info| Self::convert_connection_info(&info))
    }

    async fn set_spectate_target(&self, connection_id: Uuid, pc_id: Option<Uuid>) {
        self.manager.set_spectate_target(connection_id, pc_id).await;
    }

    async fn get_world_connections(&self, world_id: Uuid) -> Vec<Uuid> {
        self.manager.get_world_connections(world_id).await
    }

    async fn send_to_connection(&self, connection_id: Uuid, user_joined: UserJoinedEvent) {
        // Convert UserJoinedEvent to ServerMessage
        let message = ServerMessage::UserJoined {
            user_id: user_joined.user_id.clone(),
            username: user_joined.pc.as_ref().map(|pc| pc.name.clone()),
            role: Self::convert_role_to_protocol(user_joined.role),
            pc: user_joined.pc.as_ref().map(|pc| {
                serde_json::json!({
                    "id": pc.id.to_string(),
                    "name": pc.name,
                })
            }),
        };

        self.manager
            .send_to_connection(connection_id, message)
            .await;
    }

    async fn get_dm_user_id(&self, world_id: Uuid) -> Option<String> {
        self.manager
            .get_dm_info(&world_id)
            .await
            .map(|info| info.user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_conversion_round_trip() {
        let dm = UseCaseWorldRole::DM;
        let player = UseCaseWorldRole::Player;
        let spectator = UseCaseWorldRole::Spectator;

        assert!(matches!(
            ConnectionManagerAdapter::convert_role_from_protocol(
                ConnectionManagerAdapter::convert_role_to_protocol(dm)
            ),
            UseCaseWorldRole::DM
        ));

        assert!(matches!(
            ConnectionManagerAdapter::convert_role_from_protocol(
                ConnectionManagerAdapter::convert_role_to_protocol(player)
            ),
            UseCaseWorldRole::Player
        ));

        assert!(matches!(
            ConnectionManagerAdapter::convert_role_from_protocol(
                ConnectionManagerAdapter::convert_role_to_protocol(spectator)
            ),
            UseCaseWorldRole::Spectator
        ));
    }
}
