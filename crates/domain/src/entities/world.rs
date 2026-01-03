//! World entity - The top-level container for a campaign setting

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::value_objects::RuleSystemConfig;
use crate::{GameTime, WorldId};

// Re-export MonomythStage from types module
pub use crate::types::MonomythStage;

/// A complete campaign world
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    pub id: WorldId,
    pub name: String,
    pub description: String,
    pub rule_system: RuleSystemConfig,
    /// In-game time for the world (persisted, not session-scoped)
    pub game_time: GameTime,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl World {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: WorldId::new(),
            name: name.into(),
            description: description.into(),
            rule_system: RuleSystemConfig::default(),
            game_time: GameTime::new(now),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_rule_system(mut self, rule_system: RuleSystemConfig) -> Self {
        self.rule_system = rule_system;
        self
    }

    pub fn update_name(&mut self, name: impl Into<String>, now: DateTime<Utc>) {
        self.name = name.into();
        self.updated_at = now;
    }

    pub fn update_description(&mut self, description: impl Into<String>, now: DateTime<Utc>) {
        self.description = description.into();
        self.updated_at = now;
    }
}

// MonomythStage is now defined in and re-exported from wrldbldr-domain-types

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
