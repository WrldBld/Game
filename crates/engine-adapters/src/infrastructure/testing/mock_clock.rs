//! Mock clock implementation for testing.
//!
//! Provides a controllable clock for deterministic testing of time-dependent code.

use chrono::{DateTime, Utc};
use std::sync::RwLock;
use std::time::Instant;
use wrldbldr_engine_ports::outbound::ClockPort;

/// Mock clock for testing with controllable time
pub struct MockClockPort {
    frozen_time: RwLock<DateTime<Utc>>,
    frozen_instant: Instant,
}

impl MockClockPort {
    /// Create a new mock clock frozen at the given time
    pub fn new(frozen_time: DateTime<Utc>) -> Self {
        Self {
            frozen_time: RwLock::new(frozen_time),
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
