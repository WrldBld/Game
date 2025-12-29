//! Session service for managing Engine WebSocket connection
//!
//! This service handles:
//! - Connecting to the Engine WebSocket server
//! - Sending JoinSession messages
//! - Processing server messages and updating application state
//!
//! # Protocol Dependencies
//!
//! This service uses `ServerMessage` from the protocol crate. This is intentional
//! and follows the same pattern as `RequestPayload`/`ResponseResult`:
//!
//! - `ServerMessage` is the wire format for server-to-client communication
//! - It's analogous to `RequestPayload` for client-to-server communication
//! - The translation to app-layer `PlayerEvent` happens in the adapters layer
//!   via `player-adapters::infrastructure::message_translator`
//!
//! The presentation layer (player-ui) can use the translator when processing
//! these events if it needs domain-friendly types.

use std::sync::Arc;

use anyhow::Result;

use wrldbldr_player_ports::outbound::{ConnectionState as PortConnectionState, GameConnectionPort};
use wrldbldr_protocol::ServerMessage;

use crate::application::dto::{AppConnectionStatus, ParticipantRole};
use futures_channel::mpsc;

/// Default WebSocket URL for the Engine server
pub const DEFAULT_ENGINE_URL: &str = "ws://localhost:3000/ws";

/// Convert port ConnectionState to application ConnectionStatus
pub fn port_connection_state_to_status(state: PortConnectionState) -> AppConnectionStatus {
    match state {
        PortConnectionState::Disconnected => AppConnectionStatus::Disconnected,
        PortConnectionState::Connecting => AppConnectionStatus::Connecting,
        PortConnectionState::Connected => AppConnectionStatus::Connected,
        PortConnectionState::Reconnecting => AppConnectionStatus::Reconnecting,
        PortConnectionState::Failed => AppConnectionStatus::Failed,
    }
}

/// Events sent from the connection callbacks to the UI task.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Connection state changed (uses port type)
    StateChanged(PortConnectionState),
    /// Server message received (wire format)
    ///
    /// NOTE: This contains protocol ServerMessage, which should be translated
    /// to PlayerEvent by the presentation layer using player-adapters' translator.
    MessageReceived(ServerMessage),
}

/// Session service for managing Engine connection (cross-platform).
///
/// This service depends on the `GameConnectionPort` abstraction.
/// The ISP sub-traits (ConnectionLifecyclePort, SessionCommandPort) are available
/// via blanket implementations on GameConnectionPort.
pub struct SessionService {
    connection: Arc<dyn GameConnectionPort>,
}

impl SessionService {
    pub fn new(connection: Arc<dyn GameConnectionPort>) -> Self {
        Self { connection }
    }

    pub fn connection(&self) -> &Arc<dyn GameConnectionPort> {
        &self.connection
    }

    pub async fn connect(
        &self,
        user_id: String,
        role: ParticipantRole,
        world_id: String,
    ) -> Result<mpsc::UnboundedReceiver<SessionEvent>> {
        let (tx, rx) = mpsc::unbounded::<SessionEvent>();

        // On connect, join when Connected is observed.
        {
            let tx = tx.clone();
            let connection = Arc::clone(&self.connection);
            let user_id_for_join = user_id.clone();
            let world_id_for_join = world_id.clone();

            self.connection.on_state_change(Box::new(move |state| {
                let _ = tx.unbounded_send(SessionEvent::StateChanged(state));
                if matches!(state, PortConnectionState::Connected) {
                    let _ = connection.join_world(&world_id_for_join, &user_id_for_join, role);
                }
            }));
        }

        // Parse and forward server messages
        {
            let tx = tx.clone();
            self.connection.on_message(Box::new(move |value| {
                match serde_json::from_value::<ServerMessage>(value) {
                    Ok(message) => {
                        let _ = tx.unbounded_send(SessionEvent::MessageReceived(message));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse server message JSON: {}", e);
                    }
                }
            }));
        }

        // Initiate connection (adapter handles async details)
        self.connection.connect()?;

        Ok(rx)
    }

    pub fn disconnect(&self) {
        self.connection.disconnect();
    }
}
