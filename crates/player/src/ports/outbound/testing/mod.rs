//! Test utilities for outbound ports
//!
//! This module provides mock implementations of outbound port traits for testing.
//! These mocks are available when the `testing` feature is enabled.
//!
//! # Usage
//!
//! Add to your Cargo.toml:
//! ```toml
//! [dev-dependencies]
//! wrldbldr-player-ports = { workspace = true, features = ["testing"] }
//! ```
//!
//! Then import the mocks:
//! ```ignore
//! use crate::ports::outbound::testing::MockGameConnectionPort;
//! ```

#[cfg(any(test, feature = "testing"))]
mod mock_game_connection;

#[cfg(any(test, feature = "testing"))]
pub use mock_game_connection::{
    MockGameConnectionPort, SentAction, SentApproval, SentChallengeTrigger, SentJoin,
    SentSceneChange,
};
