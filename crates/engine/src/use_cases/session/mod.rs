// Session use cases - fields for future session features
#![allow(dead_code)]

//! Session use cases.
//!
//! Orchestrates session-level flows (joining worlds, snapshots, etc.).

use std::sync::Arc;

mod directorial;
mod join_world;
mod join_world_flow;
mod types;

pub use directorial::{DirectorialUpdate, DirectorialUpdateContext, DirectorialUpdateInput};
pub use join_world::{JoinWorld, JoinWorldError};
pub use join_world_flow::{JoinWorldFlow, JoinWorldFlowError};

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
