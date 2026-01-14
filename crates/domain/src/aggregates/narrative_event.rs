//! NarrativeEvent aggregate - DM-designed events with triggers and outcomes
//!
//! NarrativeEvents represent future events that can trigger when conditions are met.
//! They support complex triggers, branching outcomes, and chaining to other events.
//!
//! # Graph Relationships (stored as Neo4j edges, not embedded fields)
//!
//! - `TIED_TO_SCENE` -> Scene: Optional scene this event is tied to
//! - `TIED_TO_LOCATION` -> Location: Optional location this event is tied to
//! - `BELONGS_TO_ACT` -> Act: Optional act for Monomyth integration
//! - `FEATURES_NPC` -> Character: NPCs that should be featured in this event
//! - `CONTAINS_EVENT` <- EventChain: Chain membership (edge stored on EventChain side)
//!
//! Note: `trigger_conditions` and `outcomes` remain as JSON fields because they contain
//! complex nested structures with non-entity data (keywords, descriptions, effects).
//!
//! # Rustic DDD Design
//!
//! This aggregate follows Rustic DDD principles:
//! - **Private fields**: All fields are encapsulated
//! - **Valid by construction**: `new()` takes pre-validated types
//! - **Domain behavior**: `evaluate_triggers()`, `trigger()`, `reset()`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error as DeError;

use wrldbldr_domain::{NarrativeEventId, WorldId};

use crate::value_objects::NarrativeEventName;

// Re-export complex types from entities that are used within the aggregate
pub use crate::entities::{
    ChainedEvent, EventChainMembership, EventEffect, EventOutcome, FeaturedNpc, NarrativeTrigger,
    NarrativeTriggerType, OutcomeCondition, TriggerContext, TriggerEvaluation, TriggerLogic,
};

/// A narrative event that can be triggered when conditions are met
///
/// # Invariants
///
/// - `name` is always non-empty
/// - `trigger_count` is always >= 0
/// - If `is_triggered` is true, `triggered_at` is Some
///
/// # Graph Relationships
///
/// The following associations are stored as graph edges (not embedded fields):
/// - Scene association: Use `TIED_TO_SCENE` edge via repository methods
/// - Location association: Use `TIED_TO_LOCATION` edge via repository methods
/// - Act association: Use `BELONGS_TO_ACT` edge via repository methods
/// - Featured NPCs: Use `FEATURES_NPC` edges via repository methods
/// - Chain membership: Use `CONTAINS_EVENT` edge (from EventChain) via EventChainRepositoryPort
///
/// # Example
///
/// ```
/// use chrono::Utc;
/// use wrldbldr_domain::{NarrativeEventName, WorldId, NarrativeEventId};
/// use wrldbldr_domain::aggregates::narrative_event::NarrativeEvent;
///
/// let world_id = WorldId::new();
/// use chrono::TimeZone;
/// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
/// let event = NarrativeEvent::new(
///     world_id,
///     NarrativeEventName::new("The Betrayal").unwrap(),
///     now,
/// );
///
/// assert_eq!(event.name().as_str(), "The Betrayal");
/// assert!(event.is_active());
/// assert!(!event.is_triggered());
/// ```
#[derive(Debug, Clone)]
pub struct NarrativeEvent {
    // Identity
    id: NarrativeEventId,
    world_id: WorldId,

    // Basic Info
    /// Name of the event (for DM reference)
    name: NarrativeEventName,
    /// Detailed description of what this event represents
    description: String,
    /// Tags for organization and filtering
    tags: Vec<String>,

    // Trigger Configuration
    /// Conditions that must be met to trigger this event
    /// (Kept as JSON - contains complex nested structures with non-entity data)
    trigger_conditions: Vec<NarrativeTrigger>,
    /// How multiple conditions are evaluated
    trigger_logic: TriggerLogic,

    // Scene Direction
    /// Narrative text shown to DM when event triggers (sets the scene)
    scene_direction: String,
    /// Suggested opening dialogue or action
    suggested_opening: Option<String>,
    // NOTE: featured_npcs moved to FEATURES_NPC edges

    // Outcomes
    /// Possible outcomes with their effects and chains
    /// (Kept as JSON - contains complex nested structures with non-entity data)
    outcomes: Vec<EventOutcome>,
    /// Default outcome if DM doesn't select one
    default_outcome: Option<String>,

    // Status
    /// Whether this event is currently active (can be triggered)
    is_active: bool,
    /// Whether this event has already been triggered
    is_triggered: bool,
    /// Timestamp when triggered (if triggered)
    triggered_at: Option<DateTime<Utc>>,
    /// Which outcome was selected (if triggered)
    selected_outcome: Option<String>,
    /// Whether this event can repeat (trigger multiple times)
    is_repeatable: bool,
    /// Times this event has been triggered (for repeatable events)
    trigger_count: u32,

    // Timing
    /// Optional delay before event actually fires (in turns/exchanges)
    delay_turns: u32,
    /// Expiration - event becomes inactive after this (optional)
    expires_after_turns: Option<u32>,

    // NOTE: scene_id, location_id, act_id moved to graph edges

    // Organization
    /// Priority for ordering multiple triggered events (higher = first)
    priority: i32,
    /// Is this a favorite for quick access
    is_favorite: bool,

    // NOTE: chain_id, chain_position moved to CONTAINS_EVENT edge (from EventChain)

    // Metadata
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl NarrativeEvent {
    // =========================================================================
    // Constructor
    // =========================================================================

    /// Create a new narrative event with the given world and name.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::Utc;
    /// use wrldbldr_domain::WorldId;
    /// use wrldbldr_domain::aggregates::narrative_event::NarrativeEvent;
    ///
    /// let world_id = WorldId::new();
    /// use chrono::TimeZone;
    /// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    /// let event = NarrativeEvent::new(
    ///     world_id,
    ///     NarrativeEventName::new("Secret Meeting").unwrap(),
    ///     now,
    /// );
    ///
    /// assert_eq!(event.name().as_str(), "Secret Meeting");
    /// assert!(event.is_active());
    /// ```
    pub fn new(world_id: WorldId, name: NarrativeEventName, now: DateTime<Utc>) -> Self {
        Self {
            id: NarrativeEventId::new(),
            world_id,
            name,
            description: String::new(),
            tags: Vec::new(),
            trigger_conditions: Vec::new(),
            trigger_logic: TriggerLogic::All,
            scene_direction: String::new(),
            suggested_opening: None,
            outcomes: Vec::new(),
            default_outcome: None,
            is_active: true,
            is_triggered: false,
            triggered_at: None,
            selected_outcome: None,
            is_repeatable: false,
            trigger_count: 0,
            delay_turns: 0,
            expires_after_turns: None,
            priority: 0,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }

    // =========================================================================
    // Identity Accessors (read-only)
    // =========================================================================

    /// Returns the event's unique identifier.
    #[inline]
    pub fn id(&self) -> NarrativeEventId {
        self.id
    }

    /// Returns the ID of the world this event belongs to.
    #[inline]
    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    // =========================================================================
    // Basic Info Accessors
    // =========================================================================

    /// Returns the event's name.
    #[inline]
    pub fn name(&self) -> &NarrativeEventName {
        &self.name
    }

    /// Returns the event's description.
    #[inline]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the event's tags.
    #[inline]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    // =========================================================================
    // Trigger Configuration Accessors
    // =========================================================================

    /// Returns the event's trigger conditions.
    #[inline]
    pub fn trigger_conditions(&self) -> &[NarrativeTrigger] {
        &self.trigger_conditions
    }

    /// Returns the event's trigger logic.
    #[inline]
    pub fn trigger_logic(&self) -> TriggerLogic {
        self.trigger_logic
    }

    // =========================================================================
    // Scene Direction Accessors
    // =========================================================================

    /// Returns the event's scene direction.
    #[inline]
    pub fn scene_direction(&self) -> &str {
        &self.scene_direction
    }

    /// Returns the event's suggested opening.
    #[inline]
    pub fn suggested_opening(&self) -> Option<&str> {
        self.suggested_opening.as_deref()
    }

    // =========================================================================
    // Outcomes Accessors
    // =========================================================================

    /// Returns the event's outcomes.
    #[inline]
    pub fn outcomes(&self) -> &[EventOutcome] {
        &self.outcomes
    }

    /// Returns the event's default outcome name.
    #[inline]
    pub fn default_outcome(&self) -> Option<&str> {
        self.default_outcome.as_deref()
    }

    // =========================================================================
    // Status Accessors
    // =========================================================================

    /// Returns true if the event is currently active.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Returns true if the event has been triggered.
    #[inline]
    pub fn is_triggered(&self) -> bool {
        self.is_triggered
    }

    /// Returns when the event was triggered, if it was.
    #[inline]
    pub fn triggered_at(&self) -> Option<DateTime<Utc>> {
        self.triggered_at
    }

    /// Returns the selected outcome name, if triggered.
    #[inline]
    pub fn selected_outcome(&self) -> Option<&str> {
        self.selected_outcome.as_deref()
    }

    /// Returns true if the event is repeatable.
    #[inline]
    pub fn is_repeatable(&self) -> bool {
        self.is_repeatable
    }

    /// Returns the number of times this event has been triggered.
    #[inline]
    pub fn trigger_count(&self) -> u32 {
        self.trigger_count
    }

    // =========================================================================
    // Timing Accessors
    // =========================================================================

    /// Returns the delay in turns before the event fires.
    #[inline]
    pub fn delay_turns(&self) -> u32 {
        self.delay_turns
    }

    /// Returns the expiration in turns, if any.
    #[inline]
    pub fn expires_after_turns(&self) -> Option<u32> {
        self.expires_after_turns
    }

    // =========================================================================
    // Organization Accessors
    // =========================================================================

    /// Returns the event's priority.
    #[inline]
    pub fn priority(&self) -> i32 {
        self.priority
    }

    /// Returns true if the event is a favorite.
    #[inline]
    pub fn is_favorite(&self) -> bool {
        self.is_favorite
    }

    // =========================================================================
    // Timestamp Accessors
    // =========================================================================

    /// Returns when the event was created.
    #[inline]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns when the event was last updated.
    #[inline]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // =========================================================================
    // Builder Methods (for construction)
    // =========================================================================

    /// Set the event's ID (used when loading from storage).
    pub fn with_id(mut self, id: NarrativeEventId) -> Self {
        self.id = id;
        self
    }

    /// Set the event's description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the event's tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a tag to the event.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the event's trigger conditions.
    pub fn with_trigger_conditions(mut self, conditions: Vec<NarrativeTrigger>) -> Self {
        self.trigger_conditions = conditions;
        self
    }

    /// Add a trigger condition.
    pub fn with_trigger_condition(mut self, condition: NarrativeTrigger) -> Self {
        self.trigger_conditions.push(condition);
        self
    }

    /// Set the event's trigger logic.
    pub fn with_trigger_logic(mut self, logic: TriggerLogic) -> Self {
        self.trigger_logic = logic;
        self
    }

    /// Set the event's scene direction.
    pub fn with_scene_direction(mut self, direction: impl Into<String>) -> Self {
        self.scene_direction = direction.into();
        self
    }

    /// Set the event's suggested opening.
    pub fn with_suggested_opening(mut self, opening: impl Into<String>) -> Self {
        self.suggested_opening = Some(opening.into());
        self
    }

    /// Set the event's outcomes.
    pub fn with_outcomes(mut self, outcomes: Vec<EventOutcome>) -> Self {
        self.outcomes = outcomes;
        self
    }

    /// Add an outcome.
    pub fn with_outcome(mut self, outcome: EventOutcome) -> Self {
        self.outcomes.push(outcome);
        self
    }

    /// Set the event's default outcome.
    pub fn with_default_outcome(mut self, outcome_name: impl Into<String>) -> Self {
        self.default_outcome = Some(outcome_name.into());
        self
    }

    /// Set whether the event is active.
    pub fn with_active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Set whether the event is repeatable.
    pub fn with_repeatable(mut self, repeatable: bool) -> Self {
        self.is_repeatable = repeatable;
        self
    }

    /// Set the event's delay in turns.
    pub fn with_delay_turns(mut self, turns: u32) -> Self {
        self.delay_turns = turns;
        self
    }

    /// Set the event's expiration in turns.
    pub fn with_expires_after_turns(mut self, turns: u32) -> Self {
        self.expires_after_turns = Some(turns);
        self
    }

    /// Set the event's priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set whether the event is a favorite.
    pub fn with_favorite(mut self, favorite: bool) -> Self {
        self.is_favorite = favorite;
        self
    }

    /// Set the event's triggered state (used when loading from storage).
    pub fn with_triggered_state(
        mut self,
        is_triggered: bool,
        triggered_at: Option<DateTime<Utc>>,
        selected_outcome: Option<String>,
        trigger_count: u32,
    ) -> Self {
        self.is_triggered = is_triggered;
        self.triggered_at = triggered_at;
        self.selected_outcome = selected_outcome;
        self.trigger_count = trigger_count;
        self
    }

    /// Set the created_at timestamp (used when loading from storage).
    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    /// Set the updated_at timestamp (used when loading from storage).
    pub fn with_updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = updated_at;
        self
    }

    // =========================================================================
    // Mutation Methods
    // =========================================================================

    /// Set the event's name.
    pub fn set_name(&mut self, name: NarrativeEventName, now: DateTime<Utc>) {
        self.name = name;
        self.updated_at = now;
    }

    /// Set the event's description.
    pub fn set_description(&mut self, description: impl Into<String>, now: DateTime<Utc>) {
        self.description = description.into();
        self.updated_at = now;
    }

    /// Set the event's scene direction.
    pub fn set_scene_direction(&mut self, direction: impl Into<String>, now: DateTime<Utc>) {
        self.scene_direction = direction.into();
        self.updated_at = now;
    }

    /// Set the event's trigger conditions.
    pub fn set_trigger_conditions(
        &mut self,
        conditions: Vec<NarrativeTrigger>,
        now: DateTime<Utc>,
    ) {
        self.trigger_conditions = conditions;
        self.updated_at = now;
    }

    /// Set the event's outcomes.
    pub fn set_outcomes(&mut self, outcomes: Vec<EventOutcome>, now: DateTime<Utc>) {
        self.outcomes = outcomes;
        self.updated_at = now;
    }

    /// Set the event's active state.
    pub fn set_active(&mut self, active: bool, now: DateTime<Utc>) {
        self.is_active = active;
        self.updated_at = now;
    }

    /// Set the event's priority.
    pub fn set_priority(&mut self, priority: i32, now: DateTime<Utc>) {
        self.priority = priority;
        self.updated_at = now;
    }

    /// Set the event's favorite state.
    pub fn set_favorite(&mut self, favorite: bool, now: DateTime<Utc>) {
        self.is_favorite = favorite;
        self.updated_at = now;
    }

    // =========================================================================
    // Domain Methods - Trigger Evaluation
    // =========================================================================

    /// Check if this event's triggers match the current game context.
    pub fn evaluate_triggers(&self, context: &TriggerContext) -> TriggerEvaluation {
        let mut matched = Vec::new();
        let mut unmatched = Vec::new();

        for trigger in &self.trigger_conditions {
            if self.trigger_matches(&trigger.trigger_type, context) {
                matched.push(trigger.trigger_id.clone());
            } else {
                unmatched.push(trigger.trigger_id.clone());
            }
        }

        let total = self.trigger_conditions.len();
        let matched_count = matched.len();

        let is_triggered = if total == 0 {
            false // No triggers means can't be automatically triggered
        } else {
            match self.trigger_logic {
                TriggerLogic::All => matched_count == total,
                TriggerLogic::Any => matched_count > 0,
                TriggerLogic::AtLeast(n) => matched_count >= n as usize,
            }
        };

        // Check required triggers
        let required_met = self
            .trigger_conditions
            .iter()
            .filter(|t| t.is_required)
            .all(|t| matched.contains(&t.trigger_id));

        TriggerEvaluation {
            is_triggered: is_triggered && required_met,
            matched_triggers: matched,
            unmatched_triggers: unmatched,
            total_triggers: total,
            confidence: if total > 0 {
                matched_count as f32 / total as f32
            } else {
                0.0
            },
        }
    }

    fn trigger_matches(&self, trigger: &NarrativeTriggerType, context: &TriggerContext) -> bool {
        match trigger {
            NarrativeTriggerType::FlagSet { flag_name } => {
                context.flags.get(flag_name).copied().unwrap_or(false)
            }
            NarrativeTriggerType::FlagNotSet { flag_name } => {
                !context.flags.get(flag_name).copied().unwrap_or(false)
            }
            NarrativeTriggerType::PlayerEntersLocation { location_id, .. } => {
                context.current_location.as_ref() == Some(location_id)
            }
            NarrativeTriggerType::HasItem {
                item_name,
                quantity,
            } => {
                let count = context.inventory.iter().filter(|i| i == &item_name).count() as u32;
                count >= quantity.unwrap_or(1)
            }
            NarrativeTriggerType::MissingItem { item_name } => {
                !context.inventory.contains(item_name)
            }
            NarrativeTriggerType::EventCompleted {
                event_id,
                outcome_name,
                ..
            } => {
                if context.completed_events.contains(event_id) {
                    if let Some(required_outcome) = outcome_name {
                        context
                            .event_outcomes
                            .get(event_id)
                            .map(|o| o == required_outcome)
                            .unwrap_or(false)
                    } else {
                        true
                    }
                } else {
                    false
                }
            }
            NarrativeTriggerType::TurnCount { turns, since_event } => {
                if let Some(event_id) = since_event {
                    context
                        .turns_since_event
                        .get(event_id)
                        .map(|t| *t >= *turns)
                        .unwrap_or(false)
                } else {
                    context.turn_count >= *turns
                }
            }
            NarrativeTriggerType::ChallengeCompleted {
                challenge_id,
                requires_success,
                ..
            } => {
                if context.completed_challenges.contains(challenge_id) {
                    if let Some(need_success) = requires_success {
                        context
                            .challenge_successes
                            .get(challenge_id)
                            .map(|s| *s == *need_success)
                            .unwrap_or(false)
                    } else {
                        true
                    }
                } else {
                    false
                }
            }
            NarrativeTriggerType::DialogueTopic { keywords, .. } => keywords
                .iter()
                .any(|k| context.recent_dialogue_topics.contains(k)),
            NarrativeTriggerType::TimeAtLocation {
                location_id,
                time_context: required_time,
                ..
            } => {
                let at_location = context.current_location.as_ref() == Some(location_id);
                let time_matches = context
                    .time_context
                    .as_ref()
                    .map(|current_time| {
                        current_time.trim().to_lowercase() == required_time.trim().to_lowercase()
                    })
                    .unwrap_or(false);
                at_location && time_matches
            }
            NarrativeTriggerType::NpcAction { action_keywords, .. } => {
                context
                    .recent_player_action
                    .as_ref()
                    .map(|action| {
                        action_keywords
                            .iter()
                            .any(|kw| action.to_lowercase().contains(&kw.to_lowercase()))
                    })
                    .unwrap_or(false)
            }
            NarrativeTriggerType::RelationshipThreshold {
                character_id,
                with_character,
                min_sentiment,
                max_sentiment,
                ..
            } => {
                context
                    .get_relationship(*character_id, *with_character)
                    .map(|sentiment| {
                        let meets_min = min_sentiment.is_none_or(|min| sentiment >= min);
                        let meets_max = max_sentiment.is_none_or(|max| sentiment <= max);
                        meets_min && meets_max
                    })
                    .unwrap_or(false)
            }
            NarrativeTriggerType::StatThreshold {
                character_id,
                stat_name,
                min_value,
                max_value,
            } => {
                context
                    .get_character_stat(*character_id, stat_name)
                    .map(|stat_value| {
                        let meets_min = min_value.is_none_or(|min| stat_value >= min);
                        let meets_max = max_value.is_none_or(|max| stat_value <= max);
                        meets_min && meets_max
                    })
                    .unwrap_or(false)
            }
            NarrativeTriggerType::CombatResult { .. } => {
                // KNOWN LIMITATION: CombatResult trigger is not yet implemented
                false
            }
            NarrativeTriggerType::Custom {
                description,
                llm_evaluation,
            } => {
                if *llm_evaluation {
                    context
                        .custom_trigger_results
                        .get(description)
                        .copied()
                        .unwrap_or(false)
                } else {
                    false
                }
            }

            // === Compendium-based triggers ===

            NarrativeTriggerType::KnowsSpell { spell_id, .. } => {
                context
                    .known_spells
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(spell_id))
            }

            NarrativeTriggerType::HasFeat { feat_id, .. } => {
                context
                    .character_feats
                    .iter()
                    .any(|f| f.eq_ignore_ascii_case(feat_id))
            }

            NarrativeTriggerType::HasClass {
                class_id,
                min_level,
                ..
            } => {
                context
                    .class_levels
                    .iter()
                    .find(|(id, _)| id.eq_ignore_ascii_case(class_id))
                    .map(|(_, level)| min_level.is_none_or(|min| *level >= min))
                    .unwrap_or(false)
            }

            NarrativeTriggerType::HasOrigin { origin_id, .. } => {
                context
                    .origin_id
                    .as_ref()
                    .map(|o| o.eq_ignore_ascii_case(origin_id))
                    .unwrap_or(false)
            }

            NarrativeTriggerType::KnowsCreature { creature_id, .. } => {
                context
                    .known_creatures
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(creature_id))
            }
        }
    }

    // =========================================================================
    // Domain Methods - Triggering
    // =========================================================================

    /// Mark this event as triggered with the given outcome.
    pub fn trigger(&mut self, outcome_name: Option<String>, now: DateTime<Utc>) {
        self.is_triggered = true;
        self.triggered_at = Some(now);
        self.selected_outcome = outcome_name;
        self.trigger_count += 1;
        self.updated_at = now;

        // If not repeatable, deactivate
        if !self.is_repeatable {
            self.is_active = false;
        }
    }

    /// Reset the triggered state (for repeatable events).
    pub fn reset(&mut self, now: DateTime<Utc>) {
        self.is_triggered = false;
        self.triggered_at = None;
        self.selected_outcome = None;
        self.updated_at = now;
    }

    // =========================================================================
    // Domain Methods - Outcome Access
    // =========================================================================

    /// Get the outcome by name.
    pub fn get_outcome(&self, name: &str) -> Option<&EventOutcome> {
        self.outcomes.iter().find(|o| o.name == name)
    }

    /// Get the default outcome.
    pub fn get_default_outcome(&self) -> Option<&EventOutcome> {
        self.default_outcome
            .as_ref()
            .and_then(|name| self.get_outcome(name))
    }
}

// ============================================================================
// Serde Implementation
// ============================================================================

/// Intermediate format for serialization that matches the wire format
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NarrativeEventWireFormat {
    id: NarrativeEventId,
    world_id: WorldId,
    name: String,
    description: String,
    tags: Vec<String>,
    trigger_conditions: Vec<NarrativeTrigger>,
    trigger_logic: TriggerLogic,
    scene_direction: String,
    suggested_opening: Option<String>,
    outcomes: Vec<EventOutcome>,
    default_outcome: Option<String>,
    is_active: bool,
    is_triggered: bool,
    triggered_at: Option<DateTime<Utc>>,
    selected_outcome: Option<String>,
    is_repeatable: bool,
    trigger_count: u32,
    delay_turns: u32,
    expires_after_turns: Option<u32>,
    priority: i32,
    is_favorite: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Serialize for NarrativeEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire = NarrativeEventWireFormat {
            id: self.id,
            world_id: self.world_id,
            name: self.name.to_string(),
            description: self.description.clone(),
            tags: self.tags.clone(),
            trigger_conditions: self.trigger_conditions.clone(),
            trigger_logic: self.trigger_logic,
            scene_direction: self.scene_direction.clone(),
            suggested_opening: self.suggested_opening.clone(),
            outcomes: self.outcomes.clone(),
            default_outcome: self.default_outcome.clone(),
            is_active: self.is_active,
            is_triggered: self.is_triggered,
            triggered_at: self.triggered_at,
            selected_outcome: self.selected_outcome.clone(),
            is_repeatable: self.is_repeatable,
            trigger_count: self.trigger_count,
            delay_turns: self.delay_turns,
            expires_after_turns: self.expires_after_turns,
            priority: self.priority,
            is_favorite: self.is_favorite,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NarrativeEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = NarrativeEventWireFormat::deserialize(deserializer)?;

        let name = NarrativeEventName::new(wire.name).map_err(DeError::custom)?;

        Ok(NarrativeEvent {
            id: wire.id,
            world_id: wire.world_id,
            name,
            description: wire.description,
            tags: wire.tags,
            trigger_conditions: wire.trigger_conditions,
            trigger_logic: wire.trigger_logic,
            scene_direction: wire.scene_direction,
            suggested_opening: wire.suggested_opening,
            outcomes: wire.outcomes,
            default_outcome: wire.default_outcome,
            is_active: wire.is_active,
            is_triggered: wire.is_triggered,
            triggered_at: wire.triggered_at,
            selected_outcome: wire.selected_outcome,
            is_repeatable: wire.is_repeatable,
            trigger_count: wire.trigger_count,
            delay_turns: wire.delay_turns,
            expires_after_turns: wire.expires_after_turns,
            priority: wire.priority,
            is_favorite: wire.is_favorite,
            created_at: wire.created_at,
            updated_at: wire.updated_at,
        })
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

    fn create_test_event() -> NarrativeEvent {
        let world_id = WorldId::new();
        let now = fixed_time();
        NarrativeEvent::new(
            world_id,
            NarrativeEventName::new("Test Event").unwrap(),
            now,
        )
    }

    mod constructor {
        use super::*;

        #[test]
        fn new_creates_event_with_correct_defaults() {
            let world_id = WorldId::new();
            let now = fixed_time();
            let event = NarrativeEvent::new(
                world_id,
                NarrativeEventName::new("The Revelation").unwrap(),
                now,
            );

            assert_eq!(event.name().as_str(), "The Revelation");
            assert_eq!(event.world_id(), world_id);
            assert!(event.description().is_empty());
            assert!(event.tags().is_empty());
            assert!(event.trigger_conditions().is_empty());
            assert!(matches!(event.trigger_logic(), TriggerLogic::All));
            assert!(event.scene_direction().is_empty());
            assert!(event.suggested_opening().is_none());
            assert!(event.outcomes().is_empty());
            assert!(event.default_outcome().is_none());
            assert!(event.is_active());
            assert!(!event.is_triggered());
            assert!(event.triggered_at().is_none());
            assert!(event.selected_outcome().is_none());
            assert!(!event.is_repeatable());
            assert_eq!(event.trigger_count(), 0);
            assert_eq!(event.delay_turns(), 0);
            assert!(event.expires_after_turns().is_none());
            assert_eq!(event.priority(), 0);
            assert!(!event.is_favorite());
        }

        #[test]
        fn builder_methods_work() {
            let world_id = WorldId::new();
            let now = fixed_time();

            let event = NarrativeEvent::new(
                world_id,
                NarrativeEventName::new("Epic Event").unwrap(),
                now,
            )
                .with_description("A dramatic event")
                .with_tag("drama")
                .with_tag("important")
                .with_scene_direction("Build tension slowly")
                .with_suggested_opening("The air grows thick...")
                .with_repeatable(true)
                .with_priority(10)
                .with_favorite(true);

            assert_eq!(event.name().as_str(), "Epic Event");
            assert_eq!(event.description(), "A dramatic event");
            assert_eq!(event.tags(), &["drama", "important"]);
            assert_eq!(event.scene_direction(), "Build tension slowly");
            assert_eq!(event.suggested_opening(), Some("The air grows thick..."));
            assert!(event.is_repeatable());
            assert_eq!(event.priority(), 10);
            assert!(event.is_favorite());
        }
    }

    mod triggering {
        use super::*;

        #[test]
        fn trigger_sets_state_correctly() {
            let mut event = create_test_event();
            let now = fixed_time();

            event.trigger(Some("success".to_string()), now);

            assert!(event.is_triggered());
            assert!(event.triggered_at().is_some());
            assert_eq!(event.selected_outcome(), Some("success"));
            assert_eq!(event.trigger_count(), 1);
            assert!(!event.is_active()); // Non-repeatable events become inactive
        }

        #[test]
        fn trigger_repeatable_stays_active() {
            let mut event = create_test_event();
            event = event.with_repeatable(true);
            let now = fixed_time();

            event.trigger(Some("success".to_string()), now);

            assert!(event.is_triggered());
            assert!(event.is_active()); // Repeatable events stay active
        }

        #[test]
        fn reset_clears_triggered_state() {
            let mut event = create_test_event();
            event = event.with_repeatable(true);
            let now = fixed_time();

            event.trigger(Some("success".to_string()), now);
            event.reset(now);

            assert!(!event.is_triggered());
            assert!(event.triggered_at().is_none());
            assert!(event.selected_outcome().is_none());
            assert_eq!(event.trigger_count(), 1); // Count is preserved
        }

        #[test]
        fn multiple_triggers_increment_count() {
            let mut event = create_test_event();
            event = event.with_repeatable(true);
            let now = fixed_time();

            event.trigger(None, now);
            event.reset(now);
            event.trigger(None, now);
            event.reset(now);
            event.trigger(None, now);

            assert_eq!(event.trigger_count(), 3);
        }
    }

    mod mutation {
        use super::*;

        #[test]
        fn set_name_works() {
            let mut event = create_test_event();
            event.set_name(NarrativeEventName::new("New Name").unwrap(), fixed_time());
            assert_eq!(event.name().as_str(), "New Name");
        }

        #[test]
        fn set_description_works() {
            let mut event = create_test_event();
            event.set_description("New description");
            assert_eq!(event.description(), "New description");
        }

        #[test]
        fn set_active_works() {
            let mut event = create_test_event();
            event.set_active(false);
            assert!(!event.is_active());

            event.set_active(true);
            assert!(event.is_active());
        }

        #[test]
        fn set_priority_works() {
            let mut event = create_test_event();
            event.set_priority(5);
            assert_eq!(event.priority(), 5);
        }

        #[test]
        fn set_favorite_works() {
            let mut event = create_test_event();
            event.set_favorite(true);
            assert!(event.is_favorite());

            event.set_favorite(false);
            assert!(!event.is_favorite());
        }
    }

    mod trigger_evaluation {
        use super::*;

        #[test]
        fn no_triggers_returns_not_triggered() {
            let event = create_test_event();
            let context = TriggerContext::new();

            let eval = event.evaluate_triggers(&context);
            assert!(!eval.is_triggered);
            assert_eq!(eval.total_triggers, 0);
        }

        #[test]
        fn flag_set_trigger_works() {
            let world_id = WorldId::new();
            let now = fixed_time();

            let trigger = NarrativeTrigger {
                trigger_type: NarrativeTriggerType::FlagSet {
                    flag_name: "quest_started".to_string(),
                },
                description: "Quest must be started".to_string(),
                is_required: true,
                trigger_id: "flag-1".to_string(),
            };

            let event = NarrativeEvent::new(
                world_id,
                NarrativeEventName::new("Test").unwrap(),
                now,
            )
                .with_trigger_condition(trigger);

            // Without flag set
            let context = TriggerContext::new();
            let eval = event.evaluate_triggers(&context);
            assert!(!eval.is_triggered);

            // With flag set
            let mut context = TriggerContext::new();
            context.flags.insert("quest_started".to_string(), true);
            let eval = event.evaluate_triggers(&context);
            assert!(eval.is_triggered);
        }
    }

    mod serde {
        use super::*;

        #[test]
        fn serialize_deserialize_roundtrip() {
            let world_id = WorldId::new();
            let now = fixed_time();

            let event = NarrativeEvent::new(
                world_id,
                NarrativeEventName::new("Test Event").unwrap(),
                now,
            )
                .with_description("A test event")
                .with_tags(vec!["test".to_string(), "important".to_string()])
                .with_priority(5);

            let json = serde_json::to_string(&event).unwrap();
            let deserialized: NarrativeEvent = serde_json::from_str(&json).unwrap();

            assert_eq!(deserialized.id(), event.id());
            assert_eq!(deserialized.world_id(), world_id);
            assert_eq!(deserialized.name(), "Test Event");
            assert_eq!(deserialized.description(), "A test event");
            assert_eq!(deserialized.tags(), &["test", "important"]);
            assert_eq!(deserialized.priority(), 5);
        }

        #[test]
        fn serialize_produces_camel_case() {
            let event = create_test_event();
            let json = serde_json::to_string(&event).unwrap();

            assert!(json.contains("worldId"));
            assert!(json.contains("triggerConditions"));
            assert!(json.contains("triggerLogic"));
            assert!(json.contains("sceneDirection"));
            assert!(json.contains("suggestedOpening"));
            assert!(json.contains("defaultOutcome"));
            assert!(json.contains("isActive"));
            assert!(json.contains("isTriggered"));
            assert!(json.contains("triggeredAt"));
            assert!(json.contains("selectedOutcome"));
            assert!(json.contains("isRepeatable"));
            assert!(json.contains("triggerCount"));
            assert!(json.contains("delayTurns"));
            assert!(json.contains("expiresAfterTurns"));
            assert!(json.contains("isFavorite"));
            assert!(json.contains("createdAt"));
            assert!(json.contains("updatedAt"));
        }
    }
}
