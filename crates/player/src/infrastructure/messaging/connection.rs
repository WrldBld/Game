//! Connection lifecycle management.
//!
//! This module provides types for managing the WebSocket connection lifecycle,
//! including connection state observation and disconnect control.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::oneshot;

#[cfg(target_arch = "wasm32")]
use futures_channel::oneshot;

#[cfg(target_arch = "wasm32")]
use send_wrapper::SendWrapper;

/// Connection state for the game session.
///
/// This is the same enum as in the ports layer, but defined here to avoid
/// circular dependencies. The bridge maps between these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected to the server
    Disconnected,
    /// Attempting to establish connection
    Connecting,
    /// Successfully connected
    Connected,
    /// Connection lost, attempting to reconnect
    Reconnecting,
    /// Connection failed (max retries exceeded)
    Failed,
}

impl ConnectionState {
    /// Convert to u8 for atomic storage.
    pub fn to_u8(self) -> u8 {
        match self {
            ConnectionState::Disconnected => 0,
            ConnectionState::Connecting => 1,
            ConnectionState::Connected => 2,
            ConnectionState::Reconnecting => 3,
            ConnectionState::Failed => 4,
        }
    }

    /// Convert from u8 (atomic storage).
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => ConnectionState::Connecting,
            2 => ConnectionState::Connected,
            3 => ConnectionState::Reconnecting,
            4 => ConnectionState::Failed,
            _ => ConnectionState::Disconnected,
        }
    }
}

/// Handle to manage connection lifecycle.
///
/// This is returned when creating a connection and allows:
/// - Querying connection state
/// - Requesting disconnect
///
/// When this handle is dropped, it does NOT automatically disconnect.
/// Call `disconnect()` explicitly to close the connection.
pub struct ConnectionHandle {
    /// Shared state for reading current connection state
    state: Arc<AtomicU8>,
    /// Channel to request disconnect (consumed on disconnect)
    disconnect_tx: Option<oneshot::Sender<()>>,
}

impl ConnectionHandle {
    /// Create a new ConnectionHandle.
    ///
    /// Called by the bridge when spawning the connection task.
    pub fn new(state: Arc<AtomicU8>, disconnect_tx: oneshot::Sender<()>) -> Self {
        Self {
            state,
            disconnect_tx: Some(disconnect_tx),
        }
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionState {
        ConnectionState::from_u8(self.state.load(Ordering::SeqCst))
    }

    /// Check if currently connected.
    pub fn is_connected(&self) -> bool {
        self.state() == ConnectionState::Connected
    }

    /// Request disconnect.
    ///
    /// This sends a signal to the bridge task to close the connection.
    /// The connection may not close immediately - check `state()` to verify.
    ///
    /// This method consumes the handle since a disconnected connection
    /// cannot be reused. Create a new connection to reconnect.
    pub fn disconnect(mut self) {
        if let Some(tx) = self.disconnect_tx.take() {
            let _ = tx.send(());
        }
    }

    /// Get a clone of the state Arc for sharing with observers.
    pub fn state_arc(&self) -> Arc<AtomicU8> {
        Arc::clone(&self.state)
    }
}

/// Observable connection state for UI binding.
///
/// This provides a way to observe connection state changes without
/// owning the ConnectionHandle. Multiple observers can share the same
/// underlying state.
#[derive(Clone)]
pub struct ConnectionStateObserver {
    state: Arc<AtomicU8>,
}

impl ConnectionStateObserver {
    /// Create a new observer from a ConnectionHandle.
    pub fn from_handle(handle: &ConnectionHandle) -> Self {
        Self {
            state: handle.state_arc(),
        }
    }

    /// Create a new observer from a shared state Arc.
    pub fn new(state: Arc<AtomicU8>) -> Self {
        Self { state }
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionState {
        ConnectionState::from_u8(self.state.load(Ordering::SeqCst))
    }

    /// Check if currently connected.
    pub fn is_connected(&self) -> bool {
        self.state() == ConnectionState::Connected
    }
}

/// Internal helper to update connection state (used by bridge).
pub fn set_connection_state(state_ref: &AtomicU8, new_state: ConnectionState) {
    state_ref.store(new_state.to_u8(), Ordering::SeqCst);
}

// =============================================================================
// ConnectionKeepAlive - Keeps connection alive for Dioxus context
// =============================================================================

/// Keeps the connection alive by holding onto the ConnectionHandle.
///
/// This is a wrapper that can be stored in Dioxus context to prevent the
/// connection from being dropped. It implements Clone by using Arc internally.
///
/// When all clones of this are dropped, the connection will be closed.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct ConnectionKeepAlive {
    _handle: Arc<std::sync::Mutex<Option<ConnectionHandle>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ConnectionKeepAlive {
    /// Create a new keep-alive wrapper from a ConnectionHandle.
    pub fn new(handle: ConnectionHandle) -> Self {
        Self {
            _handle: Arc::new(std::sync::Mutex::new(Some(handle))),
        }
    }
}

/// Keeps the connection alive by holding onto the ConnectionHandle (WASM version).
#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct ConnectionKeepAlive {
    _handle: SendWrapper<std::rc::Rc<std::cell::RefCell<Option<ConnectionHandle>>>>,
}

#[cfg(target_arch = "wasm32")]
impl ConnectionKeepAlive {
    /// Create a new keep-alive wrapper from a ConnectionHandle.
    pub fn new(handle: ConnectionHandle) -> Self {
        Self {
            _handle: SendWrapper::new(std::rc::Rc::new(std::cell::RefCell::new(Some(handle)))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state_roundtrip() {
        let states = [
            ConnectionState::Disconnected,
            ConnectionState::Connecting,
            ConnectionState::Connected,
            ConnectionState::Reconnecting,
            ConnectionState::Failed,
        ];

        for state in states {
            let u8_val = state.to_u8();
            let back = ConnectionState::from_u8(u8_val);
            assert_eq!(state, back);
        }
    }

    #[test]
    fn test_observer_reads_state() {
        let state = Arc::new(AtomicU8::new(ConnectionState::Disconnected.to_u8()));
        let observer = ConnectionStateObserver::new(Arc::clone(&state));

        assert_eq!(observer.state(), ConnectionState::Disconnected);
        assert!(!observer.is_connected());

        state.store(ConnectionState::Connected.to_u8(), Ordering::SeqCst);

        assert_eq!(observer.state(), ConnectionState::Connected);
        assert!(observer.is_connected());
    }
}
