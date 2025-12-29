//! Outbound ports - Interfaces for external services
//!
//! These ports define the contracts that infrastructure adapters must implement,
//! allowing application services to interact with external systems without
//! depending on concrete implementations.
//!
//! NOTE: Mock implementations have been moved to player-adapters/infrastructure/testing
//! where they belong. Import mocks from there:
//! `use wrldbldr_player_adapters::infrastructure::testing::MockGameConnectionPort;`

pub mod api_port;
pub mod game_connection_port;
pub mod platform;
pub mod raw_api_port;
pub mod testing;

pub use api_port::{ApiError, ApiPort};
pub use game_connection_port::{ConnectionState, GameConnectionPort};
pub use platform::{
    storage_keys, ConnectionFactoryProvider, DocumentProvider, EngineConfigProvider, LogProvider,
    RandomProvider, SleepProvider, StorageProvider, TimeProvider,
};
pub use raw_api_port::RawApiPort;
