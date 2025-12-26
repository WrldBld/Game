//! World Connection Manager - Manages world-scoped WebSocket connections
//!
//! This module handles the mapping between WebSocket connections and worlds,
//! replacing the session-based connection model with a world-scoped one.
//!
//! # Key Features
//!
//! - **World-scoped connections**: Users connect to a specific world
//! - **Role enforcement**: DM, Player, Spectator roles with different permissions
//! - **Multi-screen DM**: Same user_id can have multiple DM connections
//! - **Spectator mode**: Read-only view of player-visible data
//!
//! # Connection Lifecycle
//!
//! 1. User connects via WebSocket (anonymous connection)
//! 2. User sends `JoinWorld` message with world_id and role
//! 3. Manager validates the join request (DM availability, PC selection, etc.)
//! 4. On success, user receives `WorldJoined` with full snapshot
//! 5. Other users in the world receive `UserJoined`
//! 6. On disconnect, other users receive `UserLeft`

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use wrldbldr_protocol::{ConnectedUser, JoinError, ServerMessage, WorldRole};

// =============================================================================
// Connection Info
// =============================================================================

/// Information about a single WebSocket connection
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
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

    /// Channel to send messages to this connection
    pub message_sender: broadcast::Sender<ServerMessage>,
}

impl ConnectionInfo {
    /// Create a new anonymous connection (not yet joined to a world)
    pub fn new(
        connection_id: Uuid,
        user_id: String,
        message_sender: broadcast::Sender<ServerMessage>,
    ) -> Self {
        Self {
            connection_id,
            user_id,
            username: None,
            world_id: None,
            role: None,
            pc_id: None,
            spectate_pc_id: None,
            message_sender,
        }
    }

    /// Check if this connection is in a world
    pub fn is_in_world(&self) -> bool {
        self.world_id.is_some()
    }

    /// Check if this connection is a DM
    pub fn is_dm(&self) -> bool {
        self.role == Some(WorldRole::Dm)
    }

    /// Check if this connection is a Player
    pub fn is_player(&self) -> bool {
        self.role == Some(WorldRole::Player)
    }

    /// Check if this connection is a Spectator
    pub fn is_spectator(&self) -> bool {
        self.role == Some(WorldRole::Spectator)
    }

    /// Convert to ConnectedUser for protocol
    pub fn to_connected_user(&self) -> ConnectedUser {
        ConnectedUser {
            user_id: self.user_id.clone(),
            username: self.username.clone(),
            role: self.role.unwrap_or(WorldRole::Spectator),
            pc_id: self.pc_id.map(|id| id.to_string()),
            connection_count: 1, // Will be updated by manager
        }
    }
}

// =============================================================================
// World Connection State
// =============================================================================

/// State for a single world's connections
#[derive(Debug, Default)]
struct WorldConnectionState {
    /// All connection IDs in this world
    connections: HashSet<Uuid>,

    /// DM user ID (only one DM allowed per world, but may have multiple screens)
    dm_user_id: Option<String>,

    /// DM connection IDs (same user, multiple screens)
    dm_connections: HashSet<Uuid>,

    /// Player connection IDs
    player_connections: HashSet<Uuid>,

    /// Spectator connection IDs
    spectator_connections: HashSet<Uuid>,
}

impl WorldConnectionState {
    fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    fn has_dm(&self) -> bool {
        self.dm_user_id.is_some()
    }

    #[allow(dead_code)]
    fn dm_user_id(&self) -> Option<&str> {
        self.dm_user_id.as_deref()
    }

    fn add_dm(&mut self, connection_id: Uuid, user_id: &str) -> Result<(), JoinError> {
        if let Some(existing) = &self.dm_user_id {
            if existing != user_id {
                return Err(JoinError::DmAlreadyConnected {
                    existing_user_id: existing.clone(),
                });
            }
        }
        self.dm_user_id = Some(user_id.to_string());
        self.dm_connections.insert(connection_id);
        self.connections.insert(connection_id);
        Ok(())
    }

    fn add_player(&mut self, connection_id: Uuid) {
        self.player_connections.insert(connection_id);
        self.connections.insert(connection_id);
    }

    fn add_spectator(&mut self, connection_id: Uuid) {
        self.spectator_connections.insert(connection_id);
        self.connections.insert(connection_id);
    }

    fn remove(&mut self, connection_id: Uuid, role: WorldRole) {
        self.connections.remove(&connection_id);
        match role {
            WorldRole::Dm => {
                self.dm_connections.remove(&connection_id);
                if self.dm_connections.is_empty() {
                    self.dm_user_id = None;
                }
            }
            WorldRole::Player => {
                self.player_connections.remove(&connection_id);
            }
            WorldRole::Spectator => {
                self.spectator_connections.remove(&connection_id);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }
}

// =============================================================================
// World Connection Manager
// =============================================================================

/// Manager for world-scoped WebSocket connections
///
/// This is the central point for managing which users are connected to which
/// worlds, their roles, and broadcasting messages to appropriate recipients.
#[derive(Debug)]
pub struct WorldConnectionManager {
    /// All connections by connection_id
    connections: RwLock<HashMap<Uuid, ConnectionInfo>>,

    /// World connection states by world_id
    worlds: RwLock<HashMap<Uuid, WorldConnectionState>>,
}

impl Default for WorldConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            worlds: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new connection (not yet joined to a world)
    pub async fn register_connection(
        &self,
        connection_id: Uuid,
        user_id: String,
        message_sender: broadcast::Sender<ServerMessage>,
    ) {
        let info = ConnectionInfo::new(connection_id, user_id, message_sender);
        self.connections.write().await.insert(connection_id, info);
        tracing::debug!(connection_id = %connection_id, "Registered new connection");
    }

    /// Unregister a connection (on disconnect)
    pub async fn unregister_connection(&self, connection_id: Uuid) -> Option<ConnectionInfo> {
        let info = self.connections.write().await.remove(&connection_id)?;

        // Remove from world if joined
        if let (Some(world_id), Some(role)) = (info.world_id, info.role) {
            let mut worlds = self.worlds.write().await;
            if let Some(world_state) = worlds.get_mut(&world_id) {
                world_state.remove(connection_id, role);
                if world_state.is_empty() {
                    worlds.remove(&world_id);
                    tracing::debug!(world_id = %world_id, "World has no more connections, removed");
                }
            }
        }

        tracing::debug!(connection_id = %connection_id, "Unregistered connection");
        Some(info)
    }

    /// Join a world with a specific role
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection joining
    /// * `world_id` - The world to join
    /// * `role` - The role to join as
    /// * `pc_id` - Player character ID (required for Player role)
    /// * `spectate_pc_id` - Target PC to spectate (required for Spectator role)
    ///
    /// # Returns
    ///
    /// On success, returns a list of already-connected users.
    /// On failure, returns the join error.
    pub async fn join_world(
        &self,
        connection_id: Uuid,
        world_id: Uuid,
        role: WorldRole,
        pc_id: Option<Uuid>,
        spectate_pc_id: Option<Uuid>,
    ) -> Result<Vec<ConnectedUser>, JoinError> {
        // Validate role requirements
        match role {
            WorldRole::Player if pc_id.is_none() => {
                return Err(JoinError::PlayerRequiresPc);
            }
            WorldRole::Spectator if spectate_pc_id.is_none() => {
                return Err(JoinError::SpectatorRequiresTarget);
            }
            _ => {}
        }

        // Get user_id first (need separate lock scope)
        let user_id = {
            let connections = self.connections.read().await;
            connections
                .get(&connection_id)
                .ok_or(JoinError::Unauthorized)?
                .user_id
                .clone()
        };

        // Get or create world state and add connection
        {
            let mut worlds = self.worlds.write().await;
            let world_state = worlds.entry(world_id).or_insert_with(WorldConnectionState::new);

            // Check DM availability for DM role
            if role == WorldRole::Dm {
                world_state.add_dm(connection_id, &user_id)?;
            } else if role == WorldRole::Player {
                world_state.add_player(connection_id);
            } else {
                world_state.add_spectator(connection_id);
            }
        }

        // Update connection info
        {
            let mut connections = self.connections.write().await;
            if let Some(conn) = connections.get_mut(&connection_id) {
                conn.world_id = Some(world_id);
                conn.role = Some(role);
                conn.pc_id = pc_id;
                conn.spectate_pc_id = spectate_pc_id;
            }
        }

        // Collect connected users
        let connected_users = self.get_connected_users(world_id).await;

        tracing::info!(
            connection_id = %connection_id,
            world_id = %world_id,
            role = ?role,
            user_id = %user_id,
            "User joined world"
        );

        Ok(connected_users)
    }

    /// Leave the current world
    pub async fn leave_world(&self, connection_id: Uuid) -> Option<(Uuid, WorldRole)> {
        let (world_id, role) = {
            let mut connections = self.connections.write().await;
            let conn = connections.get_mut(&connection_id)?;
            let world_id = conn.world_id.take()?;
            let role = conn.role.take()?;
            conn.pc_id = None;
            conn.spectate_pc_id = None;
            (world_id, role)
        };

        // Remove from world state
        {
            let mut worlds = self.worlds.write().await;
            if let Some(world_state) = worlds.get_mut(&world_id) {
                world_state.remove(connection_id, role);
                if world_state.is_empty() {
                    worlds.remove(&world_id);
                }
            }
        }

        tracing::info!(
            connection_id = %connection_id,
            world_id = %world_id,
            role = ?role,
            "User left world"
        );

        Some((world_id, role))
    }

    /// Get all connected users in a world
    pub async fn get_connected_users(&self, world_id: Uuid) -> Vec<ConnectedUser> {
        let worlds = self.worlds.read().await;
        let world_state = match worlds.get(&world_id) {
            Some(state) => state,
            None => return vec![],
        };

        let connections = self.connections.read().await;

        // Group connections by user_id to get connection counts
        let mut user_counts: HashMap<String, u32> = HashMap::new();
        let mut user_info: HashMap<String, ConnectedUser> = HashMap::new();

        for conn_id in &world_state.connections {
            if let Some(conn) = connections.get(conn_id) {
                let count = user_counts.entry(conn.user_id.clone()).or_insert(0);
                *count += 1;

                user_info
                    .entry(conn.user_id.clone())
                    .or_insert_with(|| conn.to_connected_user());
            }
        }

        // Update connection counts
        for (user_id, user) in user_info.iter_mut() {
            if let Some(count) = user_counts.get(user_id) {
                user.connection_count = *count;
            }
        }

        user_info.into_values().collect()
    }

    /// Get connection info by connection_id
    pub async fn get_connection(&self, connection_id: Uuid) -> Option<ConnectionInfo> {
        self.connections.read().await.get(&connection_id).cloned()
    }

    /// Get all connection IDs in a world
    pub async fn get_world_connections(&self, world_id: Uuid) -> Vec<Uuid> {
        self.worlds
            .read()
            .await
            .get(&world_id)
            .map(|state| state.connections.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all DM connection IDs in a world
    pub async fn get_dm_connections(&self, world_id: Uuid) -> Vec<Uuid> {
        self.worlds
            .read()
            .await
            .get(&world_id)
            .map(|state| state.dm_connections.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all Player connection IDs in a world
    pub async fn get_player_connections(&self, world_id: Uuid) -> Vec<Uuid> {
        self.worlds
            .read()
            .await
            .get(&world_id)
            .map(|state| state.player_connections.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Send a message to a specific connection
    pub async fn send_to_connection(&self, connection_id: Uuid, message: ServerMessage) {
        if let Some(conn) = self.connections.read().await.get(&connection_id) {
            let _ = conn.message_sender.send(message);
        }
    }

    /// Broadcast a message to all connections in a world
    pub async fn broadcast_to_world(&self, world_id: Uuid, message: ServerMessage) {
        let connections = self.get_world_connections(world_id).await;
        for conn_id in connections {
            self.send_to_connection(conn_id, message.clone()).await;
        }
    }

    /// Broadcast a message to all DMs in a world
    pub async fn broadcast_to_dms(&self, world_id: Uuid, message: ServerMessage) {
        let connections = self.get_dm_connections(world_id).await;
        for conn_id in connections {
            self.send_to_connection(conn_id, message.clone()).await;
        }
    }

    /// Broadcast a message to all Players in a world
    pub async fn broadcast_to_players(&self, world_id: Uuid, message: ServerMessage) {
        let connections = self.get_player_connections(world_id).await;
        for conn_id in connections {
            self.send_to_connection(conn_id, message.clone()).await;
        }
    }

    /// Send a message to all connections for a specific user in a world
    pub async fn send_to_user(&self, user_id: &str, world_id: Uuid, message: ServerMessage) {
        let connections = self.get_world_connections(world_id).await;
        let conns_guard = self.connections.read().await;
        for conn_id in connections {
            if let Some(conn) = conns_guard.get(&conn_id) {
                if conn.user_id == user_id {
                    let _ = conn.message_sender.send(message.clone());
                }
            }
        }
    }

    /// Get statistics about the connection manager
    pub async fn stats(&self) -> WorldConnectionStats {
        let connections = self.connections.read().await;
        let worlds = self.worlds.read().await;

        let total_connections = connections.len();
        let total_worlds = worlds.len();

        let mut dm_connections = 0;
        let mut player_connections = 0;
        let mut spectator_connections = 0;

        for world_state in worlds.values() {
            dm_connections += world_state.dm_connections.len();
            player_connections += world_state.player_connections.len();
            spectator_connections += world_state.spectator_connections.len();
        }

        WorldConnectionStats {
            total_connections,
            total_worlds,
            dm_connections,
            player_connections,
            spectator_connections,
        }
    }
}

/// Statistics about the connection manager
#[derive(Debug, Clone)]
pub struct WorldConnectionStats {
    pub total_connections: usize,
    pub total_worlds: usize,
    pub dm_connections: usize,
    pub player_connections: usize,
    pub spectator_connections: usize,
}

// =============================================================================
// Arc wrapper for sharing
// =============================================================================

/// Shared reference to the connection manager
pub type SharedWorldConnectionManager = Arc<WorldConnectionManager>;

/// Create a new shared connection manager
pub fn new_shared_manager() -> SharedWorldConnectionManager {
    Arc::new(WorldConnectionManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sender() -> broadcast::Sender<ServerMessage> {
        let (tx, _rx) = broadcast::channel(16);
        tx
    }

    #[tokio::test]
    async fn test_register_and_unregister_connection() {
        let manager = WorldConnectionManager::new();
        let conn_id = Uuid::new_v4();
        let user_id = "user1".to_string();
        let sender = create_test_sender();

        manager
            .register_connection(conn_id, user_id.clone(), sender)
            .await;

        let conn = manager.get_connection(conn_id).await.unwrap();
        assert_eq!(conn.user_id, user_id);
        assert!(!conn.is_in_world());

        let removed = manager.unregister_connection(conn_id).await;
        assert!(removed.is_some());
        assert!(manager.get_connection(conn_id).await.is_none());
    }

    #[tokio::test]
    async fn test_join_world_as_dm() {
        let manager = WorldConnectionManager::new();
        let conn_id = Uuid::new_v4();
        let world_id = Uuid::new_v4();
        let user_id = "dm_user".to_string();
        let sender = create_test_sender();

        manager.register_connection(conn_id, user_id, sender).await;

        let result = manager
            .join_world(conn_id, world_id, WorldRole::Dm, None, None)
            .await;
        assert!(result.is_ok());

        let conn = manager.get_connection(conn_id).await.unwrap();
        assert!(conn.is_dm());
        assert_eq!(conn.world_id, Some(world_id));
    }

    #[tokio::test]
    async fn test_dm_already_connected_same_user() {
        let manager = WorldConnectionManager::new();
        let conn_id1 = Uuid::new_v4();
        let conn_id2 = Uuid::new_v4();
        let world_id = Uuid::new_v4();
        let user_id = "dm_user".to_string();

        manager
            .register_connection(conn_id1, user_id.clone(), create_test_sender())
            .await;
        manager
            .register_connection(conn_id2, user_id, create_test_sender())
            .await;

        // First join should succeed
        let result1 = manager
            .join_world(conn_id1, world_id, WorldRole::Dm, None, None)
            .await;
        assert!(result1.is_ok());

        // Second join by same user should also succeed (multi-screen)
        let result2 = manager
            .join_world(conn_id2, world_id, WorldRole::Dm, None, None)
            .await;
        assert!(result2.is_ok());

        // Both should be DM connections
        let dm_conns = manager.get_dm_connections(world_id).await;
        assert_eq!(dm_conns.len(), 2);
    }

    #[tokio::test]
    async fn test_dm_already_connected_different_user() {
        let manager = WorldConnectionManager::new();
        let conn_id1 = Uuid::new_v4();
        let conn_id2 = Uuid::new_v4();
        let world_id = Uuid::new_v4();

        manager
            .register_connection(conn_id1, "dm_user1".to_string(), create_test_sender())
            .await;
        manager
            .register_connection(conn_id2, "dm_user2".to_string(), create_test_sender())
            .await;

        // First join should succeed
        let result1 = manager
            .join_world(conn_id1, world_id, WorldRole::Dm, None, None)
            .await;
        assert!(result1.is_ok());

        // Second join by different user should fail
        let result2 = manager
            .join_world(conn_id2, world_id, WorldRole::Dm, None, None)
            .await;
        assert!(matches!(result2, Err(JoinError::DmAlreadyConnected { .. })));
    }

    #[tokio::test]
    async fn test_player_requires_pc() {
        let manager = WorldConnectionManager::new();
        let conn_id = Uuid::new_v4();
        let world_id = Uuid::new_v4();

        manager
            .register_connection(conn_id, "player".to_string(), create_test_sender())
            .await;

        // Join as Player without PC should fail
        let result = manager
            .join_world(conn_id, world_id, WorldRole::Player, None, None)
            .await;
        assert!(matches!(result, Err(JoinError::PlayerRequiresPc)));

        // Join as Player with PC should succeed
        let pc_id = Uuid::new_v4();
        let result = manager
            .join_world(conn_id, world_id, WorldRole::Player, Some(pc_id), None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_leave_world() {
        let manager = WorldConnectionManager::new();
        let conn_id = Uuid::new_v4();
        let world_id = Uuid::new_v4();

        manager
            .register_connection(conn_id, "user".to_string(), create_test_sender())
            .await;
        manager
            .join_world(conn_id, world_id, WorldRole::Dm, None, None)
            .await
            .unwrap();

        let result = manager.leave_world(conn_id).await;
        assert!(result.is_some());

        let conn = manager.get_connection(conn_id).await.unwrap();
        assert!(!conn.is_in_world());

        // World should be removed since it has no connections
        assert!(manager.get_world_connections(world_id).await.is_empty());
    }
}
