//! WebSocket message types for Engine-Player communication
//!
//! This module contains all message types exchanged over the WebSocket connection.
//! These types are used by both Engine (sending ServerMessage, receiving ClientMessage)
//! and Player (sending ClientMessage, receiving ServerMessage).

use serde::{Deserialize, Serialize};

use crate::types::{
    ApprovalDecision, ChallengeSuggestionInfo, NarrativeEventSuggestionInfo, ParticipantRole,
    ProposedToolInfo,
};

// =============================================================================
// Client Messages (Player → Engine)
// =============================================================================

/// Messages from client (Player) to server (Engine)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Join a game session
    JoinSession {
        user_id: String,
        role: ParticipantRole,
        /// Optional world ID to join (creates demo session if not provided)
        #[serde(default)]
        world_id: Option<String>,
    },
    /// Player performs an action
    PlayerAction {
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    },
    /// Request to change scene
    RequestSceneChange { scene_id: String },
    /// DM updates directorial context
    DirectorialUpdate { context: DirectorialContext },
    /// DM approves/rejects LLM response
    ApprovalDecision {
        request_id: String,
        decision: ApprovalDecision,
    },
    /// Player submits a challenge roll (legacy - accepts raw roll value)
    ChallengeRoll {
        challenge_id: String,
        roll: i32,
    },
    /// Player submits a challenge roll with dice input (formula or manual)
    ChallengeRollInput {
        challenge_id: String,
        /// Dice input - either "formula" with dice string, or "manual" with result
        input_type: DiceInputType,
    },
    /// DM triggers a challenge manually
    TriggerChallenge {
        challenge_id: String,
        target_character_id: String,
    },
    /// DM approves/rejects/modifies a suggested challenge
    ChallengeSuggestionDecision {
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    },
    /// DM approves/rejects a suggested narrative event trigger
    NarrativeEventSuggestionDecision {
        request_id: String,
        event_id: String,
        approved: bool,
        /// Optional selected outcome if DM pre-selects an outcome
        selected_outcome: Option<String>,
    },
    /// Heartbeat ping
    Heartbeat,

    /// DM requests regeneration of challenge outcome(s)
    RegenerateOutcome {
        request_id: String,
        outcome_type: Option<String>,
        guidance: Option<String>,
    },

    /// DM discards a challenge suggestion
    DiscardChallenge {
        request_id: String,
        feedback: Option<String>,
    },

    /// DM creates an ad-hoc challenge (no LLM involved)
    CreateAdHocChallenge {
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: String,
        outcomes: AdHocOutcomes,
    },

    /// DM approves/edits/requests suggestion for challenge outcome
    ChallengeOutcomeDecision {
        resolution_id: String,
        decision: ChallengeOutcomeDecisionData,
    },

    /// DM requests LLM to suggest alternative outcome descriptions
    RequestOutcomeSuggestion {
        resolution_id: String,
        guidance: Option<String>,
    },

    /// DM requests LLM to generate outcome branches
    RequestOutcomeBranches {
        resolution_id: String,
        guidance: Option<String>,
    },

    /// DM selects an outcome branch
    SelectOutcomeBranch {
        resolution_id: String,
        branch_id: String,
        modified_description: Option<String>,
    },

    /// DM shares NPC location with player (creates HeardAbout observation)
    ShareNpcLocation {
        pc_id: String,
        npc_id: String,
        location_id: String,
        region_id: String,
        notes: Option<String>,
    },

    /// Player selects a PC to play
    SelectPlayerCharacter { pc_id: String },

    /// Player moves to a different region within the same location
    MoveToRegion {
        pc_id: String,
        region_id: String,
    },

    /// Player exits to a different location
    ExitToLocation {
        pc_id: String,
        location_id: String,
        arrival_region_id: Option<String>,
    },

    /// DM triggers an NPC approach event (NPC approaches a player)
    TriggerApproachEvent {
        npc_id: String,
        target_pc_id: String,
        description: String,
    },

    /// DM triggers a location event (narration for all PCs in a region)
    TriggerLocationEvent {
        region_id: String,
        description: String,
    },

    /// DM advances the in-game time
    AdvanceGameTime { hours: u32 },
}

// =============================================================================
// Server Messages (Engine → Player)
// =============================================================================

/// Messages from server (Engine) to client (Player)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Session successfully joined with full details
    SessionJoined {
        session_id: String,
        role: ParticipantRole,
        participants: Vec<ParticipantInfo>,
        world_snapshot: serde_json::Value,
    },
    /// A player joined the session (broadcast to others)
    PlayerJoined {
        user_id: String,
        role: ParticipantRole,
        character_name: Option<String>,
    },
    /// A player left the session (broadcast to others)
    PlayerLeft { user_id: String },
    /// Player action was received and is being processed
    ActionReceived {
        action_id: String,
        player_id: String,
        action_type: String,
    },
    /// Scene update
    SceneUpdate {
        scene: SceneData,
        characters: Vec<CharacterData>,
        interactions: Vec<InteractionData>,
    },
    /// NPC dialogue response
    DialogueResponse {
        speaker_id: String,
        speaker_name: String,
        text: String,
        choices: Vec<DialogueChoice>,
    },
    /// LLM is processing (shown to DM)
    LLMProcessing { action_id: String },
    /// Action queued for processing
    ActionQueued {
        action_id: String,
        player_name: String,
        action_type: String,
        queue_depth: usize,
    },
    /// Queue status update (sent to DM)
    QueueStatus {
        player_actions_pending: usize,
        llm_requests_pending: usize,
        llm_requests_processing: usize,
        approvals_pending: usize,
    },
    /// Approval required (sent to DM)
    ApprovalRequired {
        request_id: String,
        npc_name: String,
        proposed_dialogue: String,
        internal_reasoning: String,
        proposed_tools: Vec<ProposedToolInfo>,
        challenge_suggestion: Option<ChallengeSuggestionInfo>,
        narrative_event_suggestion: Option<NarrativeEventSuggestionInfo>,
    },
    /// Response was approved and executed
    ResponseApproved {
        npc_dialogue: String,
        executed_tools: Vec<String>,
    },
    /// Challenge prompt sent to player
    ChallengePrompt {
        challenge_id: String,
        challenge_name: String,
        skill_name: String,
        difficulty_display: String,
        description: String,
        character_modifier: i32,
        #[serde(default)]
        suggested_dice: Option<String>,
        #[serde(default)]
        rule_system_hint: Option<String>,
    },
    /// Challenge result broadcast to all
    ChallengeResolved {
        challenge_id: String,
        challenge_name: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome: String,
        outcome_description: String,
        #[serde(default)]
        roll_breakdown: Option<String>,
        #[serde(default)]
        individual_rolls: Option<Vec<i32>>,
    },
    /// Narrative event has been triggered
    NarrativeEventTriggered {
        event_id: String,
        event_name: String,
        outcome_description: String,
        scene_direction: String,
    },
    /// Party is split across multiple locations (sent to DM)
    SplitPartyNotification {
        location_count: usize,
        locations: Vec<SplitPartyLocation>,
    },
    /// Error message
    Error { code: String, message: String },
    /// Heartbeat response
    Pong,

    // Generation events (for Creator Mode)
    /// A generation batch has been queued
    GenerationQueued {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        position: u32,
    },
    /// Generation progress update
    GenerationProgress { batch_id: String, progress: u8 },
    /// Generation batch completed
    GenerationComplete {
        batch_id: String,
        asset_count: u32,
    },
    /// Generation batch failed
    GenerationFailed { batch_id: String, error: String },
    /// A suggestion request has been queued
    SuggestionQueued {
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
    },
    /// A suggestion request is being processed
    SuggestionProgress {
        request_id: String,
        status: String,
    },
    /// A suggestion request has completed
    SuggestionComplete {
        request_id: String,
        suggestions: Vec<String>,
    },
    /// A suggestion request has failed
    SuggestionFailed {
        request_id: String,
        error: String,
    },
    /// ComfyUI connection state changed
    ComfyUIStateChanged {
        state: String,
        message: Option<String>,
        retry_in_seconds: Option<u32>,
    },

    /// Outcome has been regenerated (sent to DM)
    OutcomeRegenerated {
        request_id: String,
        outcome_type: String,
        new_outcome: OutcomeDetailData,
    },

    /// Challenge was discarded (confirmation to DM)
    ChallengeDiscarded { request_id: String },

    /// Ad-hoc challenge created and sent to player
    AdHocChallengeCreated {
        challenge_id: String,
        challenge_name: String,
        target_pc_id: String,
    },

    /// Challenge roll submitted, awaiting DM approval (sent to rolling player)
    ChallengeRollSubmitted {
        challenge_id: String,
        challenge_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome_type: String,
        status: String,
    },

    /// Pending challenge outcome for DM approval queue
    ChallengeOutcomePending {
        resolution_id: String,
        challenge_id: String,
        challenge_name: String,
        character_id: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome_type: String,
        outcome_description: String,
        outcome_triggers: Vec<ProposedToolInfo>,
        #[serde(default)]
        roll_breakdown: Option<String>,
    },

    /// LLM-generated outcome suggestions are ready (sent to DM)
    OutcomeSuggestionReady {
        resolution_id: String,
        suggestions: Vec<String>,
    },

    /// LLM-generated outcome branches are ready for selection (sent to DM)
    OutcomeBranchesReady {
        resolution_id: String,
        outcome_type: String,
        branches: Vec<OutcomeBranchData>,
    },

    /// An NPC is approaching the player (sent to target PC)
    ApproachEvent {
        npc_id: String,
        npc_name: String,
        npc_sprite: Option<String>,
        description: String,
    },

    /// A location event occurred (sent to all PCs in region)
    LocationEvent {
        region_id: String,
        description: String,
    },

    /// NPC location was shared with the player (sent to target PC)
    NpcLocationShared {
        npc_id: String,
        npc_name: String,
        region_name: String,
        notes: Option<String>,
    },

    /// PC was selected for play
    PcSelected {
        pc_id: String,
        pc_name: String,
        location_id: String,
        region_id: Option<String>,
    },

    /// Scene changed due to PC movement
    SceneChanged {
        pc_id: String,
        region: RegionData,
        npcs_present: Vec<NpcPresenceData>,
        navigation: NavigationData,
    },

    /// Movement was blocked (locked door, etc.)
    MovementBlocked { pc_id: String, reason: String },

    /// Game time has been updated (broadcast to all)
    GameTimeUpdated {
        display: String,
        time_of_day: String,
        is_paused: bool,
    },
}

// =============================================================================
// Session Types
// =============================================================================

/// Information about a session participant
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub user_id: String,
    pub role: ParticipantRole,
    pub character_name: Option<String>,
}

/// Location information for split party notification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitPartyLocation {
    pub location_id: String,
    pub location_name: String,
    pub pc_count: usize,
    pub pc_names: Vec<String>,
}

/// Directorial context from DM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectorialContext {
    pub scene_notes: String,
    pub tone: String,
    pub npc_motivations: Vec<NpcMotivationData>,
    pub forbidden_topics: Vec<String>,
}

/// NPC motivation data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcMotivationData {
    pub character_id: String,
    pub mood: String,
    pub immediate_goal: String,
    pub secret_agenda: Option<String>,
}

// =============================================================================
// Scene Types
// =============================================================================

/// Scene data from server
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneData {
    pub id: String,
    pub name: String,
    pub location_id: String,
    pub location_name: String,
    pub backdrop_asset: Option<String>,
    pub time_context: String,
    pub directorial_notes: String,
}

/// Character data for display
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterData {
    pub id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub position: CharacterPosition,
    pub is_speaking: bool,
    /// Character's current emotional state (for visual novel display)
    /// Note: Engine may send None; Player can derive from context
    #[serde(default)]
    pub emotion: Option<String>,
}

/// Character position on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterPosition {
    Left,
    Center,
    Right,
    OffScreen,
}

impl CharacterPosition {
    /// Get Tailwind CSS classes for positioning
    pub fn as_tailwind_classes(&self) -> &'static str {
        match self {
            CharacterPosition::Left => "left-[10%]",
            CharacterPosition::Center => "left-1/2 -translate-x-1/2",
            CharacterPosition::Right => "right-[10%]",
            CharacterPosition::OffScreen => "hidden",
        }
    }
}

/// Available interaction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionData {
    pub id: String,
    pub name: String,
    pub interaction_type: String,
    pub target_name: Option<String>,
    pub is_available: bool,
}

/// Dialogue choice for player
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueChoice {
    pub id: String,
    pub text: String,
    pub is_custom_input: bool,
}

// =============================================================================
// Navigation Types
// =============================================================================

/// Region data for scene display
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionData {
    pub id: String,
    pub name: String,
    pub location_id: String,
    pub location_name: String,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
}

/// NPC presence data for scene display
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcPresenceData {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// Navigation options from current region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationData {
    pub connected_regions: Vec<NavigationTarget>,
    pub exits: Vec<NavigationExit>,
}

/// A navigation target (region within same location)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationTarget {
    pub region_id: String,
    pub name: String,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

/// An exit to another location
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationExit {
    pub location_id: String,
    pub location_name: String,
    pub arrival_region_id: String,
    pub description: Option<String>,
}

// =============================================================================
// Challenge Types
// =============================================================================

/// Dice input type for challenge rolls
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DiceInputType {
    /// Roll dice using a formula string like "1d20+5"
    Formula(String),
    /// Use a manual result (physical dice roll)
    Manual(i32),
}

/// Ad-hoc challenge outcomes for DM-created challenges
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdHocOutcomes {
    pub success: String,
    pub failure: String,
    #[serde(default)]
    pub critical_success: Option<String>,
    #[serde(default)]
    pub critical_failure: Option<String>,
}

/// Outcome detail data for regenerated outcomes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeDetailData {
    pub flavor_text: String,
    pub scene_direction: String,
    #[serde(default)]
    pub proposed_tools: Vec<ProposedToolInfo>,
}

/// DM's decision on a challenge outcome (wire format)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ChallengeOutcomeDecisionData {
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

/// Outcome branch data for DM selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeBranchData {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub effects: Vec<String>,
}
