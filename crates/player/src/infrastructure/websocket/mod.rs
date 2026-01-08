//! WebSocket client for Engine connection
//!
//! Platform-specific implementations are in submodules:
//! - `desktop`: tokio-tungstenite based client
//! - `wasm`: web-sys WebSocket based client
//! - `message_builder`: shared ClientMessage construction logic

mod core;
mod message_builder;
mod protocol;
mod shared;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;

#[cfg(target_arch = "wasm32")]
mod wasm;

// Re-export shared types
pub use message_builder::ClientMessageBuilder;
pub use protocol::ConnectionState;

pub(crate) use core::*;

// Re-export platform-specific types with unified names
#[cfg(not(target_arch = "wasm32"))]
pub use desktop::{DesktopGameConnection as EngineGameConnection, EngineClient};

#[cfg(target_arch = "wasm32")]
pub use wasm::{EngineClient, WasmGameConnection as EngineGameConnection};
