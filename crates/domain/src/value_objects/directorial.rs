//! Directorial guidance for LLM responses
//!
//! Provides structured guidance for LLM on how to handle
//! NPC responses and scene interactions.
//!
//! # Tier Classification
//!
//! - **Tier 3a: Composite VO (Simple Data)** - `DirectorialNotes`, `ToneGuidance`,
//!   `PacingGuidance`, `NpcMotivation` are simple data structs with public fields.
//!
//! See [docs/architecture/tier-levels.md](../../../../docs/architecture/tier-levels.md)
//! for complete tier classification system.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use wrldbldr_domain::CharacterId;

// Custom serde module for HashMap<CharacterId, NpcMotivation>
// This handles serialization/deserialization with CharacterId as key
pub mod serde_with_character_id_key {
    use super::*;
    use serde::de::Error as DeError;

    /// Serialize HashMap<CharacterId, NpcMotivation> as HashMap<String, NpcMotivation>
    pub fn serialize<S>(
        map: &HashMap<CharacterId, NpcMotivation>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_map: std::collections::HashMap<String, NpcMotivation> = map
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        string_map.serialize(serializer)
    }

    /// Deserialize HashMap<String, NpcMotivation> as HashMap<CharacterId, NpcMotivation>
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<CharacterId, NpcMotivation>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_map: std::collections::HashMap<String, NpcMotivation> =
            Deserialize::deserialize(deserializer)?;

        let mut result = HashMap::new();
        for (key_str, value) in string_map {
            let key = CharacterId::from_uuid(uuid::Uuid::parse_str(&key_str).map_err(|e| {
                DeError::custom(format!("Invalid CharacterId '{}': {}", key_str, e))
            })?);
            result.insert(key, value);
        }

        Ok(result)
    }
}

/// Structured directorial notes for a scene
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectorialNotes {
    /// General notes about the scene (free-form text)
    pub general_notes: String,
    /// Overall tone for the scene
    pub tone: ToneGuidance,
    /// Per-NPC motivation hints (character ID -> motivation text)
    /// Uses typed CharacterId as key instead of raw String for type safety
    #[serde(with = "serde_with_character_id_key")]
    pub npc_motivations: HashMap<CharacterId, NpcMotivation>,
    /// Topics the LLM should avoid
    pub forbidden_topics: Vec<String>,
    /// Tools the LLM is allowed to call in this scene
    pub allowed_tools: Vec<String>,
    /// Suggested story beats for the scene
    pub suggested_beats: Vec<String>,
    /// Pacing guidance
    pub pacing: PacingGuidance,
}

impl DirectorialNotes {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Builder Methods ──────────────────────────────────────────────────

    pub fn with_general_notes(mut self, notes: impl Into<String>) -> Self {
        self.general_notes = notes.into();
        self
    }

    pub fn with_tone(mut self, tone: ToneGuidance) -> Self {
        self.tone = tone;
        self
    }

    pub fn with_npc_motivation(
        mut self,
        character_id: CharacterId,
        motivation: NpcMotivation,
    ) -> Self {
        self.npc_motivations.insert(character_id, motivation);
        self
    }

    pub fn with_forbidden_topic(mut self, topic: impl Into<String>) -> Self {
        self.forbidden_topics.push(topic.into());
        self
    }

    pub fn with_allowed_tool(mut self, tool: impl Into<String>) -> Self {
        self.allowed_tools.push(tool.into());
        self
    }

    pub fn with_suggested_beat(mut self, beat: impl Into<String>) -> Self {
        self.suggested_beats.push(beat.into());
        self
    }

    pub fn with_pacing(mut self, pacing: PacingGuidance) -> Self {
        self.pacing = pacing;
        self
    }

    /// Convert to a prompt-friendly string for the LLM
    pub fn to_prompt(&self) -> String {
        let mut parts = Vec::new();

        if !self.general_notes.is_empty() {
            parts.push(format!("Scene Notes: {}", self.general_notes));
        }

        parts.push(format!("Tone: {}", self.tone.description()));
        parts.push(format!("Pacing: {}", self.pacing.description()));

        if !self.npc_motivations.is_empty() {
            parts.push("NPC Motivations:".to_string());
            for (char_id, motivation) in &self.npc_motivations {
                parts.push(format!(
                    "  - {}: {} (Mood: {})",
                    char_id, motivation.immediate_goal, motivation.current_mood
                ));
                if let Some(secret) = &motivation.secret_agenda {
                    parts.push(format!("    [Hidden agenda: {}]", secret));
                }
            }
        }

        if !self.forbidden_topics.is_empty() {
            parts.push(format!(
                "Avoid these topics: {}",
                self.forbidden_topics.join(", ")
            ));
        }

        if !self.suggested_beats.is_empty() {
            parts.push("Suggested story beats:".to_string());
            for beat in &self.suggested_beats {
                parts.push(format!("  - {}", beat));
            }
        }

        parts.join("\n")
    }
}

/// Tone guidance for the scene
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToneGuidance {
    /// Default neutral tone
    #[default]
    Neutral,
    /// Serious, dramatic moments
    Serious,
    /// Light, fun interactions
    Lighthearted,
    /// Building suspense
    Tense,
    /// Unknown, cryptic elements
    Mysterious,
    /// Fast-paced excitement
    Exciting,
    /// Quiet, reflective moments
    Contemplative,
    /// Spooky, unsettling atmosphere
    Creepy,
    /// Romantic or intimate
    Romantic,
    /// Comic relief
    Comedic,
    /// Custom tone description
    Custom(String),
}

impl ToneGuidance {
    pub fn description(&self) -> &str {
        match self {
            Self::Neutral => "Neutral - balanced, conversational",
            Self::Serious => "Serious - dramatic, weighty",
            Self::Lighthearted => "Lighthearted - fun, playful",
            Self::Tense => "Tense - suspenseful, nervous energy",
            Self::Mysterious => "Mysterious - cryptic, intriguing",
            Self::Exciting => "Exciting - fast-paced, energetic",
            Self::Contemplative => "Contemplative - quiet, reflective",
            Self::Creepy => "Creepy - unsettling, eerie",
            Self::Romantic => "Romantic - intimate, emotional",
            Self::Comedic => "Comedic - humorous, witty",
            Self::Custom(s) => s,
        }
    }
}

/// Pacing guidance for the scene
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PacingGuidance {
    /// Let conversation flow naturally
    #[default]
    Natural,
    /// Keep things moving quickly
    Fast,
    /// Take time for details and atmosphere
    Slow,
    /// Build gradually to a climax
    Building,
    /// Urgent, pressing action needed
    Urgent,
}

impl PacingGuidance {
    pub fn description(&self) -> &str {
        match self {
            Self::Natural => "Natural flow",
            Self::Fast => "Quick pace, keep momentum",
            Self::Slow => "Slow, atmospheric",
            Self::Building => "Building tension",
            Self::Urgent => "Urgent, time-sensitive",
        }
    }
}

/// Motivation hints for an NPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMotivation {
    /// Current emotional state
    pub current_mood: String,
    /// What they're trying to achieve right now
    pub immediate_goal: String,
    /// Hidden agenda (not revealed to players)
    pub secret_agenda: Option<String>,
    /// How they feel about the player characters
    pub attitude_to_players: String,
    /// Keywords or phrases they might use
    pub speech_patterns: Vec<String>,
}

impl NpcMotivation {
    pub fn new(mood: impl Into<String>, goal: impl Into<String>) -> Self {
        Self {
            current_mood: mood.into(),
            immediate_goal: goal.into(),
            secret_agenda: None,
            attitude_to_players: "Neutral".to_string(),
            speech_patterns: Vec::new(),
        }
    }

    // ── Builder Methods ──────────────────────────────────────────────────

    pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.secret_agenda = Some(secret.into());
        self
    }

    pub fn with_attitude(mut self, attitude: impl Into<String>) -> Self {
        self.attitude_to_players = attitude.into();
        self
    }

    pub fn with_speech_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.speech_patterns.push(pattern.into());
        self
    }
}
