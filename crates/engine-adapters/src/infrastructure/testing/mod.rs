//! Testing utilities for engine adapters.
//!
//! Contains mock implementations and test helpers that are only compiled
//! in test mode or when the "testing" feature is enabled.

mod mock_clock;

pub use mock_clock::MockClockPort;
