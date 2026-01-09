//! Outbound ports - Interfaces for external services
//!
//! These ports define the contracts that infrastructure adapters must implement,
//! allowing application services to interact with external systems without
//! depending on concrete implementations.

pub mod api_port;
pub mod platform;
pub mod platform_port;
pub mod player_events;
pub mod raw_api_port;

pub use api_port::{ApiError, ApiPort};
pub use platform::{
    storage_keys, DocumentProvider, EngineConfigProvider, LogProvider, RandomProvider,
    SleepProvider, StorageProvider, TimeProvider,
};
pub use platform_port::PlatformPort;
pub use raw_api_port::RawApiPort;
