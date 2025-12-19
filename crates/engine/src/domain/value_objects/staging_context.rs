//! Staging context - Data passed to LLM for staging decisions
//!
//! This captures the story context needed for intelligent NPC presence decisions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Complete context for staging decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagingContext {
    // Region information
    pub region_name: String,
    pub region_description: String,
    pub location_name: String,
    pub time_of_day: String,
    pub time_display: String,
    
    // Story context
    pub active_events: Vec<ActiveEventContext>,
    pub npc_dialogues: Vec<NpcDialogueContext>,
    
    // Extensible additional context
    pub additional_context: HashMap<String, String>,
}

/// Context about an active narrative event relevant to staging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveEventContext {
    pub event_name: String,
    pub description: String,
    pub relevance: String,
}

/// Context about recent dialogues with an NPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDialogueContext {
    pub character_id: Uuid,
    pub character_name: String,
    pub last_dialogue_summary: String,
    pub game_time_of_dialogue: String,
    pub mentioned_locations: Vec<String>,
}

/// Rule-based NPC presence suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleBasedSuggestion {
    pub character_id: Uuid,
    pub character_name: String,
    pub is_present: bool,
    pub reasoning: String,
    pub roll_result: Option<RollResult>,
}

/// Result of a probabilistic presence roll
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollResult {
    pub chance_percent: u8,
    pub rolled: u8,
    pub passed: bool,
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

    pub fn with_active_events(mut self, events: Vec<ActiveEventContext>) -> Self {
        self.active_events = events;
        self
    }

    pub fn with_npc_dialogues(mut self, dialogues: Vec<NpcDialogueContext>) -> Self {
        self.npc_dialogues = dialogues;
        self
    }

    pub fn add_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
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
}

impl NpcDialogueContext {
    pub fn new(
        character_id: Uuid,
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

    pub fn with_mentioned_locations(mut self, locations: Vec<String>) -> Self {
        self.mentioned_locations = locations;
        self
    }
}

impl RuleBasedSuggestion {
    pub fn present(
        character_id: Uuid,
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
        character_id: Uuid,
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

    pub fn with_roll(mut self, chance_percent: u8, rolled: u8) -> Self {
        self.roll_result = Some(RollResult {
            chance_percent,
            rolled,
            passed: rolled <= chance_percent,
        });
        self
    }
}
