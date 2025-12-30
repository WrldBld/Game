//! World domain entity
//!
//! Represents a game world with its acts, scenes, and metadata.

use crate::domain::value_objects::WorldId;

/// A game world containing acts and scenes
#[derive(Debug, Clone, PartialEq)]
pub struct World {
    /// Unique identifier
    pub id: WorldId,
    /// World name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Rule system type (e.g., "D20", "Narrative")
    pub rule_system: Option<String>,
}

impl World {
    /// Create a new world
    pub fn new(id: impl Into<WorldId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            rule_system: None,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the rule system
    pub fn with_rule_system(mut self, system: impl Into<String>) -> Self {
        self.rule_system = Some(system.into());
        self
    }
}

/// Summary of a world for list views
#[derive(Debug, Clone, PartialEq)]
pub struct WorldSummary {
    /// World ID
    pub id: WorldId,
    /// World name
    pub name: String,
    /// Description preview
    pub description: Option<String>,
}

impl WorldSummary {
    /// Create a new world summary
    pub fn new(id: impl Into<WorldId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}
