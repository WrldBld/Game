//! Connection Lifecycle Port - Manages WebSocket connection state
//!
//! This trait handles the lifecycle of the WebSocket connection to the Engine,
//! including connecting, disconnecting, and monitoring connection health.

use crate::outbound::game_connection_port::ConnectionState;
use crate::outbound::GameConnectionPort;

/// Port for managing WebSocket connection lifecycle
///
/// Handles connection establishment, teardown, and health monitoring.
/// This is the foundation that other game connection ports depend on.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait ConnectionLifecyclePort: Send + Sync {
    /// Get the current connection state
    fn state(&self) -> ConnectionState;

    /// Get the server URL as an owned String
    ///
    /// Returns owned data for mockall compatibility.
    fn url(&self) -> String;

    /// Connect to the server
    fn connect(&self) -> anyhow::Result<()>;

    /// Disconnect from the server
    fn disconnect(&self);

    /// Send a heartbeat ping to keep the connection alive
    fn heartbeat(&self) -> anyhow::Result<()>;
}

// =============================================================================
// Blanket implementation: GameConnectionPort -> ConnectionLifecyclePort
// =============================================================================

/// Blanket implementation allowing any `GameConnectionPort` to be used as `ConnectionLifecyclePort`
impl<T: GameConnectionPort + ?Sized> ConnectionLifecyclePort for T {
    fn state(&self) -> ConnectionState {
        GameConnectionPort::state(self)
    }

    fn url(&self) -> String {
        GameConnectionPort::url(self).to_string()
    }

    fn connect(&self) -> anyhow::Result<()> {
        GameConnectionPort::connect(self)
    }

    fn disconnect(&self) {
        GameConnectionPort::disconnect(self)
    }

    fn heartbeat(&self) -> anyhow::Result<()> {
        GameConnectionPort::heartbeat(self)
    }
}
