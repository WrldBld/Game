//! DM Approval Queue Service Port - Interface for DM approval queue operations
//!
//! This port defines the interface for managing DM approval queue operations,
//! including enqueueing approval requests, retrieving pending items, and processing decisions.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(any(test, feature = "testing"))]
use mockall::automock;

use wrldbldr_domain::WorldId;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Approval request - what gets enqueued for DM review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// World where the approval is needed
    pub world_id: Uuid,
    /// ID of the source action that generated this approval
    pub source_action_id: Uuid,
    /// Type of decision required
    pub decision_type: ApprovalDecisionType,
    /// Urgency level
    pub urgency: ApprovalUrgency,
    /// Player character ID (for SPOKE_TO edge creation)
    #[serde(default)]
    pub pc_id: Option<Uuid>,
    /// NPC character ID
    #[serde(default)]
    pub npc_id: Option<String>,
    /// NPC name (for display)
    pub npc_name: String,
    /// Proposed dialogue text
    pub proposed_dialogue: String,
    /// Internal reasoning (shown to DM only)
    pub internal_reasoning: String,
    /// Proposed tool calls
    pub proposed_tools: Vec<ProposedToolInfo>,
    /// Number of times this has been rejected and retried
    pub retry_count: u32,
    /// Optional challenge suggestion
    #[serde(default)]
    pub challenge_suggestion: Option<ChallengeSuggestionInfo>,
    /// Optional narrative event suggestion
    #[serde(default)]
    pub narrative_event_suggestion: Option<NarrativeEventSuggestionInfo>,
    // Context for dialogue persistence
    /// Player's dialogue text
    #[serde(default)]
    pub player_dialogue: Option<String>,
    /// Scene ID where dialogue occurred
    #[serde(default)]
    pub scene_id: Option<String>,
    /// Location ID where dialogue occurred
    #[serde(default)]
    pub location_id: Option<String>,
    /// Game time when dialogue occurred
    #[serde(default)]
    pub game_time: Option<String>,
    /// Topics discussed
    #[serde(default)]
    pub topics: Vec<String>,
}

/// Type of decision required
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecisionType {
    NpcResponse,
    ToolUsage,
    ChallengeSuggestion,
    SceneTransition,
    ChallengeOutcome,
}

/// Urgency level for approval requests
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalUrgency {
    Normal = 0,
    AwaitingPlayer = 1,
    SceneCritical = 2,
}

/// Proposed tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arguments: serde_json::Value,
}

/// Challenge suggestion information
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
    #[serde(default)]
    pub suggested_outcome: Option<String>,
}

/// Approval queue item - wraps a request with queue metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalQueueItem {
    /// Unique item ID
    pub id: Uuid,
    /// The request payload
    pub payload: ApprovalRequest,
    /// Priority (based on urgency)
    pub priority: u8,
    /// When the item was enqueued
    pub enqueued_at: DateTime<Utc>,
    /// When the item was last updated
    pub updated_at: DateTime<Utc>,
}

/// DM's decision on an approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum ApprovalDecision {
    /// Accept the response as-is
    Accept,
    /// Accept with item distribution specified
    AcceptWithRecipients {
        /// Maps tool_id -> recipient PC IDs
        item_recipients: HashMap<String, Vec<String>>,
    },
    /// Accept with modifications
    AcceptWithModification {
        modified_dialogue: String,
        approved_tools: Vec<String>,
        rejected_tools: Vec<String>,
        #[serde(default)]
        item_recipients: HashMap<String, Vec<String>>,
    },
    /// Reject with feedback for regeneration
    Reject { feedback: String },
    /// DM takes over the response
    TakeOver { dm_response: String },
}

// ============================================================================
// Port Trait
// ============================================================================

/// Port for DM approval queue service operations
///
/// This trait defines the interface for managing the DM approval queue.
/// Approval requests are enqueued here for DM review and decision.
#[cfg_attr(any(test, feature = "testing"), automock)]
#[async_trait]
pub trait DmApprovalQueueServicePort: Send + Sync {
    /// Enqueue an approval request for DM review
    ///
    /// Returns the unique ID assigned to the queue item.
    async fn enqueue(&self, approval: ApprovalRequest) -> anyhow::Result<Uuid>;

    /// Dequeue the next item for processing
    ///
    /// Returns None if the queue is empty.
    async fn dequeue(&self) -> anyhow::Result<Option<ApprovalQueueItem>>;

    /// Process a DM decision on an approval item
    async fn complete(&self, id: Uuid, decision: ApprovalDecision) -> anyhow::Result<()>;

    /// Get all pending approvals for a world
    async fn get_pending(&self, world_id: WorldId) -> anyhow::Result<Vec<ApprovalQueueItem>>;

    /// Get an approval item by ID
    async fn get(&self, id: Uuid) -> anyhow::Result<Option<ApprovalQueueItem>>;

    /// Get approval history for a world
    async fn get_history(&self, world_id: WorldId, limit: usize)
        -> anyhow::Result<Vec<ApprovalQueueItem>>;

    /// Delay an approval for later
    async fn delay(&self, id: Uuid, until: DateTime<Utc>) -> anyhow::Result<()>;

    /// Discard a challenge suggestion from an approval
    async fn discard_challenge(&self, request_id: &str) -> anyhow::Result<()>;
}
