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

// Re-export wire-format types from protocol (single source of truth)
pub use wrldbldr_protocol::{
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, NarrativeEventSuggestionInfo,
    ProposedToolInfo,
};

pub(crate) type QueueItemId = Uuid;

// ============================================================================
// Generic Queue Infrastructure Types
// ============================================================================

/// Generic queue item with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QueueItem<T> {
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
pub(crate) enum QueueItemStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Delayed,
    Expired,
    #[serde(other)]
    Unknown,
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
            QueueItemStatus::Unknown => "unknown",
        }
    }
}

// NOTE: QueueError is defined in wrldbldr_engine_ports::outbound::queue_port
// and should be imported from there. This avoids duplication and keeps
// error types with their port definitions.

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
    #[serde(other)]
    Unknown,
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
    NPCResponse {
        action_item_id: Uuid,
    },
    Suggestion {
        field_type: String,
        entity_id: Option<String>,
    },
    #[serde(other)]
    Unknown,
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
pub(crate) struct ApprovalItem {
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
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DecisionUrgency {
    Normal = 0,
    AwaitingPlayer = 1,
    SceneCritical = 2,
    #[serde(other)]
    Unknown = 3,
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
    pub critical_success: Option<EnhancedOutcomeDetail>,
    /// Outcome for meeting or exceeding the DC
    pub success: EnhancedOutcomeDetail,
    /// Outcome for failing to meet the DC
    pub failure: EnhancedOutcomeDetail,
    /// Outcome for natural 1 or catastrophic failure (optional)
    #[serde(default)]
    pub critical_failure: Option<EnhancedOutcomeDetail>,
}

/// Detailed outcome information including narrative and tool calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedOutcomeDetail {
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

// NOTE: ProposedToolInfo, ChallengeSuggestionInfo, ChallengeSuggestionOutcomes,
// and NarrativeEventSuggestionInfo are re-exported from wrldbldr_protocol
// at the top of this file. Protocol is the single source of truth.

// ARCHITECTURE EXCEPTION: [APPROVED 2026-01-02]
// Reason: DmApprovalDecision is re-exported from protocol::ApprovalDecision as the
// single source of truth for wire-format types. This avoids duplication and ensures
// serialization consistency. The Unknown variant with #[serde(other)] provides forward
// compatibility - conversion to domain types handles Unknown -> Reject mapping.
pub use wrldbldr_protocol::ApprovalDecision as DmApprovalDecision;

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

// ============================================================================
// Domain <-> DTO Conversions
// ============================================================================

use wrldbldr_domain::value_objects::{
    ApprovalDecisionType, ApprovalRequestData, ApprovalUrgency as DomainApprovalUrgency,
    AssetGenerationData, ChallengeOutcomeData, ChallengeSuggestion,
    ChallengeSuggestionOutcomes as DomainChallengeSuggestionOutcomes, DmActionData, DmActionType,
    DmApprovalDecision as DomainDmApprovalDecision, LlmRequestData,
    LlmRequestType as DomainLlmRequestType, NarrativeEventSuggestion, PlayerActionData,
    ProposedTool, SuggestionContext as DomainSuggestionContext,
};
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, SceneId, WorldId};

// ----------------------------------------------------------------------------
// PlayerActionItem <-> PlayerActionData
// ----------------------------------------------------------------------------

impl From<PlayerActionData> for PlayerActionItem {
    fn from(data: PlayerActionData) -> Self {
        Self {
            world_id: data.world_id.into(),
            player_id: data.player_id,
            pc_id: data.pc_id.map(Into::into),
            action_type: data.action_type,
            target: data.target,
            dialogue: data.dialogue,
            timestamp: data.timestamp,
        }
    }
}

impl From<PlayerActionItem> for PlayerActionData {
    fn from(dto: PlayerActionItem) -> Self {
        Self {
            world_id: WorldId::from(dto.world_id),
            player_id: dto.player_id,
            pc_id: dto.pc_id.map(PlayerCharacterId::from),
            action_type: dto.action_type,
            target: dto.target,
            dialogue: dto.dialogue,
            timestamp: dto.timestamp,
        }
    }
}

// ----------------------------------------------------------------------------
// DMActionItem <-> DmActionData
// ----------------------------------------------------------------------------

impl From<DmActionData> for DMActionItem {
    fn from(data: DmActionData) -> Self {
        Self {
            world_id: data.world_id.into(),
            dm_id: data.dm_id,
            action: data.action.into(),
            timestamp: data.timestamp,
        }
    }
}

impl From<DMActionItem> for DmActionData {
    fn from(dto: DMActionItem) -> Self {
        Self {
            world_id: WorldId::from(dto.world_id),
            dm_id: dto.dm_id,
            action: dto.action.into(),
            timestamp: dto.timestamp,
        }
    }
}

// ----------------------------------------------------------------------------
// DMAction <-> DmActionType
// ----------------------------------------------------------------------------

impl From<DmActionType> for DMAction {
    fn from(action: DmActionType) -> Self {
        match action {
            DmActionType::ApprovalDecision {
                request_id,
                decision,
            } => DMAction::ApprovalDecision {
                request_id,
                decision: domain_decision_to_wire(decision),
            },
            DmActionType::DirectNpcControl { npc_id, dialogue } => DMAction::DirectNPCControl {
                npc_id: npc_id.to_string(),
                dialogue,
            },
            DmActionType::TriggerEvent { event_id } => DMAction::TriggerEvent { event_id },
            DmActionType::TransitionScene { scene_id } => DMAction::TransitionScene {
                scene_id: scene_id.into(),
            },
        }
    }
}

impl From<DMAction> for DmActionType {
    fn from(dto: DMAction) -> Self {
        match dto {
            DMAction::ApprovalDecision {
                request_id,
                decision,
            } => DmActionType::ApprovalDecision {
                request_id,
                decision: wire_decision_to_domain(decision),
            },
            DMAction::DirectNPCControl { npc_id, dialogue } => DmActionType::DirectNpcControl {
                npc_id: CharacterId::from(
                    Uuid::parse_str(&npc_id).unwrap_or_else(|_| Uuid::new_v4()),
                ),
                dialogue,
            },
            DMAction::TriggerEvent { event_id } => DmActionType::TriggerEvent { event_id },
            DMAction::TransitionScene { scene_id } => DmActionType::TransitionScene {
                scene_id: SceneId::from(scene_id),
            },
            // Unknown variants are treated as no-op trigger events
            DMAction::Unknown => DmActionType::TriggerEvent {
                event_id: String::new(),
            },
        }
    }
}

// ----------------------------------------------------------------------------
// DmApprovalDecision (wire) <-> DmApprovalDecision (domain) conversion functions
// ----------------------------------------------------------------------------
// NOTE: These are standalone functions rather than From impls due to orphan rules.
// Both types are defined in external crates (protocol and domain), so we cannot
// implement From<A> for B in engine-dto.

/// Convert domain DmApprovalDecision to wire format (protocol::ApprovalDecision)
pub fn domain_decision_to_wire(decision: DomainDmApprovalDecision) -> DmApprovalDecision {
    match decision {
        DomainDmApprovalDecision::Accept => DmApprovalDecision::Accept,
        DomainDmApprovalDecision::AcceptWithRecipients { item_recipients } => {
            DmApprovalDecision::AcceptWithRecipients { item_recipients }
        }
        DomainDmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => DmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        },
        DomainDmApprovalDecision::Reject { feedback } => DmApprovalDecision::Reject { feedback },
        DomainDmApprovalDecision::TakeOver { dm_response } => {
            DmApprovalDecision::TakeOver { dm_response }
        }
    }
}

/// Convert wire format (protocol::ApprovalDecision) to domain DmApprovalDecision
///
/// Unknown variants are converted to Reject with explanatory feedback.
pub fn wire_decision_to_domain(dto: DmApprovalDecision) -> DomainDmApprovalDecision {
    match dto {
        DmApprovalDecision::Accept => DomainDmApprovalDecision::Accept,
        DmApprovalDecision::AcceptWithRecipients { item_recipients } => {
            DomainDmApprovalDecision::AcceptWithRecipients { item_recipients }
        }
        DmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => DomainDmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        },
        DmApprovalDecision::Reject { feedback } => DomainDmApprovalDecision::Reject { feedback },
        DmApprovalDecision::TakeOver { dm_response } => {
            DomainDmApprovalDecision::TakeOver { dm_response }
        }
        // Unknown decisions are rejected with feedback
        DmApprovalDecision::Unknown => DomainDmApprovalDecision::Reject {
            feedback: "Unknown decision type".to_string(),
        },
    }
}

// ----------------------------------------------------------------------------
// LLMRequestItem <-> LlmRequestData
// ----------------------------------------------------------------------------

impl From<LlmRequestData> for LLMRequestItem {
    fn from(data: LlmRequestData) -> Self {
        Self {
            request_type: data.request_type.into(),
            world_id: data.world_id.into(),
            pc_id: data.pc_id.map(Into::into),
            prompt: data.prompt,
            suggestion_context: data.suggestion_context.map(Into::into),
            callback_id: data.callback_id,
        }
    }
}

impl From<LLMRequestItem> for LlmRequestData {
    fn from(dto: LLMRequestItem) -> Self {
        Self {
            request_type: dto.request_type.into(),
            world_id: WorldId::from(dto.world_id),
            pc_id: dto.pc_id.map(PlayerCharacterId::from),
            prompt: dto.prompt,
            suggestion_context: dto.suggestion_context.map(Into::into),
            callback_id: dto.callback_id,
        }
    }
}

// ----------------------------------------------------------------------------
// LLMRequestType <-> LlmRequestType
// ----------------------------------------------------------------------------

impl From<DomainLlmRequestType> for LLMRequestType {
    fn from(request_type: DomainLlmRequestType) -> Self {
        match request_type {
            DomainLlmRequestType::NpcResponse { action_item_id } => {
                LLMRequestType::NPCResponse { action_item_id }
            }
            DomainLlmRequestType::Suggestion {
                field_type,
                entity_id,
            } => LLMRequestType::Suggestion {
                field_type,
                entity_id,
            },
        }
    }
}

impl From<LLMRequestType> for DomainLlmRequestType {
    fn from(dto: LLMRequestType) -> Self {
        match dto {
            LLMRequestType::NPCResponse { action_item_id } => {
                DomainLlmRequestType::NpcResponse { action_item_id }
            }
            LLMRequestType::Suggestion {
                field_type,
                entity_id,
            } => DomainLlmRequestType::Suggestion {
                field_type,
                entity_id,
            },
            // Unknown request types are treated as empty suggestions
            LLMRequestType::Unknown => DomainLlmRequestType::Suggestion {
                field_type: String::new(),
                entity_id: None,
            },
        }
    }
}

// ----------------------------------------------------------------------------
// SuggestionContext (DTO) <-> SuggestionContext (domain)
// ----------------------------------------------------------------------------

impl From<DomainSuggestionContext> for SuggestionContext {
    fn from(ctx: DomainSuggestionContext) -> Self {
        Self {
            entity_type: ctx.entity_type,
            entity_name: ctx.entity_name,
            world_setting: ctx.world_setting,
            hints: ctx.hints,
            additional_context: ctx.additional_context,
            world_id: ctx.world_id.map(|id| id.to_string()),
        }
    }
}

impl From<SuggestionContext> for DomainSuggestionContext {
    fn from(dto: SuggestionContext) -> Self {
        Self {
            entity_type: dto.entity_type,
            entity_name: dto.entity_name,
            world_setting: dto.world_setting,
            hints: dto.hints,
            additional_context: dto.additional_context,
            world_id: dto
                .world_id
                .and_then(|s| Uuid::parse_str(&s).ok())
                .map(WorldId::from),
        }
    }
}

// ----------------------------------------------------------------------------
// ApprovalItem <-> ApprovalRequestData
// ----------------------------------------------------------------------------

impl From<ApprovalRequestData> for ApprovalItem {
    fn from(data: ApprovalRequestData) -> Self {
        Self {
            world_id: data.world_id.into(),
            source_action_id: data.source_action_id,
            decision_type: data.decision_type.into(),
            urgency: data.urgency.into(),
            pc_id: data.pc_id.map(Into::into),
            npc_id: data.npc_id.map(|id| id.to_string()),
            npc_name: data.npc_name,
            proposed_dialogue: data.proposed_dialogue,
            internal_reasoning: data.internal_reasoning,
            proposed_tools: data
                .proposed_tools
                .into_iter()
                .map(proposed_tool_to_info)
                .collect(),
            retry_count: data.retry_count,
            challenge_suggestion: data.challenge_suggestion.map(challenge_suggestion_to_info),
            narrative_event_suggestion: data
                .narrative_event_suggestion
                .map(narrative_event_suggestion_to_info),
            player_dialogue: data.player_dialogue,
            scene_id: data.scene_id.map(|id| id.to_string()),
            location_id: data.location_id.map(|id| id.to_string()),
            game_time: data.game_time,
            topics: data.topics,
        }
    }
}

impl From<ApprovalItem> for ApprovalRequestData {
    fn from(dto: ApprovalItem) -> Self {
        Self {
            world_id: WorldId::from(dto.world_id),
            source_action_id: dto.source_action_id,
            decision_type: dto.decision_type.into(),
            urgency: dto.urgency.into(),
            pc_id: dto.pc_id.map(PlayerCharacterId::from),
            npc_id: dto
                .npc_id
                .and_then(|s| Uuid::parse_str(&s).ok())
                .map(CharacterId::from),
            npc_name: dto.npc_name,
            proposed_dialogue: dto.proposed_dialogue,
            internal_reasoning: dto.internal_reasoning,
            proposed_tools: dto
                .proposed_tools
                .into_iter()
                .map(info_to_proposed_tool)
                .collect(),
            retry_count: dto.retry_count,
            challenge_suggestion: dto.challenge_suggestion.map(info_to_challenge_suggestion),
            narrative_event_suggestion: dto
                .narrative_event_suggestion
                .map(info_to_narrative_event_suggestion),
            player_dialogue: dto.player_dialogue,
            scene_id: dto
                .scene_id
                .and_then(|s| Uuid::parse_str(&s).ok())
                .map(SceneId::from),
            location_id: dto
                .location_id
                .and_then(|s| Uuid::parse_str(&s).ok())
                .map(LocationId::from),
            game_time: dto.game_time,
            topics: dto.topics,
        }
    }
}

// ----------------------------------------------------------------------------
// DecisionType <-> ApprovalDecisionType
// ----------------------------------------------------------------------------

impl From<ApprovalDecisionType> for DecisionType {
    fn from(decision_type: ApprovalDecisionType) -> Self {
        match decision_type {
            ApprovalDecisionType::NpcResponse => DecisionType::NPCResponse,
            ApprovalDecisionType::ToolUsage => DecisionType::ToolUsage,
            ApprovalDecisionType::ChallengeSuggestion => DecisionType::ChallengeSuggestion,
            ApprovalDecisionType::SceneTransition => DecisionType::SceneTransition,
            ApprovalDecisionType::ChallengeOutcome => DecisionType::ChallengeOutcome,
        }
    }
}

impl From<DecisionType> for ApprovalDecisionType {
    fn from(dto: DecisionType) -> Self {
        match dto {
            DecisionType::NPCResponse => ApprovalDecisionType::NpcResponse,
            DecisionType::ToolUsage => ApprovalDecisionType::ToolUsage,
            DecisionType::ChallengeSuggestion => ApprovalDecisionType::ChallengeSuggestion,
            DecisionType::SceneTransition => ApprovalDecisionType::SceneTransition,
            DecisionType::ChallengeOutcome => ApprovalDecisionType::ChallengeOutcome,
            // Unknown decision types default to NpcResponse
            DecisionType::Unknown => ApprovalDecisionType::NpcResponse,
        }
    }
}

// ----------------------------------------------------------------------------
// DecisionUrgency <-> ApprovalUrgency
// ----------------------------------------------------------------------------

impl From<DomainApprovalUrgency> for DecisionUrgency {
    fn from(urgency: DomainApprovalUrgency) -> Self {
        match urgency {
            DomainApprovalUrgency::Normal => DecisionUrgency::Normal,
            DomainApprovalUrgency::AwaitingPlayer => DecisionUrgency::AwaitingPlayer,
            DomainApprovalUrgency::SceneCritical => DecisionUrgency::SceneCritical,
        }
    }
}

impl From<DecisionUrgency> for DomainApprovalUrgency {
    fn from(dto: DecisionUrgency) -> Self {
        match dto {
            DecisionUrgency::Normal => DomainApprovalUrgency::Normal,
            DecisionUrgency::AwaitingPlayer => DomainApprovalUrgency::AwaitingPlayer,
            DecisionUrgency::SceneCritical => DomainApprovalUrgency::SceneCritical,
            DecisionUrgency::Unknown => DomainApprovalUrgency::Normal,
        }
    }
}

// ----------------------------------------------------------------------------
// Standalone conversion functions (required due to orphan rule)
// These convert between domain types and protocol types
// ----------------------------------------------------------------------------

/// Convert domain ProposedTool to protocol ProposedToolInfo
pub fn proposed_tool_to_info(tool: ProposedTool) -> ProposedToolInfo {
    ProposedToolInfo {
        id: tool.id,
        name: tool.name,
        description: tool.description,
        arguments: tool.arguments,
    }
}

/// Convert protocol ProposedToolInfo to domain ProposedTool
pub fn info_to_proposed_tool(info: ProposedToolInfo) -> ProposedTool {
    ProposedTool {
        id: info.id,
        name: info.name,
        description: info.description,
        arguments: info.arguments,
    }
}

/// Convert domain ChallengeSuggestion to protocol ChallengeSuggestionInfo
pub fn challenge_suggestion_to_info(suggestion: ChallengeSuggestion) -> ChallengeSuggestionInfo {
    ChallengeSuggestionInfo {
        challenge_id: suggestion.challenge_id,
        challenge_name: suggestion.challenge_name,
        skill_name: suggestion.skill_name,
        difficulty_display: suggestion.difficulty_display,
        confidence: suggestion.confidence,
        reasoning: suggestion.reasoning,
        target_pc_id: suggestion.target_pc_id.map(|id| id.to_string()),
        outcomes: suggestion.outcomes.map(outcomes_to_info),
    }
}

/// Convert protocol ChallengeSuggestionInfo to domain ChallengeSuggestion
pub fn info_to_challenge_suggestion(info: ChallengeSuggestionInfo) -> ChallengeSuggestion {
    ChallengeSuggestion {
        challenge_id: info.challenge_id,
        challenge_name: info.challenge_name,
        skill_name: info.skill_name,
        difficulty_display: info.difficulty_display,
        confidence: info.confidence,
        reasoning: info.reasoning,
        target_pc_id: info
            .target_pc_id
            .and_then(|s| Uuid::parse_str(&s).ok())
            .map(PlayerCharacterId::from),
        outcomes: info.outcomes.map(info_to_outcomes),
    }
}

/// Convert domain ChallengeSuggestionOutcomes to protocol ChallengeSuggestionOutcomes
pub fn outcomes_to_info(
    outcomes: DomainChallengeSuggestionOutcomes,
) -> ChallengeSuggestionOutcomes {
    ChallengeSuggestionOutcomes {
        success: outcomes.success,
        failure: outcomes.failure,
        critical_success: outcomes.critical_success,
        critical_failure: outcomes.critical_failure,
    }
}

/// Convert protocol ChallengeSuggestionOutcomes to domain ChallengeSuggestionOutcomes
pub fn info_to_outcomes(info: ChallengeSuggestionOutcomes) -> DomainChallengeSuggestionOutcomes {
    DomainChallengeSuggestionOutcomes {
        success: info.success,
        failure: info.failure,
        critical_success: info.critical_success,
        critical_failure: info.critical_failure,
    }
}

/// Convert domain NarrativeEventSuggestion to protocol NarrativeEventSuggestionInfo
pub fn narrative_event_suggestion_to_info(
    suggestion: NarrativeEventSuggestion,
) -> NarrativeEventSuggestionInfo {
    NarrativeEventSuggestionInfo {
        event_id: suggestion.event_id,
        event_name: suggestion.event_name,
        description: suggestion.description,
        scene_direction: suggestion.scene_direction,
        confidence: suggestion.confidence,
        reasoning: suggestion.reasoning,
        matched_triggers: suggestion.matched_triggers,
        suggested_outcome: suggestion.suggested_outcome,
    }
}

/// Convert protocol NarrativeEventSuggestionInfo to domain NarrativeEventSuggestion
pub fn info_to_narrative_event_suggestion(
    info: NarrativeEventSuggestionInfo,
) -> NarrativeEventSuggestion {
    NarrativeEventSuggestion {
        event_id: info.event_id,
        event_name: info.event_name,
        description: info.description,
        scene_direction: info.scene_direction,
        confidence: info.confidence,
        reasoning: info.reasoning,
        matched_triggers: info.matched_triggers,
        suggested_outcome: info.suggested_outcome,
    }
}

// ----------------------------------------------------------------------------
// ChallengeOutcomeApprovalItem <-> ChallengeOutcomeData
// ----------------------------------------------------------------------------

impl From<ChallengeOutcomeData> for ChallengeOutcomeApprovalItem {
    fn from(data: ChallengeOutcomeData) -> Self {
        Self {
            resolution_id: data.resolution_id,
            world_id: data.world_id.into(),
            challenge_id: data.challenge_id,
            challenge_name: data.challenge_name,
            challenge_description: data.challenge_description,
            skill_name: data.skill_name,
            character_id: data.character_id.to_string(),
            character_name: data.character_name,
            roll: data.roll,
            modifier: data.modifier,
            total: data.total,
            outcome_type: data.outcome_type,
            outcome_description: data.outcome_description,
            outcome_triggers: data
                .outcome_triggers
                .into_iter()
                .map(proposed_tool_to_info)
                .collect(),
            original_triggers: Vec::new(), // Not available in domain type
            roll_breakdown: data.roll_breakdown,
            timestamp: data.timestamp,
            suggestions: data.suggestions,
            is_generating_suggestions: data.is_generating_suggestions,
        }
    }
}

impl From<ChallengeOutcomeApprovalItem> for ChallengeOutcomeData {
    fn from(dto: ChallengeOutcomeApprovalItem) -> Self {
        Self {
            resolution_id: dto.resolution_id,
            world_id: WorldId::from(dto.world_id),
            challenge_id: dto.challenge_id,
            challenge_name: dto.challenge_name,
            challenge_description: dto.challenge_description,
            skill_name: dto.skill_name,
            character_id: CharacterId::from(
                Uuid::parse_str(&dto.character_id).unwrap_or_else(|_| Uuid::new_v4()),
            ),
            character_name: dto.character_name,
            roll: dto.roll,
            modifier: dto.modifier,
            total: dto.total,
            outcome_type: dto.outcome_type,
            outcome_description: dto.outcome_description,
            outcome_triggers: dto
                .outcome_triggers
                .into_iter()
                .map(info_to_proposed_tool)
                .collect(),
            roll_breakdown: dto.roll_breakdown,
            timestamp: dto.timestamp,
            suggestions: dto.suggestions,
            is_generating_suggestions: dto.is_generating_suggestions,
        }
    }
}

// ----------------------------------------------------------------------------
// AssetGenerationItem <-> AssetGenerationData
// ----------------------------------------------------------------------------

impl From<AssetGenerationData> for AssetGenerationItem {
    fn from(data: AssetGenerationData) -> Self {
        Self {
            world_id: data.world_id.map(Into::into),
            entity_type: data.entity_type,
            entity_id: data.entity_id,
            workflow_id: data.workflow_id,
            prompt: data.prompt,
            count: data.count,
        }
    }
}

impl From<AssetGenerationItem> for AssetGenerationData {
    fn from(dto: AssetGenerationItem) -> Self {
        Self {
            world_id: dto.world_id.map(WorldId::from),
            entity_type: dto.entity_type,
            entity_id: dto.entity_id,
            workflow_id: dto.workflow_id,
            prompt: dto.prompt,
            count: dto.count,
        }
    }
}
