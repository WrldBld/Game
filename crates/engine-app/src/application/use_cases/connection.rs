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

use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::outbound::BroadcastPort;

use super::errors::ConnectionError;

// =============================================================================
// Input/Output Types
// =============================================================================

/// World role for connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldRole {
    DM,
    Player,
    Spectator,
}

/// Input for joining a world
#[derive(Debug, Clone)]
pub struct JoinWorldInput {
    /// World to join
    pub world_id: WorldId,
    /// Role to join as
    pub role: WorldRole,
    /// PC to use (for Player role)
    pub pc_id: Option<PlayerCharacterId>,
    /// PC to spectate (for Spectator role)
    pub spectate_pc_id: Option<PlayerCharacterId>,
}

/// Input for setting spectate target
#[derive(Debug, Clone)]
pub struct SetSpectateTargetInput {
    /// PC to spectate
    pub pc_id: PlayerCharacterId,
}

/// Connected user information
#[derive(Debug, Clone)]
pub struct ConnectedUser {
    pub user_id: String,
    pub role: WorldRole,
    pub pc_id: Option<PlayerCharacterId>,
    pub pc_name: Option<String>,
}

/// PC data for responses
#[derive(Debug, Clone)]
pub struct PcData {
    pub id: String,
    pub name: String,
    pub user_id: String,
    pub world_id: String,
    pub current_location_id: String,
    pub current_region_id: Option<String>,
    pub description: Option<String>,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// Result of joining a world
#[derive(Debug, Clone)]
pub struct JoinWorldResult {
    /// World ID joined
    pub world_id: WorldId,
    /// World snapshot (JSON value for now)
    pub snapshot: serde_json::Value,
    /// List of connected users
    pub connected_users: Vec<ConnectedUser>,
    /// Your role in the world
    pub your_role: WorldRole,
    /// Your PC data (if Player role)
    pub your_pc: Option<PcData>,
}

/// Result of leaving a world
#[derive(Debug, Clone)]
pub struct LeaveWorldResult {
    /// Successfully left
    pub left: bool,
}

/// Result of setting spectate target
#[derive(Debug, Clone)]
pub struct SpectateTargetResult {
    /// Target PC ID
    pub pc_id: PlayerCharacterId,
    /// Target PC name
    pub pc_name: String,
}

// =============================================================================
// Connection Manager Port
// =============================================================================

/// Port for connection management
#[async_trait::async_trait]
pub trait ConnectionManagerPort: Send + Sync {
    /// Register a new connection
    async fn register_connection(&self, connection_id: uuid::Uuid, client_id: String, user_id: String);

    /// Join a world
    async fn join_world(
        &self,
        connection_id: uuid::Uuid,
        world_id: uuid::Uuid,
        role: WorldRole,
        pc_id: Option<uuid::Uuid>,
        spectate_pc_id: Option<uuid::Uuid>,
    ) -> Result<Vec<ConnectedUser>, String>;

    /// Leave a world
    async fn leave_world(&self, connection_id: uuid::Uuid) -> Option<(uuid::Uuid, WorldRole)>;

    /// Get connection info
    async fn get_connection(&self, connection_id: uuid::Uuid) -> Option<ConnectionInfo>;

    /// Set spectate target
    async fn set_spectate_target(&self, connection_id: uuid::Uuid, pc_id: Option<uuid::Uuid>);

    /// Get world connections
    async fn get_world_connections(&self, world_id: uuid::Uuid) -> Vec<uuid::Uuid>;

    /// Send to connection
    async fn send_to_connection(&self, connection_id: uuid::Uuid, user_joined: UserJoinedEvent);

    /// Broadcast to world
    async fn broadcast_to_world(&self, world_id: uuid::Uuid, event: UserLeftEvent);
}

/// Connection info
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub connection_id: uuid::Uuid,
    pub client_id: String,
    pub user_id: String,
    pub world_id: Option<uuid::Uuid>,
    pub role: Option<WorldRole>,
    pub pc_id: Option<uuid::Uuid>,
    pub spectate_pc_id: Option<uuid::Uuid>,
}

impl ConnectionInfo {
    pub fn is_spectator(&self) -> bool {
        matches!(self.role, Some(WorldRole::Spectator))
    }
}

/// User joined event
#[derive(Debug, Clone)]
pub struct UserJoinedEvent {
    pub user_id: String,
    pub role: WorldRole,
    pub pc: Option<PcData>,
}

/// User left event
#[derive(Debug, Clone)]
pub struct UserLeftEvent {
    pub user_id: String,
}

// =============================================================================
// World Service Port
// =============================================================================

/// Port for world service operations
#[async_trait::async_trait]
pub trait WorldServicePort: Send + Sync {
    /// Export world snapshot
    async fn export_world_snapshot(&self, world_id: WorldId) -> Result<serde_json::Value, String>;
}

/// Port for player character service
#[async_trait::async_trait]
pub trait PlayerCharacterServicePort: Send + Sync {
    /// Get PC by ID
    async fn get_pc(&self, pc_id: PlayerCharacterId) -> Result<Option<PcData>, String>;
}

/// Port for directorial context
#[async_trait::async_trait]
pub trait DirectorialContextPort: Send + Sync {
    /// Get directorial context
    async fn get(
        &self,
        world_id: &WorldId,
    ) -> Result<Option<super::scene::DirectorialContextData>, String>;
}

/// Port for world state
pub trait WorldStatePort: Send + Sync {
    /// Set directorial context
    fn set_directorial_context(
        &self,
        world_id: &WorldId,
        context: super::scene::DirectorialContextData,
    );
}

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

        // Register connection
        self.connection_manager
            .register_connection(connection_id, connection_id.to_string(), user_id.clone())
            .await;

        // Join the world
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
            .map_err(|e| ConnectionError::ConnectionFailed(e))?;

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
                let user_left = UserLeftEvent {
                    user_id: info.user_id,
                };
                self.connection_manager
                    .broadcast_to_world(world_id, user_left)
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
            .map_err(|e| ConnectionError::Database(e))?
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
