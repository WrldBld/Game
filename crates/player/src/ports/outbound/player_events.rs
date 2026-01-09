//! Player events - outbound port data types for server messages
//!
//! These types represent the application's view of server messages.
//! They are part of the outbound port contract - the interface between
//! the adapters layer (which translates ServerMessage) and the application layer.
//!
//! # Hexagonal Architecture
//!
//! This module is in player-ports/outbound because:
//! 1. PlayerEvent defines the data contract for events the app RECEIVES from infrastructure
//! 2. The server connection is an outbound dependency (app NEEDS it, doesn't OFFER it)
//! 3. Adapters (message_translator.rs) produce PlayerEvents by implementing outbound ports
//! 4. Application and UI consume PlayerEvents via dependency injection
//!
//! # Design Rationale
//!
//! This enum groups the ~65 ServerMessage variants into logical categories,
//! providing a cleaner API for the application layer. The Raw variant acts
//! as a catch-all for messages that don't need specific handling.
//!
//! # Type Consolidation (Phase 3 Remediation)
//!
//! Types are consolidated with protocol crate to eliminate duplication:
//!
//! ## Re-exported from protocol (exact field matches):
//! SceneData, CharacterData, CharacterPosition, GameTime, InteractionData,
//! DialogueChoice, RegionData, NpcPresenceData, NavigationData, NavigationTarget,
//! NavigationExit, RegionItemData, SplitPartyLocation, OutcomeDetailData,
//! OutcomeBranchData, StagedNpcInfo, PreviousStagingInfo, WaitingPcInfo,
//! NpcPresentInfo, NpcDispositionData, GoalData
//!
//! ## Intentionally different from protocol (String vs typed enums for UI binding):
//! - WorldRole: player-ports uses String wrapper; protocol uses typed enum
//! - JoinError: player-ports uses simple struct; protocol uses rich enum
//! - ResponseResult: player-ports uses flat struct; protocol uses tagged enum
//! - ConnectedUser: Different role field type (String vs WorldRole enum)
//! - WantData: String fields for visibility; protocol uses typed enums
//! - WantTargetData: String fields for target_type; protocol uses typed enums
//! - ActantialViewData: String fields for target_type/role; protocol uses typed enums
//! - EntityChangedData: String fields for entity_type/change_type; protocol uses typed enums
//! - PlayerEvent: Main event enum - must stay in player-ports (defines app contract)

use serde_json;
use uuid::Uuid;

// =============================================================================
// Re-exports from protocol (single source of truth)
// =============================================================================

// Wire-format types with exact field matches - no translation needed
pub use wrldbldr_protocol::{
    // Suggestion types (already re-exported, kept for backward compatibility)
    ChallengeSuggestionInfo,
    ChallengeSuggestionOutcomes,
    // Scene types
    CharacterData,
    CharacterPosition,
    DialogueChoice,
    // Time types
    GameTime,
    // Goal types
    GoalData,
    InteractionData,
    NarrativeEventSuggestionInfo,
    // Navigation types
    NavigationData,
    NavigationExit,
    NavigationTarget,
    // Disposition types
    NpcDispositionData,
    NpcPresenceData,
    // Staging types
    NpcPresentInfo,
    // Challenge/Outcome types
    OutcomeBranchData,
    OutcomeDetailData,
    PreviousStagingInfo,
    ProposedToolInfo,
    RegionData,
    RegionItemData,
    // Visual State types
    ResolvedVisualStateData,
    SceneData,
    // Split party
    SplitPartyLocation,
    StagedNpcInfo,
    StateOptionData,
    WaitingPcInfo,
};

// =============================================================================
// Types intentionally different from protocol
// =============================================================================
// These types use String representations instead of typed enums for UI binding
// simplicity. The message_translator.rs converts protocol enums to these strings.

/// Want data for actantial model
///
/// NOTE: Uses String for visibility field instead of protocol's WantVisibilityData enum.
/// This simplifies UI binding in Dioxus components.
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
///
/// NOTE: Uses String for target_type instead of protocol's WantTargetTypeData enum.
#[derive(Debug, Clone, PartialEq)]
pub struct WantTargetData {
    pub id: String,
    pub name: String,
    pub target_type: String,
    pub description: Option<String>,
}

/// Actantial view data
///
/// NOTE: Uses String for target_type and role instead of protocol's typed enums.
#[derive(Debug, Clone, PartialEq)]
pub struct ActantialViewData {
    pub want_id: String,
    pub target_id: String,
    pub target_name: String,
    pub target_type: String,
    pub role: String,
    pub reason: String,
}

/// Connected user info
///
/// NOTE: Uses String for role instead of protocol's WorldRole enum.
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
///
/// NOTE: Wraps String instead of using protocol's WorldRole enum.
/// This allows UI to display the role directly without enum conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldRole(pub String);

/// Join error info
///
/// NOTE: Uses simple struct with code/message instead of protocol's rich enum.
/// This provides a uniform error handling interface for UI.
#[derive(Debug, Clone, PartialEq)]
pub struct JoinError {
    pub code: String,
    pub message: String,
}

/// Entity changed data for cache invalidation
///
/// NOTE: Uses String for entity_type and change_type instead of protocol's typed enums.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityChangedData {
    pub entity_type: String,
    pub entity_id: String,
    pub change_type: String,
    pub data: Option<serde_json::Value>,
    pub world_id: String,
}

/// Response result from a request
///
/// NOTE: Uses flat struct instead of protocol's tagged enum.
/// This provides a uniform interface for handling both success and error cases.
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
    WorldJoinFailed { world_id: Uuid, error: JoinError },

    /// Another user joined the world
    UserJoined {
        user_id: String,
        username: Option<String>,
        role: WorldRole,
        pc: Option<serde_json::Value>,
    },

    /// A user left the world
    UserLeft { user_id: String },

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
    MovementBlocked { pc_id: String, reason: String },

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
    LLMProcessing { action_id: String },

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
    /// Conversation has started (returned when initiating dialogue)
    ConversationStarted {
        conversation_id: String,
        npc_id: String,
        npc_name: String,
        npc_disposition: Option<String>,
    },

    /// NPC dialogue response
    DialogueResponse {
        speaker_id: String,
        speaker_name: String,
        text: String,
        choices: Vec<DialogueChoice>,
        conversation_id: Option<String>,
    },

    /// Conversation has ended
    ConversationEnded {
        npc_id: String,
        npc_name: String,
        pc_id: String,
        summary: Option<String>,
        conversation_id: Option<String>,
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
    ChallengeDiscarded { request_id: String },

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
        // Visual State Fields
        resolved_visual_state: Option<ResolvedVisualStateData>,
        available_location_states: Vec<StateOptionData>,
        available_region_states: Vec<StateOptionData>,
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
        /// Resolved visual state for scene display
        visual_state: Option<ResolvedVisualStateData>,
    },

    /// Staging was regenerated (DM only)
    StagingRegenerated {
        request_id: String,
        llm_based_npcs: Vec<StagedNpcInfo>,
    },

    // =========================================================================
    // Lore Events
    // =========================================================================
    /// Character discovered lore
    LoreDiscovered {
        character_id: String,
        lore: wrldbldr_protocol::types::LoreData,
        discovered_chunk_ids: Vec<String>,
        discovery_source: wrldbldr_protocol::types::LoreDiscoverySourceData,
    },

    /// Lore was revoked from a character
    LoreRevoked {
        character_id: String,
        lore_id: String,
    },

    /// Lore entry was updated (DM only)
    LoreUpdated {
        lore: wrldbldr_protocol::types::LoreData,
    },

    /// Response to GetCharacterLore request
    CharacterLoreResponse {
        character_id: String,
        known_lore: Vec<wrldbldr_protocol::types::LoreSummaryData>,
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
    InventoryUpdated { pc_id: String },

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

    /// NPC disposition changed (Tier 1 of emotional model)
    NpcDispositionChanged {
        npc_id: String,
        npc_name: String,
        pc_id: String,
        disposition: String,
        relationship: String,
        reason: Option<String>,
    },

    /// NPC mood changed (Tier 2 of emotional model)
    NpcMoodChanged {
        npc_id: String,
        npc_name: String,
        old_mood: String,
        new_mood: String,
        reason: Option<String>,
        region_id: Option<String>,
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
    NpcWantCreated { npc_id: String, want: WantData },

    /// NPC want was updated
    NpcWantUpdated { npc_id: String, want: WantData },

    /// NPC want was deleted
    NpcWantDeleted { npc_id: String, want_id: String },

    /// Want target was set
    WantTargetSet {
        want_id: String,
        target: WantTargetData,
    },

    /// Want target was removed
    WantTargetRemoved { want_id: String },

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
    GoalCreated { world_id: String, goal: GoalData },

    /// Goal was updated
    GoalUpdated { goal: GoalData },

    /// Goal was deleted
    GoalDeleted { goal_id: String },

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
    GenerationProgress { batch_id: String, progress: u8 },

    /// Generation completed
    GenerationComplete { batch_id: String, asset_count: u32 },

    /// Generation failed
    GenerationFailed { batch_id: String, error: String },

    /// Suggestion request was queued
    SuggestionQueued {
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
    },

    /// Suggestion request is being processed
    SuggestionProgress { request_id: String, status: String },

    /// Suggestion request completed
    SuggestionComplete {
        request_id: String,
        suggestions: Vec<String>,
    },

    /// Suggestion request failed
    SuggestionFailed { request_id: String, error: String },

    /// ComfyUI connection state changed
    ComfyUIStateChanged {
        state: String,
        message: Option<String>,
        retry_in_seconds: Option<u32>,
    },

    // =========================================================================
    // Time Events
    // =========================================================================
    /// Game time was updated (legacy)
    GameTimeUpdated { game_time: GameTime },

    /// Game time advanced with detailed info
    GameTimeAdvanced {
        previous_time: GameTime,
        new_time: GameTime,
        minutes_advanced: u32,
        reason: String,
        period_changed: bool,
        new_period: Option<String>,
    },

    /// Time suggestion for DM approval
    TimeSuggestion {
        suggestion_id: String,
        pc_id: String,
        pc_name: String,
        action_type: String,
        action_description: String,
        suggested_minutes: u32,
        current_time: GameTime,
        resulting_time: GameTime,
        period_change: Option<(String, String)>,
    },

    /// Time mode changed
    TimeModeChanged { world_id: String, mode: String },

    /// Game time paused/unpaused
    GameTimePaused { world_id: String, paused: bool },

    /// Time config updated
    TimeConfigUpdated {
        world_id: String,
        mode: String,
        show_time_to_players: bool,
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
    SpectateTargetChanged { pc_id: Uuid, pc_name: String },

    // =========================================================================
    // Error Events
    // =========================================================================
    /// Error message from server
    Error { code: String, message: String },

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
            Self::ConversationStarted { .. } => "ConversationStarted",
            Self::DialogueResponse { .. } => "DialogueResponse",
            Self::ConversationEnded { .. } => "ConversationEnded",
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
            Self::LoreDiscovered { .. } => "LoreDiscovered",
            Self::LoreRevoked { .. } => "LoreRevoked",
            Self::LoreUpdated { .. } => "LoreUpdated",
            Self::CharacterLoreResponse { .. } => "CharacterLoreResponse",
            Self::ItemEquipped { .. } => "ItemEquipped",
            Self::ItemUnequipped { .. } => "ItemUnequipped",
            Self::ItemDropped { .. } => "ItemDropped",
            Self::ItemPickedUp { .. } => "ItemPickedUp",
            Self::InventoryUpdated { .. } => "InventoryUpdated",
            Self::CharacterStatUpdated { .. } => "CharacterStatUpdated",
            Self::NpcDispositionChanged { .. } => "NpcDispositionChanged",
            Self::NpcMoodChanged { .. } => "NpcMoodChanged",
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
            Self::GameTimeAdvanced { .. } => "GameTimeAdvanced",
            Self::TimeSuggestion { .. } => "TimeSuggestion",
            Self::TimeModeChanged { .. } => "TimeModeChanged",
            Self::GameTimePaused { .. } => "GameTimePaused",
            Self::TimeConfigUpdated { .. } => "TimeConfigUpdated",
            Self::Response { .. } => "Response",
            Self::EntityChanged { .. } => "EntityChanged",
            Self::SpectateTargetChanged { .. } => "SpectateTargetChanged",
            Self::Error { .. } => "Error",
            Self::Raw { .. } => "Raw",
        }
    }
}
