//! Shared type definitions
//!
//! Common types used across the protocol that don't fit in other modules.

use serde::{Deserialize, Serialize};

// Re-export domain types that have serde derives
pub use wrldbldr_domain::entities::MonomythStage;
pub use wrldbldr_domain::value_objects::CampbellArchetype;

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
    /// Accept all proposed tools with default recipients
    Accept,
    /// Accept with item recipient selection
    AcceptWithRecipients {
        /// For give_item tools: maps tool_id -> recipient PC IDs
        /// Empty list means "don't give this item"
        item_recipients: std::collections::HashMap<String, Vec<String>>,
    },
    /// Accept with modifications to dialogue and/or tool selection
    AcceptWithModification {
        modified_dialogue: String,
        approved_tools: Vec<String>,
        rejected_tools: Vec<String>,
        /// For give_item tools: maps tool_id -> recipient PC IDs
        /// Empty list means "don't give this item"
        #[serde(default)]
        item_recipients: std::collections::HashMap<String, Vec<String>>,
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
// Character Archetypes - Re-exported from domain (see top of file)
// =============================================================================

// =============================================================================
// Monomyth Stages - Re-exported from domain (see top of file)
// =============================================================================

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

    /// Convert from domain GameTime to protocol GameTime for wire transfer.
    ///
    /// Domain GameTime uses `chrono::DateTime<Utc>` internally for rich date/time
    /// manipulation, while protocol GameTime uses simple numeric fields for
    /// efficient JSON serialization over the wire.
    pub fn from_domain(game_time: &wrldbldr_domain::GameTime) -> Self {
        use chrono::Timelike;
        let current = game_time.current();
        Self {
            day: game_time.day_ordinal(),
            hour: current.hour() as u8,
            minute: current.minute() as u8,
            is_paused: game_time.is_paused(),
        }
    }
}
