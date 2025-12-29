//! Queue DTOs
//!
//! Types for queue operations including items, status, errors, and queue-specific payloads.
//!
//! This module contains both the generic queue infrastructure types and the specific
//! payload types for each queue (LLM, player action, asset generation, approval, etc.).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub type QueueItemId = Uuid;

// ============================================================================
// Generic Queue Infrastructure Types
// ============================================================================

/// Generic queue item with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem<T> {
    pub id: QueueItemId,
    pub payload: T,
    pub status: QueueItemStatus,
    pub priority: u8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl<T> QueueItem<T> {
    pub fn new(payload: T, priority: u8) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            payload,
            status: QueueItemStatus::Pending,
            priority,
            created_at: now,
            updated_at: now,
            scheduled_at: None,
            attempts: 0,
            max_attempts: 3,
            error_message: None,
            metadata: HashMap::new(),
        }
    }
}

/// Status of a queue item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueueItemStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Delayed,
    Expired,
}

impl QueueItemStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueueItemStatus::Pending => "pending",
            QueueItemStatus::Processing => "processing",
            QueueItemStatus::Completed => "completed",
            QueueItemStatus::Failed => "failed",
            QueueItemStatus::Delayed => "delayed",
            QueueItemStatus::Expired => "expired",
        }
    }
}

/// Errors that can occur during queue operations
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue item not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Queue is full")]
    QueueFull,

    #[error("Invalid operation for current status")]
    InvalidStatus,

    #[error("Max attempts exceeded")]
    MaxAttemptsExceeded,

    #[error("Database error: {0}")]
    Database(String),
}

// ============================================================================
// Queue Payload DTOs
// ============================================================================

/// Player action waiting to be processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerActionItem {
    pub world_id: Uuid,
    pub player_id: String,
    /// The player character ID performing this action (for challenge targeting)
    #[serde(default)]
    pub pc_id: Option<Uuid>,
    pub action_type: String,
    pub target: Option<String>,
    pub dialogue: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// DM action waiting to be processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DMActionItem {
    pub world_id: Uuid,
    pub dm_id: String,
    pub action: DMAction,
    pub timestamp: DateTime<Utc>,
}

/// DM action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DMAction {
    ApprovalDecision {
        request_id: String,
        decision: DmApprovalDecision,
    },
    DirectNPCControl {
        npc_id: String,
        dialogue: String,
    },
    TriggerEvent {
        event_id: String,
    },
    TransitionScene {
        scene_id: Uuid,
    },
}

/// LLM request waiting to be processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequestItem {
    pub request_type: LLMRequestType,
    /// World ID for routing responses (world-scoped events)
    pub world_id: Uuid,
    /// The player character ID associated with this request (for challenge targeting)
    #[serde(default)]
    pub pc_id: Option<Uuid>,
    #[serde(default)]
    pub prompt: Option<GamePromptRequest>,
    #[serde(default)]
    pub suggestion_context: Option<SuggestionContext>,
    pub callback_id: String,
}

/// LLM request type discriminator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LLMRequestType {
    NPCResponse { action_item_id: Uuid },
    Suggestion { field_type: String, entity_id: Option<String> },
}

/// Asset generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGenerationItem {
    pub world_id: Option<Uuid>,
    pub entity_type: String,
    pub entity_id: String,
    pub workflow_id: String,
    pub prompt: String,
    pub count: u32,
}

/// Decision awaiting DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalItem {
    pub world_id: Uuid,
    pub source_action_id: Uuid,
    pub decision_type: DecisionType,
    pub urgency: DecisionUrgency,
    /// Player character ID for SPOKE_TO edge creation
    #[serde(default)]
    pub pc_id: Option<Uuid>,
    /// NPC character ID for story event recording
    #[serde(default)]
    pub npc_id: Option<String>,
    pub npc_name: String,
    pub proposed_dialogue: String,
    pub internal_reasoning: String,
    pub proposed_tools: Vec<ProposedToolInfo>,
    pub retry_count: u32,
    /// Optional challenge suggestion from LLM
    #[serde(default)]
    pub challenge_suggestion: Option<ChallengeSuggestionInfo>,
    /// Optional narrative event suggestion from LLM
    #[serde(default)]
    pub narrative_event_suggestion: Option<NarrativeEventSuggestionInfo>,

    // Context for dialogue persistence
    /// Player's dialogue text (from the original action)
    #[serde(default)]
    pub player_dialogue: Option<String>,
    /// Scene ID where dialogue occurred (UUID string)
    #[serde(default)]
    pub scene_id: Option<String>,
    /// Location ID where dialogue occurred (UUID string)
    #[serde(default)]
    pub location_id: Option<String>,
    /// Game time when dialogue occurred (display string)
    #[serde(default)]
    pub game_time: Option<String>,
    /// Topics discussed in this dialogue (extracted by LLM)
    #[serde(default)]
    pub topics: Vec<String>,
}

/// Challenge outcome awaiting DM approval (P3.3)
///
/// After a player rolls, the outcome is queued here for DM review.
/// The DM can accept, edit, or request LLM suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeOutcomeApprovalItem {
    /// Unique ID for this resolution
    pub resolution_id: String,
    /// World where the challenge occurred
    pub world_id: Uuid,
    /// ID of the challenge
    pub challenge_id: String,
    /// Name of the challenge
    pub challenge_name: String,
    /// Description of the challenge (for LLM context)
    #[serde(default)]
    pub challenge_description: String,
    /// Name of the skill required for this challenge (for LLM context)
    #[serde(default)]
    pub skill_name: Option<String>,
    /// ID of the character who rolled
    pub character_id: String,
    /// Name of the character who rolled
    pub character_name: String,
    /// Raw die roll (before modifier)
    pub roll: i32,
    /// Character's skill modifier
    pub modifier: i32,
    /// Total result (roll + modifier)
    pub total: i32,
    /// Determined outcome type (e.g., "Success", "Critical Failure")
    pub outcome_type: String,
    /// The pre-defined outcome description
    pub outcome_description: String,
    /// Triggers that will execute when approved (for display in DM UI)
    pub outcome_triggers: Vec<ProposedToolInfo>,
    /// Original trigger DTOs (for execution - can convert to domain OutcomeTrigger)
    #[serde(default)]
    pub original_triggers: Vec<OutcomeTriggerRequestDto>,
    /// Roll breakdown string
    #[serde(default)]
    pub roll_breakdown: Option<String>,
    /// When the roll was submitted
    pub timestamp: DateTime<Utc>,
    /// LLM-generated suggestions (if requested)
    #[serde(default)]
    pub suggestions: Option<Vec<String>>,
    /// Whether LLM is currently generating suggestions
    #[serde(default)]
    pub is_generating_suggestions: bool,
}

// ============================================================================
// Decision Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionType {
    NPCResponse,
    ToolUsage,
    ChallengeSuggestion,
    SceneTransition,
    /// Challenge outcome pending DM approval (P3.3)
    ChallengeOutcome,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DecisionUrgency {
    Normal = 0,
    AwaitingPlayer = 1,
    SceneCritical = 2,
}

// ============================================================================
// Enhanced Challenge Suggestion Types
// ============================================================================

/// Enhanced challenge suggestion with detailed outcomes and tool receipts
///
/// This structure allows the LLM to suggest a skill challenge with
/// pre-defined outcomes for each result tier, including proposed tool calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedChallengeSuggestion {
    /// Optional reference to a predefined challenge (None for ad-hoc)
    pub challenge_id: Option<String>,
    /// Name of the challenge (e.g., "Persuasion Check", "Stealth Attempt")
    pub challenge_name: String,
    /// The skill being tested (e.g., "Persuasion", "Stealth", "Athletics")
    pub skill_name: String,
    /// Difficulty display (e.g., "DC 15", "Moderate", "70%")
    pub difficulty_display: String,
    /// What the NPC says before the challenge
    pub npc_reply: String,
    /// Detailed outcomes for each result tier
    pub outcomes: EnhancedOutcomes,
    /// Internal LLM reasoning (shown to DM only)
    pub reasoning: String,
}

/// Outcomes for each challenge result tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedOutcomes {
    /// Outcome for natural 20 or exceptional success (optional)
    #[serde(default)]
    pub critical_success: Option<OutcomeDetail>,
    /// Outcome for meeting or exceeding the DC
    pub success: OutcomeDetail,
    /// Outcome for failing to meet the DC
    pub failure: OutcomeDetail,
    /// Outcome for natural 1 or catastrophic failure (optional)
    #[serde(default)]
    pub critical_failure: Option<OutcomeDetail>,
}

/// Detailed outcome information including narrative and tool calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDetail {
    /// Narrative flavor text describing what happens
    pub flavor_text: String,
    /// Scene direction (what actions/changes occur)
    pub scene_direction: String,
    /// Tool calls that would be executed for this outcome
    #[serde(default)]
    pub proposed_tools: Vec<ProposedToolInfo>,
}

// ============================================================================
// Approval-related Types (needed by queue items)
// ============================================================================

/// Proposed tool call information
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

/// DM's decision on an approval request
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

// ============================================================================
// Outcome Trigger Types (needed by ChallengeOutcomeApprovalItem)
// ============================================================================

// Re-export from persistence module for queue payload compatibility
pub use crate::persistence::OutcomeTriggerRequestDto;

// ============================================================================
// Context Types (needed by LLMRequestItem)
// ============================================================================

// Re-export GamePromptRequest from domain for convenience
pub use wrldbldr_domain::value_objects::GamePromptRequest;

/// Context for generating suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionContext {
    /// Type of entity (e.g., "character", "location", "tavern", "forest")
    pub entity_type: Option<String>,
    /// Name of the entity (if already set)
    pub entity_name: Option<String>,
    /// World/setting name or type
    pub world_setting: Option<String>,
    /// Hints or keywords to guide generation
    pub hints: Option<String>,
    /// Additional context from other fields
    pub additional_context: Option<String>,
    /// World ID for per-world template resolution
    #[serde(default)]
    pub world_id: Option<String>,
}

impl Default for SuggestionContext {
    fn default() -> Self {
        Self {
            entity_type: None,
            entity_name: None,
            world_setting: Some("fantasy".to_string()),
            hints: None,
            additional_context: None,
            world_id: None,
        }
    }
}
