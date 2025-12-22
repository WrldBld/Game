//! Outbound ports - Interfaces for external services
//!
//! These ports define the contracts that infrastructure adapters must implement,
//! allowing application services to interact with external systems without
//! depending on concrete implementations.

pub mod api_port;
pub mod raw_api_port;
pub mod game_connection_port;
pub mod platform;

pub use api_port::{ApiError, ApiPort};
pub use raw_api_port::RawApiPort;
pub use game_connection_port::{ConnectionState, GameConnectionPort};
pub use platform::{
    Platform, storage_keys,
};
