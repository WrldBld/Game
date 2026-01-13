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
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::value_objects::{Description, RuleSystemConfig, WorldName};
use wrldbldr_domain::{GameTime, GameTimeConfig, TimeAdvanceReason, TimeCostConfig, TimeMode, WorldId};

// Re-export from entities for now (TimeAdvanceResult)
pub use crate::entities::TimeAdvanceResult;

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
/// use wrldbldr_domain::WorldId;
/// use wrldbldr_domain::aggregates::world::World;
/// use wrldbldr_domain::value_objects::{WorldName, Description};
///
/// let name = WorldName::new("Middle-earth").unwrap();
/// let world = World::new(name);
///
/// assert_eq!(world.name().as_str(), "Middle-earth");
/// ```
#[derive(Debug, Clone)]
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
    /// use wrldbldr_domain::WorldId;
    /// use wrldbldr_domain::aggregates::world::World;
    /// use wrldbldr_domain::value_objects::WorldName;
    ///
    /// let name = WorldName::new("Narnia").unwrap();
    /// let world = World::new(name);
    ///
    /// assert_eq!(world.name().as_str(), "Narnia");
    /// ```
    pub fn new(name: WorldName) -> Self {
        let now = Utc::now();
        Self {
            id: WorldId::new(),
            name,
            description: Description::empty(),
            rule_system: RuleSystemConfig::default(),
            game_time: GameTime::new(now),
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
    pub fn set_name(&mut self, name: WorldName) {
        self.name = name;
        self.updated_at = Utc::now();
    }

    /// Update the world's description.
    pub fn set_description(&mut self, description: Description) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    /// Set the time mode (manual, suggested, auto).
    pub fn set_time_mode(&mut self, mode: TimeMode) {
        self.time_config.mode = mode;
        self.updated_at = Utc::now();
    }

    /// Set the time cost configuration.
    pub fn set_time_costs(&mut self, costs: TimeCostConfig) {
        self.time_config.time_costs = costs;
        self.updated_at = Utc::now();
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
    pub fn advance_time(&mut self, minutes: u32, _reason: TimeAdvanceReason) -> TimeAdvanceResult {
        let previous_time = self.game_time.clone();
        let previous_period = self.game_time.time_of_day();

        self.game_time.advance_minutes(minutes);
        self.updated_at = Utc::now();

        let new_period = self.game_time.time_of_day();

        TimeAdvanceResult {
            previous_time,
            new_time: self.game_time.clone(),
            minutes_advanced: minutes,
            period_changed: previous_period != new_period,
        }
    }

    /// Advance game time by a number of hours.
    pub fn advance_hours(&mut self, hours: u32) -> TimeAdvanceResult {
        self.advance_time(hours * 60, TimeAdvanceReason::DmManual { hours })
    }
}

// ============================================================================
// Serde Implementation
// ============================================================================

/// Intermediate format for serialization that matches the wire format
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldWireFormat {
    id: WorldId,
    name: WorldName,
    description: Description,
    rule_system: RuleSystemConfig,
    game_time: GameTime,
    #[serde(default)]
    time_config: GameTimeConfig,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Serialize for World {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire = WorldWireFormat {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            rule_system: self.rule_system.clone(),
            game_time: self.game_time.clone(),
            time_config: self.time_config.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for World {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Support both new format (newtypes) and legacy format (raw strings)
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct LegacyWorldFormat {
            id: WorldId,
            name: WorldName,
            #[serde(default)]
            description: Description,
            #[serde(default)]
            rule_system: RuleSystemConfig,
            game_time: GameTime,
            #[serde(default)]
            time_config: GameTimeConfig,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let legacy = LegacyWorldFormat::deserialize(deserializer)?;

        Ok(World {
            id: legacy.id,
            name: legacy.name,
            description: legacy.description,
            rule_system: legacy.rule_system,
            game_time: legacy.game_time,
            time_config: legacy.time_config,
            created_at: legacy.created_at,
            updated_at: legacy.updated_at,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_world() -> World {
        let name = WorldName::new("Test World").unwrap();
        World::new(name)
    }

    mod constructor {
        use super::*;

        #[test]
        fn new_creates_world_with_correct_defaults() {
            let name = WorldName::new("Middle-earth").unwrap();
            let world = World::new(name);

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

            let world = World::new(name)
                .with_description(desc);

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
        fn set_name_updates_name_and_timestamp() {
            let mut world = create_test_world();
            let original_updated = world.updated_at();

            // Small delay to ensure timestamp changes
            std::thread::sleep(std::time::Duration::from_millis(10));

            let new_name = WorldName::new("Updated World").unwrap();
            world.set_name(new_name);

            assert_eq!(world.name().as_str(), "Updated World");
            assert!(world.updated_at() > original_updated);
        }

        #[test]
        fn set_description_updates_description_and_timestamp() {
            let mut world = create_test_world();
            let original_updated = world.updated_at();

            // Small delay to ensure timestamp changes
            std::thread::sleep(std::time::Duration::from_millis(10));

            let desc = Description::new("A new description").unwrap();
            world.set_description(desc);

            assert_eq!(world.description().as_str(), "A new description");
            assert!(world.updated_at() > original_updated);
        }

        #[test]
        fn set_time_mode_works() {
            let mut world = create_test_world();
            world.set_time_mode(TimeMode::Manual);
            assert!(matches!(world.time_config().mode, TimeMode::Manual));
        }
    }

    mod time_advancement {
        use super::*;

        #[test]
        fn advance_time_returns_result() {
            let mut world = create_test_world();
            let result = world.advance_time(60, TimeAdvanceReason::DmManual { hours: 1 });

            assert_eq!(result.minutes_advanced, 60);
        }

        #[test]
        fn advance_hours_works() {
            let mut world = create_test_world();
            let result = world.advance_hours(2);

            assert_eq!(result.minutes_advanced, 120);
        }
    }

    mod serde {
        use super::*;

        #[test]
        fn serialize_deserialize_roundtrip() {
            let name = WorldName::new("Westeros").unwrap();
            let desc = Description::new("A land of ice and fire").unwrap();

            let world = World::new(name).with_description(desc);

            let json = serde_json::to_string(&world).unwrap();
            let deserialized: World = serde_json::from_str(&json).unwrap();

            assert_eq!(deserialized.id(), world.id());
            assert_eq!(deserialized.name().as_str(), "Westeros");
            assert_eq!(deserialized.description().as_str(), "A land of ice and fire");
        }

        #[test]
        fn serialize_produces_camel_case() {
            let world = create_test_world();
            let json = serde_json::to_string(&world).unwrap();

            assert!(json.contains("ruleSystem"));
            assert!(json.contains("gameTime"));
            assert!(json.contains("timeConfig"));
            assert!(json.contains("createdAt"));
            assert!(json.contains("updatedAt"));
        }
    }
}
