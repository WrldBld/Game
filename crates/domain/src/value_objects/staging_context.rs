//! Staging context - Data passed to LLM for staging decisions
//!
//! This captures the story context needed for intelligent NPC presence decisions.
//!
//! # Tier Classification
//!
//! - **Tier 3a: Composite VO (Simple Data)** - Most types in this module are
//!   simple data structs with public fields for LLM context.
//!
//! See [docs/architecture/tier-levels.md](../../../../docs/architecture/tier-levels.md)
//! for complete tier classification system.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::CharacterId;

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
    pub character_id: CharacterId,
    pub character_name: String,
    pub last_dialogue_summary: String,
    pub game_time_of_dialogue: String,
    pub mentioned_locations: Vec<String>,
}

/// Rule-based NPC presence suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleBasedSuggestion {
    pub character_id: CharacterId,
    pub character_name: String,
    pub is_present: bool,
    pub reasoning: String,
    pub roll_result: Option<RollResult>,
}

/// Result of a probabilistic presence roll
///
/// The `passed` field is derived from `chance_percent` and `rolled` to prevent
/// inconsistent states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollResult {
    chance_percent: u8,
    rolled: u8,
    passed: bool,
}

impl RollResult {
    /// Create a new roll result with computed `passed` value.
    ///
    /// A roll passes when:
    /// - `chance_percent == 100` (always passes)
    /// - `chance_percent > 0 && rolled <= chance_percent` (standard check)
    ///
    /// A roll always fails when `chance_percent == 0`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - chance_percent is not in range 0..=100
    /// - rolled is not in range 0..=100
    pub fn new(chance_percent: u8, rolled: u8) -> Result<Self, String> {
        if chance_percent > 100 {
            return Err(format!(
                "Chance percent must be 0..=100, got {}",
                chance_percent
            ));
        }
        if rolled > 100 {
            return Err(format!("Rolled value must be 0..=100, got {}", rolled));
        }
        // 0% always fails, 100% always passes, otherwise check rolled <= chance_percent
        let passed = chance_percent == 100 || (chance_percent > 0 && rolled <= chance_percent);
        Ok(Self {
            chance_percent,
            rolled,
            passed,
        })
    }

    /// Get the success chance (0-100)
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
    // Builder methods
    // -------------------------------------------------------------------------

    /// Add a roll result
    pub fn with_roll(mut self, chance_percent: u8, rolled: u8) -> Self {
        self.roll_result = Some(
            RollResult::new(chance_percent, rolled)
                .expect("RollResult::new should succeed with valid percent values"),
        );
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roll_result_pass_when_rolled_less_than_chance() {
        let result = RollResult::new(50, 30).expect("valid value");
        assert_eq!(result.chance_percent(), 50);
        assert_eq!(result.rolled(), 30);
        assert!(result.passed());
    }

    #[test]
    fn test_roll_result_pass_when_equal() {
        let result = RollResult::new(50, 50).expect("valid value");
        assert_eq!(result.chance_percent(), 50);
        assert_eq!(result.rolled(), 50);
        assert!(result.passed());
    }

    #[test]
    fn test_roll_result_fail_when_rolled_higher() {
        let result = RollResult::new(30, 50).expect("valid value");
        assert_eq!(result.chance_percent(), 30);
        assert_eq!(result.rolled(), 50);
        assert!(!result.passed());
    }

    #[test]
    fn test_roll_result_always_fail_at_zero_chance() {
        let result = RollResult::new(0, 0).expect("valid value");
        assert!(!result.passed());

        let result = RollResult::new(0, 100).expect("valid value");
        assert!(!result.passed());
    }

    #[test]
    fn test_roll_result_always_pass_at_100_chance() {
        let result = RollResult::new(100, 0).expect("valid value");
        assert!(result.passed());

        let result = RollResult::new(100, 100).expect("valid value");
        assert!(result.passed());
    }
}
