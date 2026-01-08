//! Common utility functions shared across WrldBldr Engine and Player.
//!
//! This module provides pure utility functions that are used by multiple
//! crates across the hexagonal architecture.
//!
//! # Design Principles
//!
//! - **Pure functions only** - no side effects, no I/O
//! - **Minimal dependencies** - only chrono for datetime utilities
//! - **WASM compatible** - all code must work in both native and WASM targets
//!
//! Note: This module was previously the separate `wrldbldr-common` crate.

pub mod datetime;
pub mod string;

// Re-export commonly used functions at crate root for convenience
pub use datetime::{parse_datetime, parse_datetime_or};
pub use string::{none_if_empty, some_if_not_empty, StringExt};
