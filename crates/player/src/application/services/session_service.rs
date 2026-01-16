//! Session service for managing Engine WebSocket connection
//!
//! This service handles:
//! - Connecting to the Engine WebSocket server
//! - Sending JoinSession messages
//! - Processing server messages and updating application state
//!
//! # Event Types
//!
//! This service emits `PlayerEvent` (from player-ports) which is the application-layer
//! representation of server events. The translation from wire format (`ServerMessage`)
//! to `PlayerEvent` happens in the bridge layer.

use crate::infrastructure::messaging::{
    CommandBus, ConnectionHandle, ConnectionState, ConnectionStateObserver, EventBus,
};
use crate::infrastructure::websocket::{create_connection, ClientMessageBuilder, Connection};
use crate::ports::outbound::player_events::PlayerEvent;

use crate::application::dto::{AppConnectionStatus, ParticipantRole};
use crate::infrastructure::session_type_converters::participant_role_to_world_role;
use futures_channel::mpsc;

/// Default WebSocket URL for the Engine server
pub const DEFAULT_ENGINE_URL: &str = "ws://localhost:3000/ws";

/// Convert ConnectionState to application ConnectionStatus
pub fn connection_state_to_status(state: ConnectionState) -> AppConnectionStatus {
    match state {
        ConnectionState::Disconnected => AppConnectionStatus::Disconnected,
        ConnectionState::Connecting => AppConnectionStatus::Connecting,
        ConnectionState::Connected => AppConnectionStatus::Connected,
        ConnectionState::Reconnecting => AppConnectionStatus::Reconnecting,
        ConnectionState::Failed => AppConnectionStatus::Failed,
    }
}

/// Events sent from the connection to the UI task.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Connection state changed
    StateChanged(ConnectionState),
    /// Server event received (application-layer type)
    MessageReceived(PlayerEvent),
}

/// Session service for managing Engine connection.
///
/// This service owns the connection lifecycle and provides access to
/// the command bus and event bus for other services to use.
pub struct SessionService {
    command_bus: CommandBus,
    event_bus: EventBus,
    state_observer: ConnectionStateObserver,
    // Note: ConnectionHandle is consumed on disconnect, so we wrap in Option
    handle: Option<ConnectionHandle>,
}

impl SessionService {
    /// Create a new SessionService by establishing a connection to the given URL.
    ///
    /// This immediately begins the connection process.
    pub fn new(url: &str) -> Self {
        let Connection {
            command_bus,
            event_bus,
            handle,
            state_observer,
        } = create_connection(url);

        Self {
            command_bus,
            event_bus,
            state_observer,
            handle: Some(handle),
        }
    }

    /// Get the command bus for sending commands to the engine.
    pub fn command_bus(&self) -> &CommandBus {
        &self.command_bus
    }

    /// Get a clone of the command bus (for sharing with other services).
    pub fn command_bus_clone(&self) -> CommandBus {
        self.command_bus.clone()
    }

    /// Get the event bus for subscribing to events.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionState {
        self.state_observer.state()
    }

    /// Check if currently connected.
    pub fn is_connected(&self) -> bool {
        self.state_observer.is_connected()
    }

    /// Join a world once connected.
    ///
    /// This should be called after the connection reaches Connected state.
    pub fn join_world(
        &self,
        world_id: &str,
        user_id: &str,
        role: ParticipantRole,
    ) -> anyhow::Result<()> {
        let world_id = uuid::Uuid::parse_str(world_id)?;
        let world_role = participant_role_to_world_role(role.into());

        self.command_bus.send(ClientMessageBuilder::join_world(
            world_id,
            world_role,
            user_id.to_string(),
            None,
            None,
        ))
    }

    /// Subscribe to session events and set up automatic world join on connect.
    ///
    /// Returns a receiver that will receive all session events.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn subscribe_with_auto_join(
        &self,
        user_id: String,
        role: ParticipantRole,
        world_id: String,
    ) -> mpsc::UnboundedReceiver<SessionEvent> {
        let (tx, rx) = mpsc::unbounded::<SessionEvent>();

        // Subscribe to events
        let tx_for_events = tx.clone();
        self.event_bus
            .subscribe(move |event| {
                let _ = tx_for_events.unbounded_send(SessionEvent::MessageReceived(event));
            })
            .await;

        // Set up a task to monitor state and join when connected
        let state_observer = self.state_observer.clone();
        let command_bus = self.command_bus.clone();
        let tx_for_state = tx.clone();
        let user_id_for_task = user_id.clone();

        tokio::spawn(async move {
            let mut last_state = state_observer.state();
            let mut join_sent = false;

            // If already connected, send JoinWorld immediately
            if last_state == ConnectionState::Connected {
                if let Ok(world_uuid) = uuid::Uuid::parse_str(&world_id) {
                    let proto_role: wrldbldr_shared::ParticipantRole = role.into();
                    let world_role = participant_role_to_world_role(proto_role);
                    tracing::info!(
                        ?role,
                        ?proto_role,
                        ?world_role,
                        world_id = %world_uuid,
                        user_id = %user_id_for_task,
                        "Sending JoinWorld message (native) - already connected"
                    );
                    let _ = command_bus.send(ClientMessageBuilder::join_world(
                        world_uuid,
                        world_role,
                        user_id_for_task.clone(),
                        None,
                        None,
                    ));
                    join_sent = true;
                }
            }

            loop {
                let current_state = state_observer.state();
                if current_state != last_state {
                    let _ = tx_for_state.unbounded_send(SessionEvent::StateChanged(current_state));

                    // Auto-join when connected (if not already sent)
                    if current_state == ConnectionState::Connected && !join_sent {
                        if let Ok(world_uuid) = uuid::Uuid::parse_str(&world_id) {
                            let proto_role: wrldbldr_shared::ParticipantRole = role.into();
                            let world_role = participant_role_to_world_role(proto_role);
                            tracing::info!(
                                ?role,
                                ?proto_role,
                                ?world_role,
                                world_id = %world_uuid,
                                user_id = %user_id_for_task,
                                "Sending JoinWorld message (native)"
                            );
                            let _ = command_bus.send(ClientMessageBuilder::join_world(
                                world_uuid,
                                world_role,
                                user_id_for_task.clone(),
                                None,
                                None,
                            ));
                            join_sent = true;
                        }
                    }

                    last_state = current_state;
                }

                // Poll at reasonable interval
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                // Stop polling if disconnected or failed
                if matches!(
                    current_state,
                    ConnectionState::Disconnected | ConnectionState::Failed
                ) {
                    break;
                }
            }
        });

        rx
    }

    /// Subscribe to session events and set up automatic world join on connect (WASM version).
    ///
    /// Returns a receiver that will receive all session events.
    #[cfg(target_arch = "wasm32")]
    pub async fn subscribe_with_auto_join(
        &self,
        user_id: String,
        role: ParticipantRole,
        world_id: String,
    ) -> mpsc::UnboundedReceiver<SessionEvent> {
        let (tx, rx) = mpsc::unbounded::<SessionEvent>();

        // Subscribe to events (WASM subscribe is sync)
        let tx_for_events = tx.clone();
        self.event_bus.subscribe(move |event| {
            let _ = tx_for_events.unbounded_send(SessionEvent::MessageReceived(event));
        });

        // Set up a task to monitor state and join when connected
        let state_observer = self.state_observer.clone();
        let command_bus = self.command_bus.clone();
        let tx_for_state = tx.clone();
        let user_id_for_task = user_id.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let mut last_state = state_observer.state();
            let mut join_sent = false;

            // If already connected, send JoinWorld immediately
            if last_state == ConnectionState::Connected {
                if let Ok(world_uuid) = uuid::Uuid::parse_str(&world_id) {
                    let proto_role: wrldbldr_shared::ParticipantRole = role.into();
                    let world_role = participant_role_to_world_role(proto_role);
                    tracing::info!(
                        ?role,
                        ?proto_role,
                        ?world_role,
                        world_id = %world_uuid,
                        user_id = %user_id_for_task,
                        "Sending JoinWorld message (WASM) - already connected"
                    );
                    let _ = command_bus.send(ClientMessageBuilder::join_world(
                        world_uuid,
                        world_role,
                        user_id_for_task.clone(),
                        None,
                        None,
                    ));
                    join_sent = true;
                }
            }

            loop {
                let current_state = state_observer.state();
                if current_state != last_state {
                    let _ = tx_for_state.unbounded_send(SessionEvent::StateChanged(current_state));

                    // Auto-join when connected (if not already sent)
                    if current_state == ConnectionState::Connected && !join_sent {
                        if let Ok(world_uuid) = uuid::Uuid::parse_str(&world_id) {
                            let proto_role: wrldbldr_shared::ParticipantRole = role.into();
                            let world_role = participant_role_to_world_role(proto_role);
                            tracing::info!(
                                ?role,
                                ?proto_role,
                                ?world_role,
                                world_id = %world_uuid,
                                user_id = %user_id_for_task,
                                "Sending JoinWorld message (WASM)"
                            );
                            let _ = command_bus.send(ClientMessageBuilder::join_world(
                                world_uuid,
                                world_role,
                                user_id_for_task.clone(),
                                None,
                                None,
                            ));
                            join_sent = true;
                        }
                    }

                    last_state = current_state;
                }

                // Poll at reasonable interval
                gloo_timers::future::TimeoutFuture::new(50).await;

                // Stop polling if disconnected or failed
                if matches!(
                    current_state,
                    ConnectionState::Disconnected | ConnectionState::Failed
                ) {
                    break;
                }
            }
        });

        rx
    }

    /// Disconnect from the engine.
    ///
    /// This consumes the connection handle. After calling this, the service
    /// cannot reconnect - create a new SessionService to reconnect.
    pub fn disconnect(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.disconnect();
        }
    }
}
