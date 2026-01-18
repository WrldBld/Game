//! Scene aggregate - Complete storytelling unit
//!
//! # Graph-First Design
//!
//! The following relationships are stored as Neo4j edges, NOT embedded fields:
//! - Location: `(Scene)-[:AT_LOCATION]->(Location)` - use `scene_repo.get_location()` / `set_location()`
//! - Featured characters: `(Scene)-[:FEATURES_CHARACTER {role, entrance_cue}]->(Character)` - use `scene_repo.get_featured_characters()` / `set_featured_characters()`
//!
//! Entry conditions remain as JSON (acceptable per ADR - complex nested non-relational)
//!
//! # Rustic DDD Design
//!
//! This aggregate follows Rustic DDD principles:
//! - **Private fields**: All fields are encapsulated
//! - **Valid by construction**: `new()` takes pre-validated types
//! - **Builder pattern**: Fluent API for optional fields

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use wrldbldr_domain::{ActId, SceneId};

use crate::events::SceneUpdate;
use crate::value_objects::SceneName;

// Re-export from entities for now (TimeContext, SceneCondition, SceneCharacter, SceneCharacterRole)
pub use crate::entities::{SceneCharacter, SceneCharacterRole, SceneCondition, TimeContext};

/// A scene - a complete unit of storytelling
///
/// # Invariants
///
/// - Scene always has a valid `name` (non-empty)
/// - Scene always belongs to an `act_id`
///
/// # Graph Relationships
///
/// Location and featured characters are stored as graph edges:
/// - Location: AT_LOCATION edge via `scene_repo.get_location()` / `set_location()`
/// - Characters: FEATURES_CHARACTER edge via `scene_repo.get_featured_characters()` / `set_featured_characters()`
///
/// # Example
///
/// ```
/// use wrldbldr_domain::{ActId, SceneId, SceneName};
/// use wrldbldr_domain::aggregates::scene::Scene;
///
/// let act_id = ActId::new();
/// let scene = Scene::new(act_id, SceneName::new("The Tavern Meeting").unwrap());
///
/// assert_eq!(scene.name().as_str(), "The Tavern Meeting");
/// ```
#[derive(Debug, Clone)]
pub struct Scene {
    // Identity
    id: SceneId,
    act_id: ActId,

    // Core attributes
    name: SceneName,

    // Time and visual settings
    time_context: TimeContext,
    /// Override backdrop (if different from location default)
    backdrop_override: Option<String>,

    // Entry conditions
    /// Conditions that must be met to enter this scene (stored as JSON)
    entry_conditions: Vec<SceneCondition>,

    // Direction
    /// DM guidance for LLM responses
    directorial_notes: String,

    // Ordering
    /// Order within the act (for sequential scenes)
    order: u32,
}

impl Scene {
    // =========================================================================
    // Constructor
    // =========================================================================

    /// Create a new scene with the given act and name.
    ///
    /// After creating the scene, use `scene_repo.set_location()` to associate it with a location.
    ///
    /// # Example
    ///
    /// ```
    /// use wrldbldr_domain::{ActId, SceneName};
    /// use wrldbldr_domain::aggregates::scene::Scene;
    ///
    /// let act_id = ActId::new();
    /// let scene = Scene::new(
    ///     act_id,
    ///     SceneName::new("The Final Confrontation").unwrap(),
    /// );
    ///
    /// assert_eq!(scene.name().as_str(), "The Final Confrontation");
    /// assert_eq!(scene.order(), 0);
    /// ```
    pub fn new(act_id: ActId, name: SceneName) -> Self {
        Self {
            id: SceneId::new(),
            act_id,
            name,
            time_context: TimeContext::Unspecified,
            backdrop_override: None,
            entry_conditions: Vec::new(),
            directorial_notes: String::new(),
            order: 0,
        }
    }

    // =========================================================================
    // Identity Accessors (read-only)
    // =========================================================================

    /// Returns the scene's unique identifier.
    #[inline]
    pub fn id(&self) -> SceneId {
        self.id
    }

    /// Returns the ID of the act this scene belongs to.
    #[inline]
    pub fn act_id(&self) -> ActId {
        self.act_id
    }

    /// Returns the scene's name.
    #[inline]
    pub fn name(&self) -> &SceneName {
        &self.name
    }

    // =========================================================================
    // Time/Visual Accessors
    // =========================================================================

    /// Returns the scene's time context.
    #[inline]
    pub fn time_context(&self) -> &TimeContext {
        &self.time_context
    }

    /// Returns the scene's backdrop override, if any.
    #[inline]
    pub fn backdrop_override(&self) -> Option<&str> {
        self.backdrop_override.as_deref()
    }

    // =========================================================================
    // Entry Conditions Accessors
    // =========================================================================

    /// Returns the scene's entry conditions.
    #[inline]
    pub fn entry_conditions(&self) -> &[SceneCondition] {
        &self.entry_conditions
    }

    // =========================================================================
    // Direction Accessors
    // =========================================================================

    /// Returns the scene's directorial notes.
    #[inline]
    pub fn directorial_notes(&self) -> &str {
        &self.directorial_notes
    }

    // =========================================================================
    // Ordering Accessors
    // =========================================================================

    /// Returns the scene's order within the act.
    #[inline]
    pub fn order(&self) -> u32 {
        self.order
    }

    // =========================================================================
    // Builder Methods (for construction)
    // =========================================================================

    /// Set the scene's time context.
    pub fn with_time(mut self, time_context: TimeContext) -> Self {
        self.time_context = time_context;
        self
    }

    /// Set the scene's directorial notes.
    pub fn with_directorial_notes(mut self, notes: impl Into<String>) -> Self {
        self.directorial_notes = notes.into();
        self
    }

    /// Add an entry condition to the scene.
    pub fn with_entry_condition(mut self, condition: SceneCondition) -> Self {
        self.entry_conditions.push(condition);
        self
    }

    /// Set the scene's order within the act.
    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }

    /// Set the scene's backdrop override.
    pub fn with_backdrop_override(mut self, backdrop: impl Into<String>) -> Self {
        self.backdrop_override = Some(backdrop.into());
        self
    }

    /// Set the scene's ID (used when loading from storage).
    pub fn with_id(mut self, id: SceneId) -> Self {
        self.id = id;
        self
    }

    /// Set the scene's entry conditions (used when loading from storage).
    pub fn with_entry_conditions(mut self, conditions: Vec<SceneCondition>) -> Self {
        self.entry_conditions = conditions;
        self
    }

    // =========================================================================
    // Mutation Methods
    // =========================================================================

    /// Set the scene's name.
    pub fn set_name(&mut self, name: SceneName) -> SceneUpdate {
        let previous = std::mem::replace(&mut self.name, name);
        SceneUpdate::NameChanged {
            from: previous,
            to: self.name.clone(),
        }
    }

    /// Set the scene's time context.
    pub fn set_time_context(&mut self, time_context: TimeContext) -> SceneUpdate {
        let previous = self.time_context.clone();
        self.time_context = time_context;
        SceneUpdate::TimeContextChanged {
            from: previous,
            to: self.time_context.clone(),
        }
    }

    /// Set the scene's backdrop override.
    pub fn set_backdrop_override(&mut self, backdrop: Option<String>) -> SceneUpdate {
        let previous = std::mem::replace(&mut self.backdrop_override, backdrop);
        SceneUpdate::BackdropOverrideChanged {
            from: previous,
            to: self.backdrop_override.clone(),
        }
    }

    /// Set the scene's directorial notes.
    pub fn set_directorial_notes(&mut self, notes: impl Into<String>) -> SceneUpdate {
        let next = notes.into();
        let previous = std::mem::replace(&mut self.directorial_notes, next);
        SceneUpdate::DirectorialNotesChanged {
            from: previous,
            to: self.directorial_notes.clone(),
        }
    }

    /// Set the scene's order within the act.
    pub fn set_order(&mut self, order: u32) -> SceneUpdate {
        let previous = self.order;
        self.order = order;
        SceneUpdate::OrderChanged {
            from: previous,
            to: self.order,
        }
    }

    /// Add an entry condition.
    pub fn add_entry_condition(&mut self, condition: SceneCondition) -> SceneUpdate {
        let added = condition.clone();
        self.entry_conditions.push(condition);
        SceneUpdate::EntryConditionAdded { condition: added }
    }

    /// Clear all entry conditions.
    pub fn clear_entry_conditions(&mut self) -> SceneUpdate {
        let previous_count = self.entry_conditions.len();
        self.entry_conditions.clear();
        SceneUpdate::EntryConditionsCleared { previous_count }
    }
}

// ============================================================================
// Serde Implementation
// ============================================================================

/// Intermediate format for serialization that matches the wire format.
/// Note: location_id and featured_characters are managed via graph edges,
/// not serialized with the aggregate.
#[derive(Serialize, Deserialize)]
struct SceneWireFormat {
    id: SceneId,
    act_id: ActId,
    name: String,
    time_context: TimeContext,
    backdrop_override: Option<String>,
    entry_conditions: Vec<SceneCondition>,
    directorial_notes: String,
    order: u32,
}

impl Serialize for Scene {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire = SceneWireFormat {
            id: self.id,
            act_id: self.act_id,
            name: self.name.to_string(),
            time_context: self.time_context.clone(),
            backdrop_override: self.backdrop_override.clone(),
            entry_conditions: self.entry_conditions.clone(),
            directorial_notes: self.directorial_notes.clone(),
            order: self.order,
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Scene {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SceneWireFormat::deserialize(deserializer)?;

        let name = SceneName::new(wire.name).map_err(DeError::custom)?;

        Ok(Scene {
            id: wire.id,
            act_id: wire.act_id,
            name,
            time_context: wire.time_context,
            backdrop_override: wire.backdrop_override,
            entry_conditions: wire.entry_conditions,
            directorial_notes: wire.directorial_notes,
            order: wire.order,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_scene() -> Scene {
        let act_id = ActId::new();
        Scene::new(act_id, SceneName::new("Test Scene").unwrap())
    }

    mod constructor {
        use super::*;

        #[test]
        fn new_creates_scene_with_correct_defaults() {
            let act_id = ActId::new();
            let scene = Scene::new(act_id, SceneName::new("The Opening").unwrap());

            assert_eq!(scene.name().as_str(), "The Opening");
            assert_eq!(scene.act_id(), act_id);
            assert!(matches!(scene.time_context(), TimeContext::Unspecified));
            assert!(scene.backdrop_override().is_none());
            assert!(scene.entry_conditions().is_empty());
            assert!(scene.directorial_notes().is_empty());
            assert_eq!(scene.order(), 0);
        }

        #[test]
        fn builder_methods_work() {
            let act_id = ActId::new();

            let scene = Scene::new(act_id, SceneName::new("The Climax").unwrap())
                .with_time(TimeContext::Custom("Midnight".to_string()))
                .with_directorial_notes("Dramatic tension!")
                .with_order(5)
                .with_backdrop_override("dark_throne_room.png");

            assert_eq!(scene.name().as_str(), "The Climax");
            assert!(matches!(scene.time_context(), TimeContext::Custom(s) if s == "Midnight"));
            assert_eq!(scene.directorial_notes(), "Dramatic tension!");
            assert_eq!(scene.order(), 5);
            assert_eq!(scene.backdrop_override(), Some("dark_throne_room.png"));
        }
    }

    mod mutation {
        use super::*;

        #[test]
        fn set_name_works() {
            let mut scene = create_test_scene();
            scene.set_name(SceneName::new("New Name").unwrap());
            assert_eq!(scene.name().as_str(), "New Name");
        }

        #[test]
        fn set_order_works() {
            let mut scene = create_test_scene();
            scene.set_order(10);
            assert_eq!(scene.order(), 10);
        }

        #[test]
        fn set_backdrop_override_works() {
            let mut scene = create_test_scene();

            scene.set_backdrop_override(Some("new_backdrop.png".to_string()));
            assert_eq!(scene.backdrop_override(), Some("new_backdrop.png"));

            scene.set_backdrop_override(None);
            assert!(scene.backdrop_override().is_none());
        }
    }
}
