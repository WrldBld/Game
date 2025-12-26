//! Port for world-scoped connection management
//!
//! This replaces AsyncSessionPort with a world-centric abstraction.

use async_trait::async_trait;
use wrldbldr_domain::{WorldId, CharacterId};
use wrldbldr_protocol::ServerMessage;

/// Port for world-scoped connection management
///
/// Services use this to send messages to users in a world without
/// knowing about the underlying WebSocket infrastructure.
#[async_trait]
pub trait WorldConnectionPort: Send + Sync {
    // === Broadcast Methods ===
    
    /// Broadcast message to all users in a world
    async fn broadcast_to_world(
        &self,
        world_id: &WorldId,
        message: ServerMessage,
    ) -> Result<(), WorldConnectionError>;
    
    /// Broadcast to all except a specific user
    async fn broadcast_to_world_except(
        &self,
        world_id: &WorldId,
        exclude_user_id: &str,
        message: ServerMessage,
    ) -> Result<(), WorldConnectionError>;
    
    /// Send message only to DM
    async fn send_to_dm(
        &self,
        world_id: &WorldId,
        message: ServerMessage,
    ) -> Result<(), WorldConnectionError>;
    
    /// Send message to specific user
    async fn send_to_user(
        &self,
        world_id: &WorldId,
        user_id: &str,
        message: ServerMessage,
    ) -> Result<(), WorldConnectionError>;
    
    /// Send message to player (by PC ID)
    async fn send_to_player(
        &self,
        world_id: &WorldId,
        pc_id: &CharacterId,
        message: ServerMessage,
    ) -> Result<(), WorldConnectionError>;
    
    // === Query Methods ===
    
    /// Check if DM is connected
    async fn has_dm(&self, world_id: &WorldId) -> bool;
    
    /// Get DM user ID
    async fn get_dm_user_id(&self, world_id: &WorldId) -> Option<String>;
    
    /// Find user playing a PC
    async fn find_player_for_pc(
        &self,
        world_id: &WorldId,
        pc_id: &CharacterId,
    ) -> Option<String>;
}

#[derive(Debug, thiserror::Error)]
pub enum WorldConnectionError {
    #[error("World not found: {0}")]
    WorldNotFound(WorldId),
    
    #[error("DM not connected to world")]
    DmNotConnected,
    
    #[error("Player not found for PC")]
    PlayerNotFound,
    
    #[error("User not found: {0}")]
    UserNotFound(String),
}
