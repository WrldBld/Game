//! NarrativeEvent entity - DM-designed events with triggers and outcomes
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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use wrldbldr_domain::{
    ChallengeId, CharacterId, EventChainId, LocationId, NarrativeEventId, SceneId,
};

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
    trigger_type: NarrativeTriggerType,
    /// Human-readable description for DM
    description: String,
    /// Whether this specific condition must be met (for AtLeast logic)
    is_required: bool,
    /// Unique identifier for this trigger within the event
    trigger_id: String,
}

impl NarrativeTrigger {
    pub fn new(
        trigger_type: NarrativeTriggerType,
        description: impl Into<String>,
        trigger_id: impl Into<String>,
    ) -> Self {
        Self {
            trigger_type,
            description: description.into(),
            is_required: false,
            trigger_id: trigger_id.into(),
        }
    }

    // Read accessors
    pub fn trigger_type(&self) -> &NarrativeTriggerType {
        &self.trigger_type
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn is_required(&self) -> bool {
        self.is_required
    }

    pub fn trigger_id(&self) -> &str {
        &self.trigger_id
    }

    // Builder methods
    pub fn with_required(mut self, is_required: bool) -> Self {
        self.is_required = is_required;
        self
    }
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

    // === Compendium-based triggers ===
    /// Player character has a specific spell from the compendium
    KnowsSpell {
        /// The spell ID from the compendium
        spell_id: String,
        /// Display name for DM reference
        spell_name: String,
    },

    /// Player character has a specific feat from the compendium
    HasFeat {
        /// The feat ID from the compendium
        feat_id: String,
        /// Display name for DM reference
        feat_name: String,
    },

    /// Player character's class matches
    HasClass {
        /// The class ID from the compendium
        class_id: String,
        /// Display name for DM reference
        class_name: String,
        /// Optional: minimum level in that class
        min_level: Option<u8>,
    },

    /// Player character's origin/race matches
    HasOrigin {
        /// The race/ancestry ID from the compendium
        origin_id: String,
        /// Display name for DM reference
        origin_name: String,
    },

    /// Player character knows about a specific creature/monster
    KnowsCreature {
        /// The creature ID from the compendium or bestiary
        creature_id: String,
        /// Display name for DM reference
        creature_name: String,
    },
}

/// An outcome branch for a narrative event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventOutcome {
    /// Unique identifier for this outcome within the event
    name: String,
    /// Display label for DM
    label: String,
    /// Description of what happens in this outcome
    description: String,
    /// Conditions for this outcome (how does player reach this?)
    condition: Option<OutcomeCondition>,
    /// Effects that occur when this outcome happens
    effects: Vec<EventEffect>,
    /// Narrative events to chain to after this outcome
    chain_events: Vec<ChainedEvent>,
    /// Narrative summary to add to timeline
    timeline_summary: Option<String>,
}

impl EventOutcome {
    pub fn new(
        name: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            description: description.into(),
            condition: None,
            effects: Vec::new(),
            chain_events: Vec::new(),
            timeline_summary: None,
        }
    }

    // Read accessors
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn condition(&self) -> Option<&OutcomeCondition> {
        self.condition.as_ref()
    }

    pub fn effects(&self) -> &[EventEffect] {
        &self.effects
    }

    pub fn chain_events(&self) -> &[ChainedEvent] {
        &self.chain_events
    }

    pub fn timeline_summary(&self) -> Option<&str> {
        self.timeline_summary.as_deref()
    }

    // Builder methods
    pub fn with_condition(mut self, condition: OutcomeCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn with_effects(mut self, effects: Vec<EventEffect>) -> Self {
        self.effects = effects;
        self
    }

    pub fn with_chain_events(mut self, chain_events: Vec<ChainedEvent>) -> Self {
        self.chain_events = chain_events;
        self
    }

    pub fn with_timeline_summary(mut self, summary: impl Into<String>) -> Self {
        self.timeline_summary = Some(summary.into());
        self
    }
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
    event_id: NarrativeEventId,
    /// Name for display
    event_name: String,
    /// Delay before chain activates (turns)
    delay_turns: u32,
    /// Additional trigger condition for chain (beyond just completing parent)
    additional_trigger: Option<NarrativeTriggerType>,
    /// Description of why this chains
    chain_reason: Option<String>,
}

impl ChainedEvent {
    pub fn new(event_id: NarrativeEventId, event_name: impl Into<String>) -> Self {
        Self {
            event_id,
            event_name: event_name.into(),
            delay_turns: 0,
            additional_trigger: None,
            chain_reason: None,
        }
    }

    // Read accessors
    pub fn event_id(&self) -> NarrativeEventId {
        self.event_id
    }

    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    pub fn delay_turns(&self) -> u32 {
        self.delay_turns
    }

    pub fn additional_trigger(&self) -> Option<&NarrativeTriggerType> {
        self.additional_trigger.as_ref()
    }

    pub fn chain_reason(&self) -> Option<&str> {
        self.chain_reason.as_deref()
    }

    // Builder methods
    pub fn with_delay_turns(mut self, delay_turns: u32) -> Self {
        self.delay_turns = delay_turns;
        self
    }

    pub fn with_additional_trigger(mut self, trigger: NarrativeTriggerType) -> Self {
        self.additional_trigger = Some(trigger);
        self
    }

    pub fn with_chain_reason(mut self, reason: impl Into<String>) -> Self {
        self.chain_reason = Some(reason.into());
        self
    }
}

/// Context for evaluating triggers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerContext {
    current_location: Option<LocationId>,
    current_scene: Option<SceneId>,
    time_context: Option<String>,
    flags: HashMap<String, bool>,
    inventory: Vec<String>,
    completed_events: Vec<NarrativeEventId>,
    event_outcomes: HashMap<NarrativeEventId, String>,
    turns_since_event: HashMap<NarrativeEventId, u32>,
    completed_challenges: Vec<ChallengeId>,
    challenge_successes: HashMap<ChallengeId, bool>,
    turn_count: u32,
    recent_dialogue_topics: Vec<String>,
    recent_player_action: Option<String>,
    /// Pre-evaluated custom trigger results.
    /// Key is the trigger description, value is whether the trigger is met.
    /// If a custom trigger is not in this map, it will be treated as not triggered.
    custom_trigger_results: HashMap<String, bool>,
    /// Relationship sentiment values between characters.
    /// Outer key is the character whose feelings we're checking (e.g., NPC).
    /// Inner key is the character they have feelings toward (e.g., PC).
    /// Value is sentiment from -1.0 (hatred) to 1.0 (love).
    #[serde(default)]
    relationships: HashMap<CharacterId, HashMap<CharacterId, f32>>,
    /// Character stat values for StatThreshold trigger evaluation.
    /// Outer key is the CharacterId, inner key is the stat name.
    /// Value is the effective stat value (base + active modifiers).
    #[serde(default)]
    character_stats: HashMap<CharacterId, HashMap<String, i32>>,

    // === Compendium-based trigger context ===
    /// Player character's known spells (spell IDs from compendium).
    #[serde(default)]
    known_spells: Vec<String>,

    /// Player character's acquired feats (feat IDs from compendium).
    #[serde(default)]
    character_feats: Vec<String>,

    /// Player character's class levels (class_id -> level).
    #[serde(default)]
    class_levels: HashMap<String, u8>,

    /// Player character's origin/race ID (from compendium).
    #[serde(default)]
    origin_id: Option<String>,

    /// Creatures the player character knows about (creature IDs).
    #[serde(default)]
    known_creatures: Vec<String>,
}

impl TriggerContext {
    /// Create a new empty trigger context.
    pub fn new() -> Self {
        Self::default()
    }

    // Read accessors
    pub fn current_location(&self) -> Option<LocationId> {
        self.current_location
    }

    pub fn current_scene(&self) -> Option<SceneId> {
        self.current_scene
    }

    pub fn time_context(&self) -> Option<&str> {
        self.time_context.as_deref()
    }

    pub fn flags(&self) -> &HashMap<String, bool> {
        &self.flags
    }

    pub fn inventory(&self) -> &[String] {
        &self.inventory
    }

    pub fn completed_events(&self) -> &[NarrativeEventId] {
        &self.completed_events
    }

    pub fn event_outcomes(&self) -> &HashMap<NarrativeEventId, String> {
        &self.event_outcomes
    }

    pub fn turns_since_event(&self) -> &HashMap<NarrativeEventId, u32> {
        &self.turns_since_event
    }

    pub fn completed_challenges(&self) -> &[ChallengeId] {
        &self.completed_challenges
    }

    pub fn challenge_successes(&self) -> &HashMap<ChallengeId, bool> {
        &self.challenge_successes
    }

    pub fn turn_count(&self) -> u32 {
        self.turn_count
    }

    pub fn recent_dialogue_topics(&self) -> &[String] {
        &self.recent_dialogue_topics
    }

    pub fn recent_player_action(&self) -> Option<&str> {
        self.recent_player_action.as_deref()
    }

    pub fn custom_trigger_results(&self) -> &HashMap<String, bool> {
        &self.custom_trigger_results
    }

    pub fn relationships(&self) -> &HashMap<CharacterId, HashMap<CharacterId, f32>> {
        &self.relationships
    }

    pub fn character_stats(&self) -> &HashMap<CharacterId, HashMap<String, i32>> {
        &self.character_stats
    }

    pub fn known_spells(&self) -> &[String] {
        &self.known_spells
    }

    pub fn character_feats(&self) -> &[String] {
        &self.character_feats
    }

    pub fn class_levels(&self) -> &HashMap<String, u8> {
        &self.class_levels
    }

    pub fn origin_id(&self) -> Option<&str> {
        self.origin_id.as_deref()
    }

    pub fn known_creatures(&self) -> &[String] {
        &self.known_creatures
    }

    // Builder methods
    pub fn with_current_location(mut self, location_id: LocationId) -> Self {
        self.current_location = Some(location_id);
        self
    }

    pub fn with_current_scene(mut self, scene_id: SceneId) -> Self {
        self.current_scene = Some(scene_id);
        self
    }

    pub fn with_time_context(mut self, time_context: impl Into<String>) -> Self {
        self.time_context = Some(time_context.into());
        self
    }

    pub fn with_flags(mut self, flags: HashMap<String, bool>) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_inventory(mut self, inventory: Vec<String>) -> Self {
        self.inventory = inventory;
        self
    }

    pub fn with_completed_events(mut self, events: Vec<NarrativeEventId>) -> Self {
        self.completed_events = events;
        self
    }

    pub fn with_event_outcomes(mut self, outcomes: HashMap<NarrativeEventId, String>) -> Self {
        self.event_outcomes = outcomes;
        self
    }

    pub fn with_turns_since_event(mut self, turns: HashMap<NarrativeEventId, u32>) -> Self {
        self.turns_since_event = turns;
        self
    }

    pub fn with_completed_challenges(mut self, challenges: Vec<ChallengeId>) -> Self {
        self.completed_challenges = challenges;
        self
    }

    pub fn with_challenge_successes(mut self, successes: HashMap<ChallengeId, bool>) -> Self {
        self.challenge_successes = successes;
        self
    }

    pub fn with_turn_count(mut self, turn_count: u32) -> Self {
        self.turn_count = turn_count;
        self
    }

    pub fn with_recent_dialogue_topics(mut self, topics: Vec<String>) -> Self {
        self.recent_dialogue_topics = topics;
        self
    }

    pub fn with_recent_player_action(mut self, action: impl Into<String>) -> Self {
        self.recent_player_action = Some(action.into());
        self
    }

    pub fn with_known_spells(mut self, spells: Vec<String>) -> Self {
        self.known_spells = spells;
        self
    }

    pub fn with_character_feats(mut self, feats: Vec<String>) -> Self {
        self.character_feats = feats;
        self
    }

    pub fn with_class_levels(mut self, levels: HashMap<String, u8>) -> Self {
        self.class_levels = levels;
        self
    }

    pub fn with_origin_id(mut self, origin_id: impl Into<String>) -> Self {
        self.origin_id = Some(origin_id.into());
        self
    }

    pub fn with_known_creatures(mut self, creatures: Vec<String>) -> Self {
        self.known_creatures = creatures;
        self
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

    /// Add a character's stat value.
    ///
    /// # Arguments
    /// * `character_id` - The character whose stat we're recording
    /// * `stat_name` - The name of the stat (e.g., "STR", "health", "sanity")
    /// * `value` - The effective stat value (base + active modifiers)
    pub fn add_character_stat(
        &mut self,
        character_id: CharacterId,
        stat_name: impl Into<String>,
        value: i32,
    ) {
        self.character_stats
            .entry(character_id)
            .or_default()
            .insert(stat_name.into(), value);
    }

    /// Add all stats for a character at once.
    ///
    /// # Arguments
    /// * `character_id` - The character whose stats we're recording
    /// * `stats` - Map of stat name to effective value
    pub fn add_character_stats(&mut self, character_id: CharacterId, stats: HashMap<String, i32>) {
        self.character_stats.insert(character_id, stats);
    }

    /// Get a character's stat value.
    ///
    /// Returns None if the character or stat doesn't exist in the context.
    pub fn get_character_stat(&self, character_id: CharacterId, stat_name: &str) -> Option<i32> {
        self.character_stats
            .get(&character_id)
            .and_then(|stats| stats.get(stat_name))
            .copied()
    }
}

/// Result of trigger evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerEvaluation {
    is_triggered: bool,
    matched_triggers: Vec<String>,
    unmatched_triggers: Vec<String>,
    total_triggers: usize,
    confidence: f32,
}

impl TriggerEvaluation {
    pub fn new(
        is_triggered: bool,
        matched_triggers: Vec<String>,
        unmatched_triggers: Vec<String>,
        total_triggers: usize,
        confidence: f32,
    ) -> Self {
        Self {
            is_triggered,
            matched_triggers,
            unmatched_triggers,
            total_triggers,
            confidence,
        }
    }

    // Read accessors
    pub fn is_triggered(&self) -> bool {
        self.is_triggered
    }

    pub fn matched_triggers(&self) -> &[String] {
        &self.matched_triggers
    }

    pub fn unmatched_triggers(&self) -> &[String] {
        &self.unmatched_triggers
    }

    pub fn total_triggers(&self) -> usize {
        self.total_triggers
    }

    pub fn confidence(&self) -> f32 {
        self.confidence
    }

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
    character_id: CharacterId,
    /// Optional role description for this NPC in the event
    role: Option<String>,
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

    // Read accessors
    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    pub fn role(&self) -> Option<&str> {
        self.role.as_deref()
    }
}

/// Represents an event's membership in an EventChain (via CONTAINS_EVENT edge)
///
/// Note: This edge is stored from EventChain -> NarrativeEvent, so this struct
/// is used when querying chain membership from the event's perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventChainMembership {
    /// The chain this event belongs to
    chain_id: EventChainId,
    /// Position in the chain (0-indexed)
    position: u32,
    /// Whether this event has been completed in the chain
    is_completed: bool,
}

impl EventChainMembership {
    pub fn new(chain_id: EventChainId, position: u32) -> Self {
        Self {
            chain_id,
            position,
            is_completed: false,
        }
    }

    // Read accessors
    pub fn chain_id(&self) -> EventChainId {
        self.chain_id
    }

    pub fn position(&self) -> u32 {
        self.position
    }

    pub fn is_completed(&self) -> bool {
        self.is_completed
    }

    // Builder methods
    pub fn with_completed(mut self, is_completed: bool) -> Self {
        self.is_completed = is_completed;
        self
    }
}
