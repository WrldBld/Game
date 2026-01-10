//! Platform abstraction ports for cross-platform compatibility
//!
//! These traits abstract platform-specific operations so that:
//! 1. Application/presentation code remains platform-agnostic
//! 2. Platform-specific code is isolated in infrastructure
//! 3. Code becomes easily testable with mock implementations
//!
//! NOTE: The `Platform` struct (DI container) that aggregates these traits
//! lives in `player-adapters/src/state/platform.rs`, not here.
//! Ports layer contains only trait definitions.

use std::{future::Future, pin::Pin};

/// Time operations abstraction
pub trait TimeProvider: Clone + 'static {
    /// Get current time as Unix timestamp in seconds
    fn now_unix_secs(&self) -> u64;

    /// Get current time in milliseconds since epoch
    fn now_millis(&self) -> u64;
}

/// Async sleep abstraction
///
/// Used to avoid `#[cfg]` branches in UI code (e.g. typewriter effect).
pub trait SleepProvider: Clone + 'static {
    fn sleep_ms(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + 'static>>;
}

/// Random number generation abstraction
pub trait RandomProvider: Clone + 'static {
    /// Generate random f64 in range [0.0, 1.0)
    fn random_f64(&self) -> f64;

    /// Generate random i32 in range [min, max] (inclusive)
    fn random_range(&self, min: i32, max: i32) -> i32;
}

/// Persistent storage abstraction (localStorage/file-based)
pub trait StorageProvider: Clone + 'static {
    /// Save a string value with the given key
    fn save(&self, key: &str, value: &str);

    /// Load a string value by key, returns None if not found
    fn load(&self, key: &str) -> Option<String>;

    /// Remove a value by key
    fn remove(&self, key: &str);
}

/// Logging abstraction
pub trait LogProvider: Clone + 'static {
    fn info(&self, msg: &str);
    fn error(&self, msg: &str);
    fn debug(&self, msg: &str);
    fn warn(&self, msg: &str);
}

/// Browser document operations (page title, etc.)
pub trait DocumentProvider: Clone + 'static {
    /// Set the browser page title (no-op on desktop)
    fn set_page_title(&self, title: &str);
}

/// Engine configuration provider for API URL management
pub trait EngineConfigProvider: Clone + 'static {
    /// Configure the base Engine URL for API calls (from WebSocket URL)
    fn configure_engine_url(&self, ws_url: &str);

    /// Convert WebSocket URL to HTTP URL
    fn ws_to_http(&self, ws_url: &str) -> String;
}

/// Storage key constants
///
/// These are kept in the ports layer as they define the contract for
/// what keys are used across the application.
pub mod storage_keys {
    pub const SERVER_URL: &str = "wrldbldr_server_url";
    pub const ROLE: &str = "wrldbldr_role";
    pub const LAST_WORLD: &str = "wrldbldr_last_world";
    pub const USER_ID: &str = "wrldbldr_user_id";
}
