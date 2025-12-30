//! Types for world connection manager ports.

use uuid::Uuid;

use crate::outbound::use_case_types::WorldRole;

/// Error types for world connection manager operations
#[derive(Debug, thiserror::Error)]
pub enum ConnectionManagerError {
    /// The specified world was not found
    #[error("World not found: {0}")]
    WorldNotFound(Uuid),

    /// The DM is not connected to the specified world
    #[error("DM not connected to world: {0}")]
    DmNotConnected(Uuid),

    /// The player was not found for the given PC
    #[error("Player not found for PC: {0}")]
    PlayerNotFound(Uuid),

    /// The user was not found
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// Join operation failed
    #[error("Failed to join world: {0}")]
    JoinFailed(String),
}

/// Information about the DM in a world
#[derive(Debug, Clone)]
pub struct DmInfo {
    /// User ID of the DM
    pub user_id: String,
    /// Display name (if known)
    pub username: Option<String>,
    /// Number of active connections (multi-screen support)
    pub connection_count: usize,
}

/// Information about a connected user
#[derive(Debug, Clone)]
pub struct ConnectedUserInfo {
    /// User ID
    pub user_id: String,
    /// Display name (if known)
    pub username: Option<String>,
    /// Role in the world
    pub role: WorldRole,
    /// Player character ID (for Player role)
    pub pc_id: Option<Uuid>,
    /// Number of active connections
    pub connection_count: u32,
}

/// Statistics about connections
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// Total number of active connections
    pub total_connections: usize,
    /// Number of worlds with active connections
    pub total_worlds: usize,
    /// Number of DM connections
    pub dm_connections: usize,
    /// Number of player connections
    pub player_connections: usize,
    /// Number of spectator connections
    pub spectator_connections: usize,
}

/// Context information about a connection for request handling
///
/// This DTO provides all the connection state needed by WebSocket handlers
/// to build RequestContext without exposing infrastructure details.
#[derive(Debug, Clone)]
pub struct ConnectionContext {
    /// Unique connection identifier
    pub connection_id: Uuid,
    /// User ID (may have multiple connections with same user_id)
    pub user_id: String,
    /// Display name (if known)
    pub username: Option<String>,
    /// World this connection is joined to (None if not in a world)
    pub world_id: Option<Uuid>,
    /// Role in the world (None if not in a world)
    pub role: Option<WorldRole>,
    /// Player character ID (for Player role)
    pub pc_id: Option<Uuid>,
    /// Spectate target PC (for Spectator role)
    pub spectate_pc_id: Option<Uuid>,
}

impl ConnectionContext {
    /// Check if this connection is in a world
    pub fn is_in_world(&self) -> bool {
        self.world_id.is_some()
    }

    /// Check if this connection is a DM
    pub fn is_dm(&self) -> bool {
        self.role == Some(WorldRole::DM)
    }

    /// Check if this connection is a Player
    pub fn is_player(&self) -> bool {
        self.role == Some(WorldRole::Player)
    }

    /// Check if this connection is a Spectator
    pub fn is_spectator(&self) -> bool {
        self.role == Some(WorldRole::Spectator)
    }
}
