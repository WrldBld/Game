//! Random number generation wrapper.

use std::sync::Arc;

use crate::infrastructure::ports::RandomPort;

/// Random wrapper for use cases.
pub struct Random {
    random: Arc<dyn RandomPort>,
}

impl Random {
    pub fn new(random: Arc<dyn RandomPort>) -> Self {
        Self { random }
    }

    pub fn gen_range(&self, min: i32, max: i32) -> i32 {
        self.random.gen_range(min, max)
    }

    pub fn gen_uuid(&self) -> uuid::Uuid {
        self.random.gen_uuid()
    }
}
