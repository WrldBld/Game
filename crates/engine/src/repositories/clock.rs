//! Clock access wrapper.

use std::sync::Arc;

use crate::infrastructure::ports::ClockPort;

/// Clock wrapper for use cases.
pub struct Clock {
    clock: Arc<dyn ClockPort>,
}

impl Clock {
    pub fn new(clock: Arc<dyn ClockPort>) -> Self {
        Self { clock }
    }

    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.clock.now()
    }
}
