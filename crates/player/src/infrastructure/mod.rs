//! Infrastructure layer - External adapters

pub mod api;
pub mod connection_factory;
pub mod http_client;
pub mod platform;
pub mod storage;
pub mod url_handler;
pub mod websocket;

// Test-only infrastructure fakes (ports/adapters).
#[cfg(test)]
pub mod testing;
