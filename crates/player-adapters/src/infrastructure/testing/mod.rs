//! Test-only infrastructure fakes.
//!
//! These helpers implement outbound ports for unit tests (services/components),
//! allowing tests to run without real network / websocket connections.
//!
//! Mock implementations belong in the adapters layer (not ports) because:
//! 1. They are concrete implementations of port traits
//! 2. Mocks are infrastructure concerns, not interface definitions
//! 3. Test utilities should be close to the implementations they mock

pub mod fixtures;
pub mod mock_api_port;
pub mod mock_game_connection;

pub use mock_api_port::MockApiPort;
pub use mock_game_connection::{
    MockGameConnectionPort, SentAction, SentApproval, SentChallengeTrigger, SentJoin,
    SentSceneChange,
};
