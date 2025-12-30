//! World Connection Manager Port - Interface for managing world-scoped WebSocket connections
//!
//! This port abstracts WebSocket connection management from the application layer,
//! allowing use cases to query connection state and broadcast messages without
//! depending on WebSocket infrastructure.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        APPLICATION LAYER                                 │
//! │                                                                          │
//! │  Use cases need to:                                                      │
//! │  - Check if DM is connected                                              │
//! │  - Find which user controls a PC                                         │
//! │  - Get list of connected users                                           │
//! │                                                                          │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//!                   ┌────────────▼─────────────┐
//!                   │ WorldConnectionManagerPort│ (trait defined here)
//!                   └────────────┬─────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                         ADAPTER LAYER                                    │
//! │                                                                          │
//! │  WorldConnectionManager implements WorldConnectionManagerPort            │
//! │  - Manages WebSocket connection state                                    │
//! │  - Handles join/leave world operations                                   │
//! │  - Routes messages to appropriate connections                            │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Design Notes
//!
//! This port exposes connection query and management operations needed by the
//! application layer. Message broadcasting is handled separately by `BroadcastPort`
//! using domain-level `GameEvent` types.

use async_trait::async_trait;
use uuid::Uuid;

use wrldbldr_domain::WorldId;

use super::use_case_types::WorldRole;

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

/// Port for managing world-scoped WebSocket connections
///
/// This trait provides query access to connection state and management
/// operations for the application layer.
///
/// # Usage
///
/// Application services and use cases should depend on this trait to
/// query connection state without importing WebSocket infrastructure.
///
/// # Testing
///
/// Enable the `testing` feature to get mock implementations via mockall.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait WorldConnectionManagerPort: Send + Sync {
    // =========================================================================
    // Query Methods
    // =========================================================================

    /// Check if a DM is connected to the specified world
    async fn has_dm(&self, world_id: &WorldId) -> bool;

    /// Get information about the DM in a world
    ///
    /// Returns `None` if no DM is connected.
    async fn get_dm_info(&self, world_id: &WorldId) -> Option<DmInfo>;

    /// Get all connected users in a world
    async fn get_connected_users(&self, world_id: WorldId) -> Vec<ConnectedUserInfo>;

    /// Get a user's role in a world
    ///
    /// Returns `None` if the user is not in the world.
    async fn get_user_role(&self, world_id: &WorldId, user_id: &str) -> Option<WorldRole>;

    /// Find which user is playing a specific PC
    ///
    /// Returns the user ID if a player is controlling the PC.
    async fn find_player_for_pc(&self, world_id: &WorldId, pc_id: &Uuid) -> Option<String>;

    /// Get all PCs in a world with their controlling users
    ///
    /// Returns a list of (pc_id, user_id) pairs.
    async fn get_world_pcs(&self, world_id: &WorldId) -> Vec<(Uuid, String)>;

    /// Get all world IDs that have active connections
    async fn get_all_world_ids(&self) -> Vec<Uuid>;

    /// Get connection statistics
    async fn stats(&self) -> ConnectionStats;

    // =========================================================================
    // Client ID Lookup Methods
    // =========================================================================

    /// Get user ID by client ID
    ///
    /// Client ID is the string identifier used by WebSocket handlers.
    async fn get_user_id_by_client_id(&self, client_id: &str) -> Option<String>;

    /// Check if a client is a DM
    async fn is_dm_by_client_id(&self, client_id: &str) -> bool;

    /// Get world ID by client ID
    async fn get_world_id_by_client_id(&self, client_id: &str) -> Option<Uuid>;

    // =========================================================================
    // Connection Context Methods (for handlers)
    // =========================================================================

    /// Get full connection context by connection ID
    ///
    /// Returns all connection state needed by handlers to build RequestContext.
    /// This is the primary method for WebSocket handlers to get connection info.
    async fn get_connection_context(&self, connection_id: Uuid) -> Option<ConnectionContext>;

    /// Get full connection context by client ID string
    ///
    /// This is commonly used by handlers that receive client_id as a string.
    async fn get_connection_by_client_id(&self, client_id: &str) -> Option<ConnectionContext>;

    /// Check if a connection is a spectator
    async fn is_spectator_by_client_id(&self, client_id: &str) -> bool;

    /// Get PC ID for a connection (if Player role)
    async fn get_pc_id_by_client_id(&self, client_id: &str) -> Option<Uuid>;

    // =========================================================================
    // Broadcast Methods (for handlers)
    // =========================================================================

    /// Broadcast a serialized message to all connections in a world
    ///
    /// The message should be a JSON-serialized ServerMessage.
    async fn broadcast_to_world(&self, world_id: Uuid, message: serde_json::Value);

    /// Broadcast a serialized message to DM connections in a world
    async fn broadcast_to_dms(&self, world_id: Uuid, message: serde_json::Value);

    /// Broadcast a serialized message to player connections in a world
    async fn broadcast_to_players(&self, world_id: Uuid, message: serde_json::Value);

    /// Broadcast a serialized message to all worlds
    async fn broadcast_to_all_worlds(&self, message: serde_json::Value);

    // =========================================================================
    // Connection Lifecycle Methods
    // =========================================================================

    /// Unregister a connection when it disconnects
    ///
    /// This cleans up connection state and notifies other users in the world.
    async fn unregister_connection(&self, connection_id: Uuid);
}
