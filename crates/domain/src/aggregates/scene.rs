//! Scene aggregate - Complete storytelling unit
//!
//! # Graph-First Design (Phase 0.D)
//!
//! The following relationships are stored as Neo4j edges, NOT embedded fields:
//! - Location: `(Scene)-[:AT_LOCATION]->(Location)`
//! - Featured characters: `(Scene)-[:FEATURES_CHARACTER {role, entrance_cue}]->(Character)`
//!
//! Entry conditions remain as JSON (acceptable per ADR - complex nested non-relational)
//!
//! # Rustic DDD Design
//!
//! This aggregate follows Rustic DDD principles:
//! - **Private fields**: All fields are encapsulated
//! - **Valid by construction**: `new()` takes pre-validated types
//! - **Builder pattern**: Fluent API for optional fields

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error as DeError;
use wrldbldr_domain::{ActId, CharacterId, LocationId, SceneId};

use crate::value_objects::SceneName;

// Re-export from entities for now (TimeContext, SceneCondition, SceneCharacter, SceneCharacterRole)
pub use crate::entities::{SceneCharacter, SceneCharacterRole, SceneCondition, TimeContext};

/// A scene - a complete unit of storytelling
///
/// # Invariants
///
/// - Scene always has a valid `name` (non-empty)
/// - Scene always belongs to an `act_id`
/// - Scene always has a `location_id` (though this may be stored as an edge in graph)
///
/// # Graph Relationships
///
/// NOTE: `location_id` and `featured_characters` are kept for backward compatibility
/// during Phase 0.D migration. New code should use repository edge methods:
/// - Location: AT_LOCATION edge via `scene_repository.set_location()`
/// - Characters: FEATURES_CHARACTER edge via `scene_repository.add_featured_character()`
///
/// # Example
///
/// ```
/// use wrldbldr_domain::{ActId, LocationId, SceneId, SceneName};
/// use wrldbldr_domain::aggregates::scene::Scene;
///
/// let act_id = ActId::new();
/// let location_id = LocationId::new();
/// let scene = Scene::new(act_id, SceneName::new("The Tavern Meeting").unwrap(), location_id);
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
    /// DEPRECATED: Use AT_LOCATION edge via repository
    location_id: LocationId,

    // Time and visual settings
    time_context: TimeContext,
    /// Override backdrop (if different from location default)
    backdrop_override: Option<String>,

    // Entry conditions
    /// Conditions that must be met to enter this scene (stored as JSON)
    entry_conditions: Vec<SceneCondition>,

    // Featured characters (deprecated - use graph edges)
    /// DEPRECATED: Use FEATURES_CHARACTER edge via repository
    featured_characters: Vec<CharacterId>,

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

    /// Create a new scene with the given act, name, and location.
    ///
    /// # Example
    ///
    /// ```
    /// use wrldbldr_domain::{ActId, LocationId};
    /// use wrldbldr_domain::aggregates::scene::Scene;
    ///
    /// let act_id = ActId::new();
    /// let location_id = LocationId::new();
    /// let scene = Scene::new(
    ///     act_id,
    ///     SceneName::new("The Final Confrontation").unwrap(),
    ///     location_id,
    /// );
    ///
    /// assert_eq!(scene.name().as_str(), "The Final Confrontation");
    /// assert_eq!(scene.order(), 0);
    /// ```
    pub fn new(act_id: ActId, name: SceneName, location_id: LocationId) -> Self {
        Self {
            id: SceneId::new(),
            act_id,
            name,
            location_id,
            time_context: TimeContext::Unspecified,
            backdrop_override: None,
            entry_conditions: Vec::new(),
            featured_characters: Vec::new(),
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

    /// Returns the scene's location ID.
    /// DEPRECATED: Use AT_LOCATION edge via repository instead.
    #[inline]
    pub fn location_id(&self) -> LocationId {
        self.location_id
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
    // Featured Characters Accessors
    // =========================================================================

    /// Returns the scene's featured characters.
    /// DEPRECATED: Use FEATURES_CHARACTER edge via repository instead.
    #[inline]
    pub fn featured_characters(&self) -> &[CharacterId] {
        &self.featured_characters
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

    /// Add a featured character to the scene.
    /// DEPRECATED: Use FEATURES_CHARACTER edge via repository instead.
    pub fn with_character(mut self, character_id: CharacterId) -> Self {
        self.featured_characters.push(character_id);
        self
    }

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

    /// Set the scene's featured characters (used when loading from storage).
    /// DEPRECATED: Use FEATURES_CHARACTER edge via repository instead.
    pub fn with_featured_characters(mut self, characters: Vec<CharacterId>) -> Self {
        self.featured_characters = characters;
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
    pub fn set_name(&mut self, name: SceneName) {
        self.name = name;
    }

    /// Set the scene's location ID.
    /// DEPRECATED: Use AT_LOCATION edge via repository instead.
    pub fn set_location(&mut self, location_id: LocationId) {
        self.location_id = location_id;
    }

    /// Set the scene's time context.
    pub fn set_time_context(&mut self, time_context: TimeContext) {
        self.time_context = time_context;
    }

    /// Set the scene's backdrop override.
    pub fn set_backdrop_override(&mut self, backdrop: Option<String>) {
        self.backdrop_override = backdrop;
    }

    /// Set the scene's directorial notes.
    pub fn set_directorial_notes(&mut self, notes: impl Into<String>) {
        self.directorial_notes = notes.into();
    }

    /// Set the scene's order within the act.
    pub fn set_order(&mut self, order: u32) {
        self.order = order;
    }

    /// Add a featured character.
    /// DEPRECATED: Use FEATURES_CHARACTER edge via repository instead.
    pub fn add_featured_character(&mut self, character_id: CharacterId) {
        if !self.featured_characters.contains(&character_id) {
            self.featured_characters.push(character_id);
        }
    }

    /// Remove a featured character.
    /// DEPRECATED: Use FEATURES_CHARACTER edge via repository instead.
    pub fn remove_featured_character(&mut self, character_id: CharacterId) {
        self.featured_characters.retain(|id| *id != character_id);
    }

    /// Add an entry condition.
    pub fn add_entry_condition(&mut self, condition: SceneCondition) {
        self.entry_conditions.push(condition);
    }

    /// Clear all entry conditions.
    pub fn clear_entry_conditions(&mut self) {
        self.entry_conditions.clear();
    }
}

// ============================================================================
// Serde Implementation
// ============================================================================

/// Intermediate format for serialization that matches the wire format
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SceneWireFormat {
    id: SceneId,
    act_id: ActId,
    name: String,
    location_id: LocationId,
    time_context: TimeContext,
    backdrop_override: Option<String>,
    entry_conditions: Vec<SceneCondition>,
    featured_characters: Vec<CharacterId>,
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
            location_id: self.location_id,
            time_context: self.time_context.clone(),
            backdrop_override: self.backdrop_override.clone(),
            entry_conditions: self.entry_conditions.clone(),
            featured_characters: self.featured_characters.clone(),
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
            location_id: wire.location_id,
            time_context: wire.time_context,
            backdrop_override: wire.backdrop_override,
            entry_conditions: wire.entry_conditions,
            featured_characters: wire.featured_characters,
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
        let location_id = LocationId::new();
        Scene::new(
            act_id,
            SceneName::new("Test Scene").unwrap(),
            location_id,
        )
    }

    mod constructor {
        use super::*;

        #[test]
        fn new_creates_scene_with_correct_defaults() {
            let act_id = ActId::new();
            let location_id = LocationId::new();
            let scene = Scene::new(
                act_id,
                SceneName::new("The Opening").unwrap(),
                location_id,
            );

            assert_eq!(scene.name().as_str(), "The Opening");
            assert_eq!(scene.act_id(), act_id);
            assert_eq!(scene.location_id(), location_id);
            assert!(matches!(scene.time_context(), TimeContext::Unspecified));
            assert!(scene.backdrop_override().is_none());
            assert!(scene.entry_conditions().is_empty());
            assert!(scene.featured_characters().is_empty());
            assert!(scene.directorial_notes().is_empty());
            assert_eq!(scene.order(), 0);
        }

        #[test]
        fn builder_methods_work() {
            let act_id = ActId::new();
            let location_id = LocationId::new();
            let char_id = CharacterId::new();

            let scene = Scene::new(
                act_id,
                SceneName::new("The Climax").unwrap(),
                location_id,
            )
                .with_character(char_id)
                .with_time(TimeContext::Custom("Midnight".to_string()))
                .with_directorial_notes("Dramatic tension!")
                .with_order(5)
                .with_backdrop_override("dark_throne_room.png");

            assert_eq!(scene.name().as_str(), "The Climax");
            assert_eq!(scene.featured_characters(), &[char_id]);
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
        fn set_location_works() {
            let mut scene = create_test_scene();
            let new_location = LocationId::new();
            scene.set_location(new_location);
            assert_eq!(scene.location_id(), new_location);
        }

        #[test]
        fn set_order_works() {
            let mut scene = create_test_scene();
            scene.set_order(10);
            assert_eq!(scene.order(), 10);
        }

        #[test]
        fn add_remove_featured_character_works() {
            let mut scene = create_test_scene();
            let char_id = CharacterId::new();

            scene.add_featured_character(char_id);
            assert_eq!(scene.featured_characters(), &[char_id]);

            // Adding same character again should not duplicate
            scene.add_featured_character(char_id);
            assert_eq!(scene.featured_characters().len(), 1);

            scene.remove_featured_character(char_id);
            assert!(scene.featured_characters().is_empty());
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

    mod serde {
        use super::*;

        #[test]
        fn serialize_deserialize_roundtrip() {
            let act_id = ActId::new();
            let location_id = LocationId::new();
            let char_id = CharacterId::new();

            let scene = Scene::new(
                act_id,
                SceneName::new("Test Scene").unwrap(),
                location_id,
            )
                .with_character(char_id)
                .with_directorial_notes("Test notes")
                .with_order(3);

            let json = serde_json::to_string(&scene).unwrap();
            let deserialized: Scene = serde_json::from_str(&json).unwrap();

            assert_eq!(deserialized.id(), scene.id());
            assert_eq!(deserialized.act_id(), act_id);
            assert_eq!(deserialized.name(), "Test Scene");
            assert_eq!(deserialized.location_id(), location_id);
            assert_eq!(deserialized.featured_characters(), &[char_id]);
            assert_eq!(deserialized.directorial_notes(), "Test notes");
            assert_eq!(deserialized.order(), 3);
        }

        #[test]
        fn serialize_produces_camel_case() {
            let scene = create_test_scene();
            let json = serde_json::to_string(&scene).unwrap();

            assert!(json.contains("actId"));
            assert!(json.contains("locationId"));
            assert!(json.contains("timeContext"));
            assert!(json.contains("backdropOverride"));
            assert!(json.contains("entryConditions"));
            assert!(json.contains("featuredCharacters"));
            assert!(json.contains("directorialNotes"));
        }
    }
}
