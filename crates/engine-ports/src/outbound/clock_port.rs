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

/// Mock clock for testing with controllable time
#[cfg(any(test, feature = "testing"))]
pub struct MockClockPort {
    frozen_time: std::sync::RwLock<DateTime<Utc>>,
    frozen_instant: Instant,
}

#[cfg(any(test, feature = "testing"))]
impl MockClockPort {
    /// Create a new mock clock frozen at the given time
    pub fn new(frozen_time: DateTime<Utc>) -> Self {
        Self {
            frozen_time: std::sync::RwLock::new(frozen_time),
            frozen_instant: Instant::now(),
        }
    }

    /// Create a mock clock frozen at "now"
    pub fn now_frozen() -> Self {
        Self::new(Utc::now())
    }

    /// Advance the frozen time by the given duration
    pub fn advance(&self, duration: chrono::Duration) {
        let mut time = self.frozen_time.write().unwrap();
        *time = *time + duration;
    }

    /// Set the frozen time to a specific value
    pub fn set_time(&self, time: DateTime<Utc>) {
        *self.frozen_time.write().unwrap() = time;
    }
}

#[cfg(any(test, feature = "testing"))]
impl ClockPort for MockClockPort {
    fn now(&self) -> DateTime<Utc> {
        *self.frozen_time.read().unwrap()
    }

    fn now_unix_secs(&self) -> u64 {
        self.now().timestamp() as u64
    }

    fn now_millis(&self) -> u64 {
        self.now().timestamp_millis() as u64
    }

    fn instant_now(&self) -> Instant {
        self.frozen_instant
    }
}
