//! Domain types for staging use cases.

use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId, StagingId};

/// Domain type for a staged NPC (for approval UI).
#[derive(Debug, Clone)]
pub struct StagedNpc {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub reasoning: String,
    pub is_hidden_from_players: bool,
    pub mood: Option<String>,
}

/// Domain type for approved NPC info.
#[derive(Debug, Clone)]
pub struct ApprovedNpc {
    pub character_id: CharacterId,
    pub is_present: bool,
    pub reasoning: Option<String>,
    pub is_hidden_from_players: bool,
    pub mood: Option<String>,
}

/// Domain type for a waiting PC.
#[derive(Debug, Clone)]
pub struct WaitingPc {
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub player_id: String,
}

/// Domain type for NPC presence info (for players).
#[derive(Debug, Clone)]
pub struct NpcPresent {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_hidden_from_players: bool,
    pub mood: Option<String>,
}

/// Domain type for previous staging info.
#[derive(Debug, Clone)]
pub struct PreviousStagingData {
    pub staging_id: StagingId,
    pub approved_at: chrono::DateTime<chrono::Utc>,
    pub npcs: Vec<StagedNpc>,
}

/// Domain type for game time.
#[derive(Debug, Clone)]
pub struct GameTimeData {
    pub day: u32,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub is_paused: bool,
}

// =============================================================================
// Visual State Domain Types (for staging)
// =============================================================================

/// Domain type for resolved state info (lightweight visual state data).
#[derive(Debug, Clone)]
pub struct ResolvedStateInfo {
    pub id: String,
    pub name: String,
    pub backdrop_override: Option<String>,
    pub atmosphere_override: Option<String>,
    pub ambient_sound: Option<String>,
}

/// Domain type for resolved visual state (location + region states).
#[derive(Debug, Clone, Default)]
pub struct ResolvedVisualState {
    pub location_state: Option<ResolvedStateInfo>,
    pub region_state: Option<ResolvedStateInfo>,
}

/// Domain type for state option (for DM selection dropdown).
#[derive(Debug, Clone)]
pub struct StateOption {
    pub id: String,
    pub name: String,
    pub priority: i32,
    pub is_default: bool,
    pub match_reason: Option<String>,
}

// =============================================================================
// Staging Approval Data
// =============================================================================

/// Data for DM staging approval notification.
#[derive(Debug, Clone)]
pub struct StagingApprovalData {
    pub request_id: String,
    pub region_id: RegionId,
    pub region_name: String,
    pub location_id: LocationId,
    pub location_name: String,
    pub game_time: GameTimeData,
    pub previous_staging: Option<PreviousStagingData>,
    pub rule_based_npcs: Vec<StagedNpc>,
    pub llm_based_npcs: Vec<StagedNpc>,
    pub default_ttl_hours: i32,
    pub waiting_pcs: Vec<WaitingPc>,
    pub resolved_visual_state: Option<ResolvedVisualState>,
    pub available_location_states: Vec<StateOption>,
    pub available_region_states: Vec<StateOption>,
}

/// Result of staging request use case.
#[derive(Debug, Clone)]
pub struct StagingRequestResult {
    /// Data for the player response (StagingPending).
    pub pending: StagingPendingData,
    /// Data for DM notification (StagingApprovalRequired).
    pub approval: StagingApprovalData,
    /// Optional time suggestion for DM notification.
    pub time_suggestion: Option<crate::infrastructure::ports::TimeSuggestion>,
}

/// Data for player staging pending response.
#[derive(Debug, Clone)]
pub struct StagingPendingData {
    pub region_id: RegionId,
    pub region_name: String,
    pub timeout_seconds: u64,
}
