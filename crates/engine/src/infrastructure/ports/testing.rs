// Port traits define the full contract - many methods are for future use
#![allow(dead_code)]

//! Testability ports for injecting time and randomness.

use chrono::{DateTime, Utc};
use uuid::Uuid;

// =============================================================================
// Testability Ports
// =============================================================================

#[cfg_attr(test, mockall::automock)]
pub trait ClockPort: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub trait RandomPort: Send + Sync {
    fn gen_range(&self, min: i32, max: i32) -> i32;
    fn gen_uuid(&self) -> Uuid;
}
