//! Connection state management using Dioxus signals
//!
//! Tracks connection status, server URL, and user/world information.
//! All connections are world-scoped (no session concept).

use dioxus::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use wrldbldr_player_app::application::dto::{ConnectedUser, ParticipantRole, WorldRole};
use wrldbldr_player_ports::outbound::GameConnectionPort;

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
    /// World ID after joining (all connections are world-scoped)
    pub world_id: Signal<Option<Uuid>>,
    /// User ID (local identifier)
    pub user_id: Signal<Option<String>>,
    /// User role (DungeonMaster, Player, Spectator) - legacy
    pub user_role: Signal<Option<ParticipantRole>>,
    /// World role (DM, Player, Spectator) - WebSocket-first protocol
    pub world_role: Signal<Option<WorldRole>>,
    /// Connected users in the current world
    pub connected_users: Signal<Vec<ConnectedUser>>,
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
            world_id: Signal::new(None),
            user_id: Signal::new(None),
            user_role: Signal::new(None),
            world_role: Signal::new(None),
            connected_users: Signal::new(Vec::new()),
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

    /// Set the world as joined (WebSocket-first protocol)
    pub fn set_world_joined(
        &mut self,
        world_id: Uuid,
        role: WorldRole,
        connected_users: Vec<ConnectedUser>,
    ) {
        self.world_id.set(Some(world_id));
        self.world_role.set(Some(role));
        self.connected_users.set(connected_users);
        self.connection_status.set(ConnectionStatus::Connected);
        // Clear any previous error message on successful connection
        self.error_message.set(None);
    }

    /// Add a user to the connected users list
    pub fn add_connected_user(&mut self, user: ConnectedUser) {
        let mut users = self.connected_users.read().clone();
        // Don't add duplicates
        if !users.iter().any(|u| u.user_id == user.user_id) {
            users.push(user);
            self.connected_users.set(users);
        }
    }

    /// Remove a user from the connected users list
    pub fn remove_connected_user(&mut self, user_id: &str) {
        let users: Vec<_> = self
            .connected_users
            .read()
            .iter()
            .filter(|u| u.user_id != user_id)
            .cloned()
            .collect();
        self.connected_users.set(users);
    }

    /// Set user information (legacy)
    pub fn set_user(&mut self, user_id: String, role: ParticipantRole) {
        self.user_id.set(Some(user_id));
        self.user_role.set(Some(role));
    }

    /// Set the connection to disconnected state
    pub fn set_disconnected(&mut self) {
        self.connection_status.set(ConnectionStatus::Disconnected);
        self.engine_client.set(None);
        self.world_id.set(None);
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
        // Clear previous error since we're attempting a new connection
        self.error_message.set(None);
    }

    /// Check if we have an active client
    pub fn has_client(&self) -> bool {
        self.engine_client.read().is_some()
    }

    /// Clear all connection state
    pub fn clear(&mut self) {
        self.connection_status.set(ConnectionStatus::Disconnected);
        self.world_id.set(None);
        self.user_id.set(None);
        self.user_role.set(None);
        self.world_role.set(None);
        self.connected_users.set(Vec::new());
        self.server_url.set(None);
        self.engine_client.set(None);
        self.error_message.set(None);
        // Reset ComfyUI state on disconnect
        self.comfyui_state.set("connected".to_string());
        self.comfyui_message.set(None);
        self.comfyui_retry_in_seconds.set(None);
    }
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::new()
    }
}
