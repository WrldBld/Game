// Queue types - some types prepared for future DM action queueing
#![allow(dead_code)]

//! Queue payload types used by QueuePort implementations.
//!
//! These DTOs are engine-owned to keep the domain pure.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, SceneId, WorldId};

use crate::llm_context::GamePromptRequest;

// =============================================================================
// Player Action Data
// =============================================================================

/// Player action waiting to be processed.
///
/// Represents an action submitted by a player that needs to be processed
/// by the game engine (e.g., dialogue, item usage, movement).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerActionData {
    /// World this action belongs to
    pub world_id: WorldId,
    /// Player who submitted the action
    pub player_id: String,
    /// Player character performing the action (if applicable)
    pub pc_id: Option<PlayerCharacterId>,
    /// Type of action: "speak", "examine", "use_item", etc.
    pub action_type: String,
    /// Target of the action (NPC name, object, etc.)
    pub target: Option<String>,
    /// Dialogue content if the action is speech
    pub dialogue: Option<String>,
    /// When the action was submitted
    pub timestamp: DateTime<Utc>,
    /// Conversation ID for dialogue actions (links to Conversation node in Neo4j)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<Uuid>,
}

// =============================================================================
// DM Action Data
// =============================================================================

/// Types of actions a DM can perform.
///
/// DMs have special privileges to approve/reject NPC responses,
/// directly control NPCs, trigger events, and manage scene transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DmActionType {
    /// Decision on a pending approval request
    ApprovalDecision {
        /// ID of the approval request being decided on
        request_id: String,
        /// The DM's decision
        decision: DmApprovalDecision,
    },
    /// Direct control of an NPC's dialogue
    DirectNpcControl {
        /// NPC being controlled
        npc_id: CharacterId,
        /// Dialogue for the NPC to speak
        dialogue: String,
    },
    /// Manually trigger a narrative event
    TriggerEvent {
        /// Event to trigger
        event_id: String,
    },
    /// Force a scene transition
    TransitionScene {
        /// Scene to transition to
        scene_id: SceneId,
    },
}

/// DM action data.
///
/// Represents an action submitted by the DM for processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmActionData {
    /// World this action belongs to
    pub world_id: WorldId,
    /// DM who submitted the action
    pub dm_id: String,
    /// The specific action being taken
    pub action: DmActionType,
    /// When the action was submitted
    pub timestamp: DateTime<Utc>,
}

/// DM's decision on an approval request.
///
/// When an NPC response or tool usage requires DM approval, the DM
/// can accept, modify, reject, or take over the response entirely.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DmApprovalDecision {
    /// Accept the proposed response/action as-is
    Accept,
    /// Accept with specific item distribution
    AcceptWithRecipients {
        /// Map of item IDs to recipient character IDs
        item_recipients: HashMap<String, Vec<String>>,
    },
    /// Accept with modifications to dialogue and/or tools
    AcceptWithModification {
        /// Modified version of the proposed dialogue
        modified_dialogue: String,
        /// Tools that were approved
        approved_tools: Vec<String>,
        /// Tools that were rejected
        rejected_tools: Vec<String>,
        /// Map of item IDs to recipient character IDs
        item_recipients: HashMap<String, Vec<String>>,
    },
    /// Reject the proposed response with feedback for regeneration
    Reject {
        /// Feedback explaining why it was rejected
        feedback: String,
    },
    /// DM takes over and provides their own response
    TakeOver {
        /// The DM's replacement response
        dm_response: String,
    },
}

// =============================================================================
// LLM Request Data
// =============================================================================

/// Types of LLM requests.
///
/// The engine makes different types of requests to LLMs depending
/// on what content needs to be generated.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LlmRequestType {
    /// Generate an NPC's response to player action
    NpcResponse {
        /// Reference to the action item being responded to
        action_item_id: Uuid,
    },
    /// Generate a suggestion for a field value
    Suggestion {
        /// Type of field needing a suggestion
        field_type: String,
        /// Entity the suggestion is for (if applicable)
        entity_id: Option<String>,
    },
    /// Generate alternative outcome descriptions for a challenge
    OutcomeSuggestion {
        /// The approval queue ID (resolution_id) for this challenge outcome
        resolution_id: Uuid,
        /// World ID for broadcasting the result
        world_id: WorldId,
        /// Challenge name for context
        challenge_name: String,
        /// Current outcome description to improve upon
        current_description: String,
        /// DM's guidance for the suggestions
        guidance: Option<String>,
    },
}

/// LLM request data.
///
/// Contains all information needed to make an LLM request,
/// including the prompt context and callback information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequestData {
    /// Type of request being made
    pub request_type: LlmRequestType,
    /// World context for the request
    pub world_id: WorldId,
    /// Player character context (if applicable)
    pub pc_id: Option<PlayerCharacterId>,
    /// Full prompt request for NPC responses
    pub prompt: Option<GamePromptRequest>,
    /// Context for suggestion requests
    pub suggestion_context: Option<SuggestionContext>,
    /// Callback ID for routing the response
    pub callback_id: String,
    /// Conversation ID for dialogue tracking (flows through to ApprovalRequestData)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<uuid::Uuid>,
}

/// Context for LLM suggestion requests.
///
/// Provides contextual information to help the LLM generate
/// appropriate suggestions for entity fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SuggestionContext {
    /// Type of entity being edited
    pub entity_type: Option<String>,
    /// Name of the entity
    pub entity_name: Option<String>,
    /// World setting/theme for context
    pub world_setting: Option<String>,
    /// Specific hints for the suggestion
    pub hints: Option<String>,
    /// Any additional context information
    pub additional_context: Option<String>,
    /// World ID for template resolution
    pub world_id: Option<WorldId>,
}

// =============================================================================
// Approval Request Data
// =============================================================================

/// Information about a proposed tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedTool {
    /// Tool name
    pub id: String,
    /// Tool name for display
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool arguments as raw JSON
    pub arguments: serde_json::Value,
}

/// Proposed challenge outcomes from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSuggestionOutcomes {
    pub success: Option<String>,
    pub failure: Option<String>,
    pub critical_success: Option<String>,
    pub critical_failure: Option<String>,
}

/// LLM-suggested challenge data for the DM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSuggestion {
    pub challenge_id: String,
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty_display: String,
    pub confidence: String,
    pub reasoning: String,
    pub target_pc_id: Option<PlayerCharacterId>,
    pub outcomes: Option<ChallengeSuggestionOutcomes>,
}

/// LLM-suggested narrative event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEventSuggestion {
    pub event_id: String,
    pub event_name: String,
    pub description: String,
    pub scene_direction: String,
    pub confidence: String,
    pub reasoning: String,
    pub matched_triggers: Vec<String>,
    pub suggested_outcome: Option<String>,
}

/// Urgency level for approval requests.
///
/// Higher urgency items should be prioritized by the DM
/// to avoid blocking game flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ApprovalUrgency {
    /// Normal priority - can wait
    Normal,
    /// Player is waiting for a response
    AwaitingPlayer,
    /// Critical scene moment - immediate attention needed
    SceneCritical,
}

/// Type of decision being requested from the DM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalDecisionType {
    /// Approval of an NPC's proposed response
    NpcResponse,
    /// Approval of tool usage (give item, etc.)
    ToolUsage,
    /// Approval of a suggested challenge
    ChallengeSuggestion,
    /// Approval of a scene transition
    SceneTransition,
    /// Approval of a challenge outcome
    ChallengeOutcome,
}

/// LLM-proposed outcomes for a resolved challenge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeOutcomeData {
    pub resolution_id: String,
    pub world_id: WorldId,
    pub challenge_id: String,
    pub challenge_name: String,
    pub challenge_description: String,
    pub skill_name: Option<String>,
    pub character_id: CharacterId,
    pub character_name: String,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
    pub outcome_type: String,
    pub outcome_description: String,
    pub outcome_triggers: Vec<ProposedTool>,
    pub roll_breakdown: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub suggestions: Option<Vec<String>>,
    pub is_generating_suggestions: bool,
}

/// DM approval request data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequestData {
    pub world_id: WorldId,
    pub source_action_id: Uuid,
    pub decision_type: ApprovalDecisionType,
    pub urgency: ApprovalUrgency,
    pub pc_id: Option<PlayerCharacterId>,
    pub npc_id: Option<CharacterId>,
    pub npc_name: String,
    pub proposed_dialogue: String,
    pub internal_reasoning: String,
    pub proposed_tools: Vec<ProposedTool>,
    pub retry_count: u32,
    pub challenge_suggestion: Option<ChallengeSuggestion>,
    pub narrative_event_suggestion: Option<NarrativeEventSuggestion>,
    pub challenge_outcome: Option<ChallengeOutcomeData>,
    pub player_dialogue: Option<String>,
    pub scene_id: Option<SceneId>,
    pub location_id: Option<LocationId>,
    pub game_time: Option<String>,
    pub topics: Vec<String>,
    pub conversation_id: Option<Uuid>,
}

// =============================================================================
// Asset Generation Queue Data
// =============================================================================

/// Asset generation request data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGenerationData {
    /// World context for the generation
    pub world_id: Option<WorldId>,
    /// Entity type for the asset
    pub entity_type: String,
    /// Entity ID for the asset
    pub entity_id: String,
    /// Workflow configuration ID
    pub workflow_id: String,
    /// Prompt to send to ComfyUI
    pub prompt: String,
    /// Number of assets to generate
    pub count: u32,
}
