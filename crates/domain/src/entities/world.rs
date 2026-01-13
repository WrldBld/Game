//! World entity - The top-level container for a campaign setting

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::value_objects::RuleSystemConfig;
use crate::{GameTime, GameTimeConfig, TimeAdvanceReason, TimeCostConfig, TimeMode, WorldId};

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
    /// Configuration for how game time behaves
    #[serde(default)]
    pub time_config: GameTimeConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

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

impl World {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        let name = name.trim().to_string();
        let description = description.into();

        if name.is_empty() {
            return Err(DomainError::validation("World name cannot be empty"));
        }
        if name.len() > 200 {
            return Err(DomainError::validation(
                "World name cannot exceed 200 characters",
            ));
        }

        Ok(Self {
            id: WorldId::new(),
            name,
            description,
            rule_system: RuleSystemConfig::default(),
            game_time: GameTime::new(now),
            time_config: GameTimeConfig::default(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn with_rule_system(mut self, rule_system: RuleSystemConfig) -> Self {
        self.rule_system = rule_system;
        self
    }

    pub fn with_time_config(mut self, time_config: GameTimeConfig) -> Self {
        self.time_config = time_config;
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

    // =========================================================================
    // Time Configuration
    // =========================================================================

    /// Set the time mode (manual, suggested, auto).
    pub fn set_time_mode(&mut self, mode: TimeMode, now: DateTime<Utc>) {
        self.time_config.mode = mode;
        self.updated_at = now;
    }

    /// Set the time cost configuration.
    pub fn set_time_costs(&mut self, costs: TimeCostConfig, now: DateTime<Utc>) {
        self.time_config.time_costs = costs;
        self.updated_at = now;
    }

    /// Get the time cost for a given action type.
    pub fn time_cost_for_action(&self, action: &str) -> u32 {
        self.time_config.time_costs.cost_for_action(action)
    }

    // =========================================================================
    // Time Advancement
    // =========================================================================

    /// Advance game time by a number of minutes.
    /// Returns information about the time change for broadcasting.
    pub fn advance_time(
        &mut self,
        minutes: u32,
        _reason: TimeAdvanceReason,
        now: DateTime<Utc>,
    ) -> TimeAdvanceResult {
        let previous_time = self.game_time.clone();
        let previous_period = self.game_time.time_of_day();

        self.game_time.advance_minutes(minutes);
        self.updated_at = now;

        let new_period = self.game_time.time_of_day();

        TimeAdvanceResult {
            previous_time,
            new_time: self.game_time.clone(),
            minutes_advanced: minutes,
            period_changed: previous_period != new_period,
        }
    }

    /// Advance game time by a number of hours.
    pub fn advance_hours(&mut self, hours: u32, now: DateTime<Utc>) -> TimeAdvanceResult {
        self.advance_time(hours * 60, TimeAdvanceReason::DmManual { hours }, now)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_new_empty_name_returns_error() {
        let now = Utc::now();
        let result = World::new("", "Description", now);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_world_new_whitespace_only_name_returns_error() {
        let now = Utc::now();
        let result = World::new("   \t\n  ", "Description", now);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_world_new_name_exceeds_200_chars_returns_error() {
        let now = Utc::now();
        let long_name = "a".repeat(201);
        let result = World::new(long_name, "Description", now);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
        assert!(err.to_string().contains("exceed 200 characters"));
    }

    #[test]
    fn test_world_new_valid_name_succeeds() {
        let now = Utc::now();
        let result = World::new("My World", "Description", now);
        assert!(result.is_ok());
        let world = result.unwrap();
        assert_eq!(world.name, "My World");
        assert_eq!(world.description, "Description");
    }

    #[test]
    fn test_world_new_trims_whitespace_from_name() {
        let now = Utc::now();
        let result = World::new("  My World  ", "Description", now);
        assert!(result.is_ok());
        let world = result.unwrap();
        assert_eq!(world.name, "My World");
    }

    #[test]
    fn test_world_new_name_exactly_200_chars_succeeds() {
        let now = Utc::now();
        let name = "a".repeat(200);
        let result = World::new(name.clone(), "Description", now);
        assert!(result.is_ok());
        let world = result.unwrap();
        assert_eq!(world.name, name);
    }
}
