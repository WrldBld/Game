//! LocationState entity - Visual configurations for locations
//!
//! LocationStates represent city-wide visual configurations that activate
//! based on rules (time, events, flags, etc.). Multiple states can be defined
//! for a location, with activation rules determining which is active.
//!
//! # Neo4j Relationships
//! - `(Location)-[:HAS_STATE]->(LocationState)` - Location has this state option
//! - `(Location)-[:ACTIVE_STATE]->(LocationState)` - Currently active (set by staging)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{LocationId, LocationStateId, WorldId};
use crate::value_objects::{
    ActivationLogic, ActivationRule, AssetPath, Atmosphere, Description, StateName,
};

/// A visual configuration for a location
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationState {
    id: LocationStateId,
    location_id: LocationId,
    world_id: WorldId,

    /// Name of this state (e.g., "City Holiday", "Under Siege")
    name: StateName,
    /// Description for DM reference
    description: Description,

    // Visual Configuration
    /// Override the location's default backdrop
    backdrop_override: Option<AssetPath>,
    /// Override the location's atmosphere text
    atmosphere_override: Option<Atmosphere>,
    /// Ambient sound asset path
    ambient_sound: Option<AssetPath>,
    /// Map overlay or tint (for navigation map)
    map_overlay: Option<AssetPath>,

    // Activation Rules
    /// Rules that determine when this state is active
    activation_rules: Vec<ActivationRule>,
    /// How rules are combined
    activation_logic: ActivationLogic,

    /// Priority when multiple states match (higher = preferred)
    priority: i32,
    /// If true, use when no other state matches
    is_default: bool,

    /// Generation prompt used when creating this state (for reference)
    generation_prompt: Option<String>,
    /// ComfyUI workflow ID used for generation (for reference)
    workflow_id: Option<String>,

    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl LocationState {
    pub fn new(
        location_id: LocationId,
        world_id: WorldId,
        name: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: LocationStateId::new(),
            location_id,
            world_id,
            name: StateName::new(name).unwrap_or_default(),
            description: Description::default(),
            backdrop_override: None,
            atmosphere_override: None,
            ambient_sound: None,
            map_overlay: None,
            activation_rules: Vec::new(),
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
            generation_prompt: None,
            workflow_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a default state that's always active (fallback)
    pub fn default_state(
        location_id: LocationId,
        world_id: WorldId,
        name: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            is_default: true,
            activation_rules: vec![ActivationRule::Always],
            generation_prompt: None,
            workflow_id: None,
            ..Self::new(location_id, world_id, name, now)
        }
    }

    /// Reconstruct from stored data
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: LocationStateId,
        location_id: LocationId,
        world_id: WorldId,
        name: StateName,
        description: Description,
        backdrop_override: Option<AssetPath>,
        atmosphere_override: Option<Atmosphere>,
        ambient_sound: Option<AssetPath>,
        map_overlay: Option<AssetPath>,
        activation_rules: Vec<ActivationRule>,
        activation_logic: ActivationLogic,
        priority: i32,
        is_default: bool,
        generation_prompt: Option<String>,
        workflow_id: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            location_id,
            world_id,
            name,
            description,
            backdrop_override,
            atmosphere_override,
            ambient_sound,
            map_overlay,
            activation_rules,
            activation_logic,
            priority,
            is_default,
            generation_prompt,
            workflow_id,
            created_at,
            updated_at,
        }
    }

    // Read-only accessors

    pub fn id(&self) -> LocationStateId {
        self.id
    }

    pub fn location_id(&self) -> LocationId {
        self.location_id
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn backdrop_override(&self) -> Option<&AssetPath> {
        self.backdrop_override.as_ref()
    }

    pub fn atmosphere_override(&self) -> Option<&Atmosphere> {
        self.atmosphere_override.as_ref()
    }

    pub fn ambient_sound(&self) -> Option<&AssetPath> {
        self.ambient_sound.as_ref()
    }

    pub fn map_overlay(&self) -> Option<&AssetPath> {
        self.map_overlay.as_ref()
    }

    pub fn activation_rules(&self) -> &[ActivationRule] {
        &self.activation_rules
    }

    pub fn activation_logic(&self) -> ActivationLogic {
        self.activation_logic
    }

    pub fn priority(&self) -> i32 {
        self.priority
    }

    pub fn is_default(&self) -> bool {
        self.is_default
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn generation_prompt(&self) -> Option<&str> {
        self.generation_prompt.as_deref()
    }

    pub fn workflow_id(&self) -> Option<&str> {
        self.workflow_id.as_deref()
    }

    // Builder-style methods

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Description::new(description).unwrap_or_default();
        self
    }

    pub fn with_backdrop(mut self, asset_path: AssetPath) -> Self {
        self.backdrop_override = Some(asset_path);
        self
    }

    pub fn with_atmosphere(mut self, atmosphere: Atmosphere) -> Self {
        self.atmosphere_override = Some(atmosphere);
        self
    }

    pub fn with_ambient_sound(mut self, sound_path: AssetPath) -> Self {
        self.ambient_sound = Some(sound_path);
        self
    }

    pub fn with_map_overlay(mut self, overlay_path: AssetPath) -> Self {
        self.map_overlay = Some(overlay_path);
        self
    }

    pub fn with_rules(mut self, rules: Vec<ActivationRule>, logic: ActivationLogic) -> Self {
        self.activation_rules = rules;
        self.activation_logic = logic;
        self
    }

    pub fn with_rule(mut self, rule: ActivationRule) -> Self {
        self.activation_rules.push(rule);
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_is_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    pub fn with_generation_prompt(mut self, prompt: String) -> Self {
        self.generation_prompt = Some(prompt);
        self
    }

    pub fn with_workflow_id(mut self, workflow_id: String) -> Self {
        self.workflow_id = Some(workflow_id);
        self
    }

    /// Create with a specific ID (for deterministic ID generation)
    pub fn new_with_id(
        id: LocationStateId,
        location_id: LocationId,
        world_id: WorldId,
        name: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            location_id,
            world_id,
            name: StateName::new(name).unwrap_or_default(),
            description: Description::default(),
            backdrop_override: None,
            atmosphere_override: None,
            ambient_sound: None,
            map_overlay: None,
            activation_rules: Vec::new(),
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
            generation_prompt: None,
            workflow_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if this state has any soft rules requiring LLM evaluation
    pub fn has_soft_rules(&self) -> bool {
        self.activation_rules.iter().any(|r| r.is_soft_rule())
    }

    /// Get only the hard rules
    pub fn hard_rules(&self) -> Vec<&ActivationRule> {
        self.activation_rules
            .iter()
            .filter(|r| r.is_hard_rule())
            .collect()
    }

    /// Get only the soft rules
    pub fn soft_rules(&self) -> Vec<&ActivationRule> {
        self.activation_rules
            .iter()
            .filter(|r| r.is_soft_rule())
            .collect()
    }

    /// Get a summary of this state for display
    pub fn summary(&self) -> LocationStateSummary {
        LocationStateSummary {
            id: self.id,
            name: self.name.to_string(),
            backdrop_override: self.backdrop_override.as_ref().map(|p| p.to_string()),
            atmosphere_override: self.atmosphere_override.as_ref().map(|a| a.to_string()),
            ambient_sound: self.ambient_sound.as_ref().map(|p| p.to_string()),
            priority: self.priority,
            is_default: self.is_default,
            generation_prompt: self.generation_prompt.clone(),
            workflow_id: self.workflow_id.clone(),
        }
    }
}

/// Summary of a location state for display/wire transfer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationStateSummary {
    pub id: LocationStateId,
    pub name: String,
    pub backdrop_override: Option<String>,
    pub atmosphere_override: Option<String>,
    pub ambient_sound: Option<String>,
    pub priority: i32,
    pub is_default: bool,
    pub generation_prompt: Option<String>,
    pub workflow_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_time::TimeOfDay;
    use chrono::TimeZone;

    fn fixed_time() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn test_location_state_creation() {
        let now = fixed_time();
        let atm = Atmosphere::new("The streets are alive with music and laughter...").unwrap();
        let state = LocationState::new(LocationId::new(), WorldId::new(), "Festival Day", now)
            .with_description("City-wide festival celebration")
            .with_backdrop(AssetPath::new("/assets/city_festival.png").unwrap())
            .with_atmosphere(atm)
            .with_priority(100);

        assert_eq!(state.name(), "Festival Day");
        assert!(state.backdrop_override().is_some());
        assert_eq!(state.priority(), 100);
        assert!(!state.is_default());
    }

    #[test]
    fn test_default_state() {
        let now = fixed_time();
        let state = LocationState::default_state(LocationId::new(), WorldId::new(), "Normal", now);

        assert!(state.is_default());
        assert_eq!(state.activation_rules().len(), 1);
        assert!(matches!(
            state.activation_rules()[0],
            ActivationRule::Always
        ));
    }

    #[test]
    fn test_soft_rules_detection() {
        let now = fixed_time();
        let state = LocationState::new(LocationId::new(), WorldId::new(), "Test", now);

        // No rules - no soft rules
        assert!(!state.has_soft_rules());

        // Add hard rule
        let state = state.with_rule(ActivationRule::TimeOfDay {
            period: TimeOfDay::Evening,
        });
        assert!(!state.has_soft_rules());

        // Add soft rule
        let state = state.with_rule(ActivationRule::Custom {
            description: "When the mood is tense".to_string(),
            llm_prompt: None,
        });
        assert!(state.has_soft_rules());
        assert_eq!(state.hard_rules().len(), 1);
        assert_eq!(state.soft_rules().len(), 1);
    }

    #[test]
    fn test_generation_metadata() {
        let now = fixed_time();
        let state = LocationState::new(LocationId::new(), WorldId::new(), "Test", now)
            .with_generation_prompt("A serene forest path".to_string())
            .with_workflow_id("backdrop_v2".to_string());

        assert_eq!(state.generation_prompt(), Some("A serene forest path"));
        assert_eq!(state.workflow_id(), Some("backdrop_v2"));

        let summary = state.summary();
        assert_eq!(
            summary.generation_prompt,
            Some("A serene forest path".to_string())
        );
        assert_eq!(summary.workflow_id, Some("backdrop_v2".to_string()));
    }

    #[test]
    fn test_new_with_id() {
        let now = fixed_time();
        let specific_id = LocationStateId::new();
        let state = LocationState::new_with_id(
            specific_id,
            LocationId::new(),
            WorldId::new(),
            "Deterministic State",
            now,
        );

        assert_eq!(state.id(), specific_id);
        assert_eq!(state.name(), "Deterministic State");
    }
}
