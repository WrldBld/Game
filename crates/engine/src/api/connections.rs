//! Connection management for WebSocket clients.
//!
//! Tracks connected clients and their world associations.

use std::collections::HashMap;
use dashmap::DashMap;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_protocol::{DirectorialContext, ServerMessage};

/// Represents a connected client's role in a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldRole {
    /// Dungeon Master - can approve suggestions, control NPCs
    Dm,
    /// Player - controls a player character
    Player,
    /// Spectator - can view but not interact
    Spectator,
}

/// Information about a connected client.
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Unique ID for this connection
    pub connection_id: Uuid,
    /// User identifier (may be anonymous)
    pub user_id: String,
    /// The world this connection is associated with (if joined)
    pub world_id: Option<WorldId>,
    /// The role in the world
    pub role: WorldRole,
    /// Player character ID (if role is Player)
    pub pc_id: Option<PlayerCharacterId>,
    /// Spectate target (if role is Spectator)
    pub spectate_pc_id: Option<PlayerCharacterId>,
}

impl ConnectionInfo {
    /// Check if this connection is a DM.
    pub fn is_dm(&self) -> bool {
        matches!(self.role, WorldRole::Dm)
    }
}

/// Manages all active WebSocket connections.
pub struct ConnectionManager {
    /// Map of connection_id -> (ConnectionInfo, sender channel)
    connections: RwLock<HashMap<Uuid, (ConnectionInfo, mpsc::Sender<ServerMessage>)>>,
    /// Per-world directorial context (scene notes, NPC motivations, etc.)
    directorial_contexts: DashMap<WorldId, DirectorialContext>,
}

impl ConnectionManager {
    /// Create a new connection manager.
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            directorial_contexts: DashMap::new(),
        }
    }

    /// Register a new connection.
    pub async fn register(
        &self,
        connection_id: Uuid,
        user_id: String,
        sender: mpsc::Sender<ServerMessage>,
    ) {
        let info = ConnectionInfo {
            connection_id,
            user_id,
            world_id: None,
            role: WorldRole::Spectator,
            pc_id: None,
            spectate_pc_id: None,
        };
        let mut connections = self.connections.write().await;
        connections.insert(connection_id, (info, sender));
        tracing::debug!(connection_id = %connection_id, "Connection registered");
    }

    /// Unregister a connection.
    pub async fn unregister(&self, connection_id: Uuid) {
        let mut connections = self.connections.write().await;
        if connections.remove(&connection_id).is_some() {
            tracing::debug!(connection_id = %connection_id, "Connection unregistered");
        }
    }

    /// Get connection info by ID.
    pub async fn get(&self, connection_id: Uuid) -> Option<ConnectionInfo> {
        let connections = self.connections.read().await;
        connections.get(&connection_id).map(|(info, _)| info.clone())
    }

    /// Join a world.
    pub async fn join_world(
        &self,
        connection_id: Uuid,
        world_id: WorldId,
        role: WorldRole,
        pc_id: Option<PlayerCharacterId>,
    ) -> Result<(), ConnectionError> {
        let mut connections = self.connections.write().await;
        
        // Check if DM slot is already taken for this world
        if role == WorldRole::Dm {
            for (id, (info, _)) in connections.iter() {
                if *id != connection_id 
                    && info.world_id == Some(world_id)
                    && info.role == WorldRole::Dm 
                {
                    return Err(ConnectionError::DmAlreadyConnected);
                }
            }
        }
        
        if let Some((info, _)) = connections.get_mut(&connection_id) {
            info.world_id = Some(world_id);
            info.role = role;
            info.pc_id = pc_id;
            tracing::info!(
                connection_id = %connection_id, 
                world_id = %world_id, 
                role = ?role,
                "Connection joined world"
            );
            Ok(())
        } else {
            Err(ConnectionError::NotFound)
        }
    }

    /// Leave the current world.
    pub async fn leave_world(&self, connection_id: Uuid) {
        let mut connections = self.connections.write().await;
        if let Some((info, _)) = connections.get_mut(&connection_id) {
            let old_world = info.world_id.take();
            info.role = WorldRole::Spectator;
            info.pc_id = None;
            info.spectate_pc_id = None;
            if let Some(world_id) = old_world {
                tracing::info!(
                    connection_id = %connection_id,
                    world_id = %world_id,
                    "Connection left world"
                );
            }
        }
    }

    /// Get all connections in a world.
    pub async fn get_world_connections(&self, world_id: WorldId) -> Vec<ConnectionInfo> {
        let connections = self.connections.read().await;
        connections
            .values()
            .filter(|(info, _)| info.world_id == Some(world_id))
            .map(|(info, _)| info.clone())
            .collect()
    }

    /// Broadcast a message to all connections in a world.
    pub async fn broadcast_to_world(&self, world_id: WorldId, message: ServerMessage) {
        let connections = self.connections.read().await;
        for (info, sender) in connections.values() {
            if info.world_id == Some(world_id) {
                if let Err(e) = sender.try_send(message.clone()) {
                    tracing::warn!(
                        connection_id = %info.connection_id,
                        error = %e,
                        "Failed to broadcast message"
                    );
                }
            }
        }
    }

    /// Broadcast a message to all DMs in a world.
    pub async fn broadcast_to_dms(&self, world_id: WorldId, message: ServerMessage) {
        let connections = self.connections.read().await;
        for (info, sender) in connections.values() {
            if info.world_id == Some(world_id) && info.is_dm() {
                if let Err(e) = sender.try_send(message.clone()) {
                    tracing::warn!(
                        connection_id = %info.connection_id,
                        error = %e,
                        "Failed to broadcast to DM"
                    );
                }
            }
        }
    }

    /// Send a message to a specific PC's player.
    pub async fn send_to_pc(&self, pc_id: PlayerCharacterId, message: ServerMessage) {
        let connections = self.connections.read().await;
        for (info, sender) in connections.values() {
            if info.pc_id == Some(pc_id) || info.spectate_pc_id == Some(pc_id) {
                if let Err(e) = sender.try_send(message.clone()) {
                    tracing::warn!(
                        connection_id = %info.connection_id,
                        error = %e,
                        "Failed to send to PC"
                    );
                }
            }
        }
    }

    /// Set the directorial context for a world.
    ///
    /// This is used by the DM to provide scene notes, NPC motivations,
    /// and other guidance for LLM prompts.
    pub fn set_directorial_context(&self, world_id: WorldId, context: DirectorialContext) {
        self.directorial_contexts.insert(world_id, context);
    }

    /// Get the directorial context for a world.
    ///
    /// Returns None if no context has been set.
    pub fn get_directorial_context(&self, world_id: WorldId) -> Option<DirectorialContext> {
        self.directorial_contexts.get(&world_id).map(|r| r.clone())
    }

    /// Clear the directorial context for a world.
    pub fn clear_directorial_context(&self, world_id: WorldId) {
        self.directorial_contexts.remove(&world_id);
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during connection operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConnectionError {
    #[error("Connection not found")]
    NotFound,
    #[error("DM already connected to this world")]
    DmAlreadyConnected,
    #[error("World not found")]
    WorldNotFound,
    #[error("Not authorized for this action")]
    Unauthorized,
}
