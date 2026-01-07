//! Session use cases.
//!
//! Orchestrates session-level flows (joining worlds, snapshots, etc.).

use std::sync::Arc;

mod join_world;

pub use join_world::{JoinWorld, JoinWorldError, JoinWorldResult};

/// Container for session use cases.
pub struct SessionUseCases {
    pub join_world: Arc<JoinWorld>,
}

impl SessionUseCases {
    pub fn new(join_world: Arc<JoinWorld>) -> Self {
        Self { join_world }
    }
}
