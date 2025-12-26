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
//!     &state,
//!     world_id,
//!     ServerMessage::NpcWantCreated { npc_id, want },
//! ).await;
//! ```

use std::sync::Arc;

use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::outbound::AsyncSessionPort;
use wrldbldr_protocol::ServerMessage;

/// Broadcast a message to all active sessions for a world
///
/// This is useful for REST endpoints that modify game state and need to
/// notify connected WebSocket clients of the change.
///
/// If no sessions are active for the world, this is a no-op.
pub async fn broadcast_to_world_sessions(
    async_session_port: &Arc<dyn AsyncSessionPort>,
    world_id: WorldId,
    message: ServerMessage,
) {
    // Find active session for this world
    if let Some(session_id) = async_session_port.find_session_for_world(world_id).await {
        match serde_json::to_value(&message) {
            Ok(json) => {
                if let Err(e) = async_session_port.broadcast_to_session(session_id, json).await {
                    tracing::warn!(
                        world_id = %world_id,
                        session_id = %session_id,
                        error = %e,
                        "Failed to broadcast state change to session"
                    );
                } else {
                    tracing::debug!(
                        world_id = %world_id,
                        session_id = %session_id,
                        message_type = ?std::any::type_name_of_val(&message),
                        "Broadcast state change to session"
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to serialize broadcast message"
                );
            }
        }
    }
}

/// Broadcast a message to all active sessions (all worlds)
///
/// Use this sparingly - prefer `broadcast_to_world_sessions` when possible.
pub async fn broadcast_to_all_sessions(
    async_session_port: &Arc<dyn AsyncSessionPort>,
    message: ServerMessage,
) {
    let session_ids = async_session_port.list_session_ids().await;
    
    let json = match serde_json::to_value(&message) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!(error = %e, "Failed to serialize broadcast message");
            return;
        }
    };

    for session_id in session_ids {
        if let Err(e) = async_session_port.broadcast_to_session(session_id, json.clone()).await {
            tracing::warn!(
                session_id = %session_id,
                error = %e,
                "Failed to broadcast to session"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require mock AsyncSessionPort
    // Unit tests verify the module compiles correctly
}
