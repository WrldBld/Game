//! Connection lifecycle operations.

use async_trait::async_trait;
use wrldbldr_domain::ConnectionId;

/// Connection lifecycle management.
///
/// This trait provides methods for managing connection lifecycle events.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ConnectionLifecyclePort: Send + Sync {
    /// Unregister a connection when it disconnects
    ///
    /// This cleans up connection state and notifies other users in the world.
    async fn unregister_connection(&self, connection_id: ConnectionId);
}
