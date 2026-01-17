// Connection manager - some methods prepared for future use
#![allow(dead_code)]

//! Connection management for WebSocket clients.
//!
//! Tracks connected clients and their world associations.

use dashmap::DashMap;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use uuid::Uuid;

/// Timeout for critical message sends (5 seconds)
const CRITICAL_SEND_TIMEOUT: Duration = Duration::from_secs(5);

use wrldbldr_domain::{PlayerCharacterId, WorldId, WorldRole};
use wrldbldr_shared::ServerMessage;

use crate::infrastructure::ports::{
    ConnectionInfo as PortConnectionInfo, DirectorialContext, SessionError,
};

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
        connections
            .get(&connection_id)
            .map(|(info, _)| info.clone())
    }

    /// Update the user_id for a connection.
    ///
    /// This is used when a client provides a stable user identifier
    /// (e.g., from browser storage) during JoinWorld.
    pub async fn set_user_id(&self, connection_id: Uuid, user_id: String) {
        let mut connections = self.connections.write().await;
        if let Some((info, _)) = connections.get_mut(&connection_id) {
            tracing::debug!(
                connection_id = %connection_id,
                old_user_id = %info.user_id,
                new_user_id = %user_id,
                "Updating connection user_id"
            );
            info.user_id = user_id;
        }
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

        // Get the user_id of the joining connection
        let joining_user_id = connections
            .get(&connection_id)
            .map(|(info, _)| info.user_id.clone());

        // Check if DM slot is already taken for this world
        if role == WorldRole::Dm {
            let mut stale_connection_id = None;
            for (id, (info, _)) in connections.iter() {
                if *id != connection_id
                    && info.world_id == Some(world_id)
                    && info.role == WorldRole::Dm
                {
                    // If same user is reconnecting, allow takeover
                    if let Some(ref joining_uid) = joining_user_id {
                        if &info.user_id == joining_uid || info.user_id.is_empty() {
                            tracing::info!(
                                old_connection_id = %id,
                                new_connection_id = %connection_id,
                                user_id = %joining_uid,
                                world_id = %world_id,
                                "Same user reconnecting as DM - allowing takeover"
                            );
                            stale_connection_id = Some(*id);
                            break;
                        }
                    }
                    // Different user trying to take DM slot
                    tracing::warn!(
                        existing_dm_connection = %id,
                        existing_dm_user = %info.user_id,
                        new_connection = %connection_id,
                        new_user = ?joining_user_id,
                        world_id = %world_id,
                        "DM slot already taken by different user"
                    );
                    return Err(ConnectionError::DmAlreadyConnected);
                }
            }

            // Remove stale connection if found
            if let Some(stale_id) = stale_connection_id {
                connections.remove(&stale_id);
                tracing::info!(
                    stale_connection_id = %stale_id,
                    "Removed stale DM connection"
                );
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
                user_id = %info.user_id,
                "Connection joined world"
            );
            Ok(())
        } else {
            Err(ConnectionError::NotFound(connection_id.to_string()))
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

    /// Broadcast a message to all connections in a world except one.
    pub async fn broadcast_to_world_except(
        &self,
        world_id: WorldId,
        exclude_connection_id: Uuid,
        message: ServerMessage,
    ) {
        let connections = self.connections.read().await;
        for (info, sender) in connections.values() {
            if info.world_id == Some(world_id) && info.connection_id != exclude_connection_id {
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

    /// Send a critical message to a specific connection with timeout.
    ///
    /// Unlike try_send, this will wait (with timeout) for channel capacity.
    /// Use for messages that must not be dropped: state changes, approvals, errors.
    pub async fn send_critical(
        &self,
        connection_id: Uuid,
        message: ServerMessage,
    ) -> Result<(), CriticalSendError> {
        let connections = self.connections.read().await;
        if let Some((_, sender)) = connections.get(&connection_id) {
            match timeout(CRITICAL_SEND_TIMEOUT, sender.send(message)).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(_)) => {
                    tracing::error!(connection_id = %connection_id, "Channel closed for critical message");
                    Err(CriticalSendError::ChannelClosed)
                }
                Err(_) => {
                    tracing::error!(connection_id = %connection_id, "Timeout sending critical message");
                    Err(CriticalSendError::Timeout)
                }
            }
        } else {
            Err(CriticalSendError::ConnectionNotFound)
        }
    }

    /// Broadcast a critical message to all connections in a world.
    ///
    /// Waits with timeout for each send. Logs errors but continues to other connections.
    /// Use for messages that must not be dropped.
    pub async fn broadcast_critical_to_world(&self, world_id: WorldId, message: ServerMessage) {
        let connections = self.connections.read().await;
        for (info, sender) in connections.values() {
            if info.world_id == Some(world_id) {
                match timeout(CRITICAL_SEND_TIMEOUT, sender.send(message.clone())).await {
                    Ok(Ok(())) => {}
                    Ok(Err(_)) => {
                        tracing::error!(
                            connection_id = %info.connection_id,
                            "Channel closed during critical broadcast"
                        );
                    }
                    Err(_) => {
                        tracing::error!(
                            connection_id = %info.connection_id,
                            "Timeout during critical broadcast (slow client?)"
                        );
                    }
                }
            }
        }
    }

    /// Broadcast a critical message to all DMs in a world.
    pub async fn broadcast_critical_to_dms(&self, world_id: WorldId, message: ServerMessage) {
        let connections = self.connections.read().await;
        for (info, sender) in connections.values() {
            if info.world_id == Some(world_id) && info.is_dm() {
                match timeout(CRITICAL_SEND_TIMEOUT, sender.send(message.clone())).await {
                    Ok(Ok(())) => {}
                    Ok(Err(_)) => {
                        tracing::error!(
                            connection_id = %info.connection_id,
                            "Channel closed during critical DM broadcast"
                        );
                    }
                    Err(_) => {
                        tracing::error!(
                            connection_id = %info.connection_id,
                            "Timeout during critical DM broadcast"
                        );
                    }
                }
            }
        }
    }

    /// Send a critical message to a specific PC's player.
    pub async fn send_critical_to_pc(&self, pc_id: PlayerCharacterId, message: ServerMessage) {
        let connections = self.connections.read().await;
        for (info, sender) in connections.values() {
            if info.pc_id == Some(pc_id) || info.spectate_pc_id == Some(pc_id) {
                match timeout(CRITICAL_SEND_TIMEOUT, sender.send(message.clone())).await {
                    Ok(Ok(())) => {}
                    Ok(Err(_)) => {
                        tracing::error!(
                            connection_id = %info.connection_id,
                            pc_id = %pc_id,
                            "Channel closed during critical PC send"
                        );
                    }
                    Err(_) => {
                        tracing::error!(
                            connection_id = %info.connection_id,
                            pc_id = %pc_id,
                            "Timeout during critical PC send"
                        );
                    }
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
    #[error("Connection not found: {0}")]
    NotFound(String),
    #[error("DM already connected to this world")]
    DmAlreadyConnected,
    #[error("World not found")]
    WorldNotFound,
    #[error("Not authorized for this action")]
    Unauthorized,
}

/// Errors that can occur when sending critical messages.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CriticalSendError {
    #[error("Connection not found")]
    ConnectionNotFound,
    #[error("Channel closed")]
    ChannelClosed,
    #[error("Send timeout - client may be slow or unresponsive")]
    Timeout,
}

// =============================================================================
// Conversions
// =============================================================================

impl From<&ConnectionInfo> for PortConnectionInfo {
    fn from(info: &ConnectionInfo) -> Self {
        PortConnectionInfo {
            connection_id: info.connection_id,
            user_id: info.user_id.clone(),
            world_id: info.world_id,
            role: info.role,
            pc_id: info.pc_id,
        }
    }
}

impl From<ConnectionError> for SessionError {
    fn from(err: ConnectionError) -> Self {
        match err {
            ConnectionError::NotFound(id) => SessionError::NotFound(id),
            ConnectionError::DmAlreadyConnected => SessionError::DmAlreadyConnected,
            ConnectionError::Unauthorized => SessionError::Unauthorized,
            ConnectionError::WorldNotFound => SessionError::NotFound("world".to_string()),
        }
    }
}
