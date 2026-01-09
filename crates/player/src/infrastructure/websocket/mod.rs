//! WebSocket client for Engine connection
//!
//! Platform-specific implementations are in submodules:
//! - `desktop`: tokio-tungstenite based client (EngineClient)
//! - `wasm`: web-sys WebSocket based client (EngineClient)
//! - `message_builder`: shared ClientMessage construction logic
//! - `bridge`: CommandBus/EventBus connection

mod bridge;
mod core;
mod message_builder;
mod protocol;
mod shared;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod desktop;

#[cfg(target_arch = "wasm32")]
pub(crate) mod wasm;

// Re-export shared types
pub use message_builder::ClientMessageBuilder;

// Re-export bridge
pub use bridge::{create_connection, Connection};

// Re-export ConnectionState from messaging (canonical location)
pub use crate::infrastructure::messaging::ConnectionState;

pub(crate) use core::*;
