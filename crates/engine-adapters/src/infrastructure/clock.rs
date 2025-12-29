//! System clock adapter
//!
//! Provides the production implementation of ClockPort using
//! chrono and std::time for real system time.

use chrono::{DateTime, Utc};
use std::time::Instant;
use wrldbldr_engine_ports::outbound::ClockPort;

/// System clock implementation using real time
///
/// This is the production implementation that should be used
/// in the composition root. For testing, use MockClockPort instead.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl SystemClock {
    /// Create a new system clock
    pub fn new() -> Self {
        Self
    }
}

impl ClockPort for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn now_unix_secs(&self) -> u64 {
        Utc::now().timestamp() as u64
    }

    fn now_millis(&self) -> u64 {
        Utc::now().timestamp_millis() as u64
    }

    fn instant_now(&self) -> Instant {
        Instant::now()
    }
}
