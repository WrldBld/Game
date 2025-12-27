//! Test utilities for outbound ports
//!
//! Provides mock implementations for unit testing services that depend on outbound ports.
//! These mocks are only available on non-WASM targets since tests run on desktop.

#[cfg(not(target_arch = "wasm32"))]
mod mock_game_connection;

#[cfg(not(target_arch = "wasm32"))]
pub use mock_game_connection::MockGameConnectionPort;
