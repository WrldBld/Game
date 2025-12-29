//! Clock abstraction port for time operations
//!
//! This port abstracts time operations to enable:
//! 1. Deterministic testing with mock implementations
//! 2. Time simulation for queue delays and scheduling
//! 3. Reproducible scenarios for debugging
//!
//! Modeled after the player-side `TimeProvider` trait in player-ports.

use chrono::{DateTime, Utc};
use std::time::Instant;

/// Time operations abstraction for engine-side services
///
/// All services that need current time should inject this port
/// rather than calling `Utc::now()` or `Instant::now()` directly.
///
/// # Example
///
/// ```ignore
/// pub struct MyService {
///     clock: Arc<dyn ClockPort>,
/// }
///
/// impl MyService {
///     pub fn do_something(&self) {
///         let now = self.clock.now();
///         // ... use now
///     }
/// }
/// ```
pub trait ClockPort: Send + Sync {
    /// Get current time as DateTime<Utc>
    fn now(&self) -> DateTime<Utc>;

    /// Get current time as Unix timestamp in seconds
    fn now_unix_secs(&self) -> u64;

    /// Get current time as Unix timestamp in milliseconds
    fn now_millis(&self) -> u64;

    /// Get monotonic instant for duration measurements
    ///
    /// Note: Instant cannot be mocked easily across process boundaries,
    /// but this method allows test implementations to return consistent values.
    fn instant_now(&self) -> Instant;

    /// Format current time as RFC3339 string
    fn now_rfc3339(&self) -> String {
        self.now().to_rfc3339()
    }
}
