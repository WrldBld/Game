//! Approval-related DTOs for engine-app layer
//!
//! These types mirror wrldbldr_protocol approval types but are owned
//! by the application layer. Mapping to/from protocol types happens
//! in the adapters layer.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Proposed tool call information (app-layer version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arguments: serde_json::Value,
}

/// Challenge suggestion information for DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSuggestionInfo {
    pub challenge_id: String,
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty_display: String,
    pub confidence: String,
    pub reasoning: String,
    #[serde(default)]
    pub target_pc_id: Option<String>,
    #[serde(default)]
    pub outcomes: Option<ChallengeSuggestionOutcomes>,
}

/// Challenge suggestion outcomes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

/// Narrative event suggestion information
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// DM's decision on an approval request (for DM approval queue)
///
/// Note: This is different from `use_cases::scene::ApprovalDecision` which
/// is a simpler type for scene-specific approvals. Both are valid app-layer
/// types for their respective use cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum DmApprovalDecision {
    /// Accept as-is
    Accept,

    /// Accept with item distribution
    AcceptWithRecipients {
        /// For give_item tools: maps tool_id -> recipient PC IDs
        /// Empty list means "don't give this item"
        item_recipients: HashMap<String, Vec<String>>,
    },

    /// Accept with modifications
    AcceptWithModification {
        modified_dialogue: String,
        approved_tools: Vec<String>,
        rejected_tools: Vec<String>,
        /// For give_item tools: maps tool_id -> recipient PC IDs
        /// Empty list means "don't give this item"
        #[serde(default)]
        item_recipients: HashMap<String, Vec<String>>,
    },

    /// Reject with feedback
    Reject {
        feedback: String,
    },

    /// DM takes over response
    TakeOver {
        dm_response: String,
    },
}
