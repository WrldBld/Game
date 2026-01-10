//! Command Bus and Event Bus messaging infrastructure.
//!
//! This module provides a CQRS-style messaging layer for communication with the game engine:
//! - `CommandBus`: Send commands to the engine (fire-and-forget or request-response)
//! - `EventBus`: Receive events from the engine (push-based subscription)
//! - `ConnectionHandle`: Manage connection lifecycle
//!
//! The WebSocket bridge (in the websocket module) connects these buses to the actual transport.

// Public modules for internal crate use (bridge needs these)
pub mod command_bus;
pub mod connection;
pub mod event_bus;

pub use command_bus::{BusMessage, CommandBus, PendingRequests};
pub use connection::{
    set_connection_state, ConnectionHandle, ConnectionState, ConnectionStateObserver,
};
pub use event_bus::EventBus;
