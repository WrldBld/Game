//! Queue data value objects - pure domain representations of queue items
//!
//! These types represent the business data for queue operations.
//! Note: Serde derives are included to support queue storage backends.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CharacterId, LocationId, PlayerCharacterId, SceneId, WorldId};

use super::GamePromptRequest;

// =============================================================================
// Player Action Data
// =============================================================================

/// Player action waiting to be processed.
///
/// Represents an action submitted by a player that needs to be processed
/// by the game engine (e.g., dialogue, item usage, movement).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
}

/// LLM request data.
///
/// Contains all information needed to make an LLM request,
/// including the prompt context and callback information.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
}

/// Context for LLM suggestion requests.
///
/// Provides contextual information to help the LLM generate
/// appropriate suggestions for entity fields.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
///
/// When an NPC proposes to use a game tool (give item, trigger challenge, etc.),
/// this captures the tool details for DM review.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProposedTool {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool being called
    pub name: String,
    /// Human-readable description of what the tool does
    pub description: String,
    /// Arguments for the tool call (dynamic JSON structure)
    pub arguments: serde_json::Value,
}

/// Challenge suggestion from LLM analysis.
///
/// When the LLM detects a player action that might trigger a challenge,
/// it provides this suggestion for DM review.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChallengeSuggestion {
    /// ID of the challenge being suggested
    pub challenge_id: String,
    /// Display name of the challenge
    pub challenge_name: String,
    /// Skill being tested
    pub skill_name: String,
    /// Human-readable difficulty (e.g., "DC 15", "Hard")
    pub difficulty_display: String,
    /// LLM's confidence in this suggestion
    pub confidence: String,
    /// LLM's reasoning for suggesting this challenge
    pub reasoning: String,
    /// Target PC for the challenge (if specific)
    pub target_pc_id: Option<PlayerCharacterId>,
    /// Suggested narrative outcomes
    pub outcomes: Option<ChallengeSuggestionOutcomes>,
}

/// Suggested narrative outcomes for a challenge.
///
/// Provides optional narrative text for each outcome type
/// that the DM can use or modify.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChallengeSuggestionOutcomes {
    /// Narrative for success
    pub success: Option<String>,
    /// Narrative for failure
    pub failure: Option<String>,
    /// Narrative for critical success
    pub critical_success: Option<String>,
    /// Narrative for critical failure
    pub critical_failure: Option<String>,
}

/// Narrative event suggestion from LLM analysis.
///
/// When the LLM detects conditions that might trigger a narrative event,
/// it provides this suggestion for DM review.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NarrativeEventSuggestion {
    /// ID of the event being suggested
    pub event_id: String,
    /// Display name of the event
    pub event_name: String,
    /// Description of the event
    pub description: String,
    /// Scene direction for the DM
    pub scene_direction: String,
    /// LLM's confidence in this suggestion
    pub confidence: String,
    /// LLM's reasoning for suggesting this event
    pub reasoning: String,
    /// Trigger conditions that matched
    pub matched_triggers: Vec<String>,
    /// Suggested outcome if event is triggered
    pub suggested_outcome: Option<String>,
}

/// Type of decision being requested from the DM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// Urgency level for approval requests.
///
/// Higher urgency items should be prioritized by the DM
/// to avoid blocking game flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ApprovalUrgency {
    /// Normal priority - can wait
    Normal,
    /// Player is waiting for a response
    AwaitingPlayer,
    /// Critical scene moment - immediate attention needed
    SceneCritical,
}

/// Approval request data.
///
/// Comprehensive data for an item awaiting DM approval,
/// including all context needed for the DM to make a decision.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApprovalRequestData {
    /// World this request belongs to
    pub world_id: WorldId,
    /// ID of the source action that triggered this request
    pub source_action_id: Uuid,
    /// Type of decision being requested
    pub decision_type: ApprovalDecisionType,
    /// How urgent is this decision
    pub urgency: ApprovalUrgency,
    /// Player character involved (if applicable)
    pub pc_id: Option<PlayerCharacterId>,
    /// NPC involved in the request
    pub npc_id: Option<CharacterId>,
    /// Name of the NPC (for display)
    pub npc_name: String,
    /// The proposed dialogue from the NPC
    pub proposed_dialogue: String,
    /// LLM's internal reasoning (for DM context)
    pub internal_reasoning: String,
    /// Tools the NPC proposes to use
    pub proposed_tools: Vec<ProposedTool>,
    /// Number of times this has been regenerated
    pub retry_count: u32,
    /// Challenge suggestion if applicable
    pub challenge_suggestion: Option<ChallengeSuggestion>,
    /// Narrative event suggestion if applicable
    pub narrative_event_suggestion: Option<NarrativeEventSuggestion>,
    /// Player dialogue that triggered this (for recording)
    pub player_dialogue: Option<String>,
    /// Current scene ID (for recording)
    pub scene_id: Option<SceneId>,
    /// Current location ID (for recording)
    pub location_id: Option<LocationId>,
    /// Current game time as display string (for recording)
    pub game_time: Option<String>,
    /// Conversation topics discussed
    pub topics: Vec<String>,
}

// =============================================================================
// Challenge Outcome Data
// =============================================================================

/// Challenge outcome data for DM approval.
///
/// When a challenge is resolved (dice rolled, outcome determined),
/// this captures all the details for DM review before finalizing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChallengeOutcomeData {
    /// Unique ID for this resolution attempt
    pub resolution_id: String,
    /// World this occurred in
    pub world_id: WorldId,
    /// Challenge that was attempted
    pub challenge_id: String,
    /// Display name of the challenge
    pub challenge_name: String,
    /// Description of the challenge
    pub challenge_description: String,
    /// Skill used (if skill-based)
    pub skill_name: Option<String>,
    /// Character who attempted the challenge
    pub character_id: CharacterId,
    /// Name of the character (for display)
    pub character_name: String,
    /// Base roll value
    pub roll: i32,
    /// Modifier applied to roll
    pub modifier: i32,
    /// Final total (roll + modifier)
    pub total: i32,
    /// Type of outcome: "success", "failure", "critical_success", "critical_failure"
    pub outcome_type: String,
    /// Narrative description of the outcome
    pub outcome_description: String,
    /// Triggers that fire as a result
    pub outcome_triggers: Vec<ProposedTool>,
    /// Detailed breakdown of the roll calculation
    pub roll_breakdown: Option<String>,
    /// When the challenge was resolved
    pub timestamp: DateTime<Utc>,
    /// AI-generated narrative suggestions
    pub suggestions: Option<Vec<String>>,
    /// Whether AI is currently generating suggestions
    pub is_generating_suggestions: bool,
}

// =============================================================================
// Asset Generation Data
// =============================================================================

/// Asset generation request data.
///
/// Request to generate visual assets (images, portraits, etc.)
/// for game entities using ComfyUI workflows.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetGenerationData {
    /// World context (if world-specific)
    pub world_id: Option<WorldId>,
    /// Type of entity: "character", "location", "item", etc.
    pub entity_type: String,
    /// ID of the entity to generate assets for
    pub entity_id: String,
    /// ComfyUI workflow to use
    pub workflow_id: String,
    /// Prompt for image generation
    pub prompt: String,
    /// Number of images to generate
    pub count: u32,
}
