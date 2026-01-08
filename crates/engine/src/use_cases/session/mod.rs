//! Session use cases.
//!
//! Orchestrates session-level flows (joining worlds, snapshots, etc.).

use std::sync::Arc;

mod join_world;
mod join_world_flow;

pub use join_world::{JoinWorld, JoinWorldError, JoinWorldResult};
pub use join_world_flow::{
    JoinWorldContext, JoinWorldFlow, JoinWorldFlowError, JoinWorldFlowResult, JoinWorldInput,
    UserJoinedPayload,
};

/// Container for session use cases.
pub struct SessionUseCases {
    pub join_world: Arc<JoinWorld>,
    pub join_world_flow: Arc<JoinWorldFlow>,
}

impl SessionUseCases {
    pub fn new(join_world: Arc<JoinWorld>, join_world_flow: Arc<JoinWorldFlow>) -> Self {
        Self {
            join_world,
            join_world_flow,
        }
    }
}
