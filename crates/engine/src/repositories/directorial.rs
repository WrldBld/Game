//! Directorial context storage wrapper.

use std::sync::Arc;

use crate::infrastructure::ports::{DirectorialContext, DirectorialContextPort};
use wrldbldr_domain::WorldId;

/// Directorial context store wrapper for use cases.
pub struct DirectorialContextStore {
    store: Arc<dyn DirectorialContextPort>,
}

impl DirectorialContextStore {
    pub fn new(store: Arc<dyn DirectorialContextPort>) -> Self {
        Self { store }
    }

    pub fn set_context(&self, world_id: WorldId, context: DirectorialContext) {
        self.store.set_context(world_id, context);
    }

    pub fn get_context(&self, world_id: WorldId) -> Option<DirectorialContext> {
        self.store.get_context(world_id)
    }

    pub fn clear_context(&self, world_id: WorldId) {
        self.store.clear_context(world_id);
    }
}
