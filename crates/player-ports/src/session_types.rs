//! Session-related DTOs for GameConnectionPort
//!
//! These types are owned by the ports layer and define the contract
//! between the application layer and the adapters layer. Conversion
//! to/from protocol types happens in the adapters layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Role of a participant in a game session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantRole {
    Player,
    DungeonMaster,
    Spectator,
}

/// Type of dice input for challenge rolls
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiceInput {
    /// Dice formula (e.g., "1d20+5")
    Formula(String),
    /// Manual entry with result value
    Manual(i32),
}

/// DM's approval decision
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum ApprovalDecision {
    /// Accept all proposed tools with default recipients
    Accept,
    /// Accept with item recipient selection
    AcceptWithRecipients {
        /// For give_item tools: maps tool_id -> recipient PC IDs
        /// Empty list means "don't give this item"
        item_recipients: HashMap<String, Vec<String>>,
    },
    /// Accept with modifications to dialogue and/or tool selection
    AcceptWithModification {
        modified_dialogue: String,
        approved_tools: Vec<String>,
        rejected_tools: Vec<String>,
        /// For give_item tools: maps tool_id -> recipient PC IDs
        /// Empty list means "don't give this item"
        #[serde(default)]
        item_recipients: HashMap<String, Vec<String>>,
    },
    /// Reject the proposal
    Reject {
        feedback: String,
    },
    /// DM takes over the response
    TakeOver {
        dm_response: String,
    },
}

/// Directorial context for scene guidance
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DirectorialContext {
    pub scene_notes: String,
    pub tone: String,
    pub npc_motivations: Vec<NpcMotivationData>,
    pub forbidden_topics: Vec<String>,
}

/// NPC motivation data for directorial context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcMotivationData {
    pub character_id: String,
    /// Free-form emotional guidance for the NPC (e.g., "Conflicted about revealing secrets")
    pub emotional_guidance: String,
    pub immediate_goal: String,
    pub secret_agenda: Option<String>,
}

/// Approved NPC info for staging
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovedNpcInfo {
    pub character_id: String,
    pub is_present: bool,
    /// Optional override reasoning (if DM modified)
    #[serde(default)]
    pub reasoning: Option<String>,
    /// When true, NPC is present but hidden from players
    #[serde(default)]
    pub is_hidden_from_players: bool,
}

/// Ad-hoc challenge outcomes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdHocOutcomes {
    pub success: String,
    pub failure: String,
    #[serde(default)]
    pub critical_success: Option<String>,
    #[serde(default)]
    pub critical_failure: Option<String>,
}

/// Challenge outcome decision from DM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ChallengeOutcomeDecision {
    /// Accept the outcome as-is
    Accept,
    /// Edit the outcome description
    Edit { modified_description: String },
    /// Request LLM to suggest alternatives
    Suggest {
        #[serde(default)]
        guidance: Option<String>,
    },
}
