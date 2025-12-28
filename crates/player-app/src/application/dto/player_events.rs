//! Player events - domain-friendly representation of server messages
//!
//! These events are the application layer's view of server messages.
//! The adapters layer translates ServerMessage into PlayerEvent.
//!
//! # Design Rationale
//!
//! This enum groups the ~65 ServerMessage variants into logical categories,
//! providing a cleaner API for the application layer. The Raw variant acts
//! as a catch-all for messages that don't need specific handling.

use uuid::Uuid;

// ============================================================================
// Supporting Types
// ============================================================================

/// Scene data for display
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterData {
    pub id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub position: CharacterPosition,
    pub is_speaking: bool,
    pub emotion: Option<String>,
}

/// Character position on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionData {
    pub id: String,
    pub name: String,
    pub interaction_type: String,
    pub target_name: Option<String>,
    pub is_available: bool,
}

/// Dialogue choice for player
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueChoice {
    pub id: String,
    pub text: String,
    pub is_custom_input: bool,
}

/// Region data for scene display
#[derive(Debug, Clone, PartialEq)]
pub struct RegionData {
    pub id: String,
    pub name: String,
    pub location_id: String,
    pub location_name: String,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    pub map_asset: Option<String>,
}

/// NPC presence data for scene display
#[derive(Debug, Clone, PartialEq)]
pub struct NpcPresenceData {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// Navigation options from current region
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationData {
    pub connected_regions: Vec<NavigationTarget>,
    pub exits: Vec<NavigationExit>,
}

/// A navigation target (region within same location)
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationTarget {
    pub region_id: String,
    pub name: String,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

/// An exit to another location
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationExit {
    pub location_id: String,
    pub location_name: String,
    pub arrival_region_id: String,
    pub description: Option<String>,
}

/// Item data for region display
#[derive(Debug, Clone, PartialEq)]
pub struct RegionItemData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub item_type: Option<String>,
}

/// Location info for split party notification
#[derive(Debug, Clone, PartialEq)]
pub struct SplitPartyLocation {
    pub location_id: String,
    pub location_name: String,
    pub pc_count: usize,
    pub pc_names: Vec<String>,
}

/// Proposed tool info for DM approval
#[derive(Debug, Clone, PartialEq)]
pub struct ProposedToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arguments: serde_json::Value,
}

/// Challenge suggestion info for DM approval
#[derive(Debug, Clone, PartialEq)]
pub struct ChallengeSuggestionInfo {
    pub challenge_id: String,
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty_display: String,
    pub confidence: String,
    pub reasoning: String,
    pub target_pc_id: Option<String>,
    /// Optional editable outcomes for DM modification
    pub outcomes: Option<ChallengeSuggestionOutcomes>,
}

/// Editable challenge outcomes for DM modification
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChallengeSuggestionOutcomes {
    pub success: Option<String>,
    pub failure: Option<String>,
    pub critical_success: Option<String>,
    pub critical_failure: Option<String>,
}

/// Narrative event suggestion info
#[derive(Debug, Clone, PartialEq)]
pub struct NarrativeEventSuggestionInfo {
    pub event_id: String,
    pub event_name: String,
    pub description: String,
    pub scene_direction: String,
    pub confidence: String,
    pub reasoning: String,
    pub matched_triggers: Vec<String>,
    pub suggested_outcome: Option<String>,
}

/// Outcome detail data
#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeDetailData {
    pub flavor_text: String,
    pub scene_direction: String,
    pub proposed_tools: Vec<ProposedToolInfo>,
}

/// Outcome branch data for DM selection
#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeBranchData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub effects: Vec<String>,
}

/// Staged NPC info for approval UI
#[derive(Debug, Clone, PartialEq)]
pub struct StagedNpcInfo {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub reasoning: String,
    pub is_hidden_from_players: bool,
}

/// Previous staging info for reference
#[derive(Debug, Clone, PartialEq)]
pub struct PreviousStagingInfo {
    pub staging_id: String,
    pub approved_at: String,
    pub npcs: Vec<StagedNpcInfo>,
}

/// PC waiting for staging info
#[derive(Debug, Clone, PartialEq)]
pub struct WaitingPcInfo {
    pub pc_id: String,
    pub pc_name: String,
    pub player_id: String,
}

/// NPC present info (simplified for players)
#[derive(Debug, Clone, PartialEq)]
pub struct NpcPresentInfo {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_hidden_from_players: bool,
}

/// Game time representation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameTime {
    /// Day number (ordinal-style, 1-based)
    pub day: u32,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Whether time is paused
    pub is_paused: bool,
}

/// NPC disposition data
#[derive(Debug, Clone, PartialEq)]
pub struct NpcDispositionData {
    pub npc_id: String,
    pub npc_name: String,
    pub disposition: String,
    pub relationship: String,
    pub sentiment: f32,
    pub last_reason: Option<String>,
}

/// Want data for actantial model
#[derive(Debug, Clone, PartialEq)]
pub struct WantData {
    pub id: String,
    pub description: String,
    pub intensity: f32,
    pub priority: u32,
    pub visibility: String,
    pub target: Option<WantTargetData>,
    pub deflection_behavior: Option<String>,
    pub tells: Vec<String>,
}

/// Want target data
#[derive(Debug, Clone, PartialEq)]
pub struct WantTargetData {
    pub id: String,
    pub name: String,
    pub target_type: String,
    pub description: Option<String>,
}

/// Actantial view data
#[derive(Debug, Clone, PartialEq)]
pub struct ActantialViewData {
    pub want_id: String,
    pub target_id: String,
    pub target_name: String,
    pub target_type: String,
    pub role: String,
    pub reason: String,
}

/// Goal data
#[derive(Debug, Clone, PartialEq)]
pub struct GoalData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub usage_count: u32,
}

/// Connected user info
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectedUser {
    pub user_id: String,
    pub username: Option<String>,
    pub role: String,
    /// Player character ID (for Player role)
    pub pc_id: Option<String>,
    /// Number of active connections (for DM with multiple screens)
    pub connection_count: u32,
}

/// World role (DM, Player, Spectator)
#[derive(Debug, Clone, PartialEq)]
pub struct WorldRole(pub String);

/// Join error info
#[derive(Debug, Clone, PartialEq)]
pub struct JoinError {
    pub code: String,
    pub message: String,
}

/// Entity changed data for cache invalidation
#[derive(Debug, Clone, PartialEq)]
pub struct EntityChangedData {
    pub entity_type: String,
    pub entity_id: String,
    pub change_type: String,
    pub data: Option<serde_json::Value>,
    pub world_id: String,
}

/// Response result from a request
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub error_details: Option<serde_json::Value>,
}

// ============================================================================
// PlayerEvent Enum
// ============================================================================

/// Events received from the game server
///
/// This enum groups the ~65 ServerMessage variants into logical categories.
/// The adapters layer translates ServerMessage into PlayerEvent.
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    // =========================================================================
    // Connection Events
    // =========================================================================

    /// Successfully joined a world
    WorldJoined {
        world_id: Uuid,
        snapshot: serde_json::Value,
        connected_users: Vec<ConnectedUser>,
        your_role: WorldRole,
        your_pc: Option<serde_json::Value>,
    },

    /// Failed to join a world
    WorldJoinFailed {
        world_id: Uuid,
        error: JoinError,
    },

    /// Another user joined the world
    UserJoined {
        user_id: String,
        username: Option<String>,
        role: WorldRole,
        pc: Option<serde_json::Value>,
    },

    /// A user left the world
    UserLeft {
        user_id: String,
    },

    /// Heartbeat response
    Pong,

    // =========================================================================
    // Scene & Navigation Events
    // =========================================================================

    /// Scene update with characters and interactions
    SceneUpdate {
        scene: SceneData,
        characters: Vec<CharacterData>,
        interactions: Vec<InteractionData>,
    },

    /// Scene changed due to PC movement
    SceneChanged {
        pc_id: String,
        region: RegionData,
        npcs_present: Vec<NpcPresenceData>,
        navigation: NavigationData,
        region_items: Vec<RegionItemData>,
    },

    /// PC was selected for play
    PcSelected {
        pc_id: String,
        pc_name: String,
        location_id: String,
        region_id: Option<String>,
    },

    /// Movement was blocked
    MovementBlocked {
        pc_id: String,
        reason: String,
    },

    /// Party is split across multiple locations (DM only)
    SplitPartyNotification {
        location_count: usize,
        locations: Vec<SplitPartyLocation>,
    },

    // =========================================================================
    // Action & Queue Events
    // =========================================================================

    /// Player action was received
    ActionReceived {
        action_id: String,
        player_id: String,
        action_type: String,
    },

    /// Action queued for processing
    ActionQueued {
        action_id: String,
        player_name: String,
        action_type: String,
        queue_depth: usize,
    },

    /// LLM is processing (DM only)
    LLMProcessing {
        action_id: String,
    },

    /// Queue status update (DM only)
    QueueStatus {
        player_actions_pending: usize,
        llm_requests_pending: usize,
        llm_requests_processing: usize,
        approvals_pending: usize,
    },

    // =========================================================================
    // Dialogue Events
    // =========================================================================

    /// NPC dialogue response
    DialogueResponse {
        speaker_id: String,
        speaker_name: String,
        text: String,
        choices: Vec<DialogueChoice>,
    },

    /// Response was approved and executed
    ResponseApproved {
        npc_dialogue: String,
        executed_tools: Vec<String>,
    },

    // =========================================================================
    // Approval Events (DM)
    // =========================================================================

    /// Approval required (DM only)
    ApprovalRequired {
        request_id: String,
        npc_name: String,
        proposed_dialogue: String,
        internal_reasoning: String,
        proposed_tools: Vec<ProposedToolInfo>,
        challenge_suggestion: Option<ChallengeSuggestionInfo>,
        narrative_event_suggestion: Option<NarrativeEventSuggestionInfo>,
    },

    // =========================================================================
    // Challenge Events
    // =========================================================================

    /// Challenge prompt sent to player
    ChallengePrompt {
        challenge_id: String,
        challenge_name: String,
        skill_name: String,
        difficulty_display: String,
        description: String,
        character_modifier: i32,
        suggested_dice: Option<String>,
        rule_system_hint: Option<String>,
    },

    /// Challenge result broadcast
    ChallengeResolved {
        challenge_id: String,
        challenge_name: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome: String,
        outcome_description: String,
        roll_breakdown: Option<String>,
        individual_rolls: Option<Vec<i32>>,
    },

    /// Challenge roll submitted, awaiting DM approval
    ChallengeRollSubmitted {
        challenge_id: String,
        challenge_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome_type: String,
        status: String,
    },

    /// Pending challenge outcome for DM approval
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
        roll_breakdown: Option<String>,
    },

    /// Outcome was regenerated (DM only)
    OutcomeRegenerated {
        request_id: String,
        outcome_type: String,
        new_outcome: OutcomeDetailData,
    },

    /// Challenge was discarded (DM only)
    ChallengeDiscarded {
        request_id: String,
    },

    /// Ad-hoc challenge created
    AdHocChallengeCreated {
        challenge_id: String,
        challenge_name: String,
        target_pc_id: String,
    },

    /// LLM-generated outcome suggestions ready (DM only)
    OutcomeSuggestionReady {
        resolution_id: String,
        suggestions: Vec<String>,
    },

    /// LLM-generated outcome branches ready (DM only)
    OutcomeBranchesReady {
        resolution_id: String,
        outcome_type: String,
        branches: Vec<OutcomeBranchData>,
    },

    // =========================================================================
    // Narrative Events
    // =========================================================================

    /// Narrative event triggered
    NarrativeEventTriggered {
        event_id: String,
        event_name: String,
        outcome_description: String,
        scene_direction: String,
    },

    /// An NPC is approaching the player
    ApproachEvent {
        npc_id: String,
        npc_name: String,
        npc_sprite: Option<String>,
        description: String,
        reveal: bool,
    },

    /// A location event occurred
    LocationEvent {
        region_id: String,
        description: String,
    },

    /// NPC location was shared with the player
    NpcLocationShared {
        npc_id: String,
        npc_name: String,
        region_name: String,
        notes: Option<String>,
    },

    // =========================================================================
    // Staging Events
    // =========================================================================

    /// Staging approval required (DM only)
    StagingApprovalRequired {
        request_id: String,
        region_id: String,
        region_name: String,
        location_id: String,
        location_name: String,
        game_time: GameTime,
        previous_staging: Option<PreviousStagingInfo>,
        rule_based_npcs: Vec<StagedNpcInfo>,
        llm_based_npcs: Vec<StagedNpcInfo>,
        default_ttl_hours: i32,
        waiting_pcs: Vec<WaitingPcInfo>,
    },

    /// Staging is pending approval (Player)
    StagingPending {
        region_id: String,
        region_name: String,
    },

    /// Staging is ready (Player)
    StagingReady {
        region_id: String,
        npcs_present: Vec<NpcPresentInfo>,
    },

    /// Staging was regenerated (DM only)
    StagingRegenerated {
        request_id: String,
        llm_based_npcs: Vec<StagedNpcInfo>,
    },

    // =========================================================================
    // Inventory Events
    // =========================================================================

    /// Item was equipped
    ItemEquipped {
        pc_id: String,
        item_id: String,
        item_name: String,
    },

    /// Item was unequipped
    ItemUnequipped {
        pc_id: String,
        item_id: String,
        item_name: String,
    },

    /// Item was dropped/destroyed
    ItemDropped {
        pc_id: String,
        item_id: String,
        item_name: String,
        quantity: u32,
    },

    /// Item was picked up
    ItemPickedUp {
        pc_id: String,
        item_id: String,
        item_name: String,
    },

    /// Inventory was updated (refresh signal)
    InventoryUpdated {
        pc_id: String,
    },

    // =========================================================================
    // Character Events
    // =========================================================================

    /// Character stat was updated
    CharacterStatUpdated {
        character_id: String,
        character_name: String,
        stat_name: String,
        old_value: i32,
        new_value: i32,
        delta: i32,
        source: String,
    },

    /// NPC disposition changed
    NpcDispositionChanged {
        npc_id: String,
        npc_name: String,
        pc_id: String,
        disposition: String,
        relationship: String,
        reason: Option<String>,
    },

    /// All NPC dispositions for a PC
    NpcDispositionsResponse {
        pc_id: String,
        dispositions: Vec<NpcDispositionData>,
    },

    // =========================================================================
    // Actantial Model Events
    // =========================================================================

    /// NPC want was created
    NpcWantCreated {
        npc_id: String,
        want: WantData,
    },

    /// NPC want was updated
    NpcWantUpdated {
        npc_id: String,
        want: WantData,
    },

    /// NPC want was deleted
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
        role: String,
    },

    /// Full NPC actantial context response
    NpcActantialContextResponse {
        npc_id: String,
        context: serde_json::Value, // Complex nested structure
    },

    /// All goals for a world
    WorldGoalsResponse {
        world_id: String,
        goals: Vec<GoalData>,
    },

    /// Goal was created
    GoalCreated {
        world_id: String,
        goal: GoalData,
    },

    /// Goal was updated
    GoalUpdated {
        goal: GoalData,
    },

    /// Goal was deleted
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
        role: String,
        suggestions: Vec<String>,
    },

    // =========================================================================
    // Generation Events
    // =========================================================================

    /// Generation batch was queued
    GenerationQueued {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        position: u32,
    },

    /// Generation progress update
    GenerationProgress {
        batch_id: String,
        progress: u8,
    },

    /// Generation completed
    GenerationComplete {
        batch_id: String,
        asset_count: u32,
    },

    /// Generation failed
    GenerationFailed {
        batch_id: String,
        error: String,
    },

    /// Suggestion request was queued
    SuggestionQueued {
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
    },

    /// Suggestion request is being processed
    SuggestionProgress {
        request_id: String,
        status: String,
    },

    /// Suggestion request completed
    SuggestionComplete {
        request_id: String,
        suggestions: Vec<String>,
    },

    /// Suggestion request failed
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

    // =========================================================================
    // Time Events
    // =========================================================================

    /// Game time was updated
    GameTimeUpdated {
        game_time: GameTime,
    },

    // =========================================================================
    // Request/Response Events
    // =========================================================================

    /// Response to a request message
    Response {
        request_id: String,
        result: ResponseResult,
    },

    /// Entity changed broadcast (cache invalidation)
    EntityChanged(EntityChangedData),

    /// Spectate target changed
    SpectateTargetChanged {
        pc_id: Uuid,
        pc_name: String,
    },

    // =========================================================================
    // Error Events
    // =========================================================================

    /// Error message from server
    Error {
        code: String,
        message: String,
    },

    // =========================================================================
    // Fallback
    // =========================================================================

    /// Raw/unhandled message (catch-all for messages not yet mapped)
    ///
    /// This is used for messages that don't need specific handling or
    /// for future-proofing against new message types.
    Raw {
        message_type: String,
        payload: serde_json::Value,
    },
}

impl PlayerEvent {
    /// Returns the event type name for logging/debugging
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::WorldJoined { .. } => "WorldJoined",
            Self::WorldJoinFailed { .. } => "WorldJoinFailed",
            Self::UserJoined { .. } => "UserJoined",
            Self::UserLeft { .. } => "UserLeft",
            Self::Pong => "Pong",
            Self::SceneUpdate { .. } => "SceneUpdate",
            Self::SceneChanged { .. } => "SceneChanged",
            Self::PcSelected { .. } => "PcSelected",
            Self::MovementBlocked { .. } => "MovementBlocked",
            Self::SplitPartyNotification { .. } => "SplitPartyNotification",
            Self::ActionReceived { .. } => "ActionReceived",
            Self::ActionQueued { .. } => "ActionQueued",
            Self::LLMProcessing { .. } => "LLMProcessing",
            Self::QueueStatus { .. } => "QueueStatus",
            Self::DialogueResponse { .. } => "DialogueResponse",
            Self::ResponseApproved { .. } => "ResponseApproved",
            Self::ApprovalRequired { .. } => "ApprovalRequired",
            Self::ChallengePrompt { .. } => "ChallengePrompt",
            Self::ChallengeResolved { .. } => "ChallengeResolved",
            Self::ChallengeRollSubmitted { .. } => "ChallengeRollSubmitted",
            Self::ChallengeOutcomePending { .. } => "ChallengeOutcomePending",
            Self::OutcomeRegenerated { .. } => "OutcomeRegenerated",
            Self::ChallengeDiscarded { .. } => "ChallengeDiscarded",
            Self::AdHocChallengeCreated { .. } => "AdHocChallengeCreated",
            Self::OutcomeSuggestionReady { .. } => "OutcomeSuggestionReady",
            Self::OutcomeBranchesReady { .. } => "OutcomeBranchesReady",
            Self::NarrativeEventTriggered { .. } => "NarrativeEventTriggered",
            Self::ApproachEvent { .. } => "ApproachEvent",
            Self::LocationEvent { .. } => "LocationEvent",
            Self::NpcLocationShared { .. } => "NpcLocationShared",
            Self::StagingApprovalRequired { .. } => "StagingApprovalRequired",
            Self::StagingPending { .. } => "StagingPending",
            Self::StagingReady { .. } => "StagingReady",
            Self::StagingRegenerated { .. } => "StagingRegenerated",
            Self::ItemEquipped { .. } => "ItemEquipped",
            Self::ItemUnequipped { .. } => "ItemUnequipped",
            Self::ItemDropped { .. } => "ItemDropped",
            Self::ItemPickedUp { .. } => "ItemPickedUp",
            Self::InventoryUpdated { .. } => "InventoryUpdated",
            Self::CharacterStatUpdated { .. } => "CharacterStatUpdated",
            Self::NpcDispositionChanged { .. } => "NpcDispositionChanged",
            Self::NpcDispositionsResponse { .. } => "NpcDispositionsResponse",
            Self::NpcWantCreated { .. } => "NpcWantCreated",
            Self::NpcWantUpdated { .. } => "NpcWantUpdated",
            Self::NpcWantDeleted { .. } => "NpcWantDeleted",
            Self::WantTargetSet { .. } => "WantTargetSet",
            Self::WantTargetRemoved { .. } => "WantTargetRemoved",
            Self::ActantialViewAdded { .. } => "ActantialViewAdded",
            Self::ActantialViewRemoved { .. } => "ActantialViewRemoved",
            Self::NpcActantialContextResponse { .. } => "NpcActantialContextResponse",
            Self::WorldGoalsResponse { .. } => "WorldGoalsResponse",
            Self::GoalCreated { .. } => "GoalCreated",
            Self::GoalUpdated { .. } => "GoalUpdated",
            Self::GoalDeleted { .. } => "GoalDeleted",
            Self::DeflectionSuggestions { .. } => "DeflectionSuggestions",
            Self::TellsSuggestions { .. } => "TellsSuggestions",
            Self::WantDescriptionSuggestions { .. } => "WantDescriptionSuggestions",
            Self::ActantialReasonSuggestions { .. } => "ActantialReasonSuggestions",
            Self::GenerationQueued { .. } => "GenerationQueued",
            Self::GenerationProgress { .. } => "GenerationProgress",
            Self::GenerationComplete { .. } => "GenerationComplete",
            Self::GenerationFailed { .. } => "GenerationFailed",
            Self::SuggestionQueued { .. } => "SuggestionQueued",
            Self::SuggestionProgress { .. } => "SuggestionProgress",
            Self::SuggestionComplete { .. } => "SuggestionComplete",
            Self::SuggestionFailed { .. } => "SuggestionFailed",
            Self::ComfyUIStateChanged { .. } => "ComfyUIStateChanged",
            Self::GameTimeUpdated { .. } => "GameTimeUpdated",
            Self::Response { .. } => "Response",
            Self::EntityChanged { .. } => "EntityChanged",
            Self::SpectateTargetChanged { .. } => "SpectateTargetChanged",
            Self::Error { .. } => "Error",
            Self::Raw { .. } => "Raw"
        }
    }
}
