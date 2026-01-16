//! WebSocket Bridge - connects CommandBus/EventBus to the EngineClient.
//!
//! This module provides the `create_connection` function that sets up:
//! - A CommandBus for sending commands
//! - An EventBus for receiving events
//! - A background task that bridges these to the WebSocket transport
//!
//! Platform-specific implementations handle the differences between
//! desktop (tokio) and WASM (wasm-bindgen-futures) async runtimes.

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use crate::infrastructure::message_translator;
use crate::infrastructure::messaging::{
    set_connection_state, BusMessage, CommandBus, ConnectionHandle, ConnectionState,
    ConnectionStateObserver, EventBus, PendingRequests,
};
use wrldbldr_protocol::ClientMessage;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::{mpsc, oneshot, Mutex};

#[cfg(target_arch = "wasm32")]
use futures_channel::{mpsc, oneshot};
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

/// Result of creating a connection.
///
/// Contains all the pieces needed to use the connection:
/// - `command_bus`: Send commands to the engine
/// - `event_bus`: Subscribe to events from the engine
/// - `handle`: Control connection lifecycle
/// - `state_observer`: Observe connection state (for UI binding)
pub struct Connection {
    pub command_bus: CommandBus,
    pub event_bus: EventBus,
    pub handle: ConnectionHandle,
    pub state_observer: ConnectionStateObserver,
}

// =============================================================================
// Desktop Implementation (tokio)
// =============================================================================

#[cfg(not(target_arch = "wasm32"))]
pub fn create_connection(url: &str) -> Connection {
    use super::desktop::EngineClient;

    // Create channels
    let (cmd_tx, cmd_rx) = mpsc::channel::<BusMessage>(32);
    let (disconnect_tx, disconnect_rx) = oneshot::channel::<()>();

    // Create shared state
    let pending = Arc::new(Mutex::new(PendingRequests::default()));
    let state = Arc::new(AtomicU8::new(ConnectionState::Disconnected.to_u8()));

    // Create buses
    let command_bus = CommandBus::new(cmd_tx, Arc::clone(&pending));
    let event_bus = EventBus::new();
    let state_observer = ConnectionStateObserver::new(Arc::clone(&state));

    // Spawn bridge task
    let client = EngineClient::new(url);
    let event_bus_for_bridge = event_bus.clone();
    let state_for_bridge = Arc::clone(&state);
    let pending_for_bridge = Arc::clone(&pending);

    tokio::spawn(async move {
        desktop_bridge_task(
            client,
            cmd_rx,
            disconnect_rx,
            event_bus_for_bridge,
            state_for_bridge,
            pending_for_bridge,
        )
        .await;
    });

    // Create handle
    let handle = ConnectionHandle::new(Arc::clone(&state), disconnect_tx);

    Connection {
        command_bus,
        event_bus,
        handle,
        state_observer,
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn desktop_bridge_task(
    client: super::desktop::EngineClient,
    mut cmd_rx: mpsc::Receiver<BusMessage>,
    mut disconnect_rx: oneshot::Receiver<()>,
    event_bus: EventBus,
    state: Arc<AtomicU8>,
    pending: Arc<Mutex<PendingRequests>>,
) {
    // Set up state change callback
    let state_for_callback = Arc::clone(&state);
    client
        .set_on_state_change(move |conn_state| {
            set_connection_state(&state_for_callback, conn_state);
        })
        .await;

    // Set up message callback
    let event_bus_for_messages = event_bus.clone();
    let pending_for_messages: Arc<Mutex<PendingRequests>> = Arc::clone(&pending);
    client
        .set_on_message(move |msg| {
            // Check if it's a response to a pending request
            if let wrldbldr_protocol::ServerMessage::Response { request_id, result } = &msg {
                let pending: Arc<Mutex<PendingRequests>> = pending_for_messages.clone();
                let request_id = request_id.clone();
                let result = result.clone();
                tokio::spawn(async move {
                    let mut pending_guard = pending.lock().await;
                    pending_guard.resolve(&request_id, result);
                });
            }

            // Translate and dispatch to event bus
            let event = message_translator::translate(msg);
            let event_bus = event_bus_for_messages.clone();
            tokio::spawn(async move {
                event_bus.dispatch(event).await;
            });
        })
        .await;

    // Connect
    set_connection_state(&state, ConnectionState::Connecting);
    if let Err(e) = client.connect().await {
        tracing::error!("Failed to connect: {}", e);
        set_connection_state(&state, ConnectionState::Failed);
        return;
    }

    // Main loop: process commands until disconnect
    loop {
        tokio::select! {
            // Handle disconnect request
            _ = &mut disconnect_rx => {
                tracing::info!("Disconnect requested");
                client.disconnect().await;
                set_connection_state(&state, ConnectionState::Disconnected);
                break;
            }

            // Handle outgoing commands
            Some(bus_msg) = cmd_rx.recv() => {
                match bus_msg {
                    BusMessage::Send(msg) => {
                        if let Err(e) = client.send(msg).await {
                            tracing::error!("Failed to send message: {}", e);
                        }
                    }
                    BusMessage::Request { id, payload } => {
                        // Create the wire format message
                        let msg = ClientMessage::Request {
                            request_id: id,
                            payload,
                        };
                        if let Err(e) = client.send(msg).await {
                            tracing::error!("Failed to send request: {}", e);
                        }
                    }
                }
            }
        }
    }
}

// =============================================================================
// WASM Implementation
// =============================================================================

#[cfg(target_arch = "wasm32")]
pub fn create_connection(url: &str) -> Connection {
    use super::wasm::EngineClient;
    use wasm_bindgen_futures::spawn_local;

    // Create channels
    let (cmd_tx, cmd_rx) = mpsc::unbounded::<BusMessage>();
    let (disconnect_tx, disconnect_rx) = oneshot::channel::<()>();

    // Create shared state
    let pending = Rc::new(RefCell::new(PendingRequests::default()));
    let state = Arc::new(AtomicU8::new(ConnectionState::Disconnected.to_u8()));

    // Create buses
    let command_bus = CommandBus::new(cmd_tx, Rc::clone(&pending));
    let event_bus = EventBus::new();
    let state_observer = ConnectionStateObserver::new(Arc::clone(&state));

    // Spawn bridge task
    let client = EngineClient::new(url);
    let event_bus_for_bridge = event_bus.clone();
    let state_for_bridge = Arc::clone(&state);
    let pending_for_bridge = Rc::clone(&pending);

    spawn_local(async move {
        wasm_bridge_task(
            client,
            cmd_rx,
            disconnect_rx,
            event_bus_for_bridge,
            state_for_bridge,
            pending_for_bridge,
        )
        .await;
    });

    // Create handle
    let handle = ConnectionHandle::new(Arc::clone(&state), disconnect_tx);

    Connection {
        command_bus,
        event_bus,
        handle,
        state_observer,
    }
}

#[cfg(target_arch = "wasm32")]
async fn wasm_bridge_task(
    client: super::wasm::EngineClient,
    mut cmd_rx: mpsc::UnboundedReceiver<BusMessage>,
    disconnect_rx: oneshot::Receiver<()>,
    event_bus: EventBus,
    state: Arc<AtomicU8>,
    pending: Rc<RefCell<PendingRequests>>,
) {
    use futures_util::future::{select, Either};
    use futures_util::StreamExt;

    // Set up state change callback
    let state_for_callback = Arc::clone(&state);
    client.set_on_state_change(move |conn_state| {
        set_connection_state(&state_for_callback, conn_state);
    });

    // Set up message callback
    let event_bus_for_messages = event_bus.clone();
    let pending_for_messages = Rc::clone(&pending);
    client.set_on_message(move |msg| {
        // Check if it's a response to a pending request
        if let wrldbldr_protocol::ServerMessage::Response { request_id, result } = &msg {
            pending_for_messages
                .borrow_mut()
                .resolve(request_id, result.clone());
        }

        // Translate and dispatch to event bus
        let event = message_translator::translate(msg);
        event_bus_for_messages.dispatch(event);
    });

    // Connect
    set_connection_state(&state, ConnectionState::Connecting);
    if let Err(e) = client.connect() {
        web_sys::console::error_1(&format!("Failed to connect: {}", e).into());
        set_connection_state(&state, ConnectionState::Failed);
        return;
    }

    // Main loop: process commands until disconnect
    let mut disconnect_rx = disconnect_rx;
    loop {
        let cmd_future = cmd_rx.next();
        let disconnect_future = &mut disconnect_rx;

        match select(cmd_future, disconnect_future).await {
            Either::Left((Some(bus_msg), _)) => {
                match bus_msg {
                    BusMessage::Send(msg) => {
                        if let Err(e) = client.send(msg) {
                            web_sys::console::error_1(
                                &format!("Failed to send message: {}", e).into(),
                            );
                        }
                    }
                    BusMessage::Request { id, payload } => {
                        // Create the wire format message
                        let msg = ClientMessage::Request {
                            request_id: id,
                            payload,
                        };
                        if let Err(e) = client.send(msg) {
                            web_sys::console::error_1(
                                &format!("Failed to send request: {}", e).into(),
                            );
                        }
                    }
                }
            }
            Either::Left((None, _)) => {
                // Command channel closed
                web_sys::console::log_1(&"Command channel closed".into());
                break;
            }
            Either::Right((_, _)) => {
                // Disconnect requested
                web_sys::console::log_1(&"Disconnect requested".into());
                client.disconnect();
                set_connection_state(&state, ConnectionState::Disconnected);
                break;
            }
        }
    }
}
