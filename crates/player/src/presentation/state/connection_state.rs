//! Connection state management using Dioxus signals
//!
//! Tracks connection status, server URL, and user session information.

use dioxus::prelude::*;
use std::sync::Arc;

use crate::application::ports::outbound::{GameConnectionPort, ParticipantRole};

/// Connection status to the Engine server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
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

impl ConnectionStatus {
    /// Returns true if currently connected
    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionStatus::Connected)
    }

    /// Returns the status indicator color
    pub fn indicator_color(&self) -> &'static str {
        match self {
            ConnectionStatus::Connected => "#4ade80",    // green
            ConnectionStatus::Connecting => "#facc15",   // yellow
            ConnectionStatus::Reconnecting => "#facc15", // yellow
            ConnectionStatus::Disconnected => "#f87171", // red
            ConnectionStatus::Failed => "#ef4444",       // dark red
        }
    }

    /// Returns the status display text
    pub fn display_text(&self) -> &'static str {
        match self {
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Reconnecting => "Reconnecting...",
            ConnectionStatus::Disconnected => "Disconnected",
            ConnectionStatus::Failed => "Connection Failed",
        }
    }
}

/// Connection state for server and user information
#[derive(Clone)]
pub struct ConnectionState {
    /// Current connection status
    pub connection_status: Signal<ConnectionStatus>,
    /// Session ID after joining
    pub session_id: Signal<Option<String>>,
    /// User ID (local identifier)
    pub user_id: Signal<Option<String>>,
    /// User role (DungeonMaster, Player, Spectator)
    pub user_role: Signal<Option<ParticipantRole>>,
    /// Server URL we're connected to
    pub server_url: Signal<Option<String>>,
    /// Game connection handle (if connected)
    pub engine_client: Signal<Option<Arc<dyn GameConnectionPort>>>,
    /// Error message if connection failed
    pub error_message: Signal<Option<String>>,
    /// ComfyUI connection state
    pub comfyui_state: Signal<String>, // "connected", "degraded", "disconnected", "circuit_open"
    pub comfyui_message: Signal<Option<String>>,
    pub comfyui_retry_in_seconds: Signal<Option<u32>>,
}

impl ConnectionState {
    /// Create a new ConnectionState with disconnected status
    pub fn new() -> Self {
        Self {
            connection_status: Signal::new(ConnectionStatus::Disconnected),
            session_id: Signal::new(None),
            user_id: Signal::new(None),
            user_role: Signal::new(None),
            server_url: Signal::new(None),
            engine_client: Signal::new(None),
            error_message: Signal::new(None),
            comfyui_state: Signal::new("connected".to_string()),
            comfyui_message: Signal::new(None),
            comfyui_retry_in_seconds: Signal::new(None),
        }
    }

    /// Set the connection to connecting state
    pub fn start_connecting(&mut self, server_url: &str) {
        self.connection_status.set(ConnectionStatus::Connecting);
        self.server_url.set(Some(server_url.to_string()));
        self.error_message.set(None);
    }

    /// Set the connection to connected state
    pub fn set_connected(&mut self, client: Arc<dyn GameConnectionPort>) {
        self.connection_status.set(ConnectionStatus::Connected);
        self.engine_client.set(Some(client));
        self.error_message.set(None);
    }

    /// Store the connection handle without changing UI status.
    ///
    /// This is useful on desktop where the connection is established asynchronously
    /// and status is driven by incoming connection events.
    pub fn set_connection_handle(&mut self, client: Arc<dyn GameConnectionPort>) {
        self.engine_client.set(Some(client));
    }

    /// Set the session as joined
    pub fn set_session_joined(&mut self, session_id: String) {
        self.session_id.set(Some(session_id));
    }

    /// Set user information
    pub fn set_user(&mut self, user_id: String, role: ParticipantRole) {
        self.user_id.set(Some(user_id));
        self.user_role.set(Some(role));
    }

    /// Set the connection to disconnected state
    pub fn set_disconnected(&mut self) {
        self.connection_status.set(ConnectionStatus::Disconnected);
        self.engine_client.set(None);
        self.session_id.set(None);
    }

    /// Set the connection to failed state with error
    pub fn set_failed(&mut self, error: String) {
        self.connection_status.set(ConnectionStatus::Failed);
        self.error_message.set(Some(error));
        self.engine_client.set(None);
    }

    /// Set the connection to reconnecting state
    pub fn set_reconnecting(&mut self) {
        self.connection_status.set(ConnectionStatus::Reconnecting);
    }

    /// Check if we have an active client
    pub fn has_client(&self) -> bool {
        self.engine_client.read().is_some()
    }

    /// Clear all connection state
    pub fn clear(&mut self) {
        self.connection_status.set(ConnectionStatus::Disconnected);
        self.session_id.set(None);
        self.user_id.set(None);
        self.user_role.set(None);
        self.server_url.set(None);
        self.engine_client.set(None);
        self.error_message.set(None);
    }
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::new()
    }
}
