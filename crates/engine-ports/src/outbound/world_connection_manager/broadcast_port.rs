//! Broadcast operations for WebSocket messages.

use async_trait::async_trait;
use uuid::Uuid;

/// Broadcast operations for WebSocket messages.
///
/// This trait provides methods to broadcast serialized messages to
/// various connection groups. Messages are JSON-serialized ServerMessages.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ConnectionBroadcastPort: Send + Sync {
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
}
