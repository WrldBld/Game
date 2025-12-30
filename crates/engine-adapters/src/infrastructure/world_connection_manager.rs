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

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::outbound::{
    ConnectedUserInfo as PortConnectedUserInfo, ConnectionBroadcastPort, ConnectionContext as PortConnectionContext,
    ConnectionContextPort, ConnectionLifecyclePort, ConnectionQueryPort,
    ConnectionStats as PortConnectionStats, DmInfo as PortDmInfo,
    WorldRole as PortWorldRole,
};
use wrldbldr_protocol::{ConnectedUser, JoinError, ServerMessage, WorldRole};

/// Convert protocol WorldRole to port WorldRole
fn to_port_role(role: WorldRole) -> PortWorldRole {
    match role {
        WorldRole::Dm => PortWorldRole::DM,
        WorldRole::Player => PortWorldRole::Player,
        WorldRole::Spectator => PortWorldRole::Spectator,
        WorldRole::Unknown => PortWorldRole::Spectator, // Default unknown to spectator (read-only)
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during broadcast operations
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("World not found: {0}")]
    WorldNotFound(Uuid),

    #[error("DM not connected to world: {0}")]
    DmNotConnected(Uuid),

    #[error("Player not found for PC: {0}")]
    PlayerNotFound(Uuid),

    #[error("User not found: {0}")]
    UserNotFound(String),
}

// =============================================================================
// DM Info
// =============================================================================

/// Information about the DM in a world
#[derive(Debug, Clone)]
pub struct DmInfo {
    pub user_id: String,
    pub username: Option<String>,
    pub connection_count: usize,
}

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

    /// Add a DM connection
    ///
    /// Note: DM uniqueness validation is now handled by the application layer's
    /// WorldSessionPolicy. This method assumes validation has already been performed.
    fn add_dm(&mut self, connection_id: Uuid, user_id: &str) {
        self.dm_user_id = Some(user_id.to_string());
        self.dm_connections.insert(connection_id);
        self.connections.insert(connection_id);
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
            WorldRole::Spectator | WorldRole::Unknown => {
                // Unknown role is treated as Spectator for removal
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

    /// Client ID -> Connection ID mapping
    /// This allows looking up connections by the client_id string that handlers receive
    client_id_to_connection: RwLock<HashMap<String, Uuid>>,
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
            client_id_to_connection: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new connection (not yet joined to a world)
    pub async fn register_connection(
        &self,
        connection_id: Uuid,
        client_id: String,
        user_id: String,
        message_sender: broadcast::Sender<ServerMessage>,
    ) {
        let info = ConnectionInfo::new(connection_id, user_id, message_sender);
        self.connections.write().await.insert(connection_id, info);

        // Store client_id mapping
        self.client_id_to_connection
            .write()
            .await
            .insert(client_id, connection_id);

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

        // Remove client_id mapping
        // We need to find the client_id for this connection_id
        // Since we don't store it in ConnectionInfo, we'll scan the mapping
        {
            let mut client_mapping = self.client_id_to_connection.write().await;
            client_mapping.retain(|_, conn_id| *conn_id != connection_id);
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
    ///
    /// # Note
    ///
    /// Business rule validation (role requirements, DM uniqueness) is now handled
    /// by the application layer's WorldSessionPolicy. This method assumes validation
    /// has already been performed.
    pub async fn join_world(
        &self,
        connection_id: Uuid,
        world_id: Uuid,
        role: WorldRole,
        pc_id: Option<Uuid>,
        spectate_pc_id: Option<Uuid>,
    ) -> Result<Vec<ConnectedUser>, JoinError> {
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
            let world_state = worlds
                .entry(world_id)
                .or_insert_with(WorldConnectionState::new);

            // Add connection based on role
            match role {
                WorldRole::Dm => world_state.add_dm(connection_id, &user_id),
                WorldRole::Player => world_state.add_player(connection_id),
                _ => world_state.add_spectator(connection_id),
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

    /// Get all world IDs that have active connections
    pub async fn get_all_world_ids(&self) -> Vec<Uuid> {
        self.worlds.read().await.keys().cloned().collect()
    }

    /// Send a message to a specific connection
    pub async fn send_to_connection(&self, connection_id: Uuid, message: ServerMessage) {
        if let Some(conn) = self.connections.read().await.get(&connection_id) {
            let _ = conn.message_sender.send(message);
        }
    }

    /// Broadcast a message to all connections in a world (from JSON value)
    pub async fn broadcast_json_to_world(&self, world_id: &Uuid, message: serde_json::Value) {
        if let Ok(msg) = serde_json::from_value::<ServerMessage>(message) {
            let connections = self.get_world_connections(*world_id).await;
            for conn_id in connections {
                self.send_to_connection(conn_id, msg.clone()).await;
            }
        }
    }

    /// Broadcast a ServerMessage to all connections in a world
    pub async fn broadcast_to_world(&self, world_id: Uuid, message: ServerMessage) {
        let connections = self.get_world_connections(world_id).await;
        for conn_id in connections {
            self.send_to_connection(conn_id, message.clone()).await;
        }
    }

    /// Broadcast a ServerMessage directly to all connections in a world (alias)
    pub async fn broadcast_message_to_world(&self, world_id: Uuid, message: ServerMessage) {
        self.broadcast_to_world(world_id, message).await;
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

    // =========================================================================
    // Enhanced Broadcast Methods
    // =========================================================================

    /// Broadcast to all except a specific user
    pub async fn broadcast_to_world_except(
        &self,
        world_id: &Uuid,
        exclude_user_id: &str,
        message: ServerMessage,
    ) -> Result<(), BroadcastError> {
        // Get world state
        let worlds = self.worlds.read().await;
        let world_state = worlds
            .get(world_id)
            .ok_or(BroadcastError::WorldNotFound(*world_id))?;

        // Get connections
        let conns_guard = self.connections.read().await;

        // Iterate through users, skip the excluded user
        for conn_id in &world_state.connections {
            if let Some(conn) = conns_guard.get(conn_id) {
                if conn.user_id != exclude_user_id {
                    let _ = conn.message_sender.send(message.clone());
                }
            }
        }

        Ok(())
    }

    /// Send only to DM
    pub async fn send_to_dm(
        &self,
        world_id: &Uuid,
        message: ServerMessage,
    ) -> Result<(), BroadcastError> {
        // Get world state
        let worlds = self.worlds.read().await;
        let world_state = worlds
            .get(world_id)
            .ok_or(BroadcastError::WorldNotFound(*world_id))?;

        // Check dm_user_id exists
        let dm_user_id = world_state
            .dm_user_id
            .as_ref()
            .ok_or(BroadcastError::DmNotConnected(*world_id))?;

        // Get connections
        let conns_guard = self.connections.read().await;

        // Send to all DM connections
        for conn_id in &world_state.dm_connections {
            if let Some(conn) = conns_guard.get(conn_id) {
                if &conn.user_id == dm_user_id {
                    let _ = conn.message_sender.send(message.clone());
                }
            }
        }

        Ok(())
    }

    /// Send to specific player by PC ID
    pub async fn send_to_player(
        &self,
        world_id: &Uuid,
        pc_id: &Uuid,
        message: ServerMessage,
    ) -> Result<(), BroadcastError> {
        // Get world state
        let worlds = self.worlds.read().await;
        let world_state = worlds
            .get(world_id)
            .ok_or(BroadcastError::WorldNotFound(*world_id))?;

        // Get connections
        let conns_guard = self.connections.read().await;

        // Find user with matching pc_id
        let mut found = false;
        for conn_id in &world_state.player_connections {
            if let Some(conn) = conns_guard.get(conn_id) {
                if conn.pc_id == Some(*pc_id) {
                    let _ = conn.message_sender.send(message.clone());
                    found = true;
                }
            }
        }

        if !found {
            return Err(BroadcastError::PlayerNotFound(*pc_id));
        }

        Ok(())
    }

    /// Send to specific user (new method signature compatible with world-first API)
    pub async fn send_to_user_in_world(
        &self,
        world_id: &Uuid,
        user_id: &str,
        message: ServerMessage,
    ) -> Result<(), BroadcastError> {
        // Get world state
        let worlds = self.worlds.read().await;
        let world_state = worlds
            .get(world_id)
            .ok_or(BroadcastError::WorldNotFound(*world_id))?;

        // Check if user exists in users
        let conns_guard = self.connections.read().await;
        let mut found = false;

        for conn_id in &world_state.connections {
            if let Some(conn) = conns_guard.get(conn_id) {
                if conn.user_id == user_id {
                    let _ = conn.message_sender.send(message.clone());
                    found = true;
                }
            }
        }

        if !found {
            return Err(BroadcastError::UserNotFound(user_id.to_string()));
        }

        Ok(())
    }

    // =========================================================================
    // Query Methods
    // =========================================================================

    /// Check if DM is connected to this world
    pub async fn has_dm(&self, world_id: &Uuid) -> bool {
        let worlds = self.worlds.read().await;
        if let Some(world_state) = worlds.get(world_id) {
            return world_state.dm_user_id.is_some();
        }
        false
    }

    /// Get DM user info
    pub async fn get_dm_info(&self, world_id: &Uuid) -> Option<DmInfo> {
        let worlds = self.worlds.read().await;
        let world_state = worlds.get(world_id)?;
        let dm_user_id = world_state.dm_user_id.as_ref()?.clone();

        let conns_guard = self.connections.read().await;

        // Find a DM connection to get username
        let mut username = None;
        for conn_id in &world_state.dm_connections {
            if let Some(conn) = conns_guard.get(conn_id) {
                if conn.user_id == dm_user_id {
                    username = conn.username.clone();
                    break;
                }
            }
        }

        Some(DmInfo {
            user_id: dm_user_id,
            username,
            connection_count: world_state.dm_connections.len(),
        })
    }

    /// Get user role in world
    pub async fn get_user_role(&self, world_id: &Uuid, user_id: &str) -> Option<WorldRole> {
        let worlds = self.worlds.read().await;
        let world_state = worlds.get(world_id)?;
        let conns_guard = self.connections.read().await;

        // Look up user in world connections and return their role
        for conn_id in &world_state.connections {
            if let Some(conn) = conns_guard.get(conn_id) {
                if conn.user_id == user_id {
                    return conn.role;
                }
            }
        }

        None
    }

    /// Find which user is playing a PC
    pub async fn find_player_for_pc(&self, world_id: &Uuid, pc_id: &Uuid) -> Option<String> {
        let worlds = self.worlds.read().await;
        let world_state = worlds.get(world_id)?;
        let conns_guard = self.connections.read().await;

        // Search through player connections to find matching pc_id
        for conn_id in &world_state.player_connections {
            if let Some(conn) = conns_guard.get(conn_id) {
                if conn.pc_id == Some(*pc_id) {
                    return Some(conn.user_id.clone());
                }
            }
        }

        None
    }

    /// Get all PCs in a world with their controlling users
    pub async fn get_world_pcs(&self, world_id: &Uuid) -> Vec<(Uuid, String)> {
        let worlds = self.worlds.read().await;
        let world_state = match worlds.get(world_id) {
            Some(state) => state,
            None => return vec![],
        };

        let conns_guard = self.connections.read().await;
        let mut pcs = Vec::new();

        // Collect unique (pc_id, user_id) pairs
        let mut seen = std::collections::HashSet::new();
        for conn_id in &world_state.player_connections {
            if let Some(conn) = conns_guard.get(conn_id) {
                if let Some(pc_id) = conn.pc_id {
                    let key = (pc_id, conn.user_id.clone());
                    if seen.insert(key.clone()) {
                        pcs.push(key);
                    }
                }
            }
        }

        pcs
    }

    /// Update spectate target for a spectator connection
    pub async fn set_spectate_target(&self, connection_id: Uuid, pc_id: Option<Uuid>) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(&connection_id) {
            if conn.role == Some(WorldRole::Spectator) {
                conn.spectate_pc_id = pc_id;
                tracing::debug!(
                    connection_id = %connection_id,
                    pc_id = ?pc_id,
                    "Updated spectate target"
                );
            } else {
                tracing::warn!(
                    connection_id = %connection_id,
                    role = ?conn.role,
                    "Attempted to set spectate target on non-spectator connection"
                );
            }
        }
    }

    // =========================================================================
    // Client ID Lookup Methods
    // =========================================================================

    /// Get connection info by client ID
    pub async fn get_connection_by_client_id(&self, client_id: &str) -> Option<ConnectionInfo> {
        // Look up connection_id from client_id
        let connection_id = {
            let client_mapping = self.client_id_to_connection.read().await;
            client_mapping.get(client_id).copied()?
        };

        // Get connection info
        self.get_connection(connection_id).await
    }

    /// Get user ID by client ID
    pub async fn get_user_id_by_client_id(&self, client_id: &str) -> Option<String> {
        let conn = self.get_connection_by_client_id(client_id).await?;
        Some(conn.user_id)
    }

    /// Check if client is a DM
    pub async fn is_dm_by_client_id(&self, client_id: &str) -> bool {
        let conn = match self.get_connection_by_client_id(client_id).await {
            Some(c) => c,
            None => return false,
        };
        conn.is_dm()
    }

    /// Get world ID by client ID
    pub async fn get_world_id_by_client_id(&self, client_id: &str) -> Option<Uuid> {
        let conn = self.get_connection_by_client_id(client_id).await?;
        conn.world_id
    }

    /// Broadcast to world except a specific client
    pub async fn broadcast_except_client(
        &self,
        world_id: Uuid,
        exclude_client_id: &str,
        message: ServerMessage,
    ) {
        // Get the user_id for the excluded client
        let exclude_user_id = match self.get_user_id_by_client_id(exclude_client_id).await {
            Some(uid) => uid,
            None => {
                // If client not found, just broadcast to all
                self.broadcast_to_world(world_id, message).await;
                return;
            }
        };

        // Broadcast except this user
        let _ = self
            .broadcast_to_world_except(&world_id, &exclude_user_id, message)
            .await;
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

// =============================================================================
// ISP Sub-trait Implementations
// =============================================================================

#[async_trait]
impl ConnectionQueryPort for WorldConnectionManager {
    async fn has_dm(&self, world_id: &WorldId) -> bool {
        WorldConnectionManager::has_dm(self, &world_id.to_uuid()).await
    }

    async fn get_dm_info(&self, world_id: &WorldId) -> Option<PortDmInfo> {
        let info = WorldConnectionManager::get_dm_info(self, &world_id.to_uuid()).await?;
        Some(PortDmInfo {
            user_id: info.user_id,
            username: info.username,
            connection_count: info.connection_count,
        })
    }

    async fn get_connected_users(&self, world_id: WorldId) -> Vec<PortConnectedUserInfo> {
        let users = WorldConnectionManager::get_connected_users(self, world_id.to_uuid()).await;
        users
            .into_iter()
            .map(|u| PortConnectedUserInfo {
                user_id: u.user_id,
                username: u.username,
                role: to_port_role(u.role),
                pc_id: u.pc_id.and_then(|s| s.parse().ok()),
                connection_count: u.connection_count,
            })
            .collect()
    }

    async fn get_user_role(&self, world_id: &WorldId, user_id: &str) -> Option<PortWorldRole> {
        WorldConnectionManager::get_user_role(self, &world_id.to_uuid(), user_id)
            .await
            .map(to_port_role)
    }

    async fn find_player_for_pc(&self, world_id: &WorldId, pc_id: &Uuid) -> Option<String> {
        WorldConnectionManager::find_player_for_pc(self, &world_id.to_uuid(), pc_id).await
    }

    async fn get_world_pcs(&self, world_id: &WorldId) -> Vec<(Uuid, String)> {
        WorldConnectionManager::get_world_pcs(self, &world_id.to_uuid()).await
    }

    async fn get_all_world_ids(&self) -> Vec<Uuid> {
        WorldConnectionManager::get_all_world_ids(self).await
    }

    async fn stats(&self) -> PortConnectionStats {
        let stats = WorldConnectionManager::stats(self).await;
        PortConnectionStats {
            total_connections: stats.total_connections,
            total_worlds: stats.total_worlds,
            dm_connections: stats.dm_connections,
            player_connections: stats.player_connections,
            spectator_connections: stats.spectator_connections,
        }
    }
}

#[async_trait]
impl ConnectionContextPort for WorldConnectionManager {
    async fn get_user_id_by_client_id(&self, client_id: &str) -> Option<String> {
        WorldConnectionManager::get_user_id_by_client_id(self, client_id).await
    }

    async fn is_dm_by_client_id(&self, client_id: &str) -> bool {
        WorldConnectionManager::is_dm_by_client_id(self, client_id).await
    }

    async fn get_world_id_by_client_id(&self, client_id: &str) -> Option<Uuid> {
        WorldConnectionManager::get_world_id_by_client_id(self, client_id).await
    }

    async fn is_spectator_by_client_id(&self, client_id: &str) -> bool {
        let conn = match WorldConnectionManager::get_connection_by_client_id(self, client_id).await {
            Some(c) => c,
            None => return false,
        };
        conn.is_spectator()
    }

    async fn get_connection_context(&self, connection_id: Uuid) -> Option<PortConnectionContext> {
        let conn = self.get_connection(connection_id).await?;
        Some(PortConnectionContext {
            connection_id: conn.connection_id,
            user_id: conn.user_id,
            username: conn.username,
            world_id: conn.world_id,
            role: conn.role.map(to_port_role),
            pc_id: conn.pc_id,
            spectate_pc_id: conn.spectate_pc_id,
        })
    }

    async fn get_connection_by_client_id(&self, client_id: &str) -> Option<PortConnectionContext> {
        let conn = WorldConnectionManager::get_connection_by_client_id(self, client_id).await?;
        Some(PortConnectionContext {
            connection_id: conn.connection_id,
            user_id: conn.user_id,
            username: conn.username,
            world_id: conn.world_id,
            role: conn.role.map(to_port_role),
            pc_id: conn.pc_id,
            spectate_pc_id: conn.spectate_pc_id,
        })
    }

    async fn get_pc_id_by_client_id(&self, client_id: &str) -> Option<Uuid> {
        let conn = WorldConnectionManager::get_connection_by_client_id(self, client_id).await?;
        conn.pc_id
    }
}

#[async_trait]
impl ConnectionBroadcastPort for WorldConnectionManager {
    async fn broadcast_to_world(&self, world_id: Uuid, message: serde_json::Value) {
        // Deserialize to ServerMessage and broadcast
        if let Ok(server_msg) = serde_json::from_value::<ServerMessage>(message) {
            WorldConnectionManager::broadcast_to_world(self, world_id, server_msg).await;
        } else {
            tracing::warn!("Failed to deserialize broadcast message for world {}", world_id);
        }
    }

    async fn broadcast_to_dms(&self, world_id: Uuid, message: serde_json::Value) {
        if let Ok(server_msg) = serde_json::from_value::<ServerMessage>(message) {
            WorldConnectionManager::broadcast_to_dms(self, world_id, server_msg).await;
        } else {
            tracing::warn!("Failed to deserialize broadcast message for DMs in world {}", world_id);
        }
    }

    async fn broadcast_to_players(&self, world_id: Uuid, message: serde_json::Value) {
        if let Ok(server_msg) = serde_json::from_value::<ServerMessage>(message) {
            WorldConnectionManager::broadcast_to_players(self, world_id, server_msg).await;
        } else {
            tracing::warn!("Failed to deserialize broadcast message for players in world {}", world_id);
        }
    }

    async fn broadcast_to_all_worlds(&self, message: serde_json::Value) {
        if let Ok(server_msg) = serde_json::from_value::<ServerMessage>(message) {
            for world_id in WorldConnectionManager::get_all_world_ids(self).await {
                WorldConnectionManager::broadcast_to_world(self, world_id, server_msg.clone()).await;
            }
        } else {
            tracing::warn!("Failed to deserialize broadcast message for all worlds");
        }
    }
}

#[async_trait]
impl ConnectionLifecyclePort for WorldConnectionManager {
    async fn unregister_connection(&self, connection_id: Uuid) {
        let _ = WorldConnectionManager::unregister_connection(self, connection_id).await;
    }
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
        let client_id = "client1".to_string();
        let user_id = "user1".to_string();
        let sender = create_test_sender();

        manager
            .register_connection(conn_id, client_id.clone(), user_id.clone(), sender)
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
        let client_id = "client1".to_string();
        let user_id = "dm_user".to_string();
        let sender = create_test_sender();

        manager
            .register_connection(conn_id, client_id, user_id, sender)
            .await;

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
        let client_id1 = "client1".to_string();
        let client_id2 = "client2".to_string();
        let user_id = "dm_user".to_string();

        manager
            .register_connection(conn_id1, client_id1, user_id.clone(), create_test_sender())
            .await;
        manager
            .register_connection(conn_id2, client_id2, user_id, create_test_sender())
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
    async fn test_multiple_dms_allowed_when_validation_done_upstream() {
        // Note: DM uniqueness validation is now done in the application layer
        // (WorldSessionPolicy). The adapter just manages state.
        let manager = WorldConnectionManager::new();
        let conn_id1 = Uuid::new_v4();
        let conn_id2 = Uuid::new_v4();
        let world_id = Uuid::new_v4();

        manager
            .register_connection(
                conn_id1,
                "client1".to_string(),
                "dm_user1".to_string(),
                create_test_sender(),
            )
            .await;
        manager
            .register_connection(
                conn_id2,
                "client2".to_string(),
                "dm_user2".to_string(),
                create_test_sender(),
            )
            .await;

        // First join should succeed
        let result1 = manager
            .join_world(conn_id1, world_id, WorldRole::Dm, None, None)
            .await;
        assert!(result1.is_ok());

        // Second join by different user also succeeds at adapter level
        // (validation is done by WorldSessionPolicy in the use case)
        let result2 = manager
            .join_world(conn_id2, world_id, WorldRole::Dm, None, None)
            .await;
        assert!(result2.is_ok());

        // Note: In production, WorldSessionPolicy would reject this before
        // it reaches the adapter. This test confirms adapter doesn't duplicate
        // the validation logic.
    }

    #[tokio::test]
    async fn test_player_join_without_pc_allowed_when_validation_done_upstream() {
        // Note: Role requirement validation is now done in the application layer
        // (WorldSessionPolicy). The adapter just manages state.
        let manager = WorldConnectionManager::new();
        let conn_id = Uuid::new_v4();
        let world_id = Uuid::new_v4();

        manager
            .register_connection(
                conn_id,
                "client1".to_string(),
                "player".to_string(),
                create_test_sender(),
            )
            .await;

        // Join as Player without PC - adapter allows it
        // (validation is done by WorldSessionPolicy in the use case)
        let result = manager
            .join_world(conn_id, world_id, WorldRole::Player, None, None)
            .await;
        assert!(result.is_ok());

        // Note: In production, WorldSessionPolicy would reject this before
        // it reaches the adapter. This test confirms adapter doesn't duplicate
        // the validation logic.
    }

    #[tokio::test]
    async fn test_player_join_with_pc() {
        let manager = WorldConnectionManager::new();
        let conn_id = Uuid::new_v4();
        let world_id = Uuid::new_v4();
        let pc_id = Uuid::new_v4();

        manager
            .register_connection(
                conn_id,
                "client1".to_string(),
                "player".to_string(),
                create_test_sender(),
            )
            .await;

        // Join as Player with PC should succeed
        let result = manager
            .join_world(conn_id, world_id, WorldRole::Player, Some(pc_id), None)
            .await;
        assert!(result.is_ok());

        // Verify PC is stored
        let conn = manager.get_connection(conn_id).await.unwrap();
        assert_eq!(conn.pc_id, Some(pc_id));
    }

    #[tokio::test]
    async fn test_leave_world() {
        let manager = WorldConnectionManager::new();
        let conn_id = Uuid::new_v4();
        let world_id = Uuid::new_v4();

        manager
            .register_connection(
                conn_id,
                "client1".to_string(),
                "user".to_string(),
                create_test_sender(),
            )
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
