//! Desktop WebSocket implementation using tokio-tungstenite

mod client;
mod adapter;

pub use client::EngineClient;
pub use adapter::DesktopGameConnection;
