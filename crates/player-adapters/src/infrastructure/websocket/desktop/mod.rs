//! Desktop WebSocket implementation using tokio-tungstenite

mod adapter;
mod client;

pub use adapter::DesktopGameConnection;
pub use client::EngineClient;
