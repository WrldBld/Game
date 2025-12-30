//! Connection Use Case
//!
//! Handles connection lifecycle: join/leave world, spectator management.
//!
//! # Responsibilities
//!
//! - Join a world with a specific role (DM, Player, Spectator)
//! - Leave a world
//! - Set spectate target for spectators
//! - Broadcast connection events
//!
//! # Architecture Note
//!
//! Connection operations affect the world state and notify other participants.
//! The use case coordinates between connection management and player data.

use std::sync::Arc;
use tracing::{debug, info, warn};

use async_trait::async_trait;
use uuid::Uuid;
use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::inbound::ConnectionUseCasePort;
use wrldbldr_engine_ports::outbound::{BroadcastPort, GameEvent};

use super::errors::ConnectionError;

// Import services
use crate::application::services::{JoinValidation, WorldSessionPolicy};

// Import port traits from engine-ports
pub use wrldbldr_engine_ports::inbound::{
    ConnectionManagerPort, DirectorialContextPort, PlayerCharacterServicePort, WorldServicePort,
};

// Import types from engine-ports
pub use wrldbldr_engine_ports::outbound::{
    ConnectedUser, ConnectionInfo, JoinWorldInput, JoinWorldResult, LeaveWorldResult, PcData,
    SetSpectateTargetInput, SpectateTargetResult, UserJoinedEvent, WorldRole,
};

// WorldStatePort is imported from engine-ports via scene.rs
pub use super::scene::WorldStatePort;

// =============================================================================
// Connection Use Case
// =============================================================================

/// Use case for connection operations
pub struct ConnectionUseCase {
    connection_manager: Arc<dyn ConnectionManagerPort>,
    world_service: Arc<dyn WorldServicePort>,
    pc_service: Arc<dyn PlayerCharacterServicePort>,
    directorial_repo: Arc<dyn DirectorialContextPort>,
    world_state: Arc<dyn WorldStatePort>,
    broadcast: Arc<dyn BroadcastPort>,
    session_policy: WorldSessionPolicy,
}

impl ConnectionUseCase {
    /// Create a new ConnectionUseCase with all dependencies
    pub fn new(
        connection_manager: Arc<dyn ConnectionManagerPort>,
        world_service: Arc<dyn WorldServicePort>,
        pc_service: Arc<dyn PlayerCharacterServicePort>,
        directorial_repo: Arc<dyn DirectorialContextPort>,
        world_state: Arc<dyn WorldStatePort>,
        broadcast: Arc<dyn BroadcastPort>,
    ) -> Self {
        Self {
            connection_manager,
            world_service,
            pc_service,
            directorial_repo,
            world_state,
            broadcast,
            session_policy: WorldSessionPolicy::new(),
        }
    }

    /// Join a world
    pub async fn join_world(
        &self,
        connection_id: uuid::Uuid,
        user_id: String,
        input: JoinWorldInput,
    ) -> Result<JoinWorldResult, ConnectionError> {
        info!(
            world_id = %input.world_id,
            role = ?input.role,
            pc_id = ?input.pc_id,
            "User joining world"
        );

        // Validate join request using policy (business rules)
        let current_dm_user_id = self
            .connection_manager
            .get_dm_user_id(*input.world_id.as_uuid())
            .await;

        let validation = self.session_policy.validate_join(
            input.role,
            &user_id,
            input.pc_id.map(|id| *id.as_uuid()),
            input.spectate_pc_id.map(|id| *id.as_uuid()),
            current_dm_user_id.as_deref(),
        );

        if let JoinValidation::Denied(err) = validation {
            warn!(
                world_id = %input.world_id,
                role = ?input.role,
                error = ?err,
                "Join request denied by policy"
            );
            return Err(ConnectionError::from(err));
        }

        // Register connection
        self.connection_manager
            .register_connection(connection_id, connection_id.to_string(), user_id.clone())
            .await;

        // Join the world (validation already done, this is just state management)
        let connected_users = self
            .connection_manager
            .join_world(
                connection_id,
                *input.world_id.as_uuid(),
                input.role,
                input.pc_id.map(|id| *id.as_uuid()),
                input.spectate_pc_id.map(|id| *id.as_uuid()),
            )
            .await
            .map_err(ConnectionError::ConnectionFailed)?;

        // Get world snapshot
        let snapshot = self
            .world_service
            .export_world_snapshot(input.world_id)
            .await
            .unwrap_or_else(|e| {
                warn!(error = %e, "Failed to get world snapshot");
                serde_json::json!({})
            });

        // Load persisted directorial context
        if let Ok(Some(context)) = self.directorial_repo.get(&input.world_id).await {
            self.world_state
                .set_directorial_context(&input.world_id, context);
            debug!(world_id = %input.world_id, "Loaded persisted directorial context");
        }

        // Get PC data if Player role
        let pc_data = if input.role == WorldRole::Player {
            if let Some(pc_id) = input.pc_id {
                self.pc_service.get_pc(pc_id).await.ok().flatten()
            } else {
                debug!("Player joined without PC ID");
                None
            }
        } else {
            None
        };

        // Broadcast UserJoined to other connections
        let user_joined = UserJoinedEvent {
            user_id: user_id.clone(),
            role: input.role,
            pc: pc_data.clone(),
        };

        let world_connections = self
            .connection_manager
            .get_world_connections(*input.world_id.as_uuid())
            .await;

        for other_conn_id in world_connections {
            if other_conn_id != connection_id {
                self.connection_manager
                    .send_to_connection(other_conn_id, user_joined.clone())
                    .await;
            }
        }

        info!(
            world_id = %input.world_id,
            connected_users = connected_users.len(),
            "User joined world successfully"
        );

        Ok(JoinWorldResult {
            world_id: input.world_id,
            snapshot,
            connected_users,
            your_role: input.role,
            your_pc: pc_data,
        })
    }

    /// Leave a world
    pub async fn leave_world(
        &self,
        connection_id: uuid::Uuid,
    ) -> Result<LeaveWorldResult, ConnectionError> {
        info!(connection_id = %connection_id, "User leaving world");

        // Get connection info before leaving
        let conn_info = self.connection_manager.get_connection(connection_id).await;

        // Leave the world
        if let Some((world_id, _role)) = self.connection_manager.leave_world(connection_id).await {
            // Broadcast UserLeft to remaining users
            if let Some(info) = conn_info {
                self.broadcast
                    .broadcast(
                        WorldId::from_uuid(world_id),
                        GameEvent::PlayerLeft {
                            user_id: info.user_id,
                        },
                    )
                    .await;
            }
            info!(world_id = %world_id, "User left world");
        }

        Ok(LeaveWorldResult { left: true })
    }

    /// Set spectate target
    pub async fn set_spectate_target(
        &self,
        connection_id: uuid::Uuid,
        input: SetSpectateTargetInput,
    ) -> Result<SpectateTargetResult, ConnectionError> {
        info!(
            connection_id = %connection_id,
            pc_id = %input.pc_id,
            "Setting spectate target"
        );

        // Verify connection is a spectator
        let conn_info = self
            .connection_manager
            .get_connection(connection_id)
            .await
            .ok_or(ConnectionError::NotConnected)?;

        if !conn_info.is_spectator() {
            return Err(ConnectionError::InvalidSpectateTarget(
                "Only spectators can change spectate target".to_string(),
            ));
        }

        // Get PC name
        let pc_data = self
            .pc_service
            .get_pc(input.pc_id)
            .await
            .map_err(ConnectionError::Database)?
            .ok_or(ConnectionError::PcNotFound(input.pc_id))?;

        // Set spectate target
        self.connection_manager
            .set_spectate_target(connection_id, Some(*input.pc_id.as_uuid()))
            .await;

        info!(
            pc_id = %input.pc_id,
            pc_name = %pc_data.name,
            "Spectate target changed"
        );

        Ok(SpectateTargetResult {
            pc_id: input.pc_id,
            pc_name: pc_data.name,
        })
    }
}

// =============================================================================
// ConnectionUseCasePort Implementation
// =============================================================================

#[async_trait]
impl ConnectionUseCasePort for ConnectionUseCase {
    async fn join_world(
        &self,
        connection_id: Uuid,
        user_id: String,
        input: JoinWorldInput,
    ) -> Result<JoinWorldResult, ConnectionError> {
        self.join_world(connection_id, user_id, input).await
    }

    async fn leave_world(&self, connection_id: Uuid) -> Result<LeaveWorldResult, ConnectionError> {
        self.leave_world(connection_id).await
    }

    async fn set_spectate_target(
        &self,
        connection_id: Uuid,
        input: SetSpectateTargetInput,
    ) -> Result<SpectateTargetResult, ConnectionError> {
        self.set_spectate_target(connection_id, input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_role_variants() {
        assert_eq!(WorldRole::DM, WorldRole::DM);
        assert_eq!(WorldRole::Player, WorldRole::Player);
        assert_eq!(WorldRole::Spectator, WorldRole::Spectator);
        assert_ne!(WorldRole::DM, WorldRole::Player);
    }

    #[test]
    fn test_connection_info_is_spectator() {
        let spectator = ConnectionInfo {
            connection_id: uuid::Uuid::new_v4(),
            client_id: "test".to_string(),
            user_id: "user".to_string(),
            world_id: None,
            role: Some(WorldRole::Spectator),
            pc_id: None,
            spectate_pc_id: None,
        };

        let player = ConnectionInfo {
            connection_id: uuid::Uuid::new_v4(),
            client_id: "test".to_string(),
            user_id: "user".to_string(),
            world_id: None,
            role: Some(WorldRole::Player),
            pc_id: None,
            spectate_pc_id: None,
        };

        assert!(spectator.is_spectator());
        assert!(!player.is_spectator());
    }
}
