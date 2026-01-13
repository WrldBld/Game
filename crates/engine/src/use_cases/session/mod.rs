//! Session use cases.
//!
//! Orchestrates session-level flows (joining worlds, snapshots, etc.).

use std::sync::Arc;

mod directorial;
mod join_world;
mod join_world_flow;

pub use join_world::{JoinWorld, JoinWorldError, JoinWorldResult};
pub use join_world_flow::{
    JoinWorldContext, JoinWorldFlow, JoinWorldFlowError, JoinWorldFlowResult, JoinWorldInput,
};
pub use directorial::{DirectorialUpdate, DirectorialUpdateContext, DirectorialUpdateInput};

/// Container for session use cases.
pub struct SessionUseCases {
    pub join_world: Arc<JoinWorld>,
    pub join_world_flow: Arc<JoinWorldFlow>,
    pub directorial_update: Arc<DirectorialUpdate>,
}

impl SessionUseCases {
    pub fn new(
        join_world: Arc<JoinWorld>,
        join_world_flow: Arc<JoinWorldFlow>,
        directorial_update: Arc<DirectorialUpdate>,
    ) -> Self {
        Self {
            join_world,
            join_world_flow,
            directorial_update,
        }
    }
}
