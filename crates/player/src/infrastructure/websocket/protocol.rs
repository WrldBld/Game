//! Shared WebSocket protocol types and utilities
//!
//! Platform-agnostic types used by both desktop and WASM implementations.

use crate::ports::outbound::ConnectionState as PortConnectionState;

/// Infrastructure-level connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Map infrastructure state to port state
pub fn map_state(state: ConnectionState) -> PortConnectionState {
    match state {
        ConnectionState::Disconnected => PortConnectionState::Disconnected,
        ConnectionState::Connecting => PortConnectionState::Connecting,
        ConnectionState::Connected => PortConnectionState::Connected,
        ConnectionState::Reconnecting => PortConnectionState::Reconnecting,
        ConnectionState::Failed => PortConnectionState::Failed,
    }
}

/// Convert port state to atomic-friendly u8
pub fn state_to_u8(state: PortConnectionState) -> u8 {
    match state {
        PortConnectionState::Disconnected => 0,
        PortConnectionState::Connecting => 1,
        PortConnectionState::Connected => 2,
        PortConnectionState::Reconnecting => 3,
        PortConnectionState::Failed => 4,
    }
}

/// Convert u8 back to port state
pub fn u8_to_state(v: u8) -> PortConnectionState {
    match v {
        1 => PortConnectionState::Connecting,
        2 => PortConnectionState::Connected,
        3 => PortConnectionState::Reconnecting,
        4 => PortConnectionState::Failed,
        _ => PortConnectionState::Disconnected,
    }
}
