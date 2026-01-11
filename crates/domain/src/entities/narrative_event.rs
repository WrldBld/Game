//! NarrativeEvent entity - DM-designed events with triggers and outcomes
//!
//! NarrativeEvents represent future events that can trigger when conditions are met.
//! They support complex triggers, branching outcomes, and chaining to other events.
//!
//! # Graph Relationships (stored as Neo4j edges, not embedded fields)
//!
//! - `TIED_TO_SCENE` → Scene: Optional scene this event is tied to
//! - `TIED_TO_LOCATION` → Location: Optional location this event is tied to
//! - `BELONGS_TO_ACT` → Act: Optional act for Monomyth integration
//! - `FEATURES_NPC` → Character: NPCs that should be featured in this event
//! - `CONTAINS_EVENT` ← EventChain: Chain membership (edge stored on EventChain side)
//!
//! Note: `trigger_conditions` and `outcomes` remain as JSON fields because they contain
//! complex nested structures with non-entity data (keywords, descriptions, effects).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use wrldbldr_domain::{
    ChallengeId, CharacterId, EventChainId, LocationId, NarrativeEventId, SceneId, WorldId,
};

/// A narrative event that can be triggered when conditions are met
///
/// # Graph Relationships
///
/// The following associations are stored as graph edges (not embedded fields):
/// - Scene association: Use `TIED_TO_SCENE` edge via repository methods
/// - Location association: Use `TIED_TO_LOCATION` edge via repository methods
/// - Act association: Use `BELONGS_TO_ACT` edge via repository methods
/// - Featured NPCs: Use `FEATURES_NPC` edges via repository methods
/// - Chain membership: Use `CONTAINS_EVENT` edge (from EventChain) via EventChainRepositoryPort
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeEvent {
    pub id: NarrativeEventId,
    pub world_id: WorldId,

    // Basic Info
    /// Name of the event (for DM reference)
    pub name: String,
    /// Detailed description of what this event represents
    pub description: String,
    /// Tags for organization and filtering
    pub tags: Vec<String>,

    // Trigger Configuration
    /// Conditions that must be met to trigger this event
    /// (Kept as JSON - contains complex nested structures with non-entity data)
    pub trigger_conditions: Vec<NarrativeTrigger>,
    /// How multiple conditions are evaluated
    pub trigger_logic: TriggerLogic,

    // Scene Direction
    /// Narrative text shown to DM when event triggers (sets the scene)
    pub scene_direction: String,
    /// Suggested opening dialogue or action
    pub suggested_opening: Option<String>,
    // NOTE: featured_npcs moved to FEATURES_NPC edges

    // Outcomes
    /// Possible outcomes with their effects and chains
    /// (Kept as JSON - contains complex nested structures with non-entity data)
    pub outcomes: Vec<EventOutcome>,
    /// Default outcome if DM doesn't select one
    pub default_outcome: Option<String>,

    // Status
    /// Whether this event is currently active (can be triggered)
    pub is_active: bool,
    /// Whether this event has already been triggered
    pub is_triggered: bool,
    /// Timestamp when triggered (if triggered)
    pub triggered_at: Option<DateTime<Utc>>,
    /// Which outcome was selected (if triggered)
    pub selected_outcome: Option<String>,
    /// Whether this event can repeat (trigger multiple times)
    pub is_repeatable: bool,
    /// Times this event has been triggered (for repeatable events)
    pub trigger_count: u32,

    // Timing
    /// Optional delay before event actually fires (in turns/exchanges)
    pub delay_turns: u32,
    /// Expiration - event becomes inactive after this (optional)
    pub expires_after_turns: Option<u32>,

    // NOTE: scene_id, location_id, act_id moved to graph edges

    // Organization
    /// Priority for ordering multiple triggered events (higher = first)
    pub priority: i32,
    /// Is this a favorite for quick access
    pub is_favorite: bool,

    // NOTE: chain_id, chain_position moved to CONTAINS_EVENT edge (from EventChain)

    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// How multiple trigger conditions are evaluated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerLogic {
    /// All conditions must be met (AND)
    #[default]
    All,
    /// Any single condition can trigger (OR)
    Any,
    /// At least N conditions must be met
    AtLeast(u32),
}

/// A single trigger condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeTrigger {
    /// The type and parameters of this trigger
    pub trigger_type: NarrativeTriggerType,
    /// Human-readable description for DM
    pub description: String,
    /// Whether this specific condition must be met (for AtLeast logic)
    pub is_required: bool,
    /// Unique identifier for this trigger within the event
    pub trigger_id: String,
}

/// Types of triggers for narrative events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NarrativeTriggerType {
    /// Player action involving an NPC matches specified keywords.
    ///
    /// Despite the name, this trigger fires when the PLAYER's recent action
    /// contains any of the `action_keywords`. The `npc_id` and `npc_name` fields
    /// are metadata for DM clarity (indicating which NPC the action should involve)
    /// but are NOT used in trigger evaluation.
    ///
    /// Example: If action_keywords = ["ask", "quest"], and the player's action is
    /// "I ask Marcus about the missing artifact", this trigger would fire.
    ///
    /// Note: True NPC-initiated action tracking would require additional infrastructure.
    NpcAction {
        /// The NPC this action should involve (metadata only, not evaluated)
        npc_id: CharacterId,
        /// Display name for DM reference (metadata only, not evaluated)
        npc_name: String,
        /// Keywords to match against player's recent action (case-insensitive)
        action_keywords: Vec<String>,
        /// DM description of what action triggers this (metadata only)
        action_description: String,
    },

    /// Player enters a specific location
    PlayerEntersLocation {
        location_id: LocationId,
        location_name: String,
    },

    /// Player is at location during specific time
    TimeAtLocation {
        location_id: LocationId,
        location_name: String,
        time_context: String,
    },

    /// Specific dialogue topic is discussed
    DialogueTopic {
        keywords: Vec<String>,
        with_npc: Option<CharacterId>,
        npc_name: Option<String>,
    },

    /// Challenge is completed
    ChallengeCompleted {
        challenge_id: ChallengeId,
        challenge_name: String,
        requires_success: Option<bool>,
    },

    /// Relationship reaches a threshold
    RelationshipThreshold {
        character_id: CharacterId,
        character_name: String,
        with_character: CharacterId,
        with_character_name: String,
        min_sentiment: Option<f32>,
        max_sentiment: Option<f32>,
    },

    /// Player has specific item
    HasItem {
        item_name: String,
        quantity: Option<u32>,
    },

    /// Player does NOT have specific item
    MissingItem { item_name: String },

    /// Another narrative event was completed
    EventCompleted {
        event_id: NarrativeEventId,
        event_name: String,
        outcome_name: Option<String>,
    },

    /// Turn count reached (since session start or since another event)
    TurnCount {
        turns: u32,
        since_event: Option<NarrativeEventId>,
    },

    /// Game flag is set to true
    FlagSet { flag_name: String },

    /// Game flag is not set (or false)
    FlagNotSet { flag_name: String },

    /// Character stat meets threshold
    StatThreshold {
        character_id: CharacterId,
        stat_name: String,
        min_value: Option<i32>,
        max_value: Option<i32>,
    },

    /// Combat ended with specific result
    CombatResult {
        victory: Option<bool>,
        involved_npc: Option<CharacterId>,
    },

    /// Custom condition (LLM evaluates based on description)
    Custom {
        description: String,
        /// If true, LLM will evaluate this condition against current context
        llm_evaluation: bool,
    },
}

/// An outcome branch for a narrative event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventOutcome {
    /// Unique identifier for this outcome within the event
    pub name: String,
    /// Display label for DM
    pub label: String,
    /// Description of what happens in this outcome
    pub description: String,
    /// Conditions for this outcome (how does player reach this?)
    pub condition: Option<OutcomeCondition>,
    /// Effects that occur when this outcome happens
    pub effects: Vec<EventEffect>,
    /// Narrative events to chain to after this outcome
    pub chain_events: Vec<ChainedEvent>,
    /// Narrative summary to add to timeline
    pub timeline_summary: Option<String>,
}

/// Condition for an outcome branch
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutcomeCondition {
    /// DM selects this outcome manually
    DmChoice,

    /// Challenge result determines outcome
    ChallengeResult {
        challenge_id: Option<ChallengeId>,
        success_required: bool,
    },

    /// Combat result determines outcome
    CombatResult { victory_required: bool },

    /// Specific dialogue choice made
    DialogueChoice { keywords: Vec<String> },

    /// Player takes specific action
    PlayerAction { action_keywords: Vec<String> },

    /// Player has item
    HasItem { item_name: String },

    /// Custom condition (LLM evaluates)
    Custom { description: String },
}

/// Effects that occur as part of an event outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventEffect {
    /// Change relationship between characters
    ModifyRelationship {
        from_character: CharacterId,
        from_name: String,
        to_character: CharacterId,
        to_name: String,
        sentiment_change: f32,
        reason: String,
    },

    /// Give item to player
    GiveItem {
        item_name: String,
        item_description: Option<String>,
        quantity: u32,
    },

    /// Take item from player
    TakeItem { item_name: String, quantity: u32 },

    /// Reveal information to players
    RevealInformation {
        info_type: String,
        title: String,
        content: String,
        persist_to_journal: bool,
    },

    /// Set a game flag
    SetFlag { flag_name: String, value: bool },

    /// Enable a challenge
    EnableChallenge {
        challenge_id: ChallengeId,
        challenge_name: String,
    },

    /// Disable a challenge
    DisableChallenge {
        challenge_id: ChallengeId,
        challenge_name: String,
    },

    /// Enable another narrative event
    EnableEvent {
        event_id: NarrativeEventId,
        event_name: String,
    },

    /// Disable another narrative event
    DisableEvent {
        event_id: NarrativeEventId,
        event_name: String,
    },

    /// Trigger scene transition
    TriggerScene {
        scene_id: SceneId,
        scene_name: String,
    },

    /// Start combat encounter
    StartCombat {
        participants: Vec<CharacterId>,
        participant_names: Vec<String>,
        combat_description: String,
    },

    /// Modify character stat
    ModifyStat {
        character_id: CharacterId,
        character_name: String,
        stat_name: String,
        modifier: i32,
    },

    /// Add experience/reward
    AddReward {
        reward_type: String,
        amount: i32,
        description: String,
    },

    /// Custom effect (description for DM/LLM)
    Custom {
        description: String,
        requires_dm_action: bool,
    },
}

/// Reference to a chained event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainedEvent {
    /// Event to chain to
    pub event_id: NarrativeEventId,
    /// Name for display
    pub event_name: String,
    /// Delay before chain activates (turns)
    pub delay_turns: u32,
    /// Additional trigger condition for chain (beyond just completing parent)
    pub additional_trigger: Option<NarrativeTriggerType>,
    /// Description of why this chains
    pub chain_reason: Option<String>,
}

impl NarrativeEvent {
    pub fn new(world_id: WorldId, name: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            id: NarrativeEventId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            tags: Vec::new(),
            trigger_conditions: Vec::new(),
            trigger_logic: TriggerLogic::All,
            scene_direction: String::new(),
            suggested_opening: None,
            // NOTE: featured_npcs now stored as FEATURES_NPC edges
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
            // NOTE: scene_id, location_id, act_id now stored as graph edges
            priority: 0,
            is_favorite: false,
            // NOTE: chain_id, chain_position now stored as CONTAINS_EVENT edge
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if this event's triggers match the current game context
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
                // Check if player is at location AND time context matches
                let at_location = context.current_location.as_ref() == Some(location_id);
                let time_matches = context
                    .time_context
                    .as_ref()
                    .map(|current_time| {
                        // Case-insensitive, whitespace-trimmed comparison for time context
                        current_time.trim().to_lowercase() == required_time.trim().to_lowercase()
                    })
                    .unwrap_or(false);
                at_location && time_matches
            }
            NarrativeTriggerType::NpcAction { action_keywords, .. } => {
                // Check if recent player action matches any of the NPC action keywords
                // Note: This is a limited implementation - full NPC action tracking
                // would require additional infrastructure
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
                // Check if relationship sentiment between character_id and with_character
                // falls within the specified bounds (inclusive).
                context
                    .get_relationship(*character_id, *with_character)
                    .map(|sentiment| {
                        let meets_min = min_sentiment.map_or(true, |min| sentiment >= min);
                        let meets_max = max_sentiment.map_or(true, |max| sentiment <= max);
                        meets_min && meets_max
                    })
                    .unwrap_or(false)
            }
            NarrativeTriggerType::StatThreshold { .. } => {
                // KNOWN LIMITATION: StatThreshold trigger is not yet implemented.
                //
                // To implement this trigger:
                // 1. Add character stats to TriggerContext:
                //    `character_stats: HashMap<CharacterId, HashMap<String, i32>>`
                //    where the inner HashMap maps stat names to their current values.
                //
                // 2. The stat data should be populated from the Character entity's
                //    stats/attributes system. This depends on how character stats are
                //    modeled (e.g., fixed attributes like STR/DEX/CON or dynamic stats
                //    like health/mana/reputation).
                //
                // 3. Implementation should check if the stat value for `character_id`
                //    and `stat_name` falls within the specified `min_value`/`max_value`
                //    bounds. If only min is set, check >= min. If only max is set,
                //    check <= max. If both are set, check min <= value <= max.
                //
                // 4. Stat names should use a consistent naming convention (e.g.,
                //    lowercase with underscores: "hit_points", "reputation_guild").
                false
            }
            NarrativeTriggerType::CombatResult { .. } => {
                // KNOWN LIMITATION: CombatResult trigger is not yet implemented.
                //
                // To implement this trigger:
                // 1. Add combat result tracking to TriggerContext:
                //    ```
                //    pub struct CombatOutcome {
                //        pub victory: bool,          // true if player/party won
                //        pub participants: Vec<CharacterId>,  // NPCs involved
                //        pub combat_id: Option<CombatId>,     // reference to combat encounter
                //    }
                //    ```
                //    Then add: `last_combat_result: Option<CombatOutcome>`
                //
                // 2. The combat system needs to emit events or update TriggerContext
                //    when combat ends. This requires a CombatSystem or CombatManager
                //    that tracks active combats and their outcomes.
                //
                // 3. Implementation should check:
                //    - If `victory` is Some, match against `last_combat_result.victory`
                //    - If `involved_npc` is Some, check if that NPC was in participants
                //    - Both conditions must match if both are specified
                //
                // 4. Consider whether to track multiple recent combats or just the last one.
                //    For complex narratives, a `recent_combats: Vec<CombatOutcome>` might
                //    be more flexible.
                false
            }
            NarrativeTriggerType::Custom {
                description,
                llm_evaluation,
            } => {
                // Check if this custom trigger has been pre-evaluated via LLM.
                // The llm_evaluation flag indicates whether the DM wants automatic
                // LLM evaluation (true) or manual DM judgment (false).
                //
                // When llm_evaluation is true and a pre-evaluated result exists in
                // context.custom_trigger_results, use that result. Otherwise, return
                // false (trigger not met - requires DM action or LLM evaluation at
                // a higher layer).
                if *llm_evaluation {
                    // Use pre-evaluated LLM result if available
                    context
                        .custom_trigger_results
                        .get(description)
                        .copied()
                        .unwrap_or(false)
                } else {
                    // Manual DM judgment required - always return false here
                    // DM must explicitly trigger these events
                    false
                }
            }
        }
    }

    /// Mark this event as triggered with the given outcome
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

    /// Reset the triggered state (for repeatable events)
    pub fn reset(&mut self, now: DateTime<Utc>) {
        self.is_triggered = false;
        self.triggered_at = None;
        self.selected_outcome = None;
        self.updated_at = now;
    }

    /// Get the outcome by name
    pub fn get_outcome(&self, name: &str) -> Option<&EventOutcome> {
        self.outcomes.iter().find(|o| o.name == name)
    }

    /// Get the default outcome
    pub fn get_default_outcome(&self) -> Option<&EventOutcome> {
        self.default_outcome
            .as_ref()
            .and_then(|name| self.get_outcome(name))
    }
}

/// Context for evaluating triggers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerContext {
    pub current_location: Option<LocationId>,
    pub current_scene: Option<SceneId>,
    pub time_context: Option<String>,
    pub flags: HashMap<String, bool>,
    pub inventory: Vec<String>,
    pub completed_events: Vec<NarrativeEventId>,
    pub event_outcomes: HashMap<NarrativeEventId, String>,
    pub turns_since_event: HashMap<NarrativeEventId, u32>,
    pub completed_challenges: Vec<ChallengeId>,
    pub challenge_successes: HashMap<ChallengeId, bool>,
    pub turn_count: u32,
    pub recent_dialogue_topics: Vec<String>,
    pub recent_player_action: Option<String>,
    /// Pre-evaluated custom trigger results.
    /// Key is the trigger description, value is whether the trigger is met.
    /// If a custom trigger is not in this map, it will be treated as not triggered.
    pub custom_trigger_results: HashMap<String, bool>,
    /// Relationship sentiment values between characters.
    /// Outer key is the character whose feelings we're checking (e.g., NPC).
    /// Inner key is the character they have feelings toward (e.g., PC).
    /// Value is sentiment from -1.0 (hatred) to 1.0 (love).
    #[serde(default)]
    pub relationships: HashMap<CharacterId, HashMap<CharacterId, f32>>,
}

impl TriggerContext {
    /// Create a new empty trigger context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pre-evaluated custom trigger result.
    pub fn add_custom_trigger_result(&mut self, description: String, met: bool) {
        self.custom_trigger_results.insert(description, met);
    }

    /// Add multiple pre-evaluated custom trigger results.
    pub fn with_custom_trigger_results(
        mut self,
        results: impl IntoIterator<Item = (String, bool)>,
    ) -> Self {
        self.custom_trigger_results = results.into_iter().collect();
        self
    }

    /// Add a relationship sentiment value.
    ///
    /// # Arguments
    /// * `from_character` - The character whose feelings we're recording (e.g., NPC)
    /// * `to_character` - The character they have feelings toward (e.g., PC)
    /// * `sentiment` - Sentiment value from -1.0 (hatred) to 1.0 (love)
    pub fn add_relationship(
        &mut self,
        from_character: CharacterId,
        to_character: CharacterId,
        sentiment: f32,
    ) {
        self.relationships
            .entry(from_character)
            .or_default()
            .insert(to_character, sentiment);
    }

    /// Get the relationship sentiment between two characters.
    ///
    /// Returns None if no relationship data exists for this pair.
    pub fn get_relationship(
        &self,
        from_character: CharacterId,
        to_character: CharacterId,
    ) -> Option<f32> {
        self.relationships
            .get(&from_character)
            .and_then(|inner| inner.get(&to_character))
            .copied()
    }
}


/// Result of trigger evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerEvaluation {
    pub is_triggered: bool,
    pub matched_triggers: Vec<String>,
    pub unmatched_triggers: Vec<String>,
    pub total_triggers: usize,
    pub confidence: f32,
}

impl TriggerEvaluation {
    /// Get a human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "{}/{} triggers matched ({}%)",
            self.matched_triggers.len(),
            self.total_triggers,
            (self.confidence * 100.0) as u32
        )
    }
}

// =============================================================================
// Edge Support Structs
// =============================================================================

/// Represents a featured NPC in a narrative event (via FEATURES_NPC edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedNpc {
    /// The character ID of the featured NPC
    pub character_id: CharacterId,
    /// Optional role description for this NPC in the event
    pub role: Option<String>,
}

impl FeaturedNpc {
    pub fn new(character_id: CharacterId) -> Self {
        Self {
            character_id,
            role: None,
        }
    }

    pub fn with_role(character_id: CharacterId, role: impl Into<String>) -> Self {
        Self {
            character_id,
            role: Some(role.into()),
        }
    }
}

/// Represents an event's membership in an EventChain (via CONTAINS_EVENT edge)
///
/// Note: This edge is stored from EventChain → NarrativeEvent, so this struct
/// is used when querying chain membership from the event's perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventChainMembership {
    /// The chain this event belongs to
    pub chain_id: EventChainId,
    /// Position in the chain (0-indexed)
    pub position: u32,
    /// Whether this event has been completed in the chain
    pub is_completed: bool,
}

impl EventChainMembership {
    pub fn new(chain_id: EventChainId, position: u32) -> Self {
        Self {
            chain_id,
            position,
            is_completed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_event_with_relationship_trigger(
        npc_id: CharacterId,
        pc_id: CharacterId,
        min_sentiment: Option<f32>,
        max_sentiment: Option<f32>,
    ) -> NarrativeEvent {
        NarrativeEvent {
            id: NarrativeEventId::new(),
            world_id: WorldId::new(),
            name: "Test Event".to_string(),
            description: "Test event for relationship trigger".to_string(),
            tags: vec![],
            trigger_conditions: vec![NarrativeTrigger {
                trigger_type: NarrativeTriggerType::RelationshipThreshold {
                    character_id: npc_id,
                    character_name: "Test NPC".to_string(),
                    with_character: pc_id,
                    with_character_name: "Test PC".to_string(),
                    min_sentiment,
                    max_sentiment,
                },
                description: "Test relationship trigger".to_string(),
                is_required: true,
                trigger_id: "trigger-1".to_string(),
            }],
            trigger_logic: TriggerLogic::All,
            scene_direction: "Test scene".to_string(),
            suggested_opening: None,
            outcomes: vec![],
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn relationship_trigger_fires_when_sentiment_above_min() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        let event = create_test_event_with_relationship_trigger(npc_id, pc_id, Some(0.5), None);

        let mut context = TriggerContext::new();
        context.add_relationship(npc_id, pc_id, 0.7); // Above threshold

        let eval = event.evaluate_triggers(&context);
        assert!(eval.is_triggered);
    }

    #[test]
    fn relationship_trigger_does_not_fire_when_sentiment_below_min() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        let event = create_test_event_with_relationship_trigger(npc_id, pc_id, Some(0.5), None);

        let mut context = TriggerContext::new();
        context.add_relationship(npc_id, pc_id, 0.3); // Below threshold

        let eval = event.evaluate_triggers(&context);
        assert!(!eval.is_triggered);
    }

    #[test]
    fn relationship_trigger_fires_when_sentiment_below_max() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        let event = create_test_event_with_relationship_trigger(npc_id, pc_id, None, Some(-0.3));

        let mut context = TriggerContext::new();
        context.add_relationship(npc_id, pc_id, -0.5); // Below max (more negative)

        let eval = event.evaluate_triggers(&context);
        assert!(eval.is_triggered);
    }

    #[test]
    fn relationship_trigger_does_not_fire_when_sentiment_above_max() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        let event = create_test_event_with_relationship_trigger(npc_id, pc_id, None, Some(-0.3));

        let mut context = TriggerContext::new();
        context.add_relationship(npc_id, pc_id, 0.0); // Above max

        let eval = event.evaluate_triggers(&context);
        assert!(!eval.is_triggered);
    }

    #[test]
    fn relationship_trigger_fires_when_sentiment_within_range() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        // Range: -0.2 to 0.2 (neutral zone)
        let event =
            create_test_event_with_relationship_trigger(npc_id, pc_id, Some(-0.2), Some(0.2));

        let mut context = TriggerContext::new();
        context.add_relationship(npc_id, pc_id, 0.0); // Within range

        let eval = event.evaluate_triggers(&context);
        assert!(eval.is_triggered);
    }

    #[test]
    fn relationship_trigger_does_not_fire_when_sentiment_outside_range() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        // Range: -0.2 to 0.2 (neutral zone)
        let event =
            create_test_event_with_relationship_trigger(npc_id, pc_id, Some(-0.2), Some(0.2));

        let mut context = TriggerContext::new();
        context.add_relationship(npc_id, pc_id, 0.5); // Outside range

        let eval = event.evaluate_triggers(&context);
        assert!(!eval.is_triggered);
    }

    #[test]
    fn relationship_trigger_does_not_fire_when_no_relationship_data() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        let event = create_test_event_with_relationship_trigger(npc_id, pc_id, Some(0.5), None);

        let context = TriggerContext::new(); // No relationship data

        let eval = event.evaluate_triggers(&context);
        assert!(!eval.is_triggered);
    }

    #[test]
    fn relationship_trigger_fires_at_exact_min_boundary() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        let event = create_test_event_with_relationship_trigger(npc_id, pc_id, Some(0.5), None);

        let mut context = TriggerContext::new();
        context.add_relationship(npc_id, pc_id, 0.5); // Exactly at threshold

        let eval = event.evaluate_triggers(&context);
        assert!(eval.is_triggered);
    }

    #[test]
    fn relationship_trigger_fires_at_exact_max_boundary() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        let event = create_test_event_with_relationship_trigger(npc_id, pc_id, None, Some(0.5));

        let mut context = TriggerContext::new();
        context.add_relationship(npc_id, pc_id, 0.5); // Exactly at threshold

        let eval = event.evaluate_triggers(&context);
        assert!(eval.is_triggered);
    }

    #[test]
    fn trigger_context_get_relationship_returns_none_for_unknown_pair() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        let context = TriggerContext::new();

        assert!(context.get_relationship(npc_id, pc_id).is_none());
    }

    #[test]
    fn trigger_context_stores_and_retrieves_relationship() {
        let npc_id = CharacterId::new();
        let pc_id = CharacterId::new();
        let mut context = TriggerContext::new();

        context.add_relationship(npc_id, pc_id, 0.75);

        assert_eq!(context.get_relationship(npc_id, pc_id), Some(0.75));
    }
}
