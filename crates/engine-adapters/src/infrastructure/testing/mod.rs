//! Testing utilities for engine adapters.
//!
//! Contains mock implementations and test helpers that are only compiled
//! in test mode or when the "testing" feature is enabled.

mod fixed_random;
mod mock_clock;

pub use fixed_random::FixedRandomPort;
pub use mock_clock::MockClockPort;
