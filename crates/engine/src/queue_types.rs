// Queue types - some types prepared for future DM action queueing
#![allow(dead_code)]

//! Queue payload types used by QueuePort implementations.
//!
//! These DTOs are engine-owned to keep the domain pure.

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use wrldbldr_domain::{
    ApprovalRequestId, ChallengeId, CharacterId, ConversationId, LocationId, NarrativeEventId,
    PlayerCharacterId, RegionId, SceneId, WorldId,
};

use crate::game_tools::ToolArgs;
use crate::llm_context::GamePromptRequest;
use crate::prompt_templates::keys;

// =============================================================================
// Queue Type Enum
// =============================================================================

/// Centralized enum for queue type identifiers.
///
/// This enum eliminates stringly-typed queue types across the codebase,
/// ensuring consistent serialization/deserialization and preventing drift
/// when new queue types are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueType {
    /// Player action queue for pending player actions
    PlayerAction,
    /// LLM request queue for AI generation tasks
    LlmRequest,
    /// DM approval queue for pending DM decisions
    DmApproval,
    /// Asset generation queue for image/audio generation
    AssetGeneration,
}

impl QueueType {
    /// Get the string representation of this queue type.
    ///
    /// This is the canonical identifier used in the database.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PlayerAction => "player_action",
            Self::LlmRequest => "llm_request",
            Self::DmApproval => "dm_approval",
            Self::AssetGeneration => "asset_generation",
        }
    }

    /// Get all queue types as a slice.
    pub const fn all() -> &'static [QueueType] {
        &[
            QueueType::PlayerAction,
            QueueType::LlmRequest,
            QueueType::DmApproval,
            QueueType::AssetGeneration,
        ]
    }
}

impl fmt::Display for QueueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for QueueType {
    type Err = QueueTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "player_action" => Ok(Self::PlayerAction),
            "llm_request" => Ok(Self::LlmRequest),
            "dm_approval" => Ok(Self::DmApproval),
            "asset_generation" => Ok(Self::AssetGeneration),
            unknown => Err(QueueTypeParseError::UnknownQueueType(unknown.to_string())),
        }
    }
}

/// Error type for queue type parsing failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueTypeParseError {
    /// The queue type string is not recognized.
    UnknownQueueType(String),
}

impl fmt::Display for QueueTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownQueueType(s) => write!(f, "Unknown queue type: '{}'", s),
        }
    }
}

impl std::error::Error for QueueTypeParseError {}

// =============================================================================
// Multi-Phase LLM Pipeline Types
// =============================================================================

/// Phase identifiers for the multi-phase LLM pipeline.
///
/// Each phase represents a distinct reasoning step in NPC dialogue generation.
/// Phases are processed sequentially, with outputs from earlier phases feeding
/// into later phases. This improves determinism and enables DM oversight at
/// each decision point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmPhase {
    /// Phase 0: Context digest - deterministic summary of game state
    /// This phase is typically computed non-LLM (deterministic) but can be
    /// LLM-based if context exceeds budget.
    ContextDigest,
    /// Phase 1: Challenge trigger analysis
    /// Analyzes player action against active challenges to determine triggers.
    ChallengeTriggers,
    /// Phase 2: Narrative event trigger analysis
    /// Analyzes player action against active narrative events.
    EventTriggers,
    /// Phase 3: Tool intent detection
    /// Identifies what game tools (give_item, etc.) the NPC should use.
    ToolIntent,
    /// Phase 4: Behavior analysis
    /// Determines deflection behavior, emotional stance, mood shifts.
    BehaviorAnalysis,
    /// Phase 5: Final response generation
    /// Generates player-facing dialogue based on all prior phase outputs.
    FinalResponse,
}

impl LlmPhase {
    /// Get the phase number (0-5) for ordering.
    pub fn number(&self) -> u8 {
        match self {
            Self::ContextDigest => 0,
            Self::ChallengeTriggers => 1,
            Self::EventTriggers => 2,
            Self::ToolIntent => 3,
            Self::BehaviorAnalysis => 4,
            Self::FinalResponse => 5,
        }
    }

    /// Get the next phase in the sequence.
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::ContextDigest => Some(Self::ChallengeTriggers),
            Self::ChallengeTriggers => Some(Self::EventTriggers),
            Self::EventTriggers => Some(Self::ToolIntent),
            Self::ToolIntent => Some(Self::BehaviorAnalysis),
            Self::BehaviorAnalysis => Some(Self::FinalResponse),
            Self::FinalResponse => None,
        }
    }

    /// Get the prompt template key for this phase.
    pub fn prompt_template_key(&self) -> &'static str {
        match self {
            Self::ContextDigest => keys::DIALOGUE_PHASE0_CONTEXT_DIGEST,
            Self::ChallengeTriggers => keys::DIALOGUE_PHASE1_CHALLENGE_TRIGGERS,
            Self::EventTriggers => keys::DIALOGUE_PHASE2_EVENT_TRIGGERS,
            Self::ToolIntent => keys::DIALOGUE_PHASE3_TOOL_INTENT,
            Self::BehaviorAnalysis => keys::DIALOGUE_PHASE4_BEHAVIOR_ANALYSIS,
            Self::FinalResponse => keys::DIALOGUE_PHASE5_FINAL_RESPONSE,
        }
    }

    /// Get a short identifier for this phase (for logging, VCR fingerprints).
    pub fn short_id(&self) -> &'static str {
        match self {
            Self::ContextDigest => "ctx",
            Self::ChallengeTriggers => "chal",
            Self::EventTriggers => "evt",
            Self::ToolIntent => "tool",
            Self::BehaviorAnalysis => "behav",
            Self::FinalResponse => "resp",
        }
    }
}

/// Output from Phase 0: Context Digest.
///
/// A compact, deterministic summary of game state for downstream phases.
/// This reduces token usage and stabilizes VCR fingerprints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Phase0ContextDigest {
    /// Canonical summary of the scene (location, time, present characters)
    pub scene_summary: String,
    /// Digest of the player action (normalized dialogue)
    pub player_action_digest: String,
    /// Summary of responding NPC (name, archetype, disposition)
    pub npc_summary: String,
    /// Count of active challenges (full details fetched separately if needed)
    pub active_challenge_count: usize,
    /// Count of active narrative events
    pub active_event_count: usize,
    /// Key conversation topics from recent history
    pub recent_topics: Vec<String>,
    /// Directorial notes excerpt (if any)
    pub directorial_excerpt: Option<String>,
}

/// Output from Phase 1: Challenge Trigger Analysis.
///
/// Structured trigger decisions for each active challenge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Phase1ChallengeTriggers {
    /// Trigger analysis for each challenge
    pub challenges: Vec<PhaseChallengeAnalysis>,
}

/// Per-challenge analysis from Phase 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseChallengeAnalysis {
    /// Challenge ID (aliased or full UUID)
    pub challenge_id: String,
    /// Challenge name for display
    pub challenge_name: String,
    /// Exact quote from player dialogue that triggered (if any)
    pub trigger_quote: Option<String>,
    /// Whether this challenge should trigger
    pub should_trigger: bool,
    /// Confidence level (high/medium/low)
    pub confidence: String,
    /// Brief reasoning for the decision
    pub reasoning: String,
}

/// Output from Phase 2: Event Trigger Analysis.
///
/// Structured trigger decisions for each active narrative event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Phase2EventTriggers {
    /// Trigger analysis for each event
    pub events: Vec<PhaseEventAnalysis>,
    /// The event that was selected to trigger (at most one)
    pub selected_event: Option<String>,
}

/// Per-event analysis from Phase 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseEventAnalysis {
    /// Event ID (aliased or full UUID)
    pub event_id: String,
    /// Event name for display
    pub event_name: String,
    /// Exact quote from player dialogue that triggered (if any)
    pub trigger_quote: Option<String>,
    /// Whether this event should trigger
    pub should_trigger: bool,
    /// Confidence level (high/medium/low)
    pub confidence: String,
    /// Brief reasoning for the decision
    pub reasoning: String,
}

/// Output from Phase 3: Tool Intent.
///
/// Proposed game tools based on player action and context.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Phase3ToolIntent {
    /// Proposed tools for DM approval
    pub proposed_tools: Vec<ProposedTool>,
    /// Whether any tools were proposed
    pub has_tools: bool,
    /// LLM signal for whether tools should be proposed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should_act: Option<bool>,
    /// Reasoning for tool intent decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

/// Output from Phase 4: Behavior Analysis.
///
/// NPC behavioral stance including deflection and emotional state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Phase4BehaviorAnalysis {
    /// Whether the NPC should deflect (hide secret motivations)
    pub should_deflect: bool,
    /// Deflection behavior description (if deflecting)
    pub deflection_behavior: Option<String>,
    /// Recommended mood for the response
    pub recommended_mood: Option<String>,
    /// Recommended disposition change (if any)
    pub disposition_change: Option<DispositionChange>,
    /// Emotional stance toward player
    pub emotional_stance: String,
    /// Whether the NPC is being deceptive
    pub is_deceptive: bool,
}

/// Recommended disposition change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispositionChange {
    /// New disposition level
    pub new_disposition: String,
    /// Reason for the change
    pub reason: String,
    /// Magnitude of change (slight/moderate/significant)
    pub magnitude: String,
}

/// Output from Phase 5: Final Response.
///
/// The player-facing dialogue and metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Phase5FinalResponse {
    /// NPC dialogue to display to player
    pub dialogue: String,
    /// Internal reasoning (shown to DM)
    pub reasoning: String,
    /// Topics discussed in this exchange
    pub topics: Vec<String>,
    /// Optional narrative suggestions for DM
    pub suggested_beats: Vec<String>,
}

/// Container for all phase outputs.
///
/// This is stored with the LLM request and passed between phases.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseOutputs {
    /// Phase 0 output (context digest)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase0_context: Option<Phase0ContextDigest>,
    /// Phase 1 output (challenge triggers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase1_challenges: Option<Phase1ChallengeTriggers>,
    /// Phase 2 output (event triggers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase2_events: Option<Phase2EventTriggers>,
    /// Phase 3 output (tool intent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase3_tools: Option<Phase3ToolIntent>,
    /// Phase 4 output (behavior analysis)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase4_behavior: Option<Phase4BehaviorAnalysis>,
    /// Phase 5 output (final response)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase5_response: Option<Phase5FinalResponse>,
}

/// Request data for a phased LLM request.
///
/// This extends LlmRequestData with phase-specific metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhasedLlmRequestData {
    /// The current phase being processed
    pub current_phase: LlmPhase,
    /// Stable key for this phase execution (for VCR fingerprinting)
    pub phase_key: String,
    /// Accumulated outputs from prior phases
    pub phase_outputs: PhaseOutputs,
    /// The original request data
    pub base_request: LlmRequestData,
    /// Version for cassette compatibility
    pub phase_version: String,
}

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
    pub conversation_id: Option<ConversationId>,
}

// =============================================================================
// DM Action Data
// =============================================================================

/// Types of actions a DM can perform.
///
/// DMs have special privileges to approve/reject NPC responses,
/// directly control NPCs, trigger events, and manage scene transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DmActionType {
    /// Decision on a pending approval request
    ApprovalDecision {
        /// ID of the approval request being decided on
        request_id: ApprovalRequestId,
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
        event_id: NarrativeEventId,
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
pub enum DmApprovalDecision {
    /// Accept the proposed response/action as-is
    Accept,
    /// Accept with specific item distribution
    AcceptWithRecipients {
        /// Map of item IDs to recipient character IDs
        item_recipients: HashMap<String, Vec<PlayerCharacterId>>,
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
        item_recipients: HashMap<String, Vec<PlayerCharacterId>>,
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
        action_item_id: wrldbldr_domain::QueueItemId,
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
        resolution_id: wrldbldr_domain::ApprovalRequestId,
        /// World ID for broadcasting result
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
    /// Callback ID for routing response
    pub callback_id: String,
    /// Conversation ID for dialogue tracking (flows through to ApprovalRequestData)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<wrldbldr_domain::ConversationId>,
    /// Temperature for LLM request (persisted at enqueue time to avoid drift)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Multi-phase pipeline metadata (optional - enables phased processing)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_meta: Option<PhaseMetadata>,
}

/// Metadata for multi-phase LLM pipeline processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseMetadata {
    /// Current phase being processed
    pub current_phase: LlmPhase,
    /// Stable key for this phase execution (for VCR fingerprinting)
    pub phase_key: String,
    /// Accumulated outputs from prior phases (serialized JSON)
    pub phase_outputs_json: Option<String>,
    /// Phase version for cassette compatibility
    pub phase_version: String,
    /// Prompt template version used for this phase
    pub template_version: String,
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
    /// Tool ID (UUID for tracking)
    pub id: String,
    /// Tool name for display
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool arguments as typed enum
    pub arguments: ToolArgs,
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
    pub challenge_id: ChallengeId,
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
    pub event_id: NarrativeEventId,
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
    pub resolution_id: wrldbldr_domain::ApprovalRequestId,
    pub world_id: WorldId,
    pub challenge_id: ChallengeId,
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
    pub source_action_id: wrldbldr_domain::QueueItemId,
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
    pub region_id: Option<RegionId>,
    pub game_time: Option<String>,
    pub topics: Vec<String>,
    pub conversation_id: Option<wrldbldr_domain::ConversationId>,
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
