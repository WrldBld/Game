//! Infrastructure layer - External adapters

pub mod api;
pub mod connection_factory;
pub mod http_client;
pub mod platform;
pub mod storage;
pub mod websocket;

// Re-export ConnectionFactory for convenience
pub use connection_factory::ConnectionFactory;

// Test-only infrastructure fakes (ports/adapters).
// Available for integration testing from other crates as well
pub mod testing;
