//! Unicast messaging operations.

use async_trait::async_trait;
use uuid::Uuid;

use super::ConnectionManagerError;

/// Unicast message sending operations.
///
/// This trait provides the ability to send a serialized message to a specific user
/// within a world, without broadcasting to all participants.
///
/// Messages are JSON-serialized `ServerMessage` values.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ConnectionUnicastPort: Send + Sync {
    /// Send a serialized message to a specific user in a world.
    async fn send_to_user_in_world(
        &self,
        world_id: Uuid,
        user_id: &str,
        message: serde_json::Value,
    ) -> Result<(), ConnectionManagerError>;
}
