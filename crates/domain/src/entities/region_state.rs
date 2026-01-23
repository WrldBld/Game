//! RegionState entity - Visual configurations for regions
//!
//! RegionStates represent region-level visual configurations that layer
//! on top of LocationStates. Examples: tavern at morning vs evening,
//! room before/after an explosion.
//!
//! # Neo4j Relationships
//! - `(Region)-[:HAS_STATE]->(RegionState)` - Region has this state option
//! - `(Region)-[:ACTIVE_STATE]->(RegionState)` - Currently active (set by staging)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{LocationId, RegionId, RegionStateId, WorldId};
use crate::value_objects::{
    ActivationLogic, ActivationRule, AssetPath, Atmosphere, Description, StateName,
};

/// A visual configuration for a region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionState {
    id: RegionStateId,
    region_id: RegionId,
    /// Denormalized for efficient queries
    location_id: LocationId,
    world_id: WorldId,

    /// Name of this state (e.g., "Tavern Morning", "Post-Explosion")
    name: StateName,
    /// Description for DM reference
    description: Description,

    // Visual Configuration
    /// Override the region's default backdrop
    backdrop_override: Option<AssetPath>,
    /// Override the region's atmosphere text
    atmosphere_override: Option<Atmosphere>,
    /// Ambient sound asset path
    ambient_sound: Option<AssetPath>,

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

impl RegionState {
    pub fn new(
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        name: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: RegionStateId::new(),
            region_id,
            location_id,
            world_id,
            name: StateName::new(name).unwrap_or_default(),
            description: Description::default(),
            backdrop_override: None,
            atmosphere_override: None,
            ambient_sound: None,
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
        region_id: RegionId,
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
            ..Self::new(region_id, location_id, world_id, name, now)
        }
    }

    /// Reconstruct from stored data
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: RegionStateId,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        name: StateName,
        description: Description,
        backdrop_override: Option<AssetPath>,
        atmosphere_override: Option<Atmosphere>,
        ambient_sound: Option<AssetPath>,
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
            region_id,
            location_id,
            world_id,
            name,
            description,
            backdrop_override,
            atmosphere_override,
            ambient_sound,
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

    pub fn id(&self) -> RegionStateId {
        self.id
    }

    pub fn region_id(&self) -> RegionId {
        self.region_id
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
        id: RegionStateId,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        name: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            region_id,
            location_id,
            world_id,
            name: StateName::new(name).unwrap_or_default(),
            description: Description::default(),
            backdrop_override: None,
            atmosphere_override: None,
            ambient_sound: None,
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
    pub fn summary(&self) -> RegionStateSummary {
        RegionStateSummary {
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

/// Summary of a region state for display/wire transfer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionStateSummary {
    pub id: RegionStateId,
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
    use crate::value_objects::AssetPath;
    use chrono::TimeZone;

    fn fixed_time() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn test_region_state_creation() {
        let now = fixed_time();
        let atm = Atmosphere::new("Warm candlelight flickers across polished brass...").unwrap();
        let state = RegionState::new(
            RegionId::new(),
            LocationId::new(),
            WorldId::new(),
            "Evening",
            now,
        )
        .with_description("Warm evening atmosphere")
        .with_backdrop(AssetPath::new("/assets/tavern_evening.png").unwrap())
        .with_atmosphere(atm)
        .with_rule(ActivationRule::TimeOfDay {
            period: TimeOfDay::Evening,
        })
        .with_priority(10);

        assert_eq!(state.name(), "Evening");
        assert!(state.backdrop_override().is_some());
        assert_eq!(state.priority(), 10);
        assert!(!state.is_default());
        assert_eq!(state.activation_rules().len(), 1);
    }

    #[test]
    fn test_default_state() {
        let now = fixed_time();
        let state = RegionState::default_state(
            RegionId::new(),
            LocationId::new(),
            WorldId::new(),
            "Default",
            now,
        );

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
        let state = RegionState::new(
            RegionId::new(),
            LocationId::new(),
            WorldId::new(),
            "Test",
            now,
        );

        // No rules - no soft rules
        assert!(!state.has_soft_rules());

        // Add hard rule
        let state = state.with_rule(ActivationRule::TimeOfDay {
            period: TimeOfDay::Morning,
        });
        assert!(!state.has_soft_rules());

        // Add soft rule
        let state = state.with_rule(ActivationRule::Custom {
            description: "When the party is celebrating".to_string(),
            llm_prompt: None,
        });
        assert!(state.has_soft_rules());
        assert_eq!(state.hard_rules().len(), 1);
        assert_eq!(state.soft_rules().len(), 1);
    }

    #[test]
    fn test_summary() {
        let now = fixed_time();
        let state = RegionState::new(
            RegionId::new(),
            LocationId::new(),
            WorldId::new(),
            "Morning",
            now,
        )
        .with_backdrop(AssetPath::new("/assets/tavern_morning.png").unwrap())
        .with_priority(10);

        let summary = state.summary();
        assert_eq!(summary.name, "Morning");
        assert_eq!(
            summary.backdrop_override.as_deref(),
            Some("/assets/tavern_morning.png")
        );
        assert_eq!(summary.priority, 10);
    }

    #[test]
    fn test_generation_metadata() {
        let now = fixed_time();
        let state = RegionState::new(
            RegionId::new(),
            LocationId::new(),
            WorldId::new(),
            "Evening",
            now,
        )
        .with_generation_prompt("Warm tavern with flickering candles".to_string())
        .with_workflow_id("backdrop_v2".to_string());

        assert_eq!(
            state.generation_prompt(),
            Some("Warm tavern with flickering candles")
        );
        assert_eq!(state.workflow_id(), Some("backdrop_v2"));

        let summary = state.summary();
        assert_eq!(
            summary.generation_prompt,
            Some("Warm tavern with flickering candles".to_string())
        );
        assert_eq!(summary.workflow_id, Some("backdrop_v2".to_string()));
    }

    #[test]
    fn test_new_with_id() {
        let now = fixed_time();
        let specific_id = RegionStateId::new();
        let state = RegionState::new_with_id(
            specific_id,
            RegionId::new(),
            LocationId::new(),
            WorldId::new(),
            "Deterministic State",
            now,
        );

        assert_eq!(state.id(), specific_id);
        assert_eq!(state.name(), "Deterministic State");
    }
}
