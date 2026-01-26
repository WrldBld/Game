//! Goal entity - Abstract want targets
//!
//! # Graph-First Design (Phase 0.C)
//!
//! A Goal is an abstract desire target for Wants that don't target a specific
//! Character or Item. Examples: "power", "revenge", "peace", "recognition".
//!
//! ```cypher
//! (world:World)-[:CONTAINS_GOAL]->(goal:Goal)
//! (want:Want)-[:TARGETS]->(goal:Goal)
//! ```

use serde::{Deserialize, Serialize};

use crate::value_objects::GoalName;
use wrldbldr_domain::{GoalId, WorldId};

/// Abstract desire target (for Wants that don't target a Character or Item)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    id: GoalId,
    world_id: WorldId,
    /// Name of the goal (e.g., "Power", "Revenge", "Peace")
    name: GoalName,
    /// Optional description of what this goal means
    description: Option<String>,
}

impl Goal {
    pub fn new(world_id: WorldId, name: GoalName) -> Self {
        Self {
            id: GoalId::new(),
            world_id,
            name,
            description: None,
        }
    }

    // === Accessors ===

    pub fn id(&self) -> GoalId {
        self.id
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn name(&self) -> &GoalName {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    // === Builder Methods ===

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_id(mut self, id: GoalId) -> Self {
        self.id = id;
        self
    }

    /// Reconstruct a Goal from storage
    pub fn from_storage(
        id: GoalId,
        world_id: WorldId,
        name: GoalName,
        description: Option<String>,
    ) -> Self {
        Self {
            id,
            world_id,
            name,
            description,
        }
    }
}
