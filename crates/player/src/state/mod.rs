//! State containers for player-side dependency injection
//!
//! This module contains DI containers that aggregate services and adapters.
//! These are concrete implementations that belong in the adapters layer,
//! not the ports layer.

mod platform;

pub use platform::{Platform, PlatformStorageAdapter};
