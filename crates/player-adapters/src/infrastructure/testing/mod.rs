//! Test-only infrastructure fakes.
//!
//! These helpers implement outbound ports for unit tests (services/components),
//! allowing tests to run without real network / websocket connections.

pub mod fixtures;
pub mod mock_api_port;

pub use mock_api_port::MockApiPort;
