//! Shared type definitions
//!
//! Common types used across the protocol that don't fit in other modules.

use serde::{Deserialize, Serialize};

// =============================================================================
// Session & Participant Types
// =============================================================================

/// Role of a participant in a game session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantRole {
    DungeonMaster,
    Player,
    Spectator,
}

// =============================================================================
// Approval Types
// =============================================================================

/// Proposed tool call information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposedToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arguments: serde_json::Value,
}

/// DM's decision on an approval request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum ApprovalDecision {
    Accept,
    AcceptWithModification {
        modified_dialogue: String,
        approved_tools: Vec<String>,
        rejected_tools: Vec<String>,
    },
    Reject {
        feedback: String,
    },
    TakeOver {
        dm_response: String,
    },
}

// =============================================================================
// Suggestion Types
// =============================================================================

/// Challenge suggestion information for DM approval
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChallengeSuggestionInfo {
    pub challenge_id: String,
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty_display: String,
    pub confidence: String,
    pub reasoning: String,
    /// Target player character ID for skill modifier lookup
    #[serde(default)]
    pub target_pc_id: Option<String>,
    /// Optional editable outcomes for DM modification
    #[serde(default)]
    pub outcomes: Option<ChallengeSuggestionOutcomes>,
}

/// Editable challenge outcomes for DM modification
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ChallengeSuggestionOutcomes {
    #[serde(default)]
    pub success: Option<String>,
    #[serde(default)]
    pub failure: Option<String>,
    #[serde(default)]
    pub critical_success: Option<String>,
    #[serde(default)]
    pub critical_failure: Option<String>,
}

/// Narrative event suggestion information for DM approval
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrativeEventSuggestionInfo {
    pub event_id: String,
    pub event_name: String,
    pub description: String,
    pub scene_direction: String,
    pub confidence: String,
    pub reasoning: String,
    pub matched_triggers: Vec<String>,
    /// Suggested outcome (can be cleared/modified by DM)
    #[serde(default)]
    pub suggested_outcome: Option<String>,
}

// =============================================================================
// Character Archetypes
// =============================================================================

/// Campbell's character archetypes from the Hero's Journey
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampbellArchetype {
    Hero,
    Mentor,
    ThresholdGuardian,
    Herald,
    Shapeshifter,
    Shadow,
    Trickster,
    Ally,
}

impl Default for CampbellArchetype {
    fn default() -> Self {
        Self::Ally
    }
}

// =============================================================================
// Monomyth Stages
// =============================================================================

/// Monomyth (Hero's Journey) stages for acts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonomythStage {
    OrdinaryWorld,
    CallToAdventure,
    RefusalOfTheCall,
    MeetingTheMentor,
    CrossingTheThreshold,
    TestsAlliesEnemies,
    ApproachToTheInmostCave,
    Ordeal,
    Reward,
    TheRoadBack,
    Resurrection,
    ReturnWithTheElixir,
}

impl Default for MonomythStage {
    fn default() -> Self {
        Self::OrdinaryWorld
    }
}

// =============================================================================
// Game Time
// =============================================================================

/// Game time representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTime {
    /// Day number (currently ordinal-style, 1-based; calendar is planned)
    pub day: u32,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Whether time is paused
    pub is_paused: bool,
}

impl Default for GameTime {
    fn default() -> Self {
        Self {
            day: 1,
            hour: 8,
            minute: 0,
            is_paused: true,
        }
    }
}

impl GameTime {
    /// Create a new game time
    pub fn new(day: u32, hour: u8, minute: u8, is_paused: bool) -> Self {
        Self {
            day,
            hour,
            minute,
            is_paused,
        }
    }

    pub fn display_time(&self) -> String {
        let hour = self.hour;
        let minute = self.minute;

        let period = if hour >= 12 { "PM" } else { "AM" };
        let display_hour = if hour == 0 {
            12
        } else if hour > 12 {
            hour - 12
        } else {
            hour
        };

        format!("{}:{:02} {}", display_hour, minute, period)
    }

    pub fn display_date(&self) -> String {
        format!("Day {}, {}", self.day, self.display_time())
    }

    /// Get time of day category
    pub fn time_of_day(&self) -> TimeOfDay {
        match self.hour {
            5..=11 => TimeOfDay::Morning,
            12..=17 => TimeOfDay::Afternoon,
            18..=21 => TimeOfDay::Evening,
            _ => TimeOfDay::Night,
        }
    }
}

/// Time of day category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDay {
    Morning,
    Afternoon,
    Evening,
    Night,
}

impl TimeOfDay {
    pub fn display_name(&self) -> &'static str {
        match self {
            TimeOfDay::Morning => "Morning",
            TimeOfDay::Afternoon => "Afternoon",
            TimeOfDay::Evening => "Evening",
            TimeOfDay::Night => "Night",
        }
    }
}

impl std::fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
