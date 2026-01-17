//! Directorial context storage wrapper.

use std::sync::Arc;

use crate::api::connections::ConnectionManager;
use crate::infrastructure::ports::DirectorialContext;
use wrldbldr_domain::WorldId;

/// Directorial context store wrapper for use cases.
pub struct DirectorialContextStore {
    connections: Arc<ConnectionManager>,
}

impl DirectorialContextStore {
    pub fn new(connections: Arc<ConnectionManager>) -> Self {
        Self { connections }
    }

    pub fn set_context(&self, world_id: WorldId, context: DirectorialContext) {
        self.connections.set_directorial_context(world_id, context);
    }

    pub fn get_context(&self, world_id: WorldId) -> Option<DirectorialContext> {
        self.connections.get_directorial_context(world_id)
    }

    pub fn clear_context(&self, world_id: WorldId) {
        self.connections.clear_directorial_context(world_id);
    }
}
