//! WASM WebSocket implementation using web-sys

mod adapter;
mod client;

pub use adapter::WasmGameConnection;
pub use client::EngineClient;
