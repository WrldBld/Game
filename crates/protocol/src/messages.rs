//! WebSocket message types for Engine-Player communication
//!
//! This module contains all message types exchanged over the WebSocket connection.
//! These types are used by both Engine (sending ServerMessage, receiving ClientMessage)
//! and Player (sending ClientMessage, receiving ServerMessage).
//!
//! ## Versioning Policy
//!
//! - New variants can be added at the end (forward compatible)
//! - Removing variants requires major version bump
//! - Renaming variants is a breaking change
//! - Unknown enum variants deserialize to `Unknown` variant for forward compatibility

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::requests::RequestPayload;
use crate::responses::{ConnectedUser, EntityChangedData, JoinError, ResponseResult, WorldRole};
use crate::types::{
    ApprovalDecision, ChallengeSuggestionInfo, NarrativeEventSuggestionInfo, ParticipantRole,
    ProposedToolInfo,
};

fn default_true() -> bool {
    true
}

fn default_one() -> u32 {
    1
}

// =============================================================================
// Client Messages (Player → Engine)
// =============================================================================

/// Messages from client (Player) to server (Engine)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
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

    /// Request manual ComfyUI health check
    CheckComfyUIHealth,

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
        /// When false, player sees "Unknown Figure" and no sprite
        #[serde(default = "default_true")]
        reveal: bool,
    },


    /// DM triggers a location event (narration for all PCs in a region)
    TriggerLocationEvent {
        region_id: String,
        description: String,
    },

    // =========================================================================
    // Staging System (NPC Presence Approval)
    // =========================================================================

    /// DM approves/modifies staging for a region
    StagingApprovalResponse {
        request_id: String,
        /// Final list of NPCs with presence decisions
        approved_npcs: Vec<ApprovedNpcInfo>,
        /// TTL override (or use default)
        ttl_hours: i32,
        /// How this staging was finalized: "rule", "llm", "custom"
        source: String,
    },

    /// DM requests LLM to regenerate staging suggestions
    StagingRegenerateRequest {
        request_id: String,
        /// Guidance for LLM regeneration
        guidance: String,
    },

    /// DM pre-stages a region before player arrival
    PreStageRegion {
        region_id: String,
        /// NPCs to pre-stage
        npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
    },

    // =========================================================================
    // Inventory Actions
    // =========================================================================

    /// Player equips an item
    EquipItem {
        pc_id: String,
        item_id: String,
    },

    /// Player unequips an item
    UnequipItem {
        pc_id: String,
        item_id: String,
    },

    /// Player drops an item (destroys it for now; future: place in region)
    DropItem {
        pc_id: String,
        item_id: String,
        /// Number of items to drop (default 1)
        #[serde(default = "default_one")]
        quantity: u32,
    },

    /// Player picks up an item from their current region
    PickupItem {
        pc_id: String,
        item_id: String,
    },

    // =========================================================================
    // WebSocket-First Protocol (World-scoped connections)
    // =========================================================================

    /// Join a world (replaces JoinSession)
    JoinWorld {
        /// World to join
        world_id: Uuid,
        /// Role to join as
        role: WorldRole,
        /// Player character ID (required for Player role)
        #[serde(default)]
        pc_id: Option<Uuid>,
        /// Target PC to spectate (required for Spectator role)
        #[serde(default)]
        spectate_pc_id: Option<Uuid>,
    },

    /// Leave the current world
    LeaveWorld,

    /// Send a request (CRUD operations, actions)
    Request {
        /// Unique request ID for correlation
        request_id: String,
        /// Request payload
        payload: RequestPayload,
    },

    /// Set spectate target (for Spectator role)
    SetSpectateTarget {
        /// PC to spectate
        pc_id: Uuid,
    },

    /// Unknown message type for forward compatibility
    ///
    /// When deserializing an unknown variant, this variant is used instead of
    /// failing. Allows older clients to gracefully handle new message types.
    #[serde(other)]
    Unknown,
}

// =============================================================================
// Server Messages (Engine → Player)
// =============================================================================

/// Messages from server (Engine) to client (Player)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
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
        /// When false, player sees "Unknown Figure" and no sprite
        #[serde(default = "default_true")]
        reveal: bool,
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
        /// Items visible in this region (can be picked up)
        #[serde(default)]
        region_items: Vec<RegionItemData>,
    },

    /// Movement was blocked (locked door, etc.)
    MovementBlocked { pc_id: String, reason: String },

    /// Game time has been updated (broadcast to all)
    GameTimeUpdated { game_time: crate::types::GameTime },

    // =========================================================================
    // Staging System (NPC Presence Approval)
    // =========================================================================

    /// Staging approval required (sent to DM)
    /// Sent when a PC enters a region without valid staging
    StagingApprovalRequired {
        request_id: String,
        region_id: String,
        region_name: String,
        location_id: String,
        location_name: String,
        game_time: crate::types::GameTime,
        /// Previous staging if expired (for reference)
        #[serde(default)]
        previous_staging: Option<PreviousStagingInfo>,
        /// NPCs suggested by rule-based logic
        rule_based_npcs: Vec<StagedNpcInfo>,
        /// NPCs suggested by LLM (if enabled)
        llm_based_npcs: Vec<StagedNpcInfo>,
        /// Default TTL from location settings
        default_ttl_hours: i32,
        /// PCs waiting for this staging
        waiting_pcs: Vec<WaitingPcInfo>,
    },

    /// Staging is pending approval (sent to Player)
    /// Sent while waiting for DM to approve staging
    StagingPending {
        region_id: String,
        region_name: String,
    },

    /// Staging is ready (sent to Player)
    /// Sent when DM has approved staging and PC can see NPCs
    StagingReady {
        region_id: String,
        /// NPCs present in this staging
        npcs_present: Vec<NpcPresentInfo>,
    },

    /// Staging was regenerated by LLM (sent to DM)
    StagingRegenerated {
        request_id: String,
        /// Updated LLM-based NPC suggestions
        llm_based_npcs: Vec<StagedNpcInfo>,
    },

    // =========================================================================
    // Inventory Updates
    // =========================================================================

    /// Item was equipped (sent to player)
    ItemEquipped {
        pc_id: String,
        item_id: String,
        item_name: String,
    },

    /// Item was unequipped (sent to player)
    ItemUnequipped {
        pc_id: String,
        item_id: String,
        item_name: String,
    },

    /// Item was dropped/destroyed (sent to player)
    ItemDropped {
        pc_id: String,
        item_id: String,
        item_name: String,
        quantity: u32,
    },

    /// Item was picked up from region (sent to player)
    ItemPickedUp {
        pc_id: String,
        item_id: String,
        item_name: String,
    },

    /// Inventory was updated (signals client to refresh)
    InventoryUpdated { pc_id: String },

    // =========================================================================
    // Character Stat Updates
    // =========================================================================

    /// A character's stat was updated (broadcast to player and DM)
    CharacterStatUpdated {
        character_id: String,
        character_name: String,
        stat_name: String,
        old_value: i32,
        new_value: i32,
        delta: i32,
        /// Source of the change (e.g., "challenge_outcome", "tool_call", "dm_action")
        source: String,
    },

    // =========================================================================
    // NPC Disposition Updates (P1.4)
    // =========================================================================

    /// NPC disposition/relationship changed (sent to DM and optionally PC)
    NpcDispositionChanged {
        npc_id: String,
        npc_name: String,
        pc_id: String,
        disposition: String,
        relationship: String,
        #[serde(default)]
        reason: Option<String>,
    },

    /// All NPC dispositions for a PC (response to GetNpcDispositions)
    NpcDispositionsResponse {
        pc_id: String,
        dispositions: Vec<NpcDispositionData>,
    },

    // =========================================================================
    // Actantial Model / Motivations (P1.5)
    // =========================================================================

    /// NPC want was created (broadcast to session DMs)
    NpcWantCreated {
        npc_id: String,
        want: WantData,
    },

    /// NPC want was updated (broadcast to session DMs)
    NpcWantUpdated {
        npc_id: String,
        want: WantData,
    },

    /// NPC want was deleted (broadcast to session DMs)
    NpcWantDeleted {
        npc_id: String,
        want_id: String,
    },

    /// Want target was set
    WantTargetSet {
        want_id: String,
        target: WantTargetData,
    },

    /// Want target was removed
    WantTargetRemoved {
        want_id: String,
    },

    /// Actantial view was added
    ActantialViewAdded {
        npc_id: String,
        view: ActantialViewData,
    },

    /// Actantial view was removed
    ActantialViewRemoved {
        npc_id: String,
        want_id: String,
        target_id: String,
        role: ActantialRoleData,
    },

    /// Full actantial context for an NPC (response to GetNpcActantialContext)
    NpcActantialContextResponse {
        npc_id: String,
        context: NpcActantialContextData,
    },

    /// All goals for a world (response to GetWorldGoals)
    WorldGoalsResponse {
        world_id: String,
        goals: Vec<GoalData>,
    },

    /// Goal was created (broadcast to session)
    GoalCreated {
        world_id: String,
        goal: GoalData,
    },

    /// Goal was updated (broadcast to session)
    GoalUpdated {
        goal: GoalData,
    },

    /// Goal was deleted (broadcast to session)
    GoalDeleted {
        goal_id: String,
    },

    /// LLM suggestions for deflection behavior
    DeflectionSuggestions {
        npc_id: String,
        want_id: String,
        suggestions: Vec<String>,
    },

    /// LLM suggestions for behavioral tells
    TellsSuggestions {
        npc_id: String,
        want_id: String,
        suggestions: Vec<String>,
    },

    /// LLM suggestions for want description
    WantDescriptionSuggestions {
        npc_id: String,
        suggestions: Vec<String>,
    },

    /// LLM suggestions for actantial view reason
    ActantialReasonSuggestions {
        npc_id: String,
        want_id: String,
        target_id: String,
        role: ActantialRoleData,
        suggestions: Vec<String>,
    },

    // =========================================================================
    // WebSocket-First Protocol (World-scoped connections)
    // =========================================================================

    /// Successfully joined a world
    WorldJoined {
        /// World that was joined
        world_id: Uuid,
        /// Full world snapshot (for initial load)
        snapshot: serde_json::Value,
        /// Users currently connected to this world
        connected_users: Vec<ConnectedUser>,
        /// Your role in this world
        your_role: WorldRole,
        /// Your player character (if Player role)
        #[serde(default)]
        your_pc: Option<serde_json::Value>,
    },

    /// Failed to join a world
    WorldJoinFailed {
        /// World that was attempted
        world_id: Uuid,
        /// Reason for failure
        error: JoinError,
    },

    /// Another user joined the world
    UserJoined {
        /// User who joined
        user_id: String,
        /// User's display name
        #[serde(default)]
        username: Option<String>,
        /// User's role
        role: WorldRole,
        /// User's PC (if Player role)
        #[serde(default)]
        pc: Option<serde_json::Value>,
    },

    /// A user left the world
    UserLeft {
        /// User who left
        user_id: String,
    },

    /// Response to a Request message
    Response {
        /// Correlated request ID
        request_id: String,
        /// Result of the operation
        result: ResponseResult,
    },

    /// Entity changed broadcast (for cache invalidation)
    EntityChanged(EntityChangedData),

    /// Spectate target changed (for Spectator role)
    SpectateTargetChanged {
        /// New PC being spectated
        pc_id: Uuid,
        /// PC's name
        pc_name: String,
    },

    /// Unknown message type for forward compatibility
    ///
    /// When deserializing an unknown variant, this variant is used instead of
    /// failing. Allows older clients to gracefully handle new message types.
    #[serde(other)]
    Unknown,
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

/// NPC motivation data for directorial context
///
/// Note: `emotional_guidance` is a free-form string for DM guidance,
/// not the same as DispositionLevel or MoodState enums.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcMotivationData {
    pub character_id: String,
    /// Free-form emotional guidance for the NPC (e.g., "Conflicted about revealing secrets")
    /// UI label: "Demeanor"
    pub emotional_guidance: String,
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
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}

impl CharacterPosition {
    /// Get Tailwind CSS classes for positioning
    pub fn as_tailwind_classes(&self) -> &'static str {
        match self {
            CharacterPosition::Left => "left-[10%]",
            CharacterPosition::Center => "left-1/2 -translate-x-1/2",
            CharacterPosition::Right => "right-[10%]",
            CharacterPosition::OffScreen | CharacterPosition::Unknown => "hidden",
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
    /// Location's top-down map image for mini-map display
    #[serde(default)]
    pub map_asset: Option<String>,
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

/// Item data for region display (items visible in the current region)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionItemData {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub item_type: Option<String>,
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
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
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
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
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

// =============================================================================
// Staging Types (NPC Presence Approval)
// =============================================================================

/// Info about a staged NPC (for approval UI)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StagedNpcInfo {
    pub character_id: String,
    pub name: String,
    #[serde(default)]
    pub sprite_asset: Option<String>,
    #[serde(default)]
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub reasoning: String,
    /// When true, NPC is present but hidden from players
    #[serde(default)]
    pub is_hidden_from_players: bool,
}

/// Info about previous staging (for reference)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviousStagingInfo {
    pub staging_id: String,
    pub approved_at: String,
    pub npcs: Vec<StagedNpcInfo>,
}

/// Info about a PC waiting for staging
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaitingPcInfo {
    pub pc_id: String,
    pub pc_name: String,
    pub player_id: String,
}

/// NPC presence info for players (simplified, no reasoning)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcPresentInfo {
    pub character_id: String,
    pub name: String,
    #[serde(default)]
    pub sprite_asset: Option<String>,
    #[serde(default)]
    pub portrait_asset: Option<String>,
    /// When true, NPC is present but hidden from players
    #[serde(default)]
    pub is_hidden_from_players: bool,
}

/// DM's decision for an NPC in staging
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

// =============================================================================
// NPC Disposition Types (P1.4)
// =============================================================================

/// NPC disposition/relationship data for a specific PC
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcDispositionData {
    pub npc_id: String,
    pub npc_name: String,
    pub disposition: String,
    pub relationship: String,
    /// Sentiment value (-1.0 to 1.0)
    #[serde(default)]
    pub sentiment: f32,
    /// Last reason for disposition change
    #[serde(default)]
    pub last_reason: Option<String>,
}

// =============================================================================
// Actantial Model Types (P1.5)
// =============================================================================

/// Want visibility level - how much the player knows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WantVisibilityData {
    /// Player knows this motivation openly
    Known,
    /// Player suspects something but doesn't know details
    Suspected,
    /// Player has no idea (default)
    #[default]
    Hidden,
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}

/// Type discriminator for actors (NPC vs PC)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorTypeData {
    Npc,
    Pc,
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}

/// Actantial role type for character views
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActantialRoleData {
    Helper,
    Opponent,
    Sender,
    Receiver,
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}

/// Target type for wants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WantTargetTypeData {
    Character,
    Item,
    Goal,
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}

/// Want data for wire transfer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WantData {
    pub id: String,
    pub description: String,
    pub intensity: f32,
    pub priority: u32,
    pub visibility: WantVisibilityData,
    #[serde(default)]
    pub target: Option<WantTargetData>,
    #[serde(default)]
    pub deflection_behavior: Option<String>,
    #[serde(default)]
    pub tells: Vec<String>,
    /// Actantial actors for this want
    #[serde(default)]
    pub helpers: Vec<ActantialActorData>,
    #[serde(default)]
    pub opponents: Vec<ActantialActorData>,
    #[serde(default)]
    pub sender: Option<ActantialActorData>,
    #[serde(default)]
    pub receiver: Option<ActantialActorData>,
}

/// Want target data (resolved target info)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WantTargetData {
    pub id: String,
    pub name: String,
    pub target_type: WantTargetTypeData,
    /// Description for Goal targets
    #[serde(default)]
    pub description: Option<String>,
}

/// Data for creating a new want
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateWantData {
    pub description: String,
    #[serde(default = "default_intensity")]
    pub intensity: f32,
    #[serde(default = "default_priority")]
    pub priority: u32,
    #[serde(default)]
    pub visibility: WantVisibilityData,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub target_type: Option<WantTargetTypeData>,
    #[serde(default)]
    pub deflection_behavior: Option<String>,
    #[serde(default)]
    pub tells: Vec<String>,
}

fn default_intensity() -> f32 {
    0.5
}

fn default_priority() -> u32 {
    1
}

/// Data for updating an existing want
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateWantData {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub intensity: Option<f32>,
    #[serde(default)]
    pub priority: Option<u32>,
    #[serde(default)]
    pub visibility: Option<WantVisibilityData>,
    #[serde(default)]
    pub deflection_behavior: Option<String>,
    #[serde(default)]
    pub tells: Option<Vec<String>>,
}

/// Actantial actor data (helper, opponent, sender, receiver)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActantialActorData {
    pub id: String,
    pub name: String,
    pub actor_type: ActorTypeData,
    pub reason: String,
}

/// Actantial view data (for adding/removing views)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActantialViewData {
    pub want_id: String,
    pub target_id: String,
    pub target_name: String,
    pub target_type: ActorTypeData,
    pub role: ActantialRoleData,
    pub reason: String,
}

/// Goal data for wire transfer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalData {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Number of wants targeting this goal (for UI display)
    #[serde(default)]
    pub usage_count: u32,
}

/// Data for creating a new goal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateGoalData {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Data for updating an existing goal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateGoalData {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Full NPC actantial context data (response to GetNpcActantialContext)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcActantialContextData {
    pub npc_id: String,
    pub npc_name: String,
    pub wants: Vec<WantData>,
    pub social_views: SocialViewsData,
}

/// Social views summary data
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SocialViewsData {
    pub allies: Vec<SocialRelationData>,
    pub enemies: Vec<SocialRelationData>,
}

/// Social relation data (ally or enemy)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocialRelationData {
    pub id: String,
    pub name: String,
    pub actor_type: ActorTypeData,
    pub reasons: Vec<String>,
}
