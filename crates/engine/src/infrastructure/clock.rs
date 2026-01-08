//! Clock and random implementations.

use crate::infrastructure::ports::{ClockPort, RandomPort};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// System clock - uses real time.
pub struct SystemClock;

impl SystemClock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockPort for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// System random - uses real randomness.
pub struct SystemRandom;

impl SystemRandom {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemRandom {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomPort for SystemRandom {
    fn gen_range(&self, min: i32, max: i32) -> i32 {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    }

    fn gen_uuid(&self) -> Uuid {
        Uuid::new_v4()
    }
}

/// Fixed clock for testing.
#[cfg(test)]
pub struct FixedClock(pub DateTime<Utc>);

#[cfg(test)]
impl ClockPort for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

/// Fixed random for testing.
#[cfg(test)]
pub struct FixedRandom(pub i32);

#[cfg(test)]
impl RandomPort for FixedRandom {
    fn gen_range(&self, _min: i32, _max: i32) -> i32 {
        self.0
    }

    fn gen_uuid(&self) -> Uuid {
        Uuid::nil()
    }
}
