//! Movement use cases.

mod enter_region;
mod exit_location;

pub use enter_region::EnterRegion;
pub use exit_location::ExitLocation;

use std::sync::Arc;

/// Container for movement use cases.
pub struct MovementUseCases {
    pub enter_region: Arc<EnterRegion>,
    pub exit_location: Arc<ExitLocation>,
}

impl MovementUseCases {
    pub fn new(enter_region: Arc<EnterRegion>, exit_location: Arc<ExitLocation>) -> Self {
        Self {
            enter_region,
            exit_location,
        }
    }
}
