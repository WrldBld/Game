//! PlatformPort - Unified platform services interface
//!
//! This trait provides a unified interface for all platform-specific operations
//! needed by the UI layer. It abstracts the Platform DI container so that
//! player-ui doesn't need to depend on player-adapters.
//!
//! The concrete implementation (`Platform`) lives in player-adapters.

use std::{future::Future, pin::Pin, sync::Arc};

use super::GameConnectionPort;

/// Unified platform services port
///
/// This trait provides all platform abstractions through a single injectable type.
/// Implemented by the `Platform` struct in player-adapters.
///
/// Use via Dioxus context: `use_context::<Arc<dyn PlatformPort>>()`
pub trait PlatformPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Time operations
    // -------------------------------------------------------------------------

    /// Get current time as Unix timestamp in seconds
    fn now_unix_secs(&self) -> u64;

    /// Get current time in milliseconds since epoch
    fn now_millis(&self) -> u64;

    // -------------------------------------------------------------------------
    // Sleep operations
    // -------------------------------------------------------------------------

    /// Sleep for the given number of milliseconds
    fn sleep_ms(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + 'static>>;

    // -------------------------------------------------------------------------
    // Random operations
    // -------------------------------------------------------------------------

    /// Generate random f64 in range [0.0, 1.0)
    fn random_f64(&self) -> f64;

    /// Generate random i32 in range [min, max] (inclusive)
    fn random_range(&self, min: i32, max: i32) -> i32;

    // -------------------------------------------------------------------------
    // Storage operations
    // -------------------------------------------------------------------------

    /// Save a string value with the given key
    fn storage_save(&self, key: &str, value: &str);

    /// Load a string value by key, returns None if not found
    fn storage_load(&self, key: &str) -> Option<String>;

    /// Remove a value by key
    fn storage_remove(&self, key: &str);

    // -------------------------------------------------------------------------
    // User identity operations
    // -------------------------------------------------------------------------

    /// Get or create a stable anonymous user ID
    ///
    /// This ID is persisted in storage and reused across sessions until local
    /// storage is cleared, effectively acting as an anonymous user identity.
    fn get_user_id(&self) -> String;

    // -------------------------------------------------------------------------
    // Logging operations
    // -------------------------------------------------------------------------

    /// Log an info message
    fn log_info(&self, msg: &str);

    /// Log an error message
    fn log_error(&self, msg: &str);

    /// Log a debug message
    fn log_debug(&self, msg: &str);

    /// Log a warning message
    fn log_warn(&self, msg: &str);

    // -------------------------------------------------------------------------
    // Document operations
    // -------------------------------------------------------------------------

    /// Set the browser page title (no-op on desktop)
    fn set_page_title(&self, title: &str);

    // -------------------------------------------------------------------------
    // Engine config operations
    // -------------------------------------------------------------------------

    /// Configure the base Engine URL for API calls (from WebSocket URL)
    fn configure_engine_url(&self, ws_url: &str);

    /// Convert WebSocket URL to HTTP URL
    fn ws_to_http(&self, ws_url: &str) -> String;

    // -------------------------------------------------------------------------
    // Connection factory operations
    // -------------------------------------------------------------------------

    /// Create a game connection to the engine
    fn create_game_connection(&self, server_url: &str) -> Arc<dyn GameConnectionPort>;
}
