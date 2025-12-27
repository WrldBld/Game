//! WASM WebSocket implementation using web-sys

mod client;
mod adapter;

pub use client::EngineClient;
pub use adapter::WasmGameConnection;
