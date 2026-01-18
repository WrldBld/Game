//! Domain types for staging use cases.

use uuid::Uuid;
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId};

use super::StagingError;

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

impl StagedNpc {
    /// Convert to protocol type for wire transmission.
    pub fn to_protocol(&self) -> wrldbldr_shared::StagedNpcInfo {
        wrldbldr_shared::StagedNpcInfo {
            character_id: self.character_id.to_string(),
            name: self.name.clone(),
            sprite_asset: self.sprite_asset.clone(),
            portrait_asset: self.portrait_asset.clone(),
            is_present: self.is_present,
            reasoning: self.reasoning.clone(),
            is_hidden_from_players: self.is_hidden_from_players,
            mood: self.mood.clone(),
        }
    }
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

impl ApprovedNpc {
    /// Convert from protocol type.
    pub fn from_protocol(info: &wrldbldr_shared::ApprovedNpcInfo) -> Result<Self, StagingError> {
        let character_id = info
            .character_id
            .parse::<uuid::Uuid>()
            .map(CharacterId::from)
            .map_err(|_| StagingError::Validation("Invalid character ID".to_string()))?;
        Ok(Self {
            character_id,
            is_present: info.is_present,
            reasoning: info.reasoning.clone(),
            is_hidden_from_players: info.is_hidden_from_players,
            mood: info.mood.clone(),
        })
    }
}

/// Domain type for a waiting PC.
#[derive(Debug, Clone)]
pub struct WaitingPc {
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub player_id: String,
}

impl WaitingPc {
    /// Convert to protocol type for wire transmission.
    pub fn to_protocol(&self) -> wrldbldr_shared::WaitingPcInfo {
        wrldbldr_shared::WaitingPcInfo {
            pc_id: self.pc_id.to_string(),
            pc_name: self.pc_name.clone(),
            player_id: self.player_id.clone(),
        }
    }
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

impl NpcPresent {
    /// Convert to protocol type for wire transmission.
    pub fn to_protocol(&self) -> wrldbldr_shared::NpcPresentInfo {
        wrldbldr_shared::NpcPresentInfo {
            character_id: self.character_id.to_string(),
            name: self.name.clone(),
            sprite_asset: self.sprite_asset.clone(),
            portrait_asset: self.portrait_asset.clone(),
            is_hidden_from_players: self.is_hidden_from_players,
            mood: self.mood.clone(),
        }
    }
}

/// Domain type for previous staging info.
#[derive(Debug, Clone)]
pub struct PreviousStagingData {
    pub staging_id: Uuid,
    pub approved_at: chrono::DateTime<chrono::Utc>,
    pub npcs: Vec<StagedNpc>,
}

impl PreviousStagingData {
    /// Convert to protocol type for wire transmission.
    pub fn to_protocol(&self) -> wrldbldr_shared::PreviousStagingInfo {
        wrldbldr_shared::PreviousStagingInfo {
            staging_id: self.staging_id.to_string(),
            approved_at: self.approved_at.to_rfc3339(),
            npcs: self.npcs.iter().map(|n| n.to_protocol()).collect(),
        }
    }
}

/// Domain type for game time.
#[derive(Debug, Clone)]
pub struct GameTimeData {
    pub day: u32,
    pub hour: u8,
    pub minute: u8,
    pub is_paused: bool,
}

impl GameTimeData {
    /// Convert to protocol type for wire transmission.
    pub fn to_protocol(&self) -> wrldbldr_shared::types::GameTime {
        let period = match self.hour {
            5..=11 => "Morning",
            12..=17 => "Afternoon",
            18..=21 => "Evening",
            _ => "Night",
        }
        .to_string();

        // Format time as 12-hour display
        let am_pm = if self.hour >= 12 { "PM" } else { "AM" };
        let display_hour = if self.hour == 0 {
            12
        } else if self.hour > 12 {
            self.hour - 12
        } else {
            self.hour
        };
        let formatted_time = Some(format!("{}:{:02} {}", display_hour, self.minute, am_pm));

        // Calculate total_minutes from day/hour/minute
        let total_minutes =
            (self.day as i64 - 1) * 24 * 60 + (self.hour as i64) * 60 + (self.minute as i64);

        wrldbldr_shared::types::GameTime {
            total_minutes,
            day: self.day,
            hour: self.hour,
            minute: self.minute,
            is_paused: self.is_paused,
            formatted_date: None, // Calendar formatting handled elsewhere
            formatted_time,
            period,
        }
    }
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

impl ResolvedStateInfo {
    /// Convert to protocol type for wire transmission.
    pub fn to_protocol(&self) -> wrldbldr_shared::types::ResolvedStateInfoData {
        wrldbldr_shared::types::ResolvedStateInfoData {
            id: self.id.clone(),
            name: self.name.clone(),
            backdrop_override: self.backdrop_override.clone(),
            atmosphere_override: self.atmosphere_override.clone(),
            ambient_sound: self.ambient_sound.clone(),
        }
    }
}

/// Domain type for resolved visual state (location + region states).
#[derive(Debug, Clone, Default)]
pub struct ResolvedVisualState {
    pub location_state: Option<ResolvedStateInfo>,
    pub region_state: Option<ResolvedStateInfo>,
}

impl ResolvedVisualState {
    /// Convert to protocol type for wire transmission.
    pub fn to_protocol(&self) -> wrldbldr_shared::types::ResolvedVisualStateData {
        wrldbldr_shared::types::ResolvedVisualStateData {
            location_state: self.location_state.as_ref().map(|s| s.to_protocol()),
            region_state: self.region_state.as_ref().map(|s| s.to_protocol()),
        }
    }
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

impl StateOption {
    /// Convert to protocol type for wire transmission.
    pub fn to_protocol(&self) -> wrldbldr_shared::types::StateOptionData {
        wrldbldr_shared::types::StateOptionData {
            id: self.id.clone(),
            name: self.name.clone(),
            priority: self.priority,
            is_default: self.is_default,
            match_reason: self.match_reason.clone(),
        }
    }
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
