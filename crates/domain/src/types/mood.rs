//! Mood types for the three-tier emotional model
//!
//! This module defines MoodState - an NPC's current emotional state (Tier 2).
//! This is distinct from:
//! - DispositionLevel (Tier 1): How an NPC feels about a specific PC (per NPC-PC pair)
//! - Expression (Tier 3): Transient visual state during dialogue (sprite changes)
//!
//! MoodState is:
//! - Per NPC (not per-PC)
//! - Semi-persistent (set during staging, cached until next staging)
//! - Affects default expression and dialogue tone
//! - Included in LLM context for richer responses

use crate::error::DomainError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// NPC's current emotional state (Tier 2 of the three-tier emotional model)
///
/// This represents how an NPC is feeling emotionally, independent of any
/// specific PC relationship. It affects their default expression, dialogue
/// tone, and is included in LLM context.
///
/// # Examples
/// - "Marcus is anxious because the town is under threat"
/// - "Elara is curious about the ancient ruins"
/// - "The guard is alert due to recent robberies"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MoodState {
    /// Feeling good, positive outlook
    Happy,
    /// Peaceful, at ease (default state)
    #[default]
    Calm,
    /// Worried, uneasy, stressed
    Anxious,
    /// Enthusiastic, energized
    Excited,
    /// Sad, wistful, reflective
    Melancholic,
    /// Annoyed, frustrated
    Irritated,
    /// Watchful, on guard
    Alert,
    /// Uninterested, lacking motivation
    Bored,
    /// Scared, apprehensive
    Fearful,
    /// Optimistic about the future
    Hopeful,
    /// Interested, inquisitive
    Curious,
    /// Deep in thought, pensive
    Contemplative,
    /// Finding something funny or entertaining
    Amused,
    /// Tired, exhausted
    Weary,
    /// Self-assured, certain
    Confident,
    /// Uneasy, jittery
    Nervous,
    /// Unknown mood (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl MoodState {
    /// Get all mood states for UI dropdowns (excludes Unknown)
    pub fn all() -> &'static [MoodState] {
        &[
            MoodState::Happy,
            MoodState::Calm,
            MoodState::Anxious,
            MoodState::Excited,
            MoodState::Melancholic,
            MoodState::Irritated,
            MoodState::Alert,
            MoodState::Bored,
            MoodState::Fearful,
            MoodState::Hopeful,
            MoodState::Curious,
            MoodState::Contemplative,
            MoodState::Amused,
            MoodState::Weary,
            MoodState::Confident,
            MoodState::Nervous,
        ]
    }

    /// Get a display name for the mood
    pub fn display_name(&self) -> &'static str {
        match self {
            MoodState::Happy => "Happy",
            MoodState::Calm => "Calm",
            MoodState::Anxious => "Anxious",
            MoodState::Excited => "Excited",
            MoodState::Melancholic => "Melancholic",
            MoodState::Irritated => "Irritated",
            MoodState::Alert => "Alert",
            MoodState::Bored => "Bored",
            MoodState::Fearful => "Fearful",
            MoodState::Hopeful => "Hopeful",
            MoodState::Curious => "Curious",
            MoodState::Contemplative => "Contemplative",
            MoodState::Amused => "Amused",
            MoodState::Weary => "Weary",
            MoodState::Confident => "Confident",
            MoodState::Nervous => "Nervous",
            MoodState::Unknown => "Unknown",
        }
    }

    /// Get an emoji representation
    pub fn emoji(&self) -> &'static str {
        match self {
            MoodState::Happy => "😊",
            MoodState::Calm => "😌",
            MoodState::Anxious => "😰",
            MoodState::Excited => "🤩",
            MoodState::Melancholic => "😢",
            MoodState::Irritated => "😤",
            MoodState::Alert => "👀",
            MoodState::Bored => "😑",
            MoodState::Fearful => "😨",
            MoodState::Hopeful => "🙂",
            MoodState::Curious => "🤔",
            MoodState::Contemplative => "🧐",
            MoodState::Amused => "😄",
            MoodState::Weary => "😩",
            MoodState::Confident => "😎",
            MoodState::Nervous => "😬",
            MoodState::Unknown => "❓",
        }
    }

    /// Get suggested default expression for this mood
    ///
    /// This is used when no explicit expression marker is provided.
    /// The LLM can override this with explicit markers.
    pub fn default_expression(&self) -> &'static str {
        match self {
            MoodState::Happy => "happy",
            MoodState::Calm => "neutral",
            MoodState::Anxious => "afraid",
            MoodState::Excited => "happy",
            MoodState::Melancholic => "sad",
            MoodState::Irritated => "angry",
            MoodState::Alert => "suspicious",
            MoodState::Bored => "neutral",
            MoodState::Fearful => "afraid",
            MoodState::Hopeful => "happy",
            MoodState::Curious => "thoughtful",
            MoodState::Contemplative => "thoughtful",
            MoodState::Amused => "happy",
            MoodState::Weary => "sad",
            MoodState::Confident => "neutral",
            MoodState::Nervous => "afraid",
            MoodState::Unknown => "neutral",
        }
    }

    /// Get a brief description for LLM context
    pub fn description(&self) -> &'static str {
        match self {
            MoodState::Happy => "feeling good and positive",
            MoodState::Calm => "at peace and relaxed",
            MoodState::Anxious => "worried and uneasy",
            MoodState::Excited => "enthusiastic and energized",
            MoodState::Melancholic => "sad and reflective",
            MoodState::Irritated => "annoyed and frustrated",
            MoodState::Alert => "watchful and on guard",
            MoodState::Bored => "uninterested and lacking motivation",
            MoodState::Fearful => "scared and apprehensive",
            MoodState::Hopeful => "optimistic about the future",
            MoodState::Curious => "interested and inquisitive",
            MoodState::Contemplative => "deep in thought",
            MoodState::Amused => "finding things entertaining",
            MoodState::Weary => "tired and exhausted",
            MoodState::Confident => "self-assured and certain",
            MoodState::Nervous => "uneasy and jittery",
            MoodState::Unknown => "in an unclear emotional state",
        }
    }
}

impl fmt::Display for MoodState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name().to_lowercase())
    }
}

impl FromStr for MoodState {
    type Err = DomainError;

    /// Parses a string into a MoodState.
    ///
    /// Unlike serde deserialization (which falls back to `Unknown` for unknown values
    /// via `#[serde(other)]`), this returns an error for unrecognized inputs.
    ///
    /// **Rationale**: `FromStr` is typically used for internal/validated sources
    /// (e.g., database values) where unknown values indicate data corruption or a bug.
    /// Failing fast surfaces these issues immediately. Serde's fallback handles
    /// forward compatibility for external JSON payloads from updated clients.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "happy" => Ok(MoodState::Happy),
            "calm" => Ok(MoodState::Calm),
            "anxious" => Ok(MoodState::Anxious),
            "excited" => Ok(MoodState::Excited),
            "melancholic" => Ok(MoodState::Melancholic),
            "irritated" => Ok(MoodState::Irritated),
            "alert" => Ok(MoodState::Alert),
            "bored" => Ok(MoodState::Bored),
            "fearful" => Ok(MoodState::Fearful),
            "hopeful" => Ok(MoodState::Hopeful),
            "curious" => Ok(MoodState::Curious),
            "contemplative" => Ok(MoodState::Contemplative),
            "amused" => Ok(MoodState::Amused),
            "weary" => Ok(MoodState::Weary),
            "confident" => Ok(MoodState::Confident),
            "nervous" => Ok(MoodState::Nervous),
            "unknown" => Ok(MoodState::Unknown),
            _ => Err(DomainError::parse(format!(
                "Unknown mood state: '{}'. Valid values: happy, calm, anxious, excited, \
                melancholic, irritated, alert, bored, fearful, hopeful, curious, \
                contemplative, amused, weary, confident, nervous, unknown",
                s
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mood_default() {
        assert_eq!(MoodState::default(), MoodState::Calm);
    }

    #[test]
    fn test_mood_parse() {
        assert_eq!("happy".parse::<MoodState>().unwrap(), MoodState::Happy);
        assert_eq!("ANXIOUS".parse::<MoodState>().unwrap(), MoodState::Anxious);
        assert_eq!("unknown".parse::<MoodState>().unwrap(), MoodState::Unknown);
        assert!("unknown_value".parse::<MoodState>().is_err());
    }

    #[test]
    fn test_mood_display() {
        assert_eq!(MoodState::Happy.to_string(), "happy");
        assert_eq!(MoodState::Contemplative.to_string(), "contemplative");
    }

    #[test]
    fn test_mood_all_excludes_unknown() {
        let all = MoodState::all();
        assert!(!all.contains(&MoodState::Unknown));
        assert_eq!(all.len(), 16);
    }

    #[test]
    fn test_default_expression_mapping() {
        assert_eq!(MoodState::Happy.default_expression(), "happy");
        assert_eq!(MoodState::Anxious.default_expression(), "afraid");
        assert_eq!(MoodState::Calm.default_expression(), "neutral");
    }
}
