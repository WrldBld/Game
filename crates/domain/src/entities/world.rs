//! World entity - The top-level container for a campaign setting

use serde::{Deserialize, Serialize};

use crate::{GameTime, WorldId};

// Re-export MonomythStage from types module
pub use crate::types::MonomythStage;

/// Result of advancing time
#[derive(Debug, Clone)]
pub struct TimeAdvanceResult {
    /// The previous game time
    pub previous_time: GameTime,
    /// The new game time
    pub new_time: GameTime,
    /// Minutes that were advanced
    pub minutes_advanced: u32,
    /// Whether the time period changed
    pub period_changed: bool,
}

/// A story arc within a world
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Act {
    pub id: wrldbldr_domain::ActId,
    pub world_id: WorldId,
    pub name: String,
    pub stage: MonomythStage,
    pub description: String,
    pub order: u32,
}

impl Act {
    pub fn new(
        world_id: WorldId,
        name: impl Into<String>,
        stage: MonomythStage,
        order: u32,
    ) -> Self {
        Self {
            id: wrldbldr_domain::ActId::new(),
            world_id,
            name: name.into(),
            stage,
            description: String::new(),
            order,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}
