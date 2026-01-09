//! Shared WebSocket protocol types and utilities
//!
//! Platform-agnostic types used by both desktop and WASM implementations.

use crate::infrastructure::messaging::ConnectionState;

/// Convert connection state to atomic-friendly u8
pub fn state_to_u8(state: ConnectionState) -> u8 {
    match state {
        ConnectionState::Disconnected => 0,
        ConnectionState::Connecting => 1,
        ConnectionState::Connected => 2,
        ConnectionState::Reconnecting => 3,
        ConnectionState::Failed => 4,
    }
}

/// Convert u8 back to connection state
pub fn u8_to_state(v: u8) -> ConnectionState {
    match v {
        1 => ConnectionState::Connecting,
        2 => ConnectionState::Connected,
        3 => ConnectionState::Reconnecting,
        4 => ConnectionState::Failed,
        _ => ConnectionState::Disconnected,
    }
}
