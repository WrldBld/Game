//! Session-related DTOs for application layer
//!
//! These types represent session and challenge data in the application layer,
//! independent of presentation layer state structures.

/// Connection status as seen by application layer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppConnectionStatus {
    /// Not connected to any server
    Disconnected,
    /// Attempting to connect
    Connecting,
    /// Connected and ready
    Connected,
    /// Connection lost, attempting to reconnect
    Reconnecting,
    /// Connection failed
    Failed,
}
