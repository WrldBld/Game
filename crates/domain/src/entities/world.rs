//! World entity - The top-level container for a campaign setting

use serde::{Deserialize, Serialize};

use crate::{GameTime, WorldId};

// Re-export MonomythStage from types module
pub use crate::types::MonomythStage;

/// Result of advancing time
///
/// Simple data struct with public fields (ADR-008: no invariants to protect).
#[derive(Debug, Clone)]
pub struct TimeAdvanceResult {
    /// The previous game time
    pub previous_time: GameTime,
    /// The new game time
    pub new_time: GameTime,
    /// Seconds that were advanced
    pub seconds_advanced: u32,
    /// Whether the time period changed
    pub period_changed: bool,
}

/// A story arc within a world
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Act {
    id: wrldbldr_domain::ActId,
    world_id: WorldId,
    name: String,
    stage: MonomythStage,
    description: String,
    order: u32,
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

    /// Create an act from parts (for reconstitution from storage)
    pub fn from_parts(
        id: wrldbldr_domain::ActId,
        world_id: WorldId,
        name: String,
        stage: MonomythStage,
        description: String,
        order: u32,
    ) -> Self {
        Self {
            id,
            world_id,
            name,
            stage,
            description,
            order,
        }
    }

    // Read-only accessors

    pub fn id(&self) -> wrldbldr_domain::ActId {
        self.id
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn stage(&self) -> MonomythStage {
        self.stage
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn order(&self) -> u32 {
        self.order
    }

    // Builder methods

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}
