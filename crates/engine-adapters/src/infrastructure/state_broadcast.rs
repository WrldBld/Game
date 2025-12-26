//! State broadcast utilities for REST endpoints
//!
//! Provides helpers for broadcasting state changes from REST handlers to
//! connected WebSocket clients. This ensures multiplayer consistency when
//! game state is modified via REST API.
//!
//! # Usage
//!
//! ```ignore
//! use crate::infrastructure::state_broadcast::broadcast_to_world_sessions;
//!
//! // After creating a want via REST
//! broadcast_to_world_sessions(
//!     &world_connection_manager,
//!     world_id,
//!     ServerMessage::NpcWantCreated { npc_id, want },
//! ).await;
//! ```

use wrldbldr_domain::WorldId;
use wrldbldr_protocol::ServerMessage;

use crate::infrastructure::world_connection_manager::SharedWorldConnectionManager;

/// Broadcast a message to all active connections for a world
///
/// This is useful for REST endpoints that modify game state and need to
/// notify connected WebSocket clients of the change.
///
/// If no connections are active for the world, this is a no-op.
pub async fn broadcast_to_world_sessions(
    world_connection_manager: &SharedWorldConnectionManager,
    world_id: WorldId,
    message: ServerMessage,
) {
    world_connection_manager
        .broadcast_to_world(*world_id.as_uuid(), message)
        .await;
}

/// Broadcast a message to all active connections (all worlds)
///
/// Use this sparingly - prefer `broadcast_to_world_sessions` when possible.
pub async fn broadcast_to_all_sessions(
    world_connection_manager: &SharedWorldConnectionManager,
    message: ServerMessage,
) {
    let world_ids = world_connection_manager.get_all_world_ids().await;
    for world_id in world_ids {
        world_connection_manager
            .broadcast_to_world(world_id, message.clone())
            .await;
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require mock AsyncSessionPort
    // Unit tests verify the module compiles correctly
}
