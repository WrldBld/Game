//! Staging context - Data passed to LLM for staging decisions
//!
//! This captures the story context needed for intelligent NPC presence decisions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::CharacterId;

/// Complete context for staging decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagingContext {
    // Region information
    region_name: String,
    region_description: String,
    location_name: String,
    time_of_day: String,
    time_display: String,

    // Story context
    active_events: Vec<ActiveEventContext>,
    npc_dialogues: Vec<NpcDialogueContext>,

    // Extensible additional context
    additional_context: HashMap<String, String>,
}

/// Context about an active narrative event relevant to staging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveEventContext {
    event_name: String,
    description: String,
    relevance: String,
}

/// Context about recent dialogues with an NPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDialogueContext {
    character_id: CharacterId,
    character_name: String,
    last_dialogue_summary: String,
    game_time_of_dialogue: String,
    mentioned_locations: Vec<String>,
}

/// Rule-based NPC presence suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleBasedSuggestion {
    character_id: CharacterId,
    character_name: String,
    is_present: bool,
    reasoning: String,
    roll_result: Option<RollResult>,
}

/// Result of a probabilistic presence roll
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollResult {
    chance_percent: u8,
    rolled: u8,
    passed: bool,
}

impl RollResult {
    /// Create a new roll result
    pub fn new(chance_percent: u8, rolled: u8) -> Self {
        Self {
            chance_percent,
            rolled,
            passed: rolled <= chance_percent,
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the chance percentage
    pub fn chance_percent(&self) -> u8 {
        self.chance_percent
    }

    /// Get the rolled value
    pub fn rolled(&self) -> u8 {
        self.rolled
    }

    /// Check if the roll passed
    pub fn passed(&self) -> bool {
        self.passed
    }
}

impl StagingContext {
    pub fn new(
        region_name: impl Into<String>,
        region_description: impl Into<String>,
        location_name: impl Into<String>,
        time_of_day: impl Into<String>,
        time_display: impl Into<String>,
    ) -> Self {
        Self {
            region_name: region_name.into(),
            region_description: region_description.into(),
            location_name: location_name.into(),
            time_of_day: time_of_day.into(),
            time_display: time_display.into(),
            active_events: Vec::new(),
            npc_dialogues: Vec::new(),
            additional_context: HashMap::new(),
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the region name
    pub fn region_name(&self) -> &str {
        &self.region_name
    }

    /// Get the region description
    pub fn region_description(&self) -> &str {
        &self.region_description
    }

    /// Get the location name
    pub fn location_name(&self) -> &str {
        &self.location_name
    }

    /// Get the time of day
    pub fn time_of_day(&self) -> &str {
        &self.time_of_day
    }

    /// Get the time display string
    pub fn time_display(&self) -> &str {
        &self.time_display
    }

    /// Get the active events
    pub fn active_events(&self) -> &[ActiveEventContext] {
        &self.active_events
    }

    /// Get the NPC dialogues
    pub fn npc_dialogues(&self) -> &[NpcDialogueContext] {
        &self.npc_dialogues
    }

    /// Get the additional context
    pub fn additional_context(&self) -> &HashMap<String, String> {
        &self.additional_context
    }

    // -------------------------------------------------------------------------
    // Builder methods
    // -------------------------------------------------------------------------

    /// Set the active events
    pub fn with_active_events(mut self, events: Vec<ActiveEventContext>) -> Self {
        self.active_events = events;
        self
    }

    /// Set the NPC dialogues
    pub fn with_npc_dialogues(mut self, dialogues: Vec<NpcDialogueContext>) -> Self {
        self.npc_dialogues = dialogues;
        self
    }

    /// Add additional context
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_context.insert(key.into(), value.into());
        self
    }
}

impl ActiveEventContext {
    pub fn new(
        event_name: impl Into<String>,
        description: impl Into<String>,
        relevance: impl Into<String>,
    ) -> Self {
        Self {
            event_name: event_name.into(),
            description: description.into(),
            relevance: relevance.into(),
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the event name
    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    /// Get the event description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the relevance
    pub fn relevance(&self) -> &str {
        &self.relevance
    }
}

impl NpcDialogueContext {
    pub fn new(
        character_id: CharacterId,
        character_name: impl Into<String>,
        last_dialogue_summary: impl Into<String>,
        game_time_of_dialogue: impl Into<String>,
    ) -> Self {
        Self {
            character_id,
            character_name: character_name.into(),
            last_dialogue_summary: last_dialogue_summary.into(),
            game_time_of_dialogue: game_time_of_dialogue.into(),
            mentioned_locations: Vec::new(),
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the character ID
    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    /// Get the character name
    pub fn character_name(&self) -> &str {
        &self.character_name
    }

    /// Get the last dialogue summary
    pub fn last_dialogue_summary(&self) -> &str {
        &self.last_dialogue_summary
    }

    /// Get the game time of dialogue
    pub fn game_time_of_dialogue(&self) -> &str {
        &self.game_time_of_dialogue
    }

    /// Get the mentioned locations
    pub fn mentioned_locations(&self) -> &[String] {
        &self.mentioned_locations
    }

    // -------------------------------------------------------------------------
    // Builder methods
    // -------------------------------------------------------------------------

    /// Set the mentioned locations
    pub fn with_mentioned_locations(mut self, locations: Vec<String>) -> Self {
        self.mentioned_locations = locations;
        self
    }
}

impl RuleBasedSuggestion {
    pub fn present(
        character_id: CharacterId,
        character_name: impl Into<String>,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            character_id,
            character_name: character_name.into(),
            is_present: true,
            reasoning: reasoning.into(),
            roll_result: None,
        }
    }

    pub fn absent(
        character_id: CharacterId,
        character_name: impl Into<String>,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            character_id,
            character_name: character_name.into(),
            is_present: false,
            reasoning: reasoning.into(),
            roll_result: None,
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the character ID
    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    /// Get the character name
    pub fn character_name(&self) -> &str {
        &self.character_name
    }

    /// Check if the NPC is present
    pub fn is_present(&self) -> bool {
        self.is_present
    }

    /// Get the reasoning
    pub fn reasoning(&self) -> &str {
        &self.reasoning
    }

    /// Get the roll result (if any)
    pub fn roll_result(&self) -> Option<&RollResult> {
        self.roll_result.as_ref()
    }

    // -------------------------------------------------------------------------
    // Builder methods
    // -------------------------------------------------------------------------

    /// Add a roll result
    pub fn with_roll(mut self, chance_percent: u8, rolled: u8) -> Self {
        self.roll_result = Some(RollResult::new(chance_percent, rolled));
        self
    }
}
