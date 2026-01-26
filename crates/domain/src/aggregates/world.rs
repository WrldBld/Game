//! World aggregate - The top-level container for a campaign setting
//!
//! # Rustic DDD Design
//!
//! This aggregate follows Rustic DDD principles:
//! - **Private fields**: All fields are encapsulated
//! - **Newtypes**: `WorldName` and `Description` for validated strings
//! - **Valid by construction**: `new()` takes pre-validated types
//! - **Builder pattern**: Fluent API for optional fields

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::value_objects::{Description, RuleSystemConfig, WorldName};
use wrldbldr_domain::{
    GameTime, GameTimeConfig, TimeAdvanceReason, TimeCostConfig, TimeMode, WorldId,
};

// Re-export from entities for now (TimeAdvanceResult)
pub use crate::entities::TimeAdvanceResult;

// =============================================================================
// Domain Events
// =============================================================================

/// Events emitted when a World aggregate is mutated
///
/// These events capture both old and new values, allowing systems to track
/// what changed and respond appropriately (e.g., log changes, trigger reactions).
#[derive(Debug, Clone, PartialEq)]
pub enum WorldUpdate {
    /// The world's name was changed
    NameChanged {
        old_name: WorldName,
        new_name: WorldName,
    },
    /// The world's description was changed
    DescriptionChanged {
        old_description: Description,
        new_description: Description,
    },
    /// The time mode (manual/suggested) was changed
    TimeModeChanged {
        old_mode: TimeMode,
        new_mode: TimeMode,
    },
    /// The time cost configuration was changed
    TimeCostsChanged {
        old_costs: TimeCostConfig,
        new_costs: TimeCostConfig,
    },
}

/// A complete campaign world
///
/// # Invariants
///
/// - `name` is always non-empty and <= 200 characters (enforced by `WorldName`)
/// - `description` is always <= 5000 characters (enforced by `Description`)
///
/// # Example
///
/// ```
/// use chrono::TimeZone;
/// use wrldbldr_domain::WorldId;
/// use wrldbldr_domain::aggregates::world::World;
/// use wrldbldr_domain::value_objects::{WorldName, Description};
///
/// let name = WorldName::new("Middle-earth").unwrap();
/// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
/// let world = World::new(name, now);
///
/// assert_eq!(world.name().as_str(), "Middle-earth");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    // Identity
    id: WorldId,

    // Core attributes (newtypes)
    name: WorldName,
    description: Description,

    // Configuration
    rule_system: RuleSystemConfig,

    // Time management
    /// In-game time for the world (persisted, not session-scoped)
    game_time: GameTime,
    /// Configuration for how game time behaves
    time_config: GameTimeConfig,

    // Timestamps
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl World {
    // =========================================================================
    // Constructor
    // =========================================================================

    /// Create a new world with the given name.
    ///
    /// The `name` parameter must be a pre-validated `WorldName` - validation
    /// happens when creating the `WorldName`, not here.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::TimeZone;
    /// use wrldbldr_domain::WorldId;
    /// use wrldbldr_domain::aggregates::world::World;
    /// use wrldbldr_domain::value_objects::WorldName;
    ///
    /// let name = WorldName::new("Narnia").unwrap();
    /// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    /// let world = World::new(name, now);
    ///
    /// assert_eq!(world.name().as_str(), "Narnia");
    /// ```
    pub fn new(name: WorldName, now: DateTime<Utc>) -> Self {
        Self {
            id: WorldId::new(),
            name,
            description: Description::empty(),
            rule_system: RuleSystemConfig::default(),
            game_time: GameTime::at_epoch(),
            time_config: GameTimeConfig::default(),
            created_at: now,
            updated_at: now,
        }
    }

    // =========================================================================
    // Identity Accessors (read-only)
    // =========================================================================

    /// Returns the world's unique identifier.
    #[inline]
    pub fn id(&self) -> WorldId {
        self.id
    }

    /// Returns the world's name.
    #[inline]
    pub fn name(&self) -> &WorldName {
        &self.name
    }

    /// Returns the world's description.
    #[inline]
    pub fn description(&self) -> &Description {
        &self.description
    }

    // =========================================================================
    // Configuration Accessors
    // =========================================================================

    /// Returns the world's rule system configuration.
    #[inline]
    pub fn rule_system(&self) -> &RuleSystemConfig {
        &self.rule_system
    }

    // =========================================================================
    // Time Accessors
    // =========================================================================

    /// Returns the world's current game time.
    #[inline]
    pub fn game_time(&self) -> &GameTime {
        &self.game_time
    }

    /// Returns a mutable reference to the world's game time.
    #[inline]
    pub fn game_time_mut(&mut self) -> &mut GameTime {
        &mut self.game_time
    }

    /// Returns the world's time configuration.
    #[inline]
    pub fn time_config(&self) -> &GameTimeConfig {
        &self.time_config
    }

    // =========================================================================
    // Timestamp Accessors
    // =========================================================================

    /// Returns when the world was created.
    #[inline]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns when the world was last updated.
    #[inline]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // =========================================================================
    // Builder Methods (for construction)
    // =========================================================================

    /// Set the world's description.
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = description;
        self
    }

    /// Set the world's rule system configuration.
    pub fn with_rule_system(mut self, rule_system: RuleSystemConfig) -> Self {
        self.rule_system = rule_system;
        self
    }

    /// Set the world's time configuration.
    pub fn with_time_config(mut self, time_config: GameTimeConfig) -> Self {
        self.time_config = time_config;
        self
    }

    /// Set the world's ID (used when loading from storage).
    pub fn with_id(mut self, id: WorldId) -> Self {
        self.id = id;
        self
    }

    /// Set the world's game time (used when loading from storage).
    pub fn with_game_time(mut self, game_time: GameTime) -> Self {
        self.game_time = game_time;
        self
    }

    /// Set the world's created_at timestamp (used when loading from storage).
    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    /// Set the world's updated_at timestamp (used when loading from storage).
    pub fn with_updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = updated_at;
        self
    }

    // =========================================================================
    // Mutation Methods
    // =========================================================================

    /// Update the world's name.
    ///
    /// Returns a `WorldUpdate` event capturing the old and new names.
    pub fn set_name(&mut self, name: WorldName, now: DateTime<Utc>) -> WorldUpdate {
        let old_name = self.name.clone();
        self.name = name.clone();
        self.updated_at = now;
        WorldUpdate::NameChanged {
            old_name,
            new_name: name,
        }
    }

    /// Update the world's description.
    ///
    /// Returns a `WorldUpdate` event capturing the old and new descriptions.
    pub fn set_description(&mut self, description: Description, now: DateTime<Utc>) -> WorldUpdate {
        let old_description = self.description.clone();
        self.description = description.clone();
        self.updated_at = now;
        WorldUpdate::DescriptionChanged {
            old_description,
            new_description: description,
        }
    }

    /// Set the time mode (manual, suggested).
    ///
    /// Returns a `WorldUpdate` event capturing the old and new time modes.
    pub fn set_time_mode(&mut self, mode: TimeMode, now: DateTime<Utc>) -> WorldUpdate {
        let old_mode = self.time_config.mode();
        self.time_config.set_mode(mode);
        self.updated_at = now;
        WorldUpdate::TimeModeChanged {
            old_mode,
            new_mode: mode,
        }
    }

    /// Set the time cost configuration.
    ///
    /// Returns a `WorldUpdate` event capturing the old and new time cost configs.
    pub fn set_time_costs(&mut self, costs: TimeCostConfig, now: DateTime<Utc>) -> WorldUpdate {
        let old_costs = self.time_config.time_costs().clone();
        self.time_config.set_time_costs(costs.clone());
        self.updated_at = now;
        WorldUpdate::TimeCostsChanged {
            old_costs,
            new_costs: costs,
        }
    }

    /// Get the time cost for a given action type.
    pub fn time_cost_for_action(&self, action: &str) -> u32 {
        self.time_config.time_costs().cost_for_action(action)
    }

    // =========================================================================
    // Time Advancement
    // =========================================================================

    /// Advance game time by a number of seconds.
    /// Returns information about the time change for broadcasting.
    pub fn advance_time(
        &mut self,
        seconds: u32,
        _reason: TimeAdvanceReason,
        now: DateTime<Utc>,
    ) -> TimeAdvanceResult {
        let previous_time = self.game_time.clone();
        let previous_period = self.game_time.time_of_day();

        self.game_time.advance_seconds(seconds);
        self.updated_at = now;

        let new_period = self.game_time.time_of_day();

        TimeAdvanceResult {
            previous_time,
            new_time: self.game_time.clone(),
            seconds_advanced: seconds,
            period_changed: previous_period != new_period,
        }
    }

    /// Advance game time by a number of hours.
    pub fn advance_hours(&mut self, hours: u32, now: DateTime<Utc>) -> TimeAdvanceResult {
        self.advance_time(hours * 3600, TimeAdvanceReason::DmManual { hours }, now)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_time() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    fn create_test_world() -> World {
        let name = WorldName::new("Test World").unwrap();
        World::new(name, fixed_time())
    }

    mod constructor {
        use super::*;

        #[test]
        fn new_creates_world_with_correct_defaults() {
            let name = WorldName::new("Middle-earth").unwrap();
            let world = World::new(name, fixed_time());

            assert_eq!(world.name().as_str(), "Middle-earth");
            assert!(world.description().is_empty());
            // Default rule system is GenericD20 (D20 type)
            assert!(matches!(
                world.rule_system().system_type,
                crate::value_objects::RuleSystemType::D20
            ));
        }

        #[test]
        fn builder_methods_work() {
            let name = WorldName::new("Narnia").unwrap();
            let desc = Description::new("A magical land beyond the wardrobe").unwrap();

            let world = World::new(name, fixed_time()).with_description(desc);

            assert_eq!(world.name().as_str(), "Narnia");
            assert_eq!(
                world.description().as_str(),
                "A magical land beyond the wardrobe"
            );
        }
    }

    mod mutation {
        use super::*;

        #[test]
        fn set_name_returns_name_changed_event() {
            let mut world = create_test_world();
            let original_updated = world.updated_at();
            let original_name = world.name().clone();

            let new_name = WorldName::new("Updated World").unwrap();
            let event = world.set_name(
                new_name.clone(),
                original_updated + chrono::Duration::seconds(1),
            );

            assert!(matches!(
                event,
                WorldUpdate::NameChanged { ref old_name, ref new_name } if old_name.as_str() == "Test World" && new_name.as_str() == "Updated World"
            ));

            if let WorldUpdate::NameChanged { old_name, new_name } = event {
                assert_eq!(old_name.as_str(), "Test World");
                assert_eq!(new_name.as_str(), "Updated World");
                assert_eq!(&old_name, &original_name);
            }

            assert_eq!(world.name().as_str(), "Updated World");
            assert!(world.updated_at() > original_updated);
        }

        #[test]
        fn set_description_returns_description_changed_event() {
            let mut world = create_test_world();
            let original_updated = world.updated_at();
            let original_desc = world.description().clone();

            let desc = Description::new("A new description").unwrap();
            let event = world.set_description(
                desc.clone(),
                original_updated + chrono::Duration::seconds(1),
            );

            assert!(matches!(
                event,
                WorldUpdate::DescriptionChanged { ref old_description, ref new_description } if old_description.is_empty() && new_description.as_str() == "A new description"
            ));

            if let WorldUpdate::DescriptionChanged {
                old_description,
                new_description,
            } = event
            {
                assert!(old_description.is_empty());
                assert_eq!(new_description.as_str(), "A new description");
                assert_eq!(&old_description, &original_desc);
            }

            assert_eq!(world.description().as_str(), "A new description");
            assert!(world.updated_at() > original_updated);
        }

        #[test]
        fn set_time_mode_returns_time_mode_changed_event() {
            let mut world = create_test_world();

            let new_mode = TimeMode::Manual;
            let event =
                world.set_time_mode(new_mode, world.updated_at() + chrono::Duration::seconds(1));

            assert!(matches!(
                event,
                WorldUpdate::TimeModeChanged {
                    old_mode: TimeMode::Suggested,
                    new_mode: TimeMode::Manual
                }
            ));

            if let WorldUpdate::TimeModeChanged {
                old_mode: om,
                new_mode: nm,
            } = event
            {
                assert_eq!(om, TimeMode::Suggested);
                assert_eq!(nm, TimeMode::Manual);
            }

            assert!(matches!(world.time_config().mode(), TimeMode::Manual));
        }

        #[test]
        fn set_time_costs_returns_time_costs_changed_event() {
            let mut world = create_test_world();
            let old_costs = world.time_config().time_costs().clone();

            let new_costs = TimeCostConfig {
                travel_location: 30,
                travel_region: 5,
                rest_short: 30,
                rest_long: 240,
                conversation: 1,
                challenge: 5,
                scene_transition: 1,
            };

            let event = world.set_time_costs(
                new_costs.clone(),
                world.updated_at() + chrono::Duration::seconds(1),
            );

            assert!(matches!(
                event,
                WorldUpdate::TimeCostsChanged {
                    old_costs: _,
                    new_costs: _
                }
            ));

            if let WorldUpdate::TimeCostsChanged {
                old_costs: oc,
                new_costs: nc,
            } = event
            {
                assert_eq!(oc, old_costs);
                assert_eq!(nc, new_costs);
            }

            assert_eq!(world.time_config().time_costs(), &new_costs);
        }
    }

    mod time_advancement {
        use super::*;

        #[test]
        fn advance_time_returns_result() {
            let mut world = create_test_world();
            let result = world.advance_time(
                3600, // 1 hour = 3600 seconds
                TimeAdvanceReason::DmManual { hours: 1 },
                world.updated_at() + chrono::Duration::seconds(1),
            );

            assert_eq!(result.seconds_advanced, 3600);
        }

        #[test]
        fn advance_hours_works() {
            let mut world = create_test_world();
            let result = world.advance_hours(2, world.updated_at() + chrono::Duration::seconds(1));

            assert_eq!(result.seconds_advanced, 7200); // 2 * 3600
        }
    }
}
