//! Clock access wrapper.

use std::sync::Arc;

use crate::infrastructure::ports::ClockPort;

/// Clock service wrapper for use cases.
pub struct ClockService {
    clock: Arc<dyn ClockPort>,
}

impl ClockService {
    pub fn new(clock: Arc<dyn ClockPort>) -> Self {
        Self { clock }
    }

    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.clock.now()
    }
}
