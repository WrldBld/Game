//! Connection context resolution operations.

use async_trait::async_trait;
use uuid::Uuid;

use super::ConnectionContext;

/// Connection context resolution operations.
///
/// This trait provides methods to resolve connection information from
/// client IDs and connection IDs. Primarily used by WebSocket handlers
/// to build RequestContext.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ConnectionContextPort: Send + Sync {
    /// Get user ID by client ID
    ///
    /// Client ID is the string identifier used by WebSocket handlers.
    async fn get_user_id_by_client_id(&self, client_id: &str) -> Option<String>;

    /// Check if a client is a DM
    async fn is_dm_by_client_id(&self, client_id: &str) -> bool;

    /// Get world ID by client ID
    async fn get_world_id_by_client_id(&self, client_id: &str) -> Option<Uuid>;

    /// Check if a connection is a spectator
    async fn is_spectator_by_client_id(&self, client_id: &str) -> bool;

    /// Get full connection context by connection ID
    ///
    /// Returns all connection state needed by handlers to build RequestContext.
    /// This is the primary method for WebSocket handlers to get connection info.
    async fn get_connection_context(&self, connection_id: Uuid) -> Option<ConnectionContext>;

    /// Get full connection context by client ID string
    ///
    /// This is commonly used by handlers that receive client_id as a string.
    async fn get_connection_by_client_id(&self, client_id: &str) -> Option<ConnectionContext>;

    /// Get PC ID for a connection (if Player role)
    async fn get_pc_id_by_client_id(&self, client_id: &str) -> Option<Uuid>;
}
